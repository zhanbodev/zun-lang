use crate::ast::{FnDeclStmt, FnParam, TypeExpr};
use crate::error::Error;
use crate::interpreter::{TypeValidationError, type_expr_name, validate_value_against_type_expr};
use crate::runtime::{ProgramState, RuntimeContext};

use super::super::env::{FnEnv, RuntimeFn};
use super::super::value::ZunValue;
use super::{Flow, exec_block};

pub(super) fn handle_fn_decl(stmt: &FnDeclStmt, state: &mut ProgramState) -> Flow {
    if state.lookup_fn(&stmt.name).is_some() {
        return Flow::Err(Error::Interpreter(format!(
            "function '{}' is already defined",
            stmt.name
        )));
    }
    let module_env = state.capture_env_snapshot();
    state.fns.insert(
        stmt.name.clone(),
        RuntimeFn {
            decl: stmt.clone(),
            source_file: None,
            module_env,
        },
    );
    Flow::Normal
}

pub fn call_fn(
    fn_def: &RuntimeFn,
    args: &[ZunValue],
    fns: &mut FnEnv,
    context: &mut RuntimeContext,
    w: &mut dyn std::io::Write,
) -> Result<Option<ZunValue>, Error> {
    let fn_decl = &fn_def.decl;
    let min_params = fn_decl.params.len();
    let has_variadic = fn_decl.variadic_param.is_some();
    if has_variadic && args.len() < min_params {
        return Err(Error::Interpreter(format!(
            "function '{}' expects at least {} arguments, got {}",
            fn_decl.name,
            min_params,
            args.len()
        )));
    }
    if !has_variadic && args.len() != min_params {
        return Err(Error::Interpreter(format!(
            "function '{}' expects {} arguments, got {}",
            fn_decl.name,
            min_params,
            args.len()
        )));
    }

    let mut local_state = ProgramState::new();
    local_state.set_env_fallback(fn_def.module_env.clone());

    bind_typed_params(&fn_decl.params, args, &mut local_state, context)?;

    if let Some(var_param) = &fn_decl.variadic_param {
        let extra_args: Vec<ZunValue> = args[min_params..].to_vec();
        local_state.insert_env(var_param.clone(), ZunValue::List(extra_args));
    }

    local_state.fns = fns.clone();

    let flow = exec_block(&fn_decl.body, &mut local_state, context, w);
    *fns = local_state.fns.clone();
    match flow {
        Flow::Return(val) => {
            validate_declared_return_type(
                "function",
                &fn_decl.name,
                fn_decl.return_type.as_ref(),
                val.as_ref(),
                context,
            )?;
            Ok(val)
        }
        Flow::Normal => Ok(None),
        Flow::Err(err) => Err(err),
        Flow::Exit => Ok(None),
        Flow::Throw(val) => Err(Error::Interpreter(format!("uncaught throw: {val}"))),
        Flow::Break | Flow::Continue => Err(Error::Interpreter(
            "break/continue used outside of loop".to_string(),
        )),
    }
}

/// 调用模块内的 Zun 函数，预注入模块执行时的环境（已导入的模块代理、常量等）。
pub fn call_module_fn(
    fn_def: &RuntimeFn,
    args: &[ZunValue],
    caller_fns: &mut FnEnv,
    module_fns: &FnEnv,
    module_env: &crate::interpreter::env::Env,
    context: &mut RuntimeContext,
    w: &mut dyn std::io::Write,
) -> Result<Option<ZunValue>, Error> {
    let fn_decl = &fn_def.decl;
    let min_params = fn_decl.params.len();
    let has_variadic = fn_decl.variadic_param.is_some();
    if has_variadic && args.len() < min_params {
        return Err(Error::Interpreter(format!(
            "function '{}' expects at least {} arguments, got {}",
            fn_decl.name,
            min_params,
            args.len()
        )));
    }
    if !has_variadic && args.len() != min_params {
        return Err(Error::Interpreter(format!(
            "function '{}' expects {} arguments, got {}",
            fn_decl.name,
            min_params,
            args.len()
        )));
    }

    let mut local_state = ProgramState::new();

    // 模块环境作为只读 fallback，本地绑定按需覆写。
    local_state.set_env_fallback(std::sync::Arc::new(module_env.clone()));

    // 参数绑定（覆盖同名的模块环境变量）
    bind_typed_params(&fn_decl.params, args, &mut local_state, context)?;
    if let Some(var_param) = &fn_decl.variadic_param {
        let extra = args[min_params..].to_vec();
        local_state.insert_env(var_param.clone(), ZunValue::List(extra));
    }

    local_state.fns = module_fns.clone();
    local_state.set_fn_fallback(caller_fns.clone());

    let flow = exec_block(&fn_decl.body, &mut local_state, context, w);

    match flow {
        Flow::Return(val) => {
            validate_declared_return_type(
                "function",
                &fn_decl.name,
                fn_decl.return_type.as_ref(),
                val.as_ref(),
                context,
            )?;
            Ok(val)
        }
        Flow::Normal => Ok(None),
        Flow::Err(err) => Err(err),
        Flow::Exit => Ok(None),
        Flow::Throw(val) => Err(Error::Interpreter(format!("uncaught throw: {val}"))),
        Flow::Break | Flow::Continue => Err(Error::Interpreter(
            "break/continue used outside of loop".to_string(),
        )),
    }
}

pub fn validate_declared_return_type(
    kind: &str,
    name: &str,
    expected_type: Option<&TypeExpr>,
    value: Option<&ZunValue>,
    context: &RuntimeContext,
) -> Result<(), Error> {
    let Some(expected_type) = expected_type else {
        return Ok(());
    };

    let Some(value) = value else {
        return Err(Error::Interpreter(format!(
            "{kind} '{name}' expects return type '{}' but returned nothing",
            type_expr_name(expected_type)
        )));
    };

    match validate_value_against_type_expr(expected_type, value, context) {
        Ok(()) => Ok(()),
        Err(TypeValidationError::BareList) => Err(Error::Interpreter(format!(
            "{kind} '{name}' declares return type 'List' without a type parameter; use 'List<T>' instead (e.g. 'List<User>')"
        ))),
        Err(TypeValidationError::UnsupportedOptionalListItem) => Err(Error::Interpreter(format!(
            "{kind} '{name}' uses unsupported return type '{}'; 'List<T?>' is not supported",
            type_expr_name(expected_type)
        ))),
        Err(TypeValidationError::ExpectedListClose) => Err(Error::Interpreter(format!(
            "{kind} '{name}' declares invalid return type '{}'",
            type_expr_name(expected_type)
        ))),
        Err(TypeValidationError::UnknownType { expected_type }) => Err(Error::Interpreter(
            format!("{kind} '{name}' references unknown return type '{expected_type}'"),
        )),
        Err(TypeValidationError::Mismatch {
            expected_type,
            actual_type,
        }) => Err(Error::Interpreter(format!(
            "{kind} '{name}' expects return type '{expected_type}' but got '{actual_type}'"
        ))),
        Err(TypeValidationError::MissingRequiredField { .. })
        | Err(TypeValidationError::UnknownField { .. }) => Err(Error::Interpreter(format!(
            "{kind} '{name}' expects return type '{}' but got '{}'",
            type_expr_name(expected_type),
            value.type_name()
        ))),
    }
}

pub(crate) fn bind_typed_params(
    params: &[FnParam],
    args: &[ZunValue],
    state: &mut ProgramState,
    context: &RuntimeContext,
) -> Result<(), Error> {
    for (param, arg) in params.iter().zip(args.iter()) {
        if let Some(type_expr) = &param.type_annotation {
            validate_value_against_type_expr(type_expr, arg, context)
                .map_err(|error| parameter_type_error(&param.name, type_expr, error))?;
        }
        state.insert_env(param.name.clone(), arg.clone());
    }

    Ok(())
}

pub(crate) fn parameter_type_error(
    param_name: &str,
    type_expr: &TypeExpr,
    error: TypeValidationError,
) -> Error {
    let message = match error {
        TypeValidationError::Mismatch { actual_type, .. } => format!(
            "parameter '{param_name}' expects type '{}', got '{actual_type}'",
            type_expr_name(type_expr)
        ),
        other => format!(
            "parameter '{param_name}' expects type '{}': {}",
            type_expr_name(type_expr),
            type_validation_detail(other)
        ),
    };

    Error::Interpreter(message)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{
        RuntimeMode,
        context::{TypeField, TypeShape},
    };
    use indexmap::IndexMap;
    use std::path::PathBuf;

    fn test_context() -> RuntimeContext {
        RuntimeContext::new(RuntimeMode::Test, PathBuf::from("."))
    }

    #[test]
    fn shared_return_type_validator_accepts_matching_int() {
        let context = test_context();
        let value = ZunValue::Int(1);
        let expected = TypeExpr::Named("Int".to_string());

        assert!(
            validate_declared_return_type("function", "f", Some(&expected), Some(&value), &context,)
                .is_ok()
        );
    }

    #[test]
    fn shared_return_type_validator_rejects_missing_return_value() {
        let context = test_context();
        let expected = TypeExpr::Named("Int".to_string());
        let error = validate_declared_return_type("function", "f", Some(&expected), None, &context)
            .expect_err("missing return should fail");
        assert!(error.to_string().contains("returned nothing"));
    }

    #[test]
    fn shared_return_type_validator_rejects_wrong_user_type() {
        let mut context = test_context();
        context.register_type(
            "User",
            TypeShape {
                name: "User".to_string(),
                fields: vec![TypeField {
                    name: "id".to_string(),
                    type_expr: crate::ast::TypeExpr::Named("Int".to_string()),
                    hidden: false,
                }],
            },
        );

        let value = ZunValue::Map(IndexMap::new());
        let expected = TypeExpr::Named("User".to_string());

        let error =
            validate_declared_return_type("function", "f", Some(&expected), Some(&value), &context)
                .expect_err("wrong user type should fail");
        assert!(error.to_string().contains("expects return type 'User'"));
    }

    #[test]
    fn shared_return_type_validator_accepts_list_of_user_instances() {
        let mut context = test_context();
        context.register_type(
            "User",
            TypeShape {
                name: "User".to_string(),
                fields: vec![TypeField {
                    name: "id".to_string(),
                    type_expr: crate::ast::TypeExpr::Named("Int".to_string()),
                    hidden: false,
                }],
            },
        );

        let value = ZunValue::List(vec![ZunValue::TypedInstance {
            type_name: "User".to_string(),
            fields: IndexMap::new(),
        }]);
        let expected = TypeExpr::List(Box::new(TypeExpr::Named("User".to_string())));

        assert!(
            validate_declared_return_type(
                "function",
                "f",
                Some(&expected),
                Some(&value),
                &context,
            )
            .is_ok()
        );
    }
}
