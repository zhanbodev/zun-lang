use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;

use crate::ast::{HttpBlockStmt, HttpFnStmt, TypeExpr};
use crate::error::Error;

use crate::module::ModuleResolver;
use crate::runtime::{ProgramState, RuntimeContext, RuntimeMode};

use super::super::{HttpRoute, RouteModuleState, ZunValue, exec_http_handler};
use super::{Flow, info_prefix};

pub(super) fn handle_http_fn(
    stmt: &HttpFnStmt,
    state: &ProgramState,
    context: &mut RuntimeContext,
    w: &mut dyn Write,
) -> Flow {
    let module_state = Arc::new(RouteModuleState {
        env: state.env.clone(),
        fns: state.fns.clone(),
    });
    let route = HttpRoute {
        method: stmt.method.clone(),
        path: stmt.path.clone(),
        name: stmt.name.clone(),
        params: stmt.params.clone(),
        variadic_param: stmt.variadic_param.clone(),
        return_type: stmt.return_type.clone(),
        cors: stmt.cors.clone(),
        parent_cors: None,
        response_headers: headers_to_pairs(&stmt.headers),
        body: stmt.body.clone(),
        module_state,
    };

    if let Err(err) = maybe_probe_http_handler(&route, context) {
        return Flow::Err(err);
    }
    context.register_http_route(route);
    writeln!(
        w,
        "{} HTTP route registered: {} {}",
        info_prefix(context),
        stmt.method,
        stmt.path
    )
    .ok();
    Flow::Normal
}

pub(super) fn handle_http_block(
    stmt: &HttpBlockStmt,
    state: &ProgramState,
    context: &mut RuntimeContext,
    w: &mut dyn Write,
) -> Flow {
    let module_state = Arc::new(RouteModuleState {
        env: state.env.clone(),
        fns: state.fns.clone(),
    });
    if let Some(link_module) = &stmt.link {
        let prefix = stmt.prefix.as_deref().unwrap_or("");
        let module_routes = match load_module_routes(link_module, context) {
            Ok(routes) => routes,
            Err(err) => return Flow::Err(err),
        };

        let route_count = module_routes.len();
        for route in module_routes {
            let prefix_trimmed = prefix.trim_start_matches('/').trim_end_matches('/');
            let path_trimmed = route.path.trim_start_matches('/');
            let full_path = if prefix_trimmed.is_empty() {
                format!("/{}", path_trimmed)
            } else {
                format!("/{}/{}", prefix_trimmed, path_trimmed)
            };
            let response_headers = merge_response_headers(&stmt.headers, &route.response_headers);
            let route = HttpRoute {
                path: full_path,
                cors: route.cors,
                parent_cors: stmt.cors.clone(),
                response_headers,
                ..route
            };
            if let Err(err) = maybe_probe_http_handler(&route, context) {
                return Flow::Err(err);
            }
            context.register_http_route(route);
        }

        writeln!(
            w,
            "{} HTTP linked module '{}' with prefix '{}' ({} routes)",
            info_prefix(context),
            link_module,
            prefix,
            route_count
        )
        .ok();
        return Flow::Normal;
    }

    for route_stmt in &stmt.routes {
        let full_path = if let Some(prefix) = &stmt.prefix {
            let prefix_trimmed = prefix.trim_start_matches('/').trim_end_matches('/');
            let path_trimmed = route_stmt.path.trim_start_matches('/');
            if prefix_trimmed.is_empty() {
                format!("/{}", path_trimmed)
            } else {
                format!("/{}/{}", prefix_trimmed, path_trimmed)
            }
        } else {
            route_stmt.path.clone()
        };

        let route = HttpRoute {
            method: route_stmt.method.clone(),
            path: full_path,
            name: route_stmt.name.clone(),
            params: route_stmt.params.clone(),
            variadic_param: route_stmt.variadic_param.clone(),
            return_type: route_stmt.return_type.clone(),
            cors: route_stmt.cors.clone(),
            parent_cors: stmt.cors.clone(),
            response_headers: merge_response_headers(
                &stmt.headers,
                &headers_to_pairs(&route_stmt.headers),
            ),
            body: route_stmt.body.clone(),
            module_state: Arc::clone(&module_state),
        };
        if let Err(err) = maybe_probe_http_handler(&route, context) {
            return Flow::Err(err);
        }
        context.register_http_route(route);
    }

    writeln!(
        w,
        "{} HTTP block registered {} routes",
        info_prefix(context),
        stmt.routes.len()
    )
    .ok();
    Flow::Normal
}

/// Load routes from a linked module by fully executing the file so that
/// `$mod` imports, `$fn` declarations, and nested `$HTTP` blocks are all
/// honoured.  The resulting routes carry `module_env`/`module_fns` so that
/// handler bodies can resolve module-namespaced calls (e.g. `hello.selectUser`).
fn load_module_routes(
    module_path: &str,
    context: &mut RuntimeContext,
) -> Result<Vec<HttpRoute>, Error> {
    use crate::parser;
    use crate::runtime::{ProgramState, execute_program_with_writer};

    let resolver = ModuleResolver::new(context.project_root().to_path_buf())
        .with_current_file(context.current_file().map(Path::new).map(Path::to_path_buf))
        .with_manifest(context.project_config().cloned());

    let resolved = match resolver.resolve_module(module_path) {
        Some(resolved) => resolved,
        None => {
            return Err(Error::Interpreter(format!(
                "module file not found: {} (tried: {})",
                module_path,
                resolver.describe_search_order(module_path)
            )));
        }
    };

    let content = match fs::read_to_string(&resolved.file_path) {
        Ok(content) => content,
        Err(err) => {
            return Err(Error::Interpreter(format!(
                "cannot read module file '{}': {}",
                resolved.file_path.display(),
                err
            )));
        }
    };

    let statements = match parser::parse(&content) {
        Ok(stmts) => stmts,
        Err(err) => {
            return Err(Error::Interpreter(format!(
                "failed to parse module '{}': {}",
                module_path, err
            )));
        }
    };

    // Execute the module fully in an isolated clone of the context.
    // This processes $mod, $fn, and $GET/$POST/... declarations.
    let mut module_context = context.clone_for_isolated_execution();
    module_context.set_current_file(Some(resolved.file_path.to_string_lossy().to_string()));

    // Track how many routes existed before execution so we can isolate
    // only the routes this module adds.
    let routes_before = module_context.routes().len();

    let mut module_state = ProgramState::new();
    execute_program_with_writer(
        &statements,
        &mut module_state,
        &mut module_context,
        &mut std::io::sink(),
    )
    .map_err(|err| {
        Error::Interpreter(format!("error loading module '{}': {}", module_path, err))
    })?;

    // Linked router modules can import `$Type` definitions from other files.
    // Request execution clones the parent RuntimeContext, not the isolated
    // module_context used during `.link()` loading, so merge newly registered
    // types back into the parent context before returning the routes.
    context.merge_types_from(&module_context);

    // Routes registered by this module share one module-level snapshot so that
    // handler bodies can resolve $mod-imported namespaces without duplicating
    // the full module env/fn graph per route.
    let module_state = Arc::new(RouteModuleState {
        env: module_state.env,
        fns: module_state.fns,
    });
    let routes: Vec<HttpRoute> = module_context.routes()[routes_before..]
        .iter()
        .map(|route| HttpRoute {
            module_state: Arc::clone(&module_state),
            ..route.clone()
        })
        .collect();

    if routes.is_empty() {
        return Err(Error::Interpreter(format!(
            "module '{}' does not define any HTTP routes",
            module_path
        )));
    }

    Ok(routes)
}

fn maybe_probe_http_handler(route: &HttpRoute, context: &RuntimeContext) -> Result<(), Error> {
    if !matches!(context.mode(), RuntimeMode::Test) {
        return Ok(());
    }

    probe_http_handler_return_type(route, context)
}

fn probe_http_handler_return_type(
    route: &HttpRoute,
    context: &RuntimeContext,
) -> Result<(), Error> {
    let mut probe_context = context.clone_for_isolated_execution();
    let mut probe_state = ProgramState::new();

    probe_state.extend_env(route.module_env().clone());
    probe_state.fns.extend(route.module_fns().clone());

    let mut bound_http_params = indexmap::IndexMap::new();
    for param in &route.params {
        bound_http_params.insert(param.name.clone(), ZunValue::Str("__dummy__".to_string()));
    }

    if let Err(err) = crate::runtime::http::bind_http_handler_params(
        &route.params,
        &bound_http_params,
        &mut probe_state,
        &probe_context,
    ) {
        return Err(err);
    }

    for (name, value) in bound_http_params {
        if !probe_state.env_contains_key(&name) {
            probe_state.insert_env(name, value);
        }
    }

    probe_state.insert_env(
        "__headers__".to_string(),
        ZunValue::Json(indexmap::IndexMap::new()),
    );
    probe_state.insert_env(
        "body".to_string(),
        default_probe_body(route.return_type.as_ref()),
    );

    let (_should_continue, result, error) =
        exec_http_handler(&route.body, &mut probe_state, &mut probe_context);
    if let Some(error) = error {
        return Err(Error::Interpreter(error));
    }

    super::functions::validate_declared_return_type(
        "http handler",
        &route.name,
        route.return_type.as_ref(),
        result.as_ref(),
        &probe_context,
    )
}

fn default_probe_body(return_type: Option<&TypeExpr>) -> ZunValue {
    match return_type {
        Some(TypeExpr::Named(name)) if name == "Int" || name == "Integer" => ZunValue::Int(0),
        Some(TypeExpr::Named(name)) if name == "Float" => ZunValue::Float(0.0),
        Some(TypeExpr::Named(name)) if name == "String" || name == "Str" => {
            ZunValue::Str("__dummy__".to_string())
        }
        Some(TypeExpr::Named(name)) if name == "Bool" || name == "Boolean" => ZunValue::Bool(false),
        Some(TypeExpr::List(_)) => ZunValue::List(vec![]),
        Some(TypeExpr::Named(name)) if name == "Map" || name == "Json" => {
            ZunValue::Map(indexmap::IndexMap::new())
        }
        Some(TypeExpr::Named(name)) if name == "Response" => ZunValue::Response {
            status: 200,
            body: None,
        },
        _ => ZunValue::Null,
    }
}

fn headers_to_pairs(headers: &[crate::ast::SetHdrEntry]) -> Vec<(String, String)> {
    headers
        .iter()
        .map(|entry| (entry.name.clone(), entry.value.clone()))
        .collect()
}

fn merge_response_headers(
    block_headers: &[crate::ast::SetHdrEntry],
    route_headers: &[(String, String)],
) -> Vec<(String, String)> {
    let mut merged = headers_to_pairs(block_headers);

    for entry in route_headers {
        let normalized = entry.0.to_ascii_lowercase();
        if let Some(existing) = merged
            .iter_mut()
            .find(|(name, _)| name.to_ascii_lowercase() == normalized)
        {
            existing.1 = entry.1.clone();
        } else {
            merged.push((entry.0.clone(), entry.1.clone()));
        }
    }

    merged
}
