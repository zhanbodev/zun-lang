use crate::ast::{ThrowStmt, TryStmt};
use crate::runtime::{ProgramState, RuntimeContext};

use super::super::eval::eval_expr;
use super::super::value::ZunValue;
use super::{Flow, exec_block};

pub(super) fn handle_throw_stmt(
    stmt: &ThrowStmt,
    state: &mut ProgramState,
    context: &mut RuntimeContext,
    w: &mut dyn std::io::Write,
) -> Flow {
    match eval_expr(&stmt.value, state, context, w, false) {
        Ok(val) => Flow::Throw(val),
        Err(err) => Flow::Err(err),
    }
}

pub(super) fn handle_try_stmt(
    stmt: &TryStmt,
    state: &mut ProgramState,
    context: &mut RuntimeContext,
    w: &mut dyn std::io::Write,
) -> Flow {
    let flow = exec_block(&stmt.body, state, context, w);
    match flow {
        Flow::Throw(val) => {
            state.insert_env(stmt.catch_var.clone(), val);
            exec_block(&stmt.catch_body, state, context, w)
        }
        Flow::Err(err) => {
            let msg = ZunValue::Str(err.to_string());
            state.insert_env(stmt.catch_var.clone(), msg);
            exec_block(&stmt.catch_body, state, context, w)
        }
        other => other,
    }
}
