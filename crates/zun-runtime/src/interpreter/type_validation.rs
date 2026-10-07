use indexmap::IndexMap;

use crate::ast::TypeExpr;
use crate::runtime::RuntimeContext;

use super::value::ZunValue;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeValidationError {
    BareList,
    UnsupportedOptionalListItem,
    ExpectedListClose,
    UnknownType {
        expected_type: String,
    },
    Mismatch {
        expected_type: String,
        actual_type: String,
    },
    MissingRequiredField {
        type_name: String,
        field_name: String,
    },
    UnknownField {
        type_name: String,
        field_name: String,
    },
}

pub fn validate_value_against_type_expr(
    expected: &TypeExpr,
    value: &ZunValue,
    context: &RuntimeContext,
) -> Result<(), TypeValidationError> {
    match expected {
        TypeExpr::Named(name) => validate_named_type(name, value, context),
        TypeExpr::List(inner) => {
            let ZunValue::List(items) = value else {
                return Err(TypeValidationError::Mismatch {
                    expected_type: type_expr_to_string(expected),
                    actual_type: value.type_name().into_owned(),
                });
            };

            for item in items {
                validate_value_against_type_expr(inner, item, context)?;
            }

            Ok(())
        }
        TypeExpr::Optional(inner) => {
            if matches!(value, ZunValue::Null) {
                Ok(())
            } else {
                validate_value_against_type_expr(inner, value, context)
            }
        }
    }
}

pub fn validate_typed_instance_fields(
    type_name: &str,
    fields: &IndexMap<String, ZunValue>,
    context: &RuntimeContext,
) -> Result<(), TypeValidationError> {
    let shape = context
        .get_type(type_name)
        .ok_or_else(|| TypeValidationError::UnknownType {
            expected_type: type_name.to_string(),
        })?;

    for (stored_key, value) in fields {
        let field_name = stored_key
            .strip_prefix('_')
            .unwrap_or(stored_key.as_str())
            .to_string();
        let Some(field) = shape.fields.iter().find(|field| field.name == field_name) else {
            return Err(TypeValidationError::UnknownField {
                type_name: type_name.to_string(),
                field_name,
            });
        };

        validate_value_against_type_expr(&field.type_expr, value, context)?;
    }

    for field in &shape.fields {
        let stored_name = if field.hidden {
            format!("_{}", field.name)
        } else {
            field.name.clone()
        };
        let present = fields.contains_key(&stored_name);
        let optional = matches!(field.type_expr, TypeExpr::Optional(_));
        if !optional && !present {
            return Err(TypeValidationError::MissingRequiredField {
                type_name: type_name.to_string(),
                field_name: field.name.clone(),
            });
        }
    }

    Ok(())
}

fn validate_named_type(
    expected_type: &str,
    value: &ZunValue,
    context: &RuntimeContext,
) -> Result<(), TypeValidationError> {
    let normalized = expected_type.to_ascii_lowercase();
    match normalized.as_str() {
        "int" | "integer" => expect_match("Int", value, matches!(value, ZunValue::Int(_))),
        "float" => expect_match(
            "Float",
            value,
            matches!(value, ZunValue::Float(_) | ZunValue::Int(_)),
        ),
        "string" | "str" => expect_match("String", value, matches!(value, ZunValue::Str(_))),
        "bool" | "boolean" => expect_match("Bool", value, matches!(value, ZunValue::Bool(_))),
        "map" => expect_match("Map", value, matches!(value, ZunValue::Map(_))),
        "response" => expect_match(
            "Response",
            value,
            matches!(value, ZunValue::Response { .. }),
        ),
        "json" => {
            if let ZunValue::Response {
                body: Some(body), ..
            } = value
            {
                return validate_named_type("Json", body.as_ref(), context);
            }

            expect_match(
                "Json",
                value,
                matches!(
                    value,
                    ZunValue::Json(_) | ZunValue::Map(_) | ZunValue::TypedInstance { .. }
                ),
            )
        }
        "html" => expect_match(
            "HTML",
            value,
            matches!(value, ZunValue::Html(_) | ZunValue::Str(_)),
        ),
        "list" => Err(TypeValidationError::BareList),
        _ => {
            if context.get_type(expected_type).is_none() {
                return Err(TypeValidationError::UnknownType {
                    expected_type: expected_type.to_string(),
                });
            }

            match value {
                ZunValue::TypedInstance { type_name, .. } if type_name == expected_type => Ok(()),
                _ => Err(TypeValidationError::Mismatch {
                    expected_type: expected_type.to_string(),
                    actual_type: value.type_name().into_owned(),
                }),
            }
        }
    }
}

fn expect_match(
    expected_type: &str,
    value: &ZunValue,
    ok: bool,
) -> Result<(), TypeValidationError> {
    if ok {
        Ok(())
    } else {
        Err(TypeValidationError::Mismatch {
            expected_type: expected_type.to_string(),
            actual_type: value.type_name().into_owned(),
        })
    }
}

fn type_expr_to_string(type_expr: &TypeExpr) -> String {
    match type_expr {
        TypeExpr::Named(name) => name.clone(),
        TypeExpr::List(inner) => format!("List<{}>", type_expr_to_string(inner)),
        TypeExpr::Optional(inner) => format!("{}?", type_expr_to_string(inner)),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::runtime::{
        RuntimeMode,
        context::{TypeField, TypeShape},
    };

    fn test_context() -> RuntimeContext {
        RuntimeContext::new(RuntimeMode::Test, PathBuf::from("."))
    }

    #[test]
    fn validate_value_against_type_expr_rejects_map_for_user_type() {
        let mut context = test_context();
        context.register_type(
            "User",
            TypeShape {
                name: "User".to_string(),
                fields: vec![],
            },
        );

        let value = ZunValue::Map(IndexMap::new());
        let ty = TypeExpr::Named("User".to_string());
        let error = validate_value_against_type_expr(&ty, &value, &context)
            .expect_err("map should not satisfy User");

        assert_eq!(
            error,
            TypeValidationError::Mismatch {
                expected_type: "User".to_string(),
                actual_type: "Map".to_string(),
            }
        );
    }

    #[test]
    fn validate_optional_string_accepts_null() {
        let context = test_context();
        let ty = TypeExpr::Optional(Box::new(TypeExpr::Named("String".into())));
        assert!(validate_value_against_type_expr(&ty, &ZunValue::Null, &context).is_ok());
    }

    #[test]
    fn validate_typed_instance_fields_requires_non_optional_fields() {
        let mut context = test_context();
        context.register_type(
            "User",
            TypeShape {
                name: "User".to_string(),
                fields: vec![TypeField {
                    name: "id".to_string(),
                    type_expr: TypeExpr::Named("Int".to_string()),
                    hidden: false,
                }],
            },
        );

        let error = validate_typed_instance_fields("User", &IndexMap::new(), &context)
            .expect_err("missing required field should fail");

        assert_eq!(
            error,
            TypeValidationError::MissingRequiredField {
                type_name: "User".to_string(),
                field_name: "id".to_string(),
            }
        );
    }

    #[test]
    fn validate_list_of_user_instances_accepts_matching_items() {
        let mut context = test_context();
        context.register_type(
            "User",
            TypeShape {
                name: "User".to_string(),
                fields: vec![],
            },
        );

        let value = ZunValue::List(vec![ZunValue::TypedInstance {
            type_name: "User".to_string(),
            fields: IndexMap::new(),
        }]);

        let ty = TypeExpr::List(Box::new(TypeExpr::Named("User".to_string())));
        assert!(validate_value_against_type_expr(&ty, &value, &context).is_ok());
    }
}
