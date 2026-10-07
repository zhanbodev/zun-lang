use crate::ast::{AssignStmt, ConstDeclStmt, Expr, MethodCall, VarDeclStmt};
use crate::diagnostics::codes;
use crate::error::Error;
use crate::interpreter::{TypeValidationError, type_expr_name, validate_value_against_type_expr};
use crate::runtime::{ProgramState, RuntimeContext};

use super::super::eval::{check_eval_result, eval_expr};
use super::super::value::ZunValue;
use super::Flow;

pub(super) fn handle_var_decl(
    stmt: &VarDeclStmt,
    state: &mut ProgramState,
    context: &mut RuntimeContext,
    w: &mut dyn std::io::Write,
) -> Flow {
    if matches!(state.const_env.get(&stmt.name), Some(true)) {
        return Flow::Err(Error::Interpreter(format!(
            "constant '{}' is already defined",
            stmt.name
        )));
    }

    match check_eval_result(eval_expr(&stmt.value, state, context, w, false)) {
        Ok(val) => {
            if let Some(ref type_expr) = stmt.type_annotation {
                if !annotation_type_supported(type_expr) {
                    return Flow::Err(Error::TypeMismatch(format!(
                        "unknown type '{}', supported types are: Int, Float, String, Bool, List<T>, Map",
                        type_expr_name(type_expr)
                    )));
                }
                if let Err(error) = validate_value_against_type_expr(type_expr, &val, context) {
                    return Flow::Err(type_annotation_error(
                        &stmt.name, type_expr, error, stmt.span,
                    ));
                }
                state.type_env.insert(stmt.name.clone(), type_expr.clone());
            }
            state.insert_env(stmt.name.clone(), val);
            Flow::Normal
        }
        Err(err) => Flow::Err(err),
    }
}

pub(super) fn handle_const_decl(
    stmt: &ConstDeclStmt,
    state: &mut ProgramState,
    context: &mut RuntimeContext,
    w: &mut dyn std::io::Write,
) -> Flow {
    if state.env_contains_key(&stmt.name) {
        return Flow::Err(Error::Interpreter(format!(
            "constant '{}' is already defined",
            stmt.name
        )));
    }

    match check_eval_result(eval_expr(&stmt.value, state, context, w, false)) {
        Ok(val) => {
            if let Some(ref type_expr) = stmt.type_annotation {
                if !annotation_type_supported(type_expr) {
                    return Flow::Err(Error::TypeMismatch(format!(
                        "unknown type '{}', supported types are: Int, Float, String, Bool, List<T>, Map",
                        type_expr_name(type_expr)
                    )));
                }
                if let Err(error) = validate_value_against_type_expr(type_expr, &val, context) {
                    return Flow::Err(type_annotation_error(
                        &stmt.name, type_expr, error, stmt.span,
                    ));
                }
                state.type_env.insert(stmt.name.clone(), type_expr.clone());
            }
            state.const_env.insert(stmt.name.clone(), true);
            state.insert_env(stmt.name.clone(), val);
            Flow::Normal
        }
        Err(err) => Flow::Err(err),
    }
}

fn annotation_type_supported(type_expr: &crate::ast::TypeExpr) -> bool {
    match type_expr {
        crate::ast::TypeExpr::Named(name) => {
            !matches!(name.as_str(), "List" | "Integer" | "Boolean" | "Str")
        }
        crate::ast::TypeExpr::List(inner) => annotation_type_supported(inner),
        crate::ast::TypeExpr::Optional(inner) => annotation_type_supported(inner),
    }
}

pub(super) fn handle_assign_stmt(
    stmt: &AssignStmt,
    state: &mut ProgramState,
    context: &mut RuntimeContext,
    w: &mut dyn std::io::Write,
) -> Flow {
    if let Expr::MethodCall(target) = &*stmt.name {
        // TypedInstance field assignment: instance.field = val
        let var_name = match eval_expr(&target.object, state, context, w, true) {
            Ok(ZunValue::Str(s)) => s,
            Ok(_) => {
                return Flow::Err(Error::InvalidAssignment(Some(
                    "field assignment requires a direct variable as receiver".to_string(),
                )));
            }
            Err(err) => return Flow::Err(err),
        };

        let val = match check_eval_result(eval_expr(&stmt.value, state, context, w, false)) {
            Ok(v) => v,
            Err(err) => return Flow::Err(err),
        };

        // Determine stored key and expected field type (accounting for @HIDE fields)
        let (instance_type_name, stored_key, expected_field_type) = {
            let existing = match state.lookup_env(&var_name) {
                Some(e) => e,
                None => {
                    return Flow::Err(Error::Interpreter(format!(
                        "variable '{}' not found",
                        var_name
                    )));
                }
            };
            if let ZunValue::TypedInstance {
                type_name,
                fields: _,
            } = existing
            {
                let shape_opt = context.get_type(type_name);
                let field_def = shape_opt
                    .and_then(|s| s.fields.iter().find(|f| f.name == target.method).cloned());
                let expected_type = field_def.as_ref().map(|f| f.type_expr.clone());
                let key = match field_def {
                    Some(f) if f.hidden => format!("_{}", target.method),
                    Some(_) => target.method.clone(),
                    None => target.method.clone(),
                };
                (type_name.clone(), key, expected_type)
            } else {
                return Flow::Err(Error::InvalidAssignment(Some(format!(
                    "'{}' is not a struct instance",
                    var_name
                ))));
            }
        };

        // Type-check: verify assigned value matches the declared field type
        let Some(expected_type) = expected_field_type.as_ref() else {
            return Flow::Err(type_assignment_error(
                TypeValidationError::UnknownField {
                    type_name: instance_type_name,
                    field_name: target.method.clone(),
                },
                stmt.span,
                &target.method,
            ));
        };

        if let Err(error) = validate_value_against_type_expr(expected_type, &val, context) {
            return Flow::Err(type_assignment_error(error, stmt.span, &target.method));
        }

        state.invalidate_env_snapshot();
        if let Some(ZunValue::TypedInstance { fields, .. }) = state.get_env_mut(&var_name) {
            fields.insert(stored_key, val);
        }

        return Flow::Normal;
    }

    if let Expr::IndexAccess(idx) = &*stmt.name {
        let var_name = match eval_expr(&idx.object, state, context, w, true) {
            Ok(ZunValue::Str(s)) => s,
            Ok(_) => {
                return Flow::Err(Error::InvalidAssignment(Some(
                    "variable name must be a string".to_string(),
                )));
            }
            Err(err) => return Flow::Err(err),
        };

        let idx_val = match eval_expr(&idx.index, state, context, w, false) {
            Ok(val) => val,
            Err(err) => return Flow::Err(err),
        };

        let val = match check_eval_result(eval_expr(&stmt.value, state, context, w, false)) {
            Ok(val) => val,
            Err(err) => return Flow::Err(err),
        };

        let existing = match state.lookup_env(&var_name) {
            Some(existing) => existing,
            None => {
                return Flow::Err(Error::Interpreter(format!(
                    "variable '{}' not found",
                    var_name
                )));
            }
        };

        match existing {
            ZunValue::List(list) => {
                let index = match &idx_val {
                    ZunValue::Int(i) => *i as usize,
                    ZunValue::Float(f) if f.fract() == 0.0 => *f as usize,
                    _ => {
                        return Flow::Err(Error::Interpreter(
                            "list index must be an integer".to_string(),
                        ));
                    }
                };

                if index >= list.len() {
                    return Flow::Err(Error::Interpreter(format!(
                        "index out of bounds: list length is {} but index is {}",
                        list.len(),
                        index
                    )));
                }

                let mut new_list = list.clone();
                if let Some(crate::ast::TypeExpr::List(item_type)) = state.type_env.get(&var_name) {
                    if let Err(error) =
                        validate_value_against_type_expr(item_type.as_ref(), &val, context)
                    {
                        return Flow::Err(type_index_assignment_error(
                            &var_name,
                            item_type.as_ref(),
                            error,
                            stmt.span,
                        ));
                    }
                }
                new_list[index] = val;
                state.insert_env(var_name, ZunValue::List(new_list));
            }
            ZunValue::Map(map) => {
                let key = idx_val.to_string();
                let mut new_map = map.clone();
                new_map.insert(key, val);
                state.insert_env(var_name, ZunValue::Map(new_map));
            }
            _ => {
                return Flow::Err(Error::Interpreter(format!(
                    "cannot index into type {}",
                    existing.type_name()
                )));
            }
        }

        return Flow::Normal;
    }

    let name = match eval_expr(&stmt.name, state, context, w, true) {
        Ok(ZunValue::Str(s)) => s,
        Ok(_) => {
            return Flow::Err(Error::InvalidAssignment(Some(
                "variable name must be a string".to_string(),
            )));
        }
        Err(err) => return Flow::Err(err),
    };
    let val = match check_eval_result(eval_expr(&stmt.value, state, context, w, false)) {
        Ok(val) => val,
        Err(err) => return Flow::Err(err),
    };

    if let Some(is_const) = state.const_env.get(&name)
        && *is_const
    {
        return Flow::Err(Error::Interpreter(format!(
            "cannot reassign constant '{}'",
            name
        )));
    }

    if let Some(declared_type) = state.type_env.get(&name) {
        if let Err(error) = validate_value_against_type_expr(declared_type, &val, context) {
            return Flow::Err(type_annotation_error(
                &name,
                declared_type,
                error,
                stmt.span,
            ));
        }
    }

    state.insert_env(name, val);
    Flow::Normal
}

fn type_assignment_error(
    error: TypeValidationError,
    span: crate::ast::Span,
    field_name: &str,
) -> Error {
    let message = match error {
        TypeValidationError::BareList => {
            format!("field '{field_name}' must declare 'List<T>' instead of bare 'List'")
        }
        TypeValidationError::UnsupportedOptionalListItem => format!(
            "field '{field_name}' does not support optional list item types; use 'List<T>' or 'List<T>?'"
        ),
        TypeValidationError::ExpectedListClose => {
            format!("field '{field_name}' expects a complete list type expression")
        }
        TypeValidationError::UnknownType { expected_type } => {
            format!("field '{field_name}' references unknown type '{expected_type}'")
        }
        TypeValidationError::Mismatch {
            expected_type,
            actual_type,
        } => format!("field '{field_name}' expects type '{expected_type}', got '{actual_type}'"),
        TypeValidationError::MissingRequiredField {
            type_name,
            field_name,
        } => format!("type '{type_name}' requires field '{field_name}'"),
        TypeValidationError::UnknownField {
            type_name,
            field_name,
        } => format!("type '{type_name}' has no field '{field_name}'"),
    };

    Error::Diagnostic(
        crate::diagnostics::Diagnostic::error(codes::RUNTIME_FIELD_TYPE_MISMATCH, message)
            .with_span(crate::ast::Span::from_token(span.start)),
    )
}

fn type_annotation_error(
    variable_name: &str,
    type_expr: &crate::ast::TypeExpr,
    error: TypeValidationError,
    span: crate::ast::Span,
) -> Error {
    let message = match error {
        TypeValidationError::BareList => {
            format!("variable '{variable_name}' must declare 'List<T>' instead of bare 'List'")
        }
        TypeValidationError::UnsupportedOptionalListItem => format!(
            "variable '{variable_name}' uses unsupported type '{}'",
            type_expr_name(type_expr)
        ),
        TypeValidationError::ExpectedListClose => format!(
            "variable '{variable_name}' uses invalid type '{}'",
            type_expr_name(type_expr)
        ),
        TypeValidationError::UnknownType { expected_type } => {
            format!("variable '{variable_name}' references unknown type '{expected_type}'")
        }
        TypeValidationError::Mismatch {
            expected_type,
            actual_type,
        } => format!(
            "variable '{variable_name}' expects type '{expected_type}' but got '{actual_type}'"
        ),
        TypeValidationError::MissingRequiredField {
            type_name,
            field_name,
        } => format!("type '{type_name}' requires field '{field_name}'"),
        TypeValidationError::UnknownField {
            type_name,
            field_name,
        } => format!("type '{type_name}' has no field '{field_name}'"),
    };

    Error::Diagnostic(
        crate::diagnostics::Diagnostic::error(codes::RUNTIME_TYPE_MISMATCH, message)
            .with_span(crate::ast::Span::from_token(span.start)),
    )
}

fn type_index_assignment_error(
    variable_name: &str,
    item_type: &crate::ast::TypeExpr,
    error: TypeValidationError,
    span: crate::ast::Span,
) -> Error {
    let message = match error {
        TypeValidationError::Mismatch { actual_type, .. } => format!(
            "list element for '{}' expects type '{}', got '{}'",
            variable_name,
            type_expr_name(item_type),
            actual_type
        ),
        other => format!(
            "list element for '{}' violates type '{}': {}",
            variable_name,
            type_expr_name(item_type),
            type_validation_detail(other)
        ),
    };

    Error::Diagnostic(
        crate::diagnostics::Diagnostic::error(codes::RUNTIME_TYPE_MISMATCH, message)
            .with_span(crate::ast::Span::from_token(span.start)),
    )
}

fn validate_list_method_item_type(
    variable_name: &str,
    method: &str,
    args: &[ZunValue],
    state: &ProgramState,
    context: &RuntimeContext,
    span: crate::ast::Span,
) -> Result<(), Error> {
    let Some(type_expr) = state.type_env.get(variable_name) else {
        return Ok(());
    };

    let (item_type, value) = match (method, type_expr, args) {
        ("push", crate::ast::TypeExpr::List(item_type), [value]) => (item_type.as_ref(), value),
        ("insert", crate::ast::TypeExpr::List(item_type), [_, value]) => {
            (item_type.as_ref(), value)
        }
        _ => return Ok(()),
    };

    validate_value_against_type_expr(item_type, value, context)
        .map_err(|error| type_list_method_error(variable_name, method, item_type, error, span))
}

fn type_list_method_error(
    variable_name: &str,
    method: &str,
    item_type: &crate::ast::TypeExpr,
    error: TypeValidationError,
    span: crate::ast::Span,
) -> Error {
    let message = match error {
        TypeValidationError::Mismatch { actual_type, .. } => format!(
            "list method '{method}' for '{variable_name}' expects item type '{}', got '{actual_type}'",
            type_expr_name(item_type)
        ),
        other => format!(
            "list method '{method}' for '{variable_name}' expects item type '{}': {}",
            type_expr_name(item_type),
            type_validation_detail(other)
        ),
    };

    Error::Diagnostic(
        crate::diagnostics::Diagnostic::error(codes::RUNTIME_TYPE_MISMATCH, message)
            .with_span(crate::ast::Span::from_token(span.start)),
    )
}

fn type_validation_detail(error: TypeValidationError) -> String {
    match error {
        TypeValidationError::BareList => "bare List is not allowed".to_string(),
        TypeValidationError::UnsupportedOptionalListItem => {
            "optional list item types are not supported".to_string()
        }
        TypeValidationError::ExpectedListClose => "incomplete list type expression".to_string(),
        TypeValidationError::UnknownType { expected_type } => {
            format!("unknown type '{expected_type}'")
        }
        TypeValidationError::Mismatch {
            expected_type,
            actual_type,
        } => format!("expected '{expected_type}', got '{actual_type}'"),
        TypeValidationError::MissingRequiredField {
            type_name,
            field_name,
        } => format!("type '{type_name}' requires field '{field_name}'"),
        TypeValidationError::UnknownField {
            type_name,
            field_name,
        } => format!("type '{type_name}' has no field '{field_name}'"),
    }
}

pub(super) fn handle_expr_stmt(
    expr: &Expr,
    state: &mut ProgramState,
    context: &mut RuntimeContext,
    w: &mut dyn std::io::Write,
) -> Flow {
    if let Expr::MethodCall(call) = expr {
        if super::super::builtins::is_method_mutating(&call.method) {
            return handle_mut_method_call(expr, call, state, context, w);
        }
    }
    match check_eval_result(eval_expr(expr, state, context, w, false)) {
        Ok(_) => Flow::Normal,
        Err(err) => Flow::Err(err),
    }
}

fn handle_mut_method_call(
    _e: &Expr,
    call: &MethodCall,
    state: &mut ProgramState,
    context: &mut RuntimeContext,
    w: &mut dyn std::io::Write,
) -> Flow {
    let var_name = match &*call.object {
        Expr::VarLookup(v) => v.name.to_string(),
        _ => {
            return Flow::Err(Error::Interpreter(format!(
                "mutable method '{}' requires a direct variable as receiver",
                call.method
            )));
        }
    };

    let mut arg_vals = Vec::new();
    for arg in &call.args {
        match eval_expr(arg, state, context, w, false) {
            Ok(v) => arg_vals.push(v),
            Err(err) => return Flow::Err(err),
        }
    }

    if let Err(error) = validate_list_method_item_type(
        &var_name,
        &call.method,
        &arg_vals,
        state,
        context,
        call.span,
    ) {
        return Flow::Err(error);
    }

    state.invalidate_env_snapshot();
    let receiver = match state.get_env_mut(&var_name) {
        Some(r) => r,
        None => {
            return Flow::Err(Error::Interpreter(format!(
                "variable '{}' is not defined",
                var_name
            )));
        }
    };

    match super::super::builtins::dispatch_mut(receiver, &call.method, &arg_vals) {
        Ok(_) => Flow::Normal,
        Err(err) => Flow::Err(err),
    }
}
