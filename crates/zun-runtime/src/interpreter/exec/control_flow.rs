use crate::ast::{ForInStmt, ForStmt, IfStmt, LoopStmt, MainDeclStmt, ReturnStmt, WhileStmt};
use crate::error::Error;
use crate::runtime::{ProgramState, RuntimeContext};

use super::super::eval::{check_eval_result, eval_expr};
use super::super::value::ZunValue;
use super::{Flow, exec_block, exec_inner, exec_with_writer};

pub(super) fn handle_main_decl(
    stmt: &MainDeclStmt,
    state: &mut ProgramState,
    context: &mut RuntimeContext,
    w: &mut dyn std::io::Write,
) -> Flow {
    let current_file = context.current_file().map(ToOwned::to_owned);
    let is_main_dol = current_file
        .as_ref()
        .map(|f| {
            std::path::Path::new(f)
                .file_name()
                .map(|n| n.to_string_lossy() == "main.zun")
                .unwrap_or(false)
        })
        .unwrap_or(false);

    if !is_main_dol {
        return Flow::Err(Error::Interpreter(
            "$main() can only be declared in main.zun".to_string(),
        ));
    }

    context.set_global_cors(stmt.global_cors.clone());

    for main_stmt in &stmt.body {
        let (cont, err) = exec_with_writer(main_stmt, state, context, w);
        if let Err(err) = err {
            return Flow::Err(err);
        }
        if !cont {
            return Flow::Normal;
        }
    }
    Flow::Normal
}

pub(super) fn handle_return_stmt(
    stmt: &ReturnStmt,
    state: &mut ProgramState,
    context: &mut RuntimeContext,
    w: &mut dyn std::io::Write,
) -> Flow {
    match &stmt.value {
        Some(expr) => match check_eval_result(eval_expr(expr, state, context, w, false)) {
            Ok(val) => Flow::Return(Some(val)),
            Err(err) => Flow::Err(err),
        },
        None => Flow::Return(None),
    }
}

pub(super) fn handle_if_stmt(
    stmt: &IfStmt,
    state: &mut ProgramState,
    context: &mut RuntimeContext,
    w: &mut dyn std::io::Write,
) -> Flow {
    for branch in &stmt.branches {
        let cond_val = check_eval_result(eval_expr(&branch.condition, state, context, w, false));
        match cond_val {
            Ok(cond_val) => {
                if cond_val.is_truthy() {
                    return exec_block(&branch.body, state, context, w);
                }
            }
            Err(err) => return Flow::Err(err),
        }
    }
    exec_block(&stmt.else_body, state, context, w)
}

pub(super) fn handle_while_stmt(
    stmt: &WhileStmt,
    state: &mut ProgramState,
    context: &mut RuntimeContext,
    w: &mut dyn std::io::Write,
) -> Flow {
    loop {
        let cond = check_eval_result(eval_expr(&stmt.condition, state, context, w, false));
        match cond {
            Ok(ref c) if c.is_truthy() => {}
            Ok(_) => break,
            Err(err) => return Flow::Err(err),
        }
        match exec_block(&stmt.body, state, context, w) {
            Flow::Break => break,
            Flow::Exit => return Flow::Exit,
            Flow::Err(err) => return Flow::Err(err),
            Flow::Throw(val) => return Flow::Throw(val),
            Flow::Return(val) => return Flow::Return(val),
            Flow::Continue | Flow::Normal => {}
        }
    }
    Flow::Normal
}

pub(super) fn handle_loop_stmt(
    stmt: &LoopStmt,
    state: &mut ProgramState,
    context: &mut RuntimeContext,
    w: &mut dyn std::io::Write,
) -> Flow {
    loop {
        match exec_block(&stmt.body, state, context, w) {
            Flow::Break => break,
            Flow::Exit => return Flow::Exit,
            Flow::Err(err) => return Flow::Err(err),
            Flow::Throw(val) => return Flow::Throw(val),
            Flow::Return(val) => return Flow::Return(val),
            Flow::Continue | Flow::Normal => {}
        }
    }
    Flow::Normal
}

pub(super) fn handle_for_stmt(
    stmt: &ForStmt,
    state: &mut ProgramState,
    context: &mut RuntimeContext,
    w: &mut dyn std::io::Write,
) -> Flow {
    if let Some(init) = &stmt.init {
        let flow = exec_inner(init, state, context, w);
        if !matches!(flow, Flow::Normal) {
            return flow;
        }
    }

    loop {
        if let Some(cond_expr) = &stmt.condition {
            let cond = check_eval_result(eval_expr(cond_expr, state, context, w, false));
            match cond {
                Ok(ref c) if c.is_truthy() => {}
                Ok(_) => break,
                Err(err) => return Flow::Err(err),
            }
        }

        match exec_block(&stmt.body, state, context, w) {
            Flow::Break => break,
            Flow::Exit => return Flow::Exit,
            Flow::Err(err) => return Flow::Err(err),
            Flow::Throw(val) => return Flow::Throw(val),
            Flow::Return(val) => return Flow::Return(val),
            Flow::Continue | Flow::Normal => {}
        }

        if let Some(update) = &stmt.update {
            let flow = exec_inner(update, state, context, w);
            if !matches!(flow, Flow::Normal) {
                return flow;
            }
        }
    }
    Flow::Normal
}

pub(super) fn handle_for_in_stmt(
    stmt: &ForInStmt,
    state: &mut ProgramState,
    context: &mut RuntimeContext,
    w: &mut dyn std::io::Write,
) -> Flow {
    let iterable_val = match eval_expr(&stmt.iterable, state, context, w, false) {
        Ok(val) => val,
        Err(err) => return Flow::Err(err),
    };

    match iterable_val {
        ZunValue::List(list) => {
            for item in list {
                state.insert_env(stmt.var.clone(), item.clone());
                match exec_block(&stmt.body, state, context, w) {
                    Flow::Break => break,
                    Flow::Exit => return Flow::Exit,
                    Flow::Err(err) => return Flow::Err(err),
                    Flow::Throw(val) => return Flow::Throw(val),
                    Flow::Return(val) => return Flow::Return(val),
                    Flow::Continue | Flow::Normal => {}
                }
            }
        }
        ZunValue::Map(map) => {
            for key in map.keys() {
                state.insert_env(stmt.var.clone(), ZunValue::Str(key.clone()));
                match exec_block(&stmt.body, state, context, w) {
                    Flow::Break => break,
                    Flow::Exit => return Flow::Exit,
                    Flow::Err(err) => return Flow::Err(err),
                    Flow::Throw(val) => return Flow::Throw(val),
                    Flow::Return(val) => return Flow::Return(val),
                    Flow::Continue | Flow::Normal => {}
                }
            }
        }
        ZunValue::Str(s) => {
            for ch in s.chars() {
                state.insert_env(stmt.var.clone(), ZunValue::Str(ch.to_string()));
                match exec_block(&stmt.body, state, context, w) {
                    Flow::Break => break,
                    Flow::Exit => return Flow::Exit,
                    Flow::Err(err) => return Flow::Err(err),
                    Flow::Throw(val) => return Flow::Throw(val),
                    Flow::Return(val) => return Flow::Return(val),
                    Flow::Continue | Flow::Normal => {}
                }
            }
        }
        _ => {
            return Flow::Err(Error::Interpreter(format!(
                "cannot iterate over value of type '{}'",
                iterable_val.type_name()
            )));
        }
    }

    Flow::Normal
}
