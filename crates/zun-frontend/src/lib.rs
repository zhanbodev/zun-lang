#![allow(clippy::module_inception, clippy::result_large_err)]

pub mod ast;
pub mod diagnostics;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod syntax;
pub mod token;

pub use parser::parse;
