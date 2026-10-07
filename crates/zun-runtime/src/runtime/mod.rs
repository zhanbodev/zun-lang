pub mod auth;
pub mod backend;
pub mod context;
pub(crate) mod gc;
pub mod http;
pub mod intrinsics;
pub mod loader;
pub mod program;
pub(crate) mod sql_intrinsics;
pub(crate) mod sql_registry;

pub use context::{RuntimeContext, RuntimeMode};
pub use http::{HandlerInput, execute_http_route, execute_http_route_in_context};
pub use intrinsics::{
    IntrinsicCall, IntrinsicId, IntrinsicRegistry, NativeFn, NativeFnMap, NativeModuleRegistry,
    RuntimePolicy,
};
pub use loader::{
    LoadedProgram, load_context_and_program, load_program_from_path, resolve_main_file,
    resolve_project_root,
};
pub use program::{ProgramState, execute_program_with_writer, execute_source_with_writer};

#[cfg(test)]
mod gc_tests;
