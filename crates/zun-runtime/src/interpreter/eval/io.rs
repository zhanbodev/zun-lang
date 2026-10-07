use crate::ast::{ConfigReadExpr, Expr, ExprRead, HdrReadExpr};
use crate::diagnostics::codes;
use crate::error::Error;
use crate::runtime::{ProgramState, RuntimeContext, intrinsics::ids};

use super::super::value::ZunValue;
use super::eval_expr;

pub fn eval_read_expr(
    e: &Expr,
    read_expr: &ExprRead,
    state: &mut ProgramState,
    context: &mut RuntimeContext,
    w: &mut dyn std::io::Write,
) -> Result<ZunValue, Error> {
    use std::io;

    match read_expr.mode {
        crate::ast::ReadMode::Env => {
            let key = if let Some(ref prompt_expr) = read_expr.prompt {
                eval_expr(prompt_expr, state, context, w, false)?.to_string()
            } else {
                return Err(super::runtime_error(
                    e,
                    codes::RUNTIME_INVALID_EXPRESSION,
                    "ENV requires a key: $<<ENV(\"KEY\")",
                ));
            };

            if key.is_empty() {
                return Err(super::runtime_error(
                    e,
                    codes::RUNTIME_INVALID_EXPRESSION,
                    "ENV requires a key: $<<ENV(\"KEY\")",
                ));
            }

            context
                .call_intrinsic(ids::ENV_GET, &[ZunValue::Str(key)])
                .map_err(|err| super::runtime_error(e, codes::RUNTIME_GENERIC, err.to_string()))
        }
        crate::ast::ReadMode::Line => {
            if let Some(ref prompt_expr) = read_expr.prompt {
                let prompt_val = eval_expr(prompt_expr, state, context, w, false)?;
                let _ = write!(w, "{prompt_val}");
                let _ = w.flush();
            }

            let mut input = String::new();
            match io::stdin().read_line(&mut input) {
                Ok(0) => Err(super::runtime_error(
                    e,
                    codes::RUNTIME_GENERIC,
                    "unexpected EOF on stdin",
                )),
                Ok(_) => {
                    let input = input.trim_end_matches('\n').trim_end_matches('\r');
                    Ok(ZunValue::Str(input.to_string()))
                }
                Err(_) => Err(super::runtime_error(
                    e,
                    codes::RUNTIME_GENERIC,
                    "failed to read from stdin",
                )),
            }
        }
    }
}

pub fn eval_config_read_expr(
    e: &Expr,
    config: &ConfigReadExpr,
    context: &mut RuntimeContext,
) -> Result<ZunValue, Error> {
    context
        .call_intrinsic(ids::CONFIG_GET, &[ZunValue::Str(config.key.clone())])
        .map_err(|err| super::runtime_error(e, codes::RUNTIME_GENERIC, err.to_string()))
}

pub fn eval_hdr_read_expr(hdr: &HdrReadExpr, state: &ProgramState) -> Result<ZunValue, Error> {
    let header_key = hdr.header_name.to_lowercase();
    if let Some(ZunValue::Json(headers)) = state.lookup_env("__headers__") {
        if let Some(value) = headers.get(&header_key) {
            return Ok(value.clone());
        }
        if let Some(value) = headers.get(&hdr.header_name) {
            return Ok(value.clone());
        }
    }
    Ok(ZunValue::Str(String::new()))
}
