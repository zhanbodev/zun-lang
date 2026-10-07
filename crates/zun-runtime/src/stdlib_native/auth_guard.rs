use std::sync::Arc;

use indexmap::IndexMap;

use crate::error::Error;
use crate::interpreter::ZunValue;
use crate::runtime::intrinsics::intrinsic_string_arg;
use crate::runtime::{NativeFnMap, RuntimeContext};

pub fn register(context: &mut RuntimeContext) {
    let mut exports = NativeFnMap::new();
    exports.insert(
        "principal".into(),
        Arc::new(|_args: &[ZunValue], ctx: &RuntimeContext| {
            let principal = ctx.with_request_auth_context(|auth| auth.principal().cloned())?;
            Ok(principal
                .map(|principal| principal_to_value(&principal))
                .unwrap_or(ZunValue::Null))
        }),
    );
    exports.insert(
        "authenticated".into(),
        Arc::new(|_args: &[ZunValue], ctx: &RuntimeContext| {
            Ok(ZunValue::Bool(ctx.with_request_auth_context(|auth| {
                auth.principal().is_some()
            })?))
        }),
    );
    exports.insert(
        "has_role".into(),
        Arc::new(|args: &[ZunValue], ctx: &RuntimeContext| {
            let role = intrinsic_string_arg("auth.guard.has_role", args, 0)?;
            Ok(ZunValue::Bool(has_role(ctx, role)?))
        }),
    );
    exports.insert(
        "has_permission".into(),
        Arc::new(|args: &[ZunValue], ctx: &RuntimeContext| {
            let permission = intrinsic_string_arg("auth.guard.has_permission", args, 0)?;
            Ok(ZunValue::Bool(has_permission(ctx, permission)?))
        }),
    );
    exports.insert(
        "has_any_role".into(),
        Arc::new(|args: &[ZunValue], ctx: &RuntimeContext| {
            let roles = string_list_arg("auth.guard.has_any_role", args, 0)?;
            Ok(ZunValue::Bool(has_any_role(ctx, &roles)?))
        }),
    );
    exports.insert(
        "has_all_roles".into(),
        Arc::new(|args: &[ZunValue], ctx: &RuntimeContext| {
            let roles = string_list_arg("auth.guard.has_all_roles", args, 0)?;
            Ok(ZunValue::Bool(has_all_roles(ctx, &roles)?))
        }),
    );
    exports.insert(
        "has_any_permission".into(),
        Arc::new(|args: &[ZunValue], ctx: &RuntimeContext| {
            let permissions = string_list_arg("auth.guard.has_any_permission", args, 0)?;
            Ok(ZunValue::Bool(has_any_permission(ctx, &permissions)?))
        }),
    );
    exports.insert(
        "has_all_permissions".into(),
        Arc::new(|args: &[ZunValue], ctx: &RuntimeContext| {
            let permissions = string_list_arg("auth.guard.has_all_permissions", args, 0)?;
            Ok(ZunValue::Bool(has_all_permissions(ctx, &permissions)?))
        }),
    );
    exports.insert(
        "require_role".into(),
        Arc::new(|args: &[ZunValue], ctx: &RuntimeContext| {
            let role = intrinsic_string_arg("auth.guard.require_role", args, 0)?;
            if has_role(ctx, role)? {
                Ok(ZunValue::Null)
            } else {
                Err(auth_forbidden_error(format!(
                    "auth.guard.require_role: missing required role '{role}'"
                )))
            }
        }),
    );
    exports.insert(
        "require_any_role".into(),
        Arc::new(|args: &[ZunValue], ctx: &RuntimeContext| {
            let roles = string_list_arg("auth.guard.require_any_role", args, 0)?;
            if has_any_role(ctx, &roles)? {
                Ok(ZunValue::Null)
            } else {
                Err(auth_forbidden_error(format!(
                    "auth.guard.require_any_role: missing any required role from [{}]",
                    roles.join(", ")
                )))
            }
        }),
    );
    exports.insert(
        "require_all_roles".into(),
        Arc::new(|args: &[ZunValue], ctx: &RuntimeContext| {
            let roles = string_list_arg("auth.guard.require_all_roles", args, 0)?;
            if has_all_roles(ctx, &roles)? {
                Ok(ZunValue::Null)
            } else {
                Err(auth_forbidden_error(format!(
                    "auth.guard.require_all_roles: missing one of required roles from [{}]",
                    roles.join(", ")
                )))
            }
        }),
    );
    exports.insert(
        "require_permission".into(),
        Arc::new(|args: &[ZunValue], ctx: &RuntimeContext| {
            let permission = intrinsic_string_arg("auth.guard.require_permission", args, 0)?;
            if has_permission(ctx, permission)? {
                Ok(ZunValue::Null)
            } else {
                Err(auth_forbidden_error(format!(
                    "auth.guard.require_permission: missing required permission '{permission}'"
                )))
            }
        }),
    );
    exports.insert(
        "require_any_permission".into(),
        Arc::new(|args: &[ZunValue], ctx: &RuntimeContext| {
            let permissions = string_list_arg("auth.guard.require_any_permission", args, 0)?;
            if has_any_permission(ctx, &permissions)? {
                Ok(ZunValue::Null)
            } else {
                Err(auth_forbidden_error(format!(
                    "auth.guard.require_any_permission: missing any required permission from [{}]",
                    permissions.join(", ")
                )))
            }
        }),
    );
    exports.insert(
        "require_all_permissions".into(),
        Arc::new(|args: &[ZunValue], ctx: &RuntimeContext| {
            let permissions = string_list_arg("auth.guard.require_all_permissions", args, 0)?;
            if has_all_permissions(ctx, &permissions)? {
                Ok(ZunValue::Null)
            } else {
                Err(auth_forbidden_error(format!(
                    "auth.guard.require_all_permissions: missing one of required permissions from [{}]",
                    permissions.join(", ")
                )))
            }
        }),
    );
    context.register_native_module("std.auth.guard", exports);
}

fn has_role(ctx: &RuntimeContext, role: &str) -> Result<bool, Error> {
    ctx.with_request_auth_context(|auth| {
        auth.principal()
            .map(|principal| principal.roles.iter().any(|value| value == role))
            .unwrap_or(false)
    })
}

fn has_permission(ctx: &RuntimeContext, permission: &str) -> Result<bool, Error> {
    ctx.with_request_auth_context(|auth| {
        auth.principal()
            .map(|principal| {
                principal
                    .permissions
                    .iter()
                    .any(|value| value == permission)
            })
            .unwrap_or(false)
    })
}

fn has_any_role(ctx: &RuntimeContext, roles: &[String]) -> Result<bool, Error> {
    ctx.with_request_auth_context(|auth| {
        auth.principal()
            .map(|principal| {
                roles
                    .iter()
                    .any(|role| principal.roles.iter().any(|value| value == role))
            })
            .unwrap_or(false)
    })
}

fn has_all_roles(ctx: &RuntimeContext, roles: &[String]) -> Result<bool, Error> {
    ctx.with_request_auth_context(|auth| {
        auth.principal()
            .map(|principal| {
                roles
                    .iter()
                    .all(|role| principal.roles.iter().any(|value| value == role))
            })
            .unwrap_or(false)
    })
}

fn has_any_permission(ctx: &RuntimeContext, permissions: &[String]) -> Result<bool, Error> {
    ctx.with_request_auth_context(|auth| {
        auth.principal()
            .map(|principal| {
                permissions.iter().any(|permission| {
                    principal
                        .permissions
                        .iter()
                        .any(|value| value == permission)
                })
            })
            .unwrap_or(false)
    })
}

fn has_all_permissions(ctx: &RuntimeContext, permissions: &[String]) -> Result<bool, Error> {
    ctx.with_request_auth_context(|auth| {
        auth.principal()
            .map(|principal| {
                permissions.iter().all(|permission| {
                    principal
                        .permissions
                        .iter()
                        .any(|value| value == permission)
                })
            })
            .unwrap_or(false)
    })
}

fn string_list_arg(
    intrinsic_id: &str,
    args: &[ZunValue],
    index: usize,
) -> Result<Vec<String>, Error> {
    let value = args.get(index).ok_or_else(|| {
        Error::Interpreter(format!("{intrinsic_id}: missing required list argument"))
    })?;

    let ZunValue::List(values) = value else {
        return Err(Error::Interpreter(format!(
            "{intrinsic_id}: argument must be List<String>"
        )));
    };

    let mut strings = Vec::with_capacity(values.len());
    for value in values {
        let ZunValue::Str(value) = value else {
            return Err(Error::Interpreter(format!(
                "{intrinsic_id}: argument must be List<String>"
            )));
        };
        strings.push(value.clone());
    }
    Ok(strings)
}

fn auth_forbidden_error(message: String) -> Error {
    Error::Interpreter(format!("__auth_forbidden__: {message}"))
}

pub fn principal_to_value(principal: &crate::runtime::auth::Principal) -> ZunValue {
    let mut map = IndexMap::new();
    map.insert(
        "subject".to_string(),
        ZunValue::Str(principal.subject.clone()),
    );
    map.insert(
        "scheme".to_string(),
        ZunValue::Str(principal.scheme.clone()),
    );
    map.insert(
        "roles".to_string(),
        ZunValue::List(
            principal
                .roles
                .iter()
                .map(|value| ZunValue::Str(value.clone()))
                .collect(),
        ),
    );
    map.insert(
        "permissions".to_string(),
        ZunValue::List(
            principal
                .permissions
                .iter()
                .map(|value| ZunValue::Str(value.clone()))
                .collect(),
        ),
    );
    map.insert(
        "claims".to_string(),
        ZunValue::Map(principal.claims.clone()),
    );
    map.insert(
        "session_id".to_string(),
        principal
            .session_id
            .as_ref()
            .map(|value| ZunValue::Str(value.clone()))
            .unwrap_or(ZunValue::Null),
    );
    ZunValue::Map(map)
}
