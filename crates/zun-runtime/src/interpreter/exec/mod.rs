mod control_flow;
mod error_handling;
mod functions;
mod http;
mod io;
mod modules;
mod types;
mod variables;

use crate::ast::Stmt;
use crate::error::Error;
use crate::runtime::{ProgramState, RuntimeContext, RuntimeMode};
use std::io::{Write, stdout};

use super::value::ZunValue;

pub(crate) use functions::parameter_type_error;
pub use functions::{call_fn, call_module_fn, validate_declared_return_type};

/// Internal control-flow signal returned by exec_inner.
#[derive(Debug)]
pub enum Flow {
    Normal,
    Break,
    Continue,
    Exit,
    Return(Option<ZunValue>),
    Err(Error),
    Throw(ZunValue),
}

pub fn exec(
    stmt: &Stmt,
    state: &mut ProgramState,
    context: &mut RuntimeContext,
) -> (bool, Result<(), Error>) {
    exec_with_writer(stmt, state, context, &mut stdout())
}

pub(super) fn info_prefix(context: &RuntimeContext) -> &'static str {
    if matches!(context.mode(), RuntimeMode::Serve) {
        "\u{1b}[97;44m[INFO]\u{1b}[0m"
    } else {
        "[INFO]"
    }
}

pub fn exec_with_writer(
    stmt: &Stmt,
    state: &mut ProgramState,
    context: &mut RuntimeContext,
    w: &mut dyn Write,
) -> (bool, Result<(), Error>) {
    match exec_inner(stmt, state, context, w) {
        Flow::Normal | Flow::Break | Flow::Continue | Flow::Return(_) => (true, Ok(())),
        Flow::Exit => (false, Ok(())),
        Flow::Err(e) => (true, Err(e)),
        Flow::Throw(val) => (
            true,
            Err(Error::Interpreter(format!("uncaught throw: {val}"))),
        ),
    }
}

pub(super) fn exec_inner(
    stmt: &Stmt,
    state: &mut ProgramState,
    context: &mut RuntimeContext,
    w: &mut dyn Write,
) -> Flow {
    match stmt {
        Stmt::Exit(_) => Flow::Exit,
        Stmt::Break(_) => Flow::Break,
        Stmt::Continue(_) => Flow::Continue,
        Stmt::ModDecl(stmt) => modules::handle_mod_decl(stmt, state, context, w),
        Stmt::MainDecl(stmt) => control_flow::handle_main_decl(stmt, state, context, w),
        Stmt::Return(stmt) => control_flow::handle_return_stmt(stmt, state, context, w),
        Stmt::Print(stmt) => io::handle_print_stmt(stmt, state, context, w),
        Stmt::Read(stmt) => io::handle_read_stmt(stmt, state, context, w),
        Stmt::VarDecl(stmt) => variables::handle_var_decl(stmt, state, context, w),
        Stmt::ConstDecl(stmt) => variables::handle_const_decl(stmt, state, context, w),
        Stmt::Assign(stmt) => variables::handle_assign_stmt(stmt, state, context, w),
        Stmt::If(stmt) => control_flow::handle_if_stmt(stmt, state, context, w),
        Stmt::While(stmt) => control_flow::handle_while_stmt(stmt, state, context, w),
        Stmt::Loop(stmt) => control_flow::handle_loop_stmt(stmt, state, context, w),
        Stmt::For(stmt) => control_flow::handle_for_stmt(stmt, state, context, w),
        Stmt::ForIn(stmt) => control_flow::handle_for_in_stmt(stmt, state, context, w),
        Stmt::FnDecl(stmt) => functions::handle_fn_decl(stmt, state),
        Stmt::HttpFn(stmt) => http::handle_http_fn(stmt, state, context, w),
        Stmt::HttpBlock(stmt) => http::handle_http_block(stmt, state, context, w),
        Stmt::Static(stmt) => modules::handle_static_stmt(stmt, context, w),
        Stmt::Try(stmt) => error_handling::handle_try_stmt(stmt, state, context, w),
        Stmt::Throw(stmt) => error_handling::handle_throw_stmt(stmt, state, context, w),
        Stmt::ExprStmt(expr) => variables::handle_expr_stmt(expr, state, context, w),
        Stmt::TypeDecl(stmt) => types::handle_type_decl(stmt, context),
    }
}

pub(super) fn exec_block(
    stmts: &[Stmt],
    state: &mut ProgramState,
    context: &mut RuntimeContext,
    w: &mut dyn Write,
) -> Flow {
    for stmt in stmts {
        let f = exec_inner(stmt, state, context, w);
        match f {
            Flow::Normal => {}
            other => return other,
        }
    }
    Flow::Normal
}
