#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use reqwest::Method;
use reqwest::blocking::Client;
use zun::interpreter::{HttpRoute, ZunValue};
use zun::runtime::backend::HttpBackend;
use zun::runtime::{
    HandlerInput, ProgramState, RuntimeContext, RuntimeMode, execute_http_route,
    execute_program_with_writer, load_context_and_program,
};
use zun_cli::backends::axum_backend::AxumBackend;

pub struct ScriptOutcome {
    pub state: ProgramState,
    pub context: RuntimeContext,
    pub stdout: String,
    pub error: Option<String>,
}

pub struct RouteOutcome {
    pub route_count: usize,
    pub should_continue: bool,
    pub result: Option<ZunValue>,
    pub error: Option<String>,
}

pub struct LiveHttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

pub fn fixture_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join(relative)
}

pub fn read_text(relative: &str) -> String {
    fs::read_to_string(fixture_path(relative)).expect("fixture should exist")
}

pub fn run_fixture(relative: &str, mode: RuntimeMode) -> ScriptOutcome {
    run_program_at_path(&fixture_path(relative), mode)
}

pub fn run_program_at_path(path: &Path, mode: RuntimeMode) -> ScriptOutcome {
    let (mut context, program) = match load_context_and_program(mode.clone(), path) {
        Ok(loaded) => loaded,
        Err(err) => {
            let project_root = if path.is_dir() {
                path.to_path_buf()
            } else {
                path.parent().unwrap_or(Path::new(".")).to_path_buf()
            };
            return ScriptOutcome {
                state: ProgramState::new(),
                context: RuntimeContext::new(mode, project_root),
                stdout: String::new(),
                error: Some(err.to_string()),
            };
        }
    };
    let mut state = ProgramState::new();
    let mut output = Vec::new();

    let error =
        execute_program_with_writer(&program.statements, &mut state, &mut context, &mut output)
            .err()
            .map(|err| err.to_string());

    ScriptOutcome {
        state,
        context,
        stdout: String::from_utf8(output).expect("stdout should be utf-8"),
        error,
    }
}

pub fn run_route_fixture(
    relative: &str,
    method: &str,
    path: &str,
    body: Option<ZunValue>,
) -> RouteOutcome {
    let outcome = run_fixture(relative, RuntimeMode::Test);
    assert!(
        outcome.error.is_none(),
        "route fixture bootstrap should succeed"
    );

    let routes = outcome.context.routes().to_vec();
    let route = routes
        .iter()
        .find(|route| route.method == method && route.path == path)
        .unwrap_or_else(|| panic!("route {method} {path} should exist"));

    let mut input = HandlerInput::new(path);
    input.body = body;

    let (should_continue, result, error) = execute_http_route(route, &input, &outcome.context);

    RouteOutcome {
        route_count: routes.len(),
        should_continue,
        result,
        error,
    }
}

pub fn start_live_http_server(relative: &str) -> String {
    let outcome = run_fixture(relative, RuntimeMode::Serve);
    assert!(outcome.error.is_none(), "live http fixture should boot");
    start_live_http_server_from_context(outcome.context)
}

pub fn start_live_http_server_from_context(context: RuntimeContext) -> String {
    try_start_live_http_server_from_context(context).expect("live http server should boot")
}

pub fn try_start_live_http_server_from_context(context: RuntimeContext) -> Result<String, String> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("free port should exist");
    let port = listener.local_addr().expect("listener addr").port();
    let (startup_tx, startup_rx) = mpsc::channel();

    thread::spawn(move || {
        let mut backend = AxumBackend::new();
        for route in context.routes().iter().cloned() {
            backend.register_route(route);
        }
        for static_route in context.static_routes().iter().cloned() {
            backend.register_static(static_route);
        }

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(backend.serve_with_std_listener(context, listener));
        let _ = startup_tx.send(result.map_err(|err| err.to_string()));
    });

    match startup_rx.recv_timeout(Duration::from_millis(150)) {
        Ok(Err(err)) => Err(err),
        Ok(Ok(())) | Err(mpsc::RecvTimeoutError::Timeout) => {
            wait_for_server(port);
            Ok(format!("http://127.0.0.1:{port}"))
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            wait_for_server(port);
            Ok(format!("http://127.0.0.1:{port}"))
        }
    }
}

pub fn http_get(base_url: &str, path: &str) -> LiveHttpResponse {
    http_request("GET", base_url, path)
}

pub fn http_request(method: &str, base_url: &str, path: &str) -> LiveHttpResponse {
    http_request_with_headers(method, base_url, path, &[])
}

pub fn http_options(base_url: &str, path: &str, headers: &[(&str, &str)]) -> LiveHttpResponse {
    http_request_with_headers("OPTIONS", base_url, path, headers)
}

pub fn http_request_with_headers(
    method: &str,
    base_url: &str,
    path: &str,
    headers: &[(&str, &str)],
) -> LiveHttpResponse {
    http_request_with_headers_and_body(method, base_url, path, headers, None)
}

pub fn http_request_with_headers_and_body(
    method: &str,
    base_url: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: Option<&str>,
) -> LiveHttpResponse {
    let url = format!("{}{}", base_url.trim_end_matches('/'), path);
    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("http client should build");
    let method = Method::from_bytes(method.as_bytes()).expect("method should be valid");
    let mut request = client.request(method, &url);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    if let Some(body) = body {
        request = request.body(body.to_string());
    }
    let response = request.send().expect("request should succeed");
    let status = response.status().as_u16();
    let headers = response
        .headers()
        .iter()
        .map(|(name, value)| {
            (
                name.as_str().to_string(),
                value
                    .to_str()
                    .expect("header value should be utf-8")
                    .to_string(),
            )
        })
        .collect();
    let body = response.text().expect("response body should read");

    LiveHttpResponse {
        status,
        headers,
        body,
    }
}

pub fn route_by_signature<'a>(
    routes: &'a [HttpRoute],
    method: &str,
    path: &str,
) -> Option<&'a HttpRoute> {
    routes
        .iter()
        .find(|route| route.method == method && route.path == path)
}

fn wait_for_server(port: u16) {
    let addr = format!("127.0.0.1:{port}");
    for _ in 0..100 {
        if std::net::TcpStream::connect(&addr).is_ok() {
            return;
        }
        thread::sleep(Duration::from_millis(20));
    }

    panic!("server did not start on port {port}");
}
