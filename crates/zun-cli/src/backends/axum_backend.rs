use axum::http::{HeaderName, HeaderValue, Method};
use axum::response::IntoResponse;
use indexmap::IndexMap;
use serde_json::Value as JsonValue;
use std::collections::HashSet;
use std::net::TcpListener;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};
use zun::ast::TypeExpr;
use zun::diagnostics::Severity;
use zun::diagnostics::{Diagnostic, codes};
use zun::error::Error;
use zun::interpreter::CorsConfig;

use zun::interpreter::{HttpRoute, StaticRoute, ZunValue, type_expr_name};
use zun::runtime::{
    HandlerInput, RuntimeContext, backend::HttpBackend, execute_http_route_in_context,
};

pub struct AxumBackend {
    routes: Vec<HttpRoute>,
    static_routes: Vec<StaticRoute>,
}

impl AxumBackend {
    pub fn new() -> Self {
        Self {
            routes: vec![],
            static_routes: vec![],
        }
    }
}

/// Convert zun's brace-style path params (`/users/{id}`) to axum 0.7's
/// colon-style (`/users/:id`). Wildcard `{*name}` becomes `*name`.
/// Idempotent for paths that already use colon syntax.
fn zun_path_to_axum(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    let mut chars = path.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '{' {
            out.push(c);
            continue;
        }
        // collect until '}'
        let mut name = String::new();
        let mut closed = false;
        for nc in chars.by_ref() {
            if nc == '}' {
                closed = true;
                break;
            }
            name.push(nc);
        }
        if !closed {
            // malformed — emit as-is so the error is visible downstream
            out.push('{');
            out.push_str(&name);
            continue;
        }
        if let Some(rest) = name.strip_prefix('*') {
            out.push('*');
            out.push_str(rest);
        } else {
            out.push(':');
            out.push_str(&name);
        }
    }
    out
}

#[cfg(test)]
mod path_conversion_tests {
    use super::zun_path_to_axum;

    #[test]
    fn converts_single_param() {
        assert_eq!(zun_path_to_axum("/greet/{name}"), "/greet/:name");
    }

    #[test]
    fn converts_multiple_params() {
        assert_eq!(
            zun_path_to_axum("/users/{uid}/posts/{pid}"),
            "/users/:uid/posts/:pid"
        );
    }

    #[test]
    fn passes_through_literal_paths() {
        assert_eq!(zun_path_to_axum("/health"), "/health");
        assert_eq!(zun_path_to_axum("/"), "/");
    }

    #[test]
    fn converts_wildcard() {
        assert_eq!(zun_path_to_axum("/static/{*path}"), "/static/*path");
    }

    #[test]
    fn idempotent_on_already_converted() {
        assert_eq!(zun_path_to_axum("/greet/:name"), "/greet/:name");
    }

    #[test]
    fn tolerates_unclosed_brace() {
        // should not panic; emits verbatim so axum gives a useful error
        let out = zun_path_to_axum("/broken/{name");
        assert!(out.contains("{name"));
    }
}

impl HttpBackend for AxumBackend {
    fn register_route(&mut self, route: HttpRoute) {
        self.routes.push(route);
    }

    fn register_static(&mut self, route: StaticRoute) {
        self.static_routes.push(route);
    }

    async fn serve(self, context: RuntimeContext, host: &str, port: u16) -> Result<(), Error> {
        let addr = format!("{}:{}", host, port);
        let router = match build_router(self.routes, self.static_routes, context) {
            Ok(router) => router,
            Err(err) => {
                return Err(err);
            }
        };
        let listener = tokio::net::TcpListener::bind(&addr).await.map_err(|err| {
            Error::Interpreter(format!("failed to bind HTTP server on {addr}: {err}"))
        })?;
        serve_router(router, listener).await
    }
}

impl AxumBackend {
    pub async fn serve_with_std_listener(
        self,
        context: RuntimeContext,
        listener: TcpListener,
    ) -> Result<(), Error> {
        listener.set_nonblocking(true).map_err(|err| {
            Error::Interpreter(format!(
                "failed to configure HTTP listener as nonblocking: {err}"
            ))
        })?;
        let listener = tokio::net::TcpListener::from_std(listener).map_err(|err| {
            Error::Interpreter(format!(
                "failed to adopt HTTP listener into tokio runtime: {err}"
            ))
        })?;
        let router = build_router(self.routes, self.static_routes, context)?;
        serve_router(router, listener).await
    }
}

// ─── Response building ────────────────────────────────────────────────────────

fn build_http_response(
    result: Option<ZunValue>,
    return_type: &TypeExpr,
) -> axum::response::Response {
    let return_type_name = type_expr_name(return_type);
    match result {
        // $RES(status, body) — status code is now respected
        Some(ZunValue::Response { status, body }) => {
            let json_body = body.map(|b| value_to_json(&b)).unwrap_or(JsonValue::Null);
            let status_code = axum::http::StatusCode::from_u16(status)
                .unwrap_or(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
            (status_code, axum::response::Json(json_body)).into_response()
        }

        // $HTML() constructor
        Some(ZunValue::Html(html_val)) => {
            let html_content = match *html_val {
                ZunValue::Str(s) => s,
                v => v.to_string(),
            };
            axum::response::Html(html_content).into_response()
        }

        result => {
            // Explicit -> HTML return type
            if return_type_name == "HTML" {
                let html_content = match result {
                    Some(ZunValue::Str(s)) => s,
                    Some(v) => v.to_string(),
                    None => String::new(),
                };
                return axum::response::Html(html_content).into_response();
            }

            // JSON and everything else
            let json_value = match result {
                Some(ZunValue::Json(map)) | Some(ZunValue::Map(map)) => {
                    let mut obj = serde_json::Map::new();
                    for (k, v) in map {
                        obj.insert(k, value_to_json(&v));
                    }
                    JsonValue::Object(obj)
                }
                Some(ZunValue::Str(s)) => {
                    if return_type_name == "JSON" {
                        serde_json::from_str::<JsonValue>(&s)
                            .unwrap_or_else(|_| serde_json::json!({"value": s}))
                    } else {
                        serde_json::json!({"value": s})
                    }
                }
                Some(v) => value_to_json(&v),
                None => JsonValue::Object(serde_json::Map::new()),
            };

            axum::response::Json(json_value).into_response()
        }
    }
}

fn append_response_headers(
    mut response: axum::response::Response,
    headers: &[(String, String)],
) -> axum::response::Response {
    let response_headers = response.headers_mut();

    for (name, value) in headers {
        let name = match HeaderName::from_bytes(name.as_bytes()) {
            Ok(name) => name,
            Err(_) => continue,
        };
        let value = match HeaderValue::from_str(value) {
            Ok(value) => value,
            Err(_) => continue,
        };
        response_headers.insert(name, value);
    }

    response
}

fn append_auth_response_headers(
    mut response: axum::response::Response,
    context: &RuntimeContext,
) -> axum::response::Response {
    if let Ok(Some(cookie)) =
        context.with_request_auth_context(|auth| auth.pending_cookie().map(str::to_string))
        && let Ok(value) = HeaderValue::from_str(&cookie)
    {
        response
            .headers_mut()
            .append(axum::http::header::SET_COOKIE, value);
    }

    if let Ok(true) = context.with_request_auth_context(|auth| auth.pending_clear_cookie()) {
        if let Ok(value) =
            HeaderValue::from_str(&context.runtime_auth_config().clear_cookie_header_value())
        {
            response
                .headers_mut()
                .append(axum::http::header::SET_COOKIE, value);
        }
    }

    response
}

fn auth_error_body(status: u16, code: &str, message: &str) -> serde_json::Value {
    serde_json::json!({
        "status": status,
        "code": code,
        "message": message
    })
}

fn auth_error_message(err_msg: &str, marker: &str, fallback: &str) -> String {
    err_msg
        .split_once(marker)
        .map(|(_, rest)| rest)
        .and_then(|rest| rest.strip_prefix(':').or(Some(rest)))
        .map(str::trim)
        .filter(|message| !message.is_empty())
        .unwrap_or(fallback)
        .to_string()
}

async fn serve_router(
    router: axum::Router,
    listener: tokio::net::TcpListener,
) -> Result<(), Error> {
    cleanup_upload_temp_files(std::path::Path::new("/tmp/zun-uploads"), 24);
    axum::serve(listener, router)
        .await
        .map_err(|err| Error::Interpreter(format!("HTTP server error: {err}")))
}

fn build_router(
    routes: Vec<HttpRoute>,
    static_routes: Vec<StaticRoute>,
    context: RuntimeContext,
) -> Result<axum::Router, Error> {
    validate_runtime_context(&context)?;
    let global_cors = context.global_cors().cloned();
    let upload_max_total = context.server_upload_max_size();
    let upload_max_file = context.server_upload_max_file_size();
    let shared_context = Arc::new(context);
    let mut router = axum::Router::new();

    for route in routes {
        // zun source uses `/foo/{id}` (brace syntax); axum 0.7's router
        // expects `/foo/:id`. Convert before registering.
        let normalized = format!("/{}", route.path.trim_start_matches('/'));
        let path_str = zun_path_to_axum(&normalized);
        let method_str = route.method.clone();
        let handler_return_type = route.return_type.clone();
        let (resolved_cors, cors_warnings) = resolve_cors(
            route.cors.as_ref(),
            route.parent_cors.as_ref(),
            global_cors.as_ref(),
        )?;
        emit_diagnostics(&cors_warnings);
        let cors_layer = build_cors_layer(resolved_cors.as_ref());
        let (response_headers, header_warnings) =
            normalize_response_headers(&route.response_headers, &route.method, &path_str);
        emit_diagnostics(&header_warnings);
        let route_definition = route;
        let handler_context = Arc::clone(&shared_context);

        let handler = move |req: axum::extract::Request| {
            let route_definition = route_definition.clone();
            let handler_context = Arc::clone(&handler_context);
            async move {
                let mut request_context = handler_context.clone_for_request_execution();
                let uri = req.uri();
                let mut input = HandlerInput::new(uri.path());
                input.query = uri.query().map(str::to_string);

                for (name, value) in req.headers() {
                    if let Ok(v) = value.to_str() {
                        input
                            .headers
                            .insert(name.as_str().to_string(), v.to_string());
                    }
                }

                if matches!(route_definition.method.as_str(), "POST" | "PUT" | "PATCH") {
                    let content_type = input
                        .headers
                        .get("content-type")
                        .map(|s| s.as_str())
                        .unwrap_or("");

                    if content_type.starts_with("multipart/form-data") {
                        match parse_multipart(req, upload_max_total, upload_max_file).await {
                            Ok((file, files)) => {
                                input.file = file;
                                input.files = files;
                            }
                            Err(e) => {
                                return (
                                    axum::http::StatusCode::BAD_REQUEST,
                                    axum::response::Json(serde_json::json!({"error": e})),
                                )
                                    .into_response();
                            }
                        }
                    } else {
                        let body_bytes = axum::body::to_bytes(req.into_body(), 1024 * 1024)
                            .await
                            .unwrap_or_default();
                        if !body_bytes.is_empty()
                            && let Ok(json) = serde_json::from_slice::<JsonValue>(&body_bytes)
                        {
                            input.body = Some(json_to_zun_value(json));
                        }
                    }
                }

                let (_should_continue, result, error) =
                    execute_http_route_in_context(&route_definition, &input, &mut request_context);

                if let Some(err_msg) = error {
                    if err_msg == "__auth_unauthorized__"
                        || err_msg.contains("__auth_unauthorized__")
                    {
                        let message = auth_error_message(
                            &err_msg,
                            "__auth_unauthorized__",
                            "authentication required",
                        );
                        return (
                            axum::http::StatusCode::UNAUTHORIZED,
                            axum::response::Json(auth_error_body(
                                401,
                                "auth_unauthorized",
                                &message,
                            )),
                        )
                            .into_response();
                    }
                    if err_msg == "__auth_forbidden__" || err_msg.contains("__auth_forbidden__") {
                        let message =
                            auth_error_message(&err_msg, "__auth_forbidden__", "forbidden");
                        return (
                            axum::http::StatusCode::FORBIDDEN,
                            axum::response::Json(auth_error_body(403, "auth_forbidden", &message)),
                        )
                            .into_response();
                    }
                    if err_msg == "exit" {
                        return (
                            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                            axum::response::Json(serde_json::json!({"error": "server stopped"})),
                        )
                            .into_response();
                    }
                    eprintln!("\u{1b}[97;41m[ERROR]\u{1b}[0m {err_msg}");
                    return (
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        axum::response::Json(serde_json::json!({"error": err_msg})),
                    )
                        .into_response();
                }

                if let Err(err) = request_context.commit_pending_auth_side_effects() {
                    eprintln!("\u{1b}[97;41m[ERROR]\u{1b}[0m {err}");
                    return (
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        axum::response::Json(serde_json::json!({"error": err.to_string()})),
                    )
                        .into_response();
                }

                let return_type = handler_return_type
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|| TypeExpr::Named("JSON".to_string()));
                let response = build_http_response(result, &return_type);
                let response = append_response_headers(response, &response_headers);
                append_auth_response_headers(response, &request_context)
            }
        };

        use axum::routing::MethodFilter;
        let method = match method_str.as_str() {
            "GET" => MethodFilter::GET,
            "POST" => MethodFilter::POST,
            "PUT" => MethodFilter::PUT,
            "DELETE" => MethodFilter::DELETE,
            "PATCH" => MethodFilter::PATCH,
            _ => continue,
        };
        let method_router = axum::routing::MethodRouter::new().on(method, handler);
        let method_router = if let Some(cors_layer) = cors_layer {
            method_router.layer(cors_layer)
        } else {
            method_router
        };
        router = router.route(&path_str, method_router);
    }

    for static_route in static_routes {
        let dir_path = static_route.module_path.replace('.', "/");
        let static_dir = std::path::Path::new(&static_route.base_dir).join(&dir_path);
        use tower_http::services::fs::ServeDir;
        router = router.nest_service(&static_route.url_prefix, ServeDir::new(static_dir));
    }

    Ok(router)
}

pub fn validate_runtime_context(context: &RuntimeContext) -> Result<(), Error> {
    validate_cors_configs(context.routes(), context.global_cors())
}

fn validate_cors_configs(
    routes: &[HttpRoute],
    global_cors: Option<&CorsConfig>,
) -> Result<(), Error> {
    if let Some(cors) = global_cors {
        validate_cors_config(cors)?;
    }
    for route in routes {
        if let Some(cors) = route.cors.as_ref() {
            validate_cors_config(cors)?;
        }
    }
    Ok(())
}

fn validate_cors_config(cors: &CorsConfig) -> Result<(), Error> {
    let has_wildcard_origin = cors.allow_all || cors.origins.iter().any(|origin| origin == "*");
    if cors.credentials && has_wildcard_origin {
        return Err(Error::from(
            Diagnostic::error(codes::CONFIG_CORS_INVALID, "invalid CORS configuration")
                .with_note("credentials: true cannot be combined with origins: [\"*\"]")
                .with_note("browsers require explicit origins when credentials are enabled"),
        ));
    }
    if let Some(max_age) = cors.max_age
        && max_age < 0
    {
        return Err(Error::from(
            Diagnostic::error(codes::CONFIG_CORS_INVALID, "invalid CORS configuration")
                .with_note("max_age must be a non-negative integer"),
        ));
    }

    Ok(())
}

fn build_cors_layer(cors: Option<&CorsConfig>) -> Option<CorsLayer> {
    let cors = cors?;
    let mut layer = CorsLayer::new();

    layer = if cors.allow_all || cors.origins.iter().any(|origin| origin == "*") {
        layer.allow_origin(AllowOrigin::any())
    } else {
        let origins = cors
            .origins
            .iter()
            .filter_map(|origin| HeaderValue::from_str(origin).ok())
            .collect::<Vec<_>>();
        if origins.is_empty() {
            return None;
        }
        layer.allow_origin(AllowOrigin::list(origins))
    };

    if !cors.methods.is_empty() {
        let methods = cors
            .methods
            .iter()
            .filter_map(|method| Method::from_str(method).ok())
            .collect::<Vec<_>>();
        if !methods.is_empty() {
            layer = layer.allow_methods(AllowMethods::list(methods));
        }
    }

    if !cors.headers.is_empty() {
        let headers = cors
            .headers
            .iter()
            .filter_map(|header| HeaderName::from_str(header).ok())
            .collect::<Vec<_>>();
        if !headers.is_empty() {
            layer = layer.allow_headers(AllowHeaders::list(headers));
        }
    }

    if let Some(max_age) = cors.max_age {
        layer = layer.max_age(Duration::from_secs(max_age as u64));
    }

    if cors.credentials {
        layer = layer.allow_credentials(true);
    }

    Some(layer)
}

fn emit_diagnostics(diagnostics: &[Diagnostic]) {
    for diagnostic in diagnostics {
        match diagnostic.severity {
            Severity::Warning => eprintln!("{diagnostic}"),
            Severity::Error => eprintln!("{diagnostic}"),
        }
    }
}

fn normalize_response_headers(
    headers: &[(String, String)],
    method: &str,
    path: &str,
) -> (Vec<(String, String)>, Vec<Diagnostic>) {
    let mut warnings = Vec::new();
    let mut normalized = Vec::new();
    let mut seen = HashSet::new();

    for (name, value) in headers {
        let lowered = name.to_ascii_lowercase();
        if !seen.insert(lowered.clone()) {
            warnings.push(
                Diagnostic::warning(
                    codes::CONFIG_HDR_INVALID,
                    format!("duplicate response header overridden: \"{name}\""),
                )
                .with_note(format!("route: {method} {path}"))
                .with_note("later @SET_HDR declarations override earlier ones on the same node"),
            );
            normalized.retain(|(existing_name, _): &(String, String)| {
                existing_name.to_ascii_lowercase() != lowered
            });
        }

        match (
            HeaderName::from_bytes(name.as_bytes()),
            HeaderValue::from_str(value),
        ) {
            (Ok(_), Ok(_)) => normalized.push((name.clone(), value.clone())),
            (Err(_), _) => warnings.push(
                Diagnostic::warning(
                    codes::CONFIG_HDR_INVALID,
                    format!("invalid response header name skipped: \"{name}\""),
                )
                .with_note(format!("route: {method} {path}"))
                .with_note("header names must not contain spaces or control characters"),
            ),
            (_, Err(_)) => warnings.push(
                Diagnostic::warning(
                    codes::CONFIG_HDR_INVALID,
                    format!("invalid response header value skipped for: \"{name}\""),
                )
                .with_note(format!("route: {method} {path}")),
            ),
        }
    }

    (normalized, warnings)
}

fn resolve_cors(
    route_cors: Option<&CorsConfig>,
    parent_cors: Option<&CorsConfig>,
    global_cors: Option<&CorsConfig>,
) -> Result<(Option<CorsConfig>, Vec<Diagnostic>), Error> {
    let mut warnings = Vec::new();

    let levels = [
        ("route-level", route_cors),
        ("block-level", parent_cors),
        ("global-level", global_cors),
    ];

    for (index, (level_name, candidate)) in levels.into_iter().enumerate() {
        let Some(candidate) = candidate else {
            continue;
        };

        let (normalized, mut candidate_warnings) = normalize_cors_config(candidate, level_name)?;
        warnings.append(&mut candidate_warnings);
        if let Some(normalized) = normalized {
            return Ok((Some(normalized), warnings));
        }

        if let Some((fallback_name, _)) = levels
            .iter()
            .skip(index + 1)
            .find(|(_, candidate)| candidate.is_some())
        {
            warnings.push(
                Diagnostic::warning(
                    codes::CONFIG_CORS_INVALID,
                    format!(
                        "{level_name} @CORS config is empty after validation, falling back to {fallback_name} config"
                    ),
                ),
            );
        }
    }

    Ok((None, warnings))
}

fn normalize_cors_config(
    cors: &CorsConfig,
    level_name: &str,
) -> Result<(Option<CorsConfig>, Vec<Diagnostic>), Error> {
    validate_cors_config(cors)?;

    if cors.allow_all || cors.origins.iter().any(|origin| origin == "*") {
        let mut normalized = cors.clone();
        normalized.allow_all = true;
        normalized.origins.clear();
        return Ok((Some(normalized), Vec::new()));
    }

    let mut warnings = Vec::new();
    let mut origins = Vec::new();
    for origin in &cors.origins {
        if is_valid_origin(origin) {
            origins.push(origin.clone());
        } else {
            warnings.push(
                Diagnostic::warning(
                    codes::CONFIG_CORS_INVALID,
                    format!("invalid origin URL skipped: \"{origin}\""),
                )
                .with_note(format!("in {level_name} @CORS"))
                .with_note("only URLs with http:// or https:// scheme are allowed"),
            );
        }
    }

    let mut methods = Vec::new();
    for method in &cors.methods {
        if is_supported_method(method) {
            methods.push(method.clone());
        } else {
            warnings.push(
                Diagnostic::warning(
                    codes::CONFIG_CORS_INVALID,
                    format!("invalid HTTP method skipped: \"{method}\""),
                )
                .with_note(format!("in {level_name} @CORS")),
            );
        }
    }

    let mut headers = Vec::new();
    for header in &cors.headers {
        if HeaderName::from_str(header).is_ok() {
            headers.push(header.clone());
        } else {
            warnings.push(
                Diagnostic::warning(
                    codes::CONFIG_CORS_INVALID,
                    format!("invalid request header name skipped: \"{header}\""),
                )
                .with_note(format!("in {level_name} @CORS")),
            );
        }
    }

    if origins.is_empty() {
        warnings.push(
            Diagnostic::warning(
                codes::CONFIG_CORS_INVALID,
                "CORS origins list is empty after validation".to_string(),
            )
            .with_note(format!("in {level_name} @CORS")),
        );
        return Ok((None, warnings));
    }

    Ok((
        Some(CorsConfig {
            allow_all: false,
            origins,
            methods,
            headers,
            max_age: cors.max_age,
            credentials: cors.credentials,
        }),
        warnings,
    ))
}

fn is_valid_origin(origin: &str) -> bool {
    (origin.starts_with("http://") || origin.starts_with("https://"))
        && HeaderValue::from_str(origin).is_ok()
}

fn is_supported_method(method: &str) -> bool {
    matches!(
        method,
        "GET" | "POST" | "PUT" | "DELETE" | "PATCH" | "HEAD" | "OPTIONS"
    )
}

async fn parse_multipart(
    req: axum::extract::Request,
    max_total: u64,
    max_file: u64,
) -> Result<(Option<ZunValue>, Option<ZunValue>), String> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static UPLOAD_COUNTER: AtomicU64 = AtomicU64::new(0);

    let boundary = req
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .and_then(|ct| multer::parse_boundary(ct).ok())
        .ok_or_else(|| "missing or invalid multipart boundary".to_string())?;

    let stream = req.into_body().into_data_stream();
    let constraints = multer::Constraints::new().size_limit(
        multer::SizeLimit::new()
            .whole_stream(max_total)
            .per_field(max_file),
    );
    let mut multipart = multer::Multipart::with_constraints(stream, boundary, constraints);

    let tmp_dir = std::path::Path::new("/tmp/zun-uploads");
    tokio::fs::create_dir_all(tmp_dir)
        .await
        .map_err(|e| format!("failed to create upload temp dir: {e}"))?;

    let mut file_values: Vec<ZunValue> = Vec::new();

    loop {
        match multipart.next_field().await {
            Ok(Some(field)) => {
                let original_filename = match field.file_name() {
                    Some(name) if !name.is_empty() => name.to_string(),
                    _ => continue,
                };
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0);
                let seq = UPLOAD_COUNTER.fetch_add(1, Ordering::Relaxed);
                let tmp_name = format!("{ts}-{seq}-{original_filename}");
                let tmp_path = tmp_dir.join(&tmp_name);

                let data = field
                    .bytes()
                    .await
                    .map_err(|e| format!("failed to read upload field: {e}"))?;
                tokio::fs::write(&tmp_path, &data)
                    .await
                    .map_err(|e| format!("failed to write upload temp file: {e}"))?;

                file_values.push(ZunValue::File {
                    path: tmp_path.to_string_lossy().to_string(),
                    mode: None,
                    filename: Some(original_filename),
                });
            }
            Ok(None) => break,
            Err(e) => return Err(format!("multipart parse error: {e}")),
        }
    }

    if file_values.is_empty() {
        return Ok((None, None));
    }
    let file = Some(file_values[0].clone());
    let files = Some(ZunValue::List(file_values));
    Ok((file, files))
}

fn cleanup_upload_temp_files(tmp_dir: &std::path::Path, max_age_hours: u64) {
    if !tmp_dir.exists() {
        return;
    }
    let threshold = std::time::SystemTime::now()
        .checked_sub(std::time::Duration::from_secs(max_age_hours * 3600))
        .unwrap_or(std::time::UNIX_EPOCH);

    if let Ok(entries) = std::fs::read_dir(tmp_dir) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if let Ok(modified) = meta.modified() {
                    if modified < threshold {
                        let _ = std::fs::remove_file(entry.path());
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use zun::runtime::RuntimeMode;

    use super::*;

    fn cors(origins: &[&str], methods: &[&str], headers: &[&str]) -> CorsConfig {
        CorsConfig {
            allow_all: false,
            origins: origins.iter().map(|v| (*v).to_string()).collect(),
            methods: methods.iter().map(|v| (*v).to_string()).collect(),
            headers: headers.iter().map(|v| (*v).to_string()).collect(),
            max_age: None,
            credentials: false,
        }
    }

    #[test]
    fn normalize_response_headers_skips_invalid_names_and_warns() {
        let (headers, warnings) = normalize_response_headers(
            &[
                ("bad header".to_string(), "ignored".to_string()),
                ("X-Valid".to_string(), "visible".to_string()),
            ],
            "GET",
            "/health",
        );

        assert_eq!(
            headers,
            vec![("X-Valid".to_string(), "visible".to_string())]
        );
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].code, codes::CONFIG_HDR_INVALID);
        assert_eq!(warnings[0].severity, Severity::Warning);
    }

    #[test]
    fn normalize_response_headers_duplicate_names_warn_and_last_wins() {
        let (headers, warnings) = normalize_response_headers(
            &[
                ("X-Test".to_string(), "1".to_string()),
                ("x-test".to_string(), "2".to_string()),
            ],
            "GET",
            "/health",
        );

        assert_eq!(headers, vec![("x-test".to_string(), "2".to_string())]);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].code, codes::CONFIG_HDR_INVALID);
    }

    #[test]
    fn serve_returns_error_when_port_is_already_in_use() {
        let blocker =
            std::net::TcpListener::bind("127.0.0.1:0").expect("test should bind a free port");
        let port = blocker.local_addr().expect("local addr").port();
        let context = RuntimeContext::new(RuntimeMode::Serve, PathBuf::from("."));
        let backend = AxumBackend::new();
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");

        let error = rt
            .block_on(backend.serve(context, "127.0.0.1", port))
            .expect_err("occupied port should return an error");

        assert!(error.to_string().contains("failed to bind HTTP server"));
        assert!(error.to_string().contains("Address already in use"));
    }

    #[test]
    fn resolve_cors_filters_invalid_entries_and_falls_back() {
        let route = cors(&["not-a-url"], &["GET"], &[]);
        let parent = cors(&["https://parent.example.com"], &["GET"], &[]);

        let (resolved, warnings) = resolve_cors(Some(&route), Some(&parent), None).expect("cors");

        assert_eq!(
            resolved.expect("resolved").origins,
            vec!["https://parent.example.com".to_string()]
        );
        assert!(
            warnings
                .iter()
                .any(|warning| warning.code == codes::CONFIG_CORS_INVALID)
        );
    }

    #[test]
    fn resolve_cors_filters_invalid_methods_and_headers() {
        let config = cors(
            &["https://client.example.com"],
            &["GET", "NOPE"],
            &["Authorization", "bad header"],
        );

        let (resolved, warnings) = resolve_cors(Some(&config), None, None).expect("cors");
        let resolved = resolved.expect("resolved");
        assert_eq!(resolved.methods, vec!["GET".to_string()]);
        assert_eq!(resolved.headers, vec!["Authorization".to_string()]);
        assert_eq!(warnings.len(), 2);
    }

    // ─── cleanup_upload_temp_files ─────────────────────────────────────────────

    #[test]
    fn cleanup_is_noop_when_dir_missing() {
        let dir = std::env::temp_dir().join("zun-test-cleanup-missing-dir");
        // Ensure it does not exist
        let _ = std::fs::remove_dir_all(&dir);
        // Must not panic
        cleanup_upload_temp_files(&dir, 24);
    }

    #[test]
    fn cleanup_removes_files_older_than_threshold() {
        use std::fs::FileTimes;
        use std::time::{Duration, SystemTime};

        let dir = std::env::temp_dir().join("zun-test-cleanup-old");
        std::fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("old-upload.tmp");
        std::fs::write(&file_path, b"data").unwrap();

        // Set mtime to 48 hours ago
        let old_time = SystemTime::now()
            .checked_sub(Duration::from_secs(48 * 3600))
            .unwrap();
        let f = std::fs::File::options()
            .write(true)
            .open(&file_path)
            .unwrap();
        f.set_times(FileTimes::new().set_modified(old_time))
            .unwrap();
        drop(f);

        cleanup_upload_temp_files(&dir, 24);

        assert!(!file_path.exists(), "file older than 24h should be deleted");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cleanup_preserves_recent_files() {
        let dir = std::env::temp_dir().join("zun-test-cleanup-recent");
        std::fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("new-upload.tmp");
        std::fs::write(&file_path, b"data").unwrap();

        cleanup_upload_temp_files(&dir, 24);

        assert!(file_path.exists(), "recently created file should be kept");
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ─── parse_multipart ──────────────────────────────────────────────────────

    fn make_multipart_request(
        boundary: &str,
        filename: &str,
        content: &[u8],
    ) -> axum::extract::Request {
        let header = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\n\r\n"
        );
        let mut bytes = header.into_bytes();
        bytes.extend_from_slice(content);
        bytes.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

        axum::http::Request::builder()
            .header(
                "content-type",
                format!("multipart/form-data; boundary={boundary}"),
            )
            .body(axum::body::Body::from(bytes))
            .unwrap()
    }

    #[tokio::test]
    async fn parse_multipart_extracts_single_file() {
        let req = make_multipart_request("testboundary", "hello.txt", b"hello world");
        let (file, files) = parse_multipart(req, 10 * 1024 * 1024, 5 * 1024 * 1024)
            .await
            .expect("parse_multipart should succeed");

        let file = file.expect("file should be Some");
        match &file {
            ZunValue::File { filename, .. } => {
                assert_eq!(filename.as_deref(), Some("hello.txt"));
            }
            other => panic!("expected File, got {:?}", other),
        }

        let files = files.expect("files should be Some");
        match files {
            ZunValue::List(list) => assert_eq!(list.len(), 1),
            other => panic!("expected List, got {:?}", other),
        }

        // Clean up written temp file
        if let ZunValue::File { path, .. } = file {
            let _ = std::fs::remove_file(path);
        }
    }

    #[tokio::test]
    async fn parse_multipart_skips_non_file_fields() {
        let boundary = "skipboundary";
        // A text field without a filename
        let body = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"description\"\r\n\r\nsome text\r\n--{boundary}--\r\n"
        );
        let req = axum::http::Request::builder()
            .header(
                "content-type",
                format!("multipart/form-data; boundary={boundary}"),
            )
            .body(axum::body::Body::from(body.into_bytes()))
            .unwrap();

        let (file, files) = parse_multipart(req, 10 * 1024 * 1024, 5 * 1024 * 1024)
            .await
            .expect("parse_multipart should succeed");

        assert!(file.is_none(), "text-only field should produce no file");
        assert!(files.is_none(), "text-only field should produce no files");
    }
}

fn value_to_json(v: &ZunValue) -> JsonValue {
    match v {
        ZunValue::Null => JsonValue::Null,
        ZunValue::Bool(b) => JsonValue::Bool(*b),
        ZunValue::Int(i) => JsonValue::Number(serde_json::Number::from(*i)),
        ZunValue::Float(f) => serde_json::Number::from_f64(*f)
            .map(JsonValue::Number)
            .unwrap_or(JsonValue::Null),
        ZunValue::Str(s) => JsonValue::String(s.clone()),
        ZunValue::List(list) => JsonValue::Array(list.iter().map(value_to_json).collect()),
        ZunValue::Json(map) | ZunValue::Map(map) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in map {
                obj.insert(k.clone(), value_to_json(v));
            }
            JsonValue::Object(obj)
        }
        ZunValue::TypedInstance { fields, .. } => {
            let mut obj = serde_json::Map::new();
            for (k, v) in fields {
                if !k.starts_with('_') {
                    obj.insert(k.clone(), value_to_json(v));
                }
            }
            JsonValue::Object(obj)
        }
        _ => JsonValue::Null,
    }
}

fn json_to_zun_value(json: JsonValue) -> ZunValue {
    match json {
        JsonValue::Null => ZunValue::Null,
        JsonValue::Bool(b) => ZunValue::Bool(b),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                ZunValue::Int(i)
            } else if let Some(f) = n.as_f64() {
                ZunValue::Float(f)
            } else {
                ZunValue::Null
            }
        }
        JsonValue::String(s) => ZunValue::Str(s),
        JsonValue::Array(arr) => ZunValue::List(arr.into_iter().map(json_to_zun_value).collect()),
        JsonValue::Object(obj) => {
            let mut map = IndexMap::new();
            for (k, v) in obj {
                map.insert(k, json_to_zun_value(v));
            }
            ZunValue::Json(map)
        }
    }
}
