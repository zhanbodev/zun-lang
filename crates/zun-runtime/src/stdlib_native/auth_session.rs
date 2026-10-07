use std::sync::Arc;

use chrono::Utc;
use indexmap::IndexMap;
use uuid::Uuid;

use crate::error::Error;
use crate::interpreter::ZunValue;
use crate::runtime::auth::{
    Principal, SessionRecord, csrf_token_from_claims, replace_session_csrf_token,
};
use crate::runtime::{NativeFnMap, RuntimeContext};

use super::auth_guard::principal_to_value;

pub fn register(context: &mut RuntimeContext) {
    let mut exports = NativeFnMap::new();
    exports.insert(
        "current".into(),
        Arc::new(|_args: &[ZunValue], ctx: &RuntimeContext| {
            let session = ctx.with_request_auth_context(|auth| auth.current_session().cloned())?;
            Ok(session
                .map(|session| session_to_value(&session))
                .unwrap_or(ZunValue::Null))
        }),
    );
    exports.insert(
        "exists".into(),
        Arc::new(|_args: &[ZunValue], ctx: &RuntimeContext| {
            Ok(ZunValue::Bool(ctx.with_request_auth_context(|auth| {
                auth.current_session().is_some()
            })?))
        }),
    );
    exports.insert(
        "id".into(),
        Arc::new(|_args: &[ZunValue], ctx: &RuntimeContext| {
            let session_id = ctx.with_request_auth_context(|auth| {
                auth.current_session()
                    .map(|session| session.session_id.clone())
            })?;
            Ok(session_id.map(ZunValue::Str).unwrap_or(ZunValue::Null))
        }),
    );
    exports.insert(
        "create".into(),
        Arc::new(|args: &[ZunValue], ctx: &RuntimeContext| {
            ensure_session_enabled(ctx)?;
            let payload = args.first().ok_or_else(|| {
                Error::Interpreter("auth.session.create: missing session payload".to_string())
            })?;
            let (ZunValue::Map(payload) | ZunValue::Json(payload)) = payload else {
                return Err(Error::Interpreter(
                    "auth.session.create: payload must be Map or Json".to_string(),
                ));
            };

            let session = session_from_payload(payload, ctx)?;
            let principal = Principal {
                subject: session.subject.clone(),
                scheme: "session".to_string(),
                roles: session.roles.clone(),
                permissions: session.permissions.clone(),
                claims: session.claims.clone(),
                session_id: Some(session.session_id.clone()),
            };

            ctx.with_request_auth_context_mut(|auth| {
                if matches!(
                    ctx.runtime_auth_config().session_rotation.as_str(),
                    "on_login" | "always"
                ) {
                    if let Some(current) = auth.current_session() {
                        auth.queue_session_delete(current.session_id.clone());
                    }
                }
                auth.queue_session_create(session.clone());
                auth.set_current_session(Some(session.clone()));
                auth.set_principal(Some(principal));
                auth.set_pending_cookie(Some(
                    ctx.runtime_auth_config()
                        .cookie_header_value(&session.session_id),
                ));
                auth.set_pending_clear_cookie(false);
            })?;

            Ok(session_to_value(&session))
        }),
    );
    exports.insert(
        "get".into(),
        Arc::new(|args: &[ZunValue], ctx: &RuntimeContext| {
            let key =
                crate::runtime::intrinsics::intrinsic_string_arg("auth.session.get", args, 0)?;
            let value = ctx.with_request_auth_context(|auth| {
                auth.current_session()
                    .and_then(|session| session.claims.get(key).cloned())
            })?;
            Ok(value.unwrap_or(ZunValue::Null))
        }),
    );
    exports.insert(
        "set".into(),
        Arc::new(|args: &[ZunValue], ctx: &RuntimeContext| {
            ensure_session_enabled(ctx)?;
            let key =
                crate::runtime::intrinsics::intrinsic_string_arg("auth.session.set", args, 0)?;
            let value = args
                .get(1)
                .cloned()
                .ok_or_else(|| Error::Interpreter("auth.session.set: missing value".to_string()))?;

            let updated = ctx.with_request_auth_context_mut(|auth| {
                let mut session = auth.current_session().cloned().ok_or_else(|| {
                    Error::Interpreter("auth.session.set: no active session".to_string())
                })?;
                session.claims.insert(key.to_string(), value.clone());
                auth.set_current_session(Some(session.clone()));
                if let Some(principal) = auth.principal().cloned() {
                    let mut principal = principal;
                    principal.claims = session.claims.clone();
                    auth.set_principal(Some(principal));
                }
                auth.queue_session_update(session.clone());
                Ok::<SessionRecord, Error>(session)
            })??;
            let _ = updated;
            Ok(ZunValue::Null)
        }),
    );
    exports.insert(
        "delete".into(),
        Arc::new(|args: &[ZunValue], ctx: &RuntimeContext| {
            ensure_session_enabled(ctx)?;
            let key =
                crate::runtime::intrinsics::intrinsic_string_arg("auth.session.delete", args, 0)?;
            let updated = ctx.with_request_auth_context_mut(|auth| {
                let mut session = auth.current_session().cloned().ok_or_else(|| {
                    Error::Interpreter("auth.session.delete: no active session".to_string())
                })?;
                session.claims.shift_remove(key);
                auth.set_current_session(Some(session.clone()));
                if let Some(principal) = auth.principal().cloned() {
                    let mut principal = principal;
                    principal.claims = session.claims.clone();
                    auth.set_principal(Some(principal));
                }
                auth.queue_session_update(session.clone());
                Ok::<SessionRecord, Error>(session)
            })??;
            let _ = updated;
            Ok(ZunValue::Null)
        }),
    );
    exports.insert(
        "rotate".into(),
        Arc::new(|_args: &[ZunValue], ctx: &RuntimeContext| {
            ensure_session_enabled(ctx)?;
            let rotated = ctx.with_request_auth_context_mut(|auth| {
                let current = auth.current_session().cloned().ok_or_else(|| {
                    Error::Interpreter("auth.session.rotate: no active session".to_string())
                })?;
                let mut rotated = current.clone();
                rotated.session_id = format!("sess_{}", Uuid::new_v4().simple());
                auth.set_current_session(Some(rotated.clone()));
                if let Some(principal) = auth.principal().cloned() {
                    let mut principal = principal;
                    principal.session_id = Some(rotated.session_id.clone());
                    auth.set_principal(Some(principal));
                }
                auth.set_pending_cookie(Some(
                    ctx.runtime_auth_config()
                        .cookie_header_value(&rotated.session_id),
                ));
                auth.set_pending_clear_cookie(false);
                auth.queue_session_rotate(current.session_id.clone(), rotated.clone());
                Ok::<(String, SessionRecord), Error>((current.session_id, rotated))
            })??;
            Ok(session_to_value(&rotated.1))
        }),
    );
    exports.insert(
        "destroy".into(),
        Arc::new(|_args: &[ZunValue], ctx: &RuntimeContext| {
            ensure_session_enabled(ctx)?;
            let session_id = ctx.with_request_auth_context(|auth| {
                auth.current_session()
                    .map(|session| session.session_id.clone())
            })?;
            ctx.with_request_auth_context_mut(|auth| {
                if let Some(session_id) = session_id {
                    auth.queue_session_delete(session_id);
                }
                auth.set_current_session(None);
                auth.set_principal(None);
                auth.set_pending_cookie(None);
                auth.set_pending_clear_cookie(true);
                auth.set_current_jwt_claims(None);
                auth.set_current_bearer_token(None);
            })?;
            Ok(ZunValue::Null)
        }),
    );
    context.register_native_module("std.auth.session", exports);
}

fn ensure_session_enabled(ctx: &RuntimeContext) -> Result<(), Error> {
    if !ctx.runtime_auth_config().enabled || ctx.runtime_auth_config().session_enabled {
        Ok(())
    } else {
        Err(Error::Interpreter(
            "auth.session: session auth is not enabled".to_string(),
        ))
    }
}

fn session_from_payload(
    payload: &IndexMap<String, ZunValue>,
    ctx: &RuntimeContext,
) -> Result<SessionRecord, Error> {
    let subject = match payload.get("subject") {
        Some(ZunValue::Str(value)) => value.clone(),
        Some(other) => {
            return Err(Error::Interpreter(format!(
                "auth.session.create: subject must be String, got {}",
                other.type_name()
            )));
        }
        None => {
            return Err(Error::Interpreter(
                "auth.session.create: missing required field 'subject'".to_string(),
            ));
        }
    };
    let now = Utc::now().timestamp();
    let auth_config = ctx.runtime_auth_config();
    let mut claims = claims_field(payload, "claims");
    replace_session_csrf_token(&mut claims);

    Ok(SessionRecord {
        session_id: format!("sess_{}", Uuid::new_v4().simple()),
        subject,
        roles: string_list_field(payload, "roles"),
        permissions: string_list_field(payload, "permissions"),
        claims,
        expires_at: now + auth_config.session_ttl_seconds,
        idle_timeout_at: if auth_config.session_idle_timeout_seconds > 0 {
            Some(now + auth_config.session_idle_timeout_seconds)
        } else {
            None
        },
    })
}

fn string_list_field(payload: &IndexMap<String, ZunValue>, key: &str) -> Vec<String> {
    match payload.get(key) {
        Some(ZunValue::List(values)) => values
            .iter()
            .filter_map(|value| match value {
                ZunValue::Str(value) => Some(value.clone()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn claims_field(payload: &IndexMap<String, ZunValue>, key: &str) -> IndexMap<String, ZunValue> {
    match payload.get(key) {
        Some(ZunValue::Map(values)) | Some(ZunValue::Json(values)) => values.clone(),
        _ => IndexMap::new(),
    }
}

fn session_to_value(session: &SessionRecord) -> ZunValue {
    let principal = Principal {
        subject: session.subject.clone(),
        scheme: "session".to_string(),
        roles: session.roles.clone(),
        permissions: session.permissions.clone(),
        claims: session.claims.clone(),
        session_id: Some(session.session_id.clone()),
    };

    let ZunValue::Map(mut map) = principal_to_value(&principal) else {
        unreachable!("principal should convert to map");
    };
    map.insert("id".to_string(), ZunValue::Str(session.session_id.clone()));
    map.insert("expires_at".to_string(), ZunValue::Int(session.expires_at));
    map.insert(
        "idle_timeout_at".to_string(),
        session
            .idle_timeout_at
            .map(ZunValue::Int)
            .unwrap_or(ZunValue::Null),
    );
    map.insert(
        "csrf_token".to_string(),
        csrf_token_from_claims(&session.claims)
            .map(ZunValue::Str)
            .unwrap_or(ZunValue::Null),
    );
    ZunValue::Map(map)
}
