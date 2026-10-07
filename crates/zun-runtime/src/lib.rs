#![allow(clippy::result_large_err)]

pub mod ast {
    pub use zun_frontend::ast::*;
}

pub mod diagnostics {
    pub use zun_frontend::diagnostics::*;
}

pub mod error {
    pub use zun_frontend::error::*;
}

pub mod lexer {
    pub use zun_frontend::lexer::*;
}

pub mod parser {
    pub use zun_frontend::parser::*;
}

pub mod syntax {
    pub use zun_frontend::syntax::*;
}

pub mod token {
    pub use zun_frontend::token::*;
}

pub mod config;
pub mod interpreter;
pub mod module;
pub mod runtime;
pub mod stdlib_native;

pub use config::{ProjectConfig, ServerConfig};
pub use interpreter::{FnEnv, ZunValue, exec};
pub use module::{ModuleNamespace, ModuleResolver, ResolvedModule};
pub use parser::parse;
pub use runtime::{
    IntrinsicCall, IntrinsicId, IntrinsicRegistry, ProgramState, RuntimeContext, RuntimeMode,
    RuntimePolicy,
};
