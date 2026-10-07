use std::collections::HashSet;
use std::sync::Arc;

pub mod builtins;
pub mod env;
pub mod eval;
pub mod exec;
pub mod type_validation;
pub mod value;

pub use crate::ast::CorsConfig;
use crate::ast::{FnParam, TypeExpr};
use crate::runtime::NativeFnMap;
pub use env::{Env, FnEnv, type_expr_name, type_name};
pub use exec::exec;
pub use type_validation::{
    TypeValidationError, validate_typed_instance_fields, validate_value_against_type_expr,
};
pub use value::ZunValue;

#[derive(Debug, Clone, Default)]
pub struct RouteModuleState {
    pub env: Env,
    pub fns: FnEnv,
}

#[derive(Clone, Default)]
pub struct ModuleNamespace {
    pub exports: FnEnv,
    pub fns: FnEnv,
    pub native_exports: NativeFnMap,
    pub module_env: Arc<Env>,
    pub visible_user_types: Arc<HashSet<String>>,
}

impl std::fmt::Debug for ModuleNamespace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModuleNamespace")
            .field("exports", &self.exports.len())
            .field("fns", &self.fns.len())
            .field("native_exports", &self.native_exports.len())
            .field("module_env", &self.module_env.len())
            .field("visible_user_types", &self.visible_user_types.len())
            .finish()
    }
}

/// HTTP route entry for route registry
#[derive(Debug, Clone)]
pub struct HttpRoute {
    pub method: String, // "GET", "POST", "PUT", "DELETE", "PATCH"
    pub path: String,   // "/user/:id"
    pub name: String,   // function name
    pub params: Vec<FnParam>,
    pub variadic_param: Option<String>,
    pub return_type: Option<TypeExpr>,
    pub cors: Option<CorsConfig>,
    pub parent_cors: Option<CorsConfig>,
    pub response_headers: Vec<(String, String)>,
    pub body: Vec<crate::ast::Stmt>,
    /// Shared module-level variables/functions captured when the route's
    /// source file was loaded. Request execution clones out of this shared
    /// snapshot to preserve isolation without duplicating the snapshot per route.
    pub module_state: Arc<RouteModuleState>,
}

impl HttpRoute {
    pub fn module_env(&self) -> &Env {
        &self.module_state.env
    }

    pub fn module_fns(&self) -> &FnEnv {
        &self.module_state.fns
    }
}

/// Static file serving entry
#[derive(Debug, Clone)]
pub struct StaticRoute {
    pub url_prefix: String,  // URL prefix, e.g., "/css"
    pub module_path: String, // module path, e.g., "css" or "css.zun"
    pub base_dir: String,    // base directory relative to main.zun
}

/// Execute HTTP handler body and capture return value
/// Returns (should_continue, result_value, error)
pub fn exec_http_handler(
    body: &[crate::ast::Stmt],
    state: &mut crate::runtime::ProgramState,
    context: &mut crate::runtime::RuntimeContext,
) -> (bool, Option<ZunValue>, Option<String>) {
    use super::exec::{Flow, exec_inner};
    use std::io::stdout;

    let mut output = stdout();
    let mut result: Option<ZunValue> = None;

    for stmt in body {
        match exec_inner(stmt, state, context, &mut output) {
            Flow::Normal | Flow::Break | Flow::Continue => {}
            Flow::Exit => return (false, None, Some("exit".to_string())),
            Flow::Return(val) => {
                result = val;
                break;
            }
            Flow::Err(e) => {
                eprintln!("\u{1b}[97;41m[ERROR]\u{1b}[0m {}", e);
                return (true, None, Some(e.to_string()));
            }
            Flow::Throw(val) => {
                eprintln!("\u{1b}[97;41m[ERROR]\u{1b}[0m uncaught throw: {val}");
                return (true, None, Some(format!("uncaught throw: {val}")));
            }
        }
    }

    (true, result, None)
}
