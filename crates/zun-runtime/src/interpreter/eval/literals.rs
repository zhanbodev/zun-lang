use indexmap::IndexMap;
use std::sync::Arc;

use crate::ast::{
    BoolLiteral, CharLiteral, FStringLiteral, FnLiteral, ListLiteral, MapLiteral, NumberLiteral,
    StringLiteral, VarLookup,
};
use crate::error::Error;
use crate::runtime::{ProgramState, RuntimeContext};

use super::super::env::{RuntimeFn, generate_fn_name};
use super::super::value::ZunValue;
use super::eval_expr;

pub fn eval_number(n: &NumberLiteral) -> Result<ZunValue, Error> {
    let is_float = n.value.contains('.') || n.value.contains('e') || n.value.contains('E');
    if let Ok(val) = n.value.parse::<f64>() {
        if is_float {
            Ok(ZunValue::Float(val))
        } else if val.fract() == 0.0 && val.is_finite() {
            Ok(ZunValue::Int(val as i64))
        } else {
            Ok(ZunValue::Float(val))
        }
    } else {
        Ok(ZunValue::Str(n.value.as_ref().to_string()))
    }
}

pub fn eval_char(c: &CharLiteral) -> Result<ZunValue, Error> {
    Ok(ZunValue::Str(c.value.as_ref().to_string()))
}

pub fn eval_bool(b: &BoolLiteral) -> Result<ZunValue, Error> {
    Ok(ZunValue::Bool(b.value))
}

pub fn eval_string_literal(s: &StringLiteral) -> Result<ZunValue, Error> {
    Ok(ZunValue::Str(s.value.as_ref().to_string()))
}

pub fn eval_fstring(
    fs: &FStringLiteral,
    state: &mut ProgramState,
    context: &mut RuntimeContext,
    w: &mut dyn std::io::Write,
) -> Result<ZunValue, Error> {
    let mut result = String::new();
    for segment in &fs.segments {
        match segment {
            crate::ast::FStringSegment::Literal(s) => result.push_str(s),
            crate::ast::FStringSegment::Expression(expr) => {
                let val = eval_expr(expr, state, context, w, false)?;
                result.push_str(&val.to_string());
            }
        }
    }
    Ok(ZunValue::Str(result))
}

pub fn eval_list_literal(
    list: &ListLiteral,
    state: &mut ProgramState,
    context: &mut RuntimeContext,
    w: &mut dyn std::io::Write,
) -> Result<ZunValue, Error> {
    let mut elements: Vec<ZunValue> = Vec::new();
    for elem in &list.elements {
        elements.push(eval_expr(elem, state, context, w, false)?);
    }
    Ok(ZunValue::List(elements))
}

pub fn eval_map_literal(
    map: &MapLiteral,
    state: &mut ProgramState,
    context: &mut RuntimeContext,
    w: &mut dyn std::io::Write,
) -> Result<ZunValue, Error> {
    let mut entries: IndexMap<String, ZunValue> = IndexMap::new();
    for (key, value_expr) in &map.entries {
        let val = eval_expr(value_expr, state, context, w, false)?;
        entries.insert(key.clone(), val);
    }
    Ok(ZunValue::Map(entries))
}

pub fn eval_var_lookup(
    e: &crate::ast::Expr,
    v: &VarLookup,
    state: &ProgramState,
    as_identifier: bool,
) -> Result<ZunValue, Error> {
    if as_identifier {
        Ok(ZunValue::Str(v.name.as_ref().to_string()))
    } else {
        let name = v.name.as_ref();
        state
            .lookup_env(name)
            .cloned()
            .ok_or_else(|| super::undefined_variable_error(e, name))
    }
}

pub fn eval_fn_literal(lit: &FnLiteral, state: &mut ProgramState) -> Result<ZunValue, Error> {
    let fn_name = generate_fn_name();
    let fn_decl = crate::ast::FnDeclStmt {
        span: lit.span,
        name: fn_name.clone(),
        is_public: false,
        params: lit.params.clone(),
        variadic_param: lit.variadic_param.clone(),
        return_type: lit.return_type.clone(),
        body: lit.body.clone(),
    };
    state.fns.insert(
        fn_name.clone(),
        RuntimeFn {
            decl: fn_decl,
            source_file: None,
            module_env: Arc::new(state.visible_env_snapshot()),
        },
    );
    Ok(ZunValue::Str(fn_name))
}
