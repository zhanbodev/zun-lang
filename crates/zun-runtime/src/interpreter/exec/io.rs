use std::io::Write;

use crate::ast::{PrintStmt, ReadStmt};
use crate::error::Error;
use crate::runtime::{ProgramState, RuntimeContext, intrinsics::ids};

use super::super::eval::{check_eval_result, eval_expr};
use super::super::value::ZunValue;
use super::Flow;

pub(super) fn handle_print_stmt(
    stmt: &PrintStmt,
    state: &mut ProgramState,
    context: &mut RuntimeContext,
    w: &mut dyn Write,
) -> Flow {
    let use_stderr = stmt.target == crate::ast::PrintTarget::Stderr;
    match check_eval_result(eval_expr(&stmt.value, state, context, w, false)) {
        Ok(val) => {
            if use_stderr {
                eprintln!("{}", val);
            } else if writeln!(w, "{}", val).is_err() {
                return Flow::Err(Error::InvalidStatement(None));
            }
            Flow::Normal
        }
        Err(err) => Flow::Err(err),
    }
}

pub(super) fn handle_read_stmt(
    stmt: &ReadStmt,
    state: &mut ProgramState,
    context: &mut RuntimeContext,
    w: &mut dyn Write,
) -> Flow {
    use std::io;

    match stmt.mode {
        crate::ast::ReadMode::Env => {
            let key_val = if let Some(ref prompt_expr) = stmt.prompt {
                match eval_expr(prompt_expr, state, context, w, false) {
                    Ok(val) => val,
                    Err(err) => return Flow::Err(err),
                }
            } else {
                return Flow::Err(Error::Interpreter(
                    "ENV requires a key: $<<ENV(\"KEY\")".to_string(),
                ));
            };

            let key = key_val.to_string();
            if key.is_empty() {
                return Flow::Err(Error::Interpreter(
                    "ENV requires a key: $<<ENV(\"KEY\")".to_string(),
                ));
            }

            match context.call_intrinsic(ids::ENV_GET, &[ZunValue::Str(key)]) {
                Ok(_) => Flow::Normal,
                Err(err) => Flow::Err(err),
            }
        }
        crate::ast::ReadMode::Line => {
            if let Some(ref prompt_expr) = stmt.prompt {
                match eval_expr(prompt_expr, state, context, w, false) {
                    Ok(prompt_val) => {
                        if write!(w, "{}", prompt_val).is_err() || w.flush().is_err() {
                            return Flow::Err(Error::InvalidStatement(None));
                        }
                    }
                    Err(err) => return Flow::Err(err),
                }
            }

            let mut input = String::new();
            match io::stdin().read_line(&mut input) {
                Ok(0) => Flow::Err(Error::Interpreter(
                    "runtime error: unexpected EOF on stdin".to_string(),
                )),
                Ok(_) => Flow::Normal,
                Err(_) => Flow::Err(Error::Interpreter(
                    "runtime error: failed to read from stdin".to_string(),
                )),
            }
        }
    }
}
