use indexmap::IndexMap;

use crate::ast::{Expr, HtmlConstructor, JsonConstructor, ResConstructor, StructConstructor};
use crate::diagnostics::codes;
use crate::error::Error;
use crate::interpreter::{TypeValidationError, validate_typed_instance_fields};
use crate::runtime::{ProgramState, RuntimeContext};

use super::super::value::ZunValue;
use super::eval_expr;

pub fn eval_json_constructor(
    json: &JsonConstructor,
    state: &mut ProgramState,
    context: &mut RuntimeContext,
    w: &mut dyn std::io::Write,
) -> Result<ZunValue, Error> {
    let mut map: IndexMap<String, ZunValue> = IndexMap::new();
    for (key, value_expr) in &json.entries {
        let val = eval_expr(value_expr, state, context, w, false)?;
        map.insert(key.clone(), val);
    }
    Ok(ZunValue::Json(map))
}

pub fn eval_html_constructor(
    html: &HtmlConstructor,
    state: &mut ProgramState,
    context: &mut RuntimeContext,
    w: &mut dyn std::io::Write,
) -> Result<ZunValue, Error> {
    let val = eval_expr(&html.content, state, context, w, false)?;
    Ok(ZunValue::Html(Box::new(val)))
}

pub fn eval_res_constructor(
    e: &Expr,
    res: &ResConstructor,
    state: &mut ProgramState,
    context: &mut RuntimeContext,
    w: &mut dyn std::io::Write,
) -> Result<ZunValue, Error> {
    let status_val = match eval_expr(&res.status, state, context, w, false) {
        Ok(ZunValue::Int(n)) => n as u16,
        _ => {
            return Err(super::runtime_error(
                e,
                codes::RUNTIME_GENERIC,
                "$RES status must be an integer",
            ));
        }
    };

    let body_val = if let Some(body_expr) = &res.body {
        Some(Box::new(eval_expr(body_expr, state, context, w, false)?))
    } else {
        None
    };

    Ok(ZunValue::Response {
        status: status_val,
        body: body_val,
    })
}

pub fn eval_struct_constructor(
    ctor: &StructConstructor,
    state: &mut ProgramState,
    context: &mut RuntimeContext,
    w: &mut dyn std::io::Write,
) -> Result<ZunValue, Error> {
    let shape = context.get_type(&ctor.type_name).cloned();
    let Some(shape) = shape else {
        return Err(Error::Diagnostic(
            crate::diagnostics::Diagnostic::error(
                codes::RUNTIME_FIELD_NOT_FOUND,
                format!("type '{}' is not defined", ctor.type_name),
            )
            .with_span(ctor.span),
        ));
    };

    let mut fields: IndexMap<String, ZunValue> = IndexMap::new();
    for (key, value_expr) in &ctor.fields {
        let val = eval_expr(value_expr, state, context, w, false)?;
        let Some(field_def) = shape.fields.iter().find(|field| field.name == *key) else {
            return Err(type_validation_error(
                TypeValidationError::UnknownField {
                    type_name: ctor.type_name.clone(),
                    field_name: key.clone(),
                },
                ctor.span,
            ));
        };
        let stored_key = if field_def.hidden {
            format!("_{}", key)
        } else {
            key.clone()
        };
        fields.insert(stored_key, val);
    }

    validate_typed_instance_fields(&ctor.type_name, &fields, context)
        .map_err(|error| type_validation_error(error, ctor.span))?;

    Ok(ZunValue::TypedInstance {
        type_name: ctor.type_name.clone(),
        fields,
    })
}

fn type_validation_error(error: TypeValidationError, span: crate::ast::Span) -> Error {
    let message = match error {
        TypeValidationError::BareList => {
            "typed fields must declare 'List<T>' instead of bare 'List'".to_string()
        }
        TypeValidationError::UnsupportedOptionalListItem => {
            "optional list item types are not supported; use 'List<T>' or 'List<T>?'".to_string()
        }
        TypeValidationError::ExpectedListClose => "expected '>' after list item type".to_string(),
        TypeValidationError::UnknownType { expected_type } => {
            format!("type '{expected_type}' is not defined")
        }
        TypeValidationError::Mismatch {
            expected_type,
            actual_type,
        } => format!("expects '{expected_type}', got '{actual_type}'"),
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
            .with_span(span),
    )
}
