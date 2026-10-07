use crate::ast::{Expr, FnCallExpr, MethodCall};
use crate::diagnostics::codes;
use crate::error::Error;
use crate::runtime::{ProgramState, RuntimeContext};

use super::super::value::ZunValue;
use super::eval_expr;

pub fn eval_method_call(
    e: &Expr,
    call: &MethodCall,
    state: &mut ProgramState,
    context: &mut RuntimeContext,
    w: &mut dyn std::io::Write,
) -> Result<ZunValue, Error> {
    let obj_val = eval_expr(&call.object, state, context, w, false)?;

    if let ZunValue::ModuleProxy {
        state: module_state,
        ..
    } = &obj_val
    {
        let method_name = &call.method;
        let exports = &module_state.exports;
        let module_fns = &module_state.fns;
        let native_exports = &module_state.native_exports;
        let module_env = module_state.module_env.as_ref();

        let mut arg_vals = Vec::new();
        for arg in &call.args {
            arg_vals.push(eval_expr(arg, state, context, w, false)?);
        }

        if let Some(fn_def) = exports.get(method_name) {
            let fn_def = fn_def.clone();
            return match super::super::exec::call_module_fn(
                &fn_def,
                &arg_vals,
                &mut state.fns,
                module_fns,
                module_env,
                context,
                w,
            ) {
                Ok(Some(val)) => Ok(val),
                Ok(None) => Ok(ZunValue::Null),
                Err(err) => Err(err),
            };
        } else if let Some(native_fn) = native_exports.get(method_name).cloned() {
            return native_fn(&arg_vals, context)
                .map_err(|err| super::runtime_error(e, codes::RUNTIME_GENERIC, err.to_string()));
        } else if call.args.is_empty() {
            // Sub-module property access: services.auth returns the auth ModuleProxy
            // so that chained calls like services.auth.login() work correctly.
            if let Some(sub_module) = module_env.get(method_name) {
                return Ok(sub_module.clone());
            }
            return Err(super::runtime_error(
                e,
                codes::RUNTIME_MODULE_LOAD,
                format!("module function or sub-module '{method_name}' not found"),
            ));
        } else {
            return Err(super::runtime_error(
                e,
                codes::RUNTIME_MODULE_LOAD,
                format!("module function '{method_name}' not found"),
            ));
        }
    }

    let mut arg_vals = Vec::new();
    for arg in &call.args {
        arg_vals.push(eval_expr(arg, state, context, w, false)?);
    }

    if super::super::builtins::is_method_mutating(&call.method) {
        return Err(super::runtime_error(
            e,
            codes::RUNTIME_GENERIC,
            format!("mutable method '{}' not yet supported", call.method),
        ));
    }

    // TypedInstance field access: instance.field_name (zero-arg)
    if let ZunValue::TypedInstance { type_name, fields } = &obj_val {
        if call.args.is_empty() {
            if let Some(value) = fields.get(&call.method) {
                return Ok(value.clone());
            }
            // @HIDE fields are stored with "_" prefix; allow access via clean name
            let hidden_key = format!("_{}", call.method);
            if let Some(value) = fields.get(&hidden_key) {
                return Ok(value.clone());
            }
            // Field declared in TypeShape but not yet set → return Null (uninitialized)
            if let Some(shape) = context.get_type(type_name) {
                if shape.fields.iter().any(|f| f.name == call.method) {
                    return Ok(ZunValue::Null);
                }
            }
            return Err(super::runtime_error(
                e,
                codes::RUNTIME_INVALID_FIELD_ACCESS,
                format!("type '{}' has no field '{}'", type_name, call.method),
            ));
        }
    }

    super::super::builtins::dispatch(&obj_val, &call.method, &arg_vals, context)
}

pub fn eval_fn_call(
    e: &Expr,
    call: &FnCallExpr,
    state: &mut ProgramState,
    context: &mut RuntimeContext,
    w: &mut dyn std::io::Write,
) -> Result<ZunValue, Error> {
    let mut arg_vals = Vec::new();
    for arg in &call.args {
        arg_vals.push(eval_expr(arg, state, context, w, false)?);
    }

    let fn_name = if let Some(var_val) = state.lookup_env(&call.name) {
        match var_val {
            ZunValue::Str(s) => s.clone(),
            _ => {
                return Err(super::runtime_error(
                    e,
                    codes::RUNTIME_INVALID_EXPRESSION,
                    format!("'{}' is not a callable function name", call.name),
                ));
            }
        }
    } else {
        call.name.clone()
    };

    if let Some(fn_def) = state.lookup_fn(&fn_name) {
        match super::super::exec::call_fn(&fn_def, &arg_vals, &mut state.fns, context, w) {
            Ok(Some(val)) => Ok(val),
            Ok(None) => Ok(ZunValue::Null),
            Err(err) => Err(err),
        }
    } else if let Some(native_fn) = context.get_native_fn(&fn_name).cloned() {
        native_fn(&arg_vals, context)
            .map_err(|err| super::runtime_error(e, codes::RUNTIME_GENERIC, err.to_string()))
    } else {
        Err(super::undefined_variable_error(e, &fn_name))
    }
}
