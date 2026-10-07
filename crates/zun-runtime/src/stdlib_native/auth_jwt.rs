use std::sync::Arc;

use chrono::Utc;
use indexmap::IndexMap;

use crate::error::Error;
use crate::interpreter::ZunValue;
use crate::runtime::auth::{
    RefreshTokenRecord, RefreshTokenStore, sign_jwt, sign_refresh_jwt, verify_jwt,
    verify_refresh_jwt,
};
use crate::runtime::intrinsics::intrinsic_string_arg;
use crate::runtime::{NativeFnMap, RuntimeContext};

pub fn register(context: &mut RuntimeContext) {
    let mut exports = NativeFnMap::new();
    exports.insert(
        "sign".into(),
        Arc::new(|args: &[ZunValue], ctx: &RuntimeContext| {
            ensure_jwt_enabled(ctx)?;
            let payload = payload_arg(args, "auth.jwt.sign")?;
            let (subject, roles, permissions, extra) = jwt_payload_parts(payload, "auth.jwt.sign")?;
            let token = sign_jwt(
                ctx.runtime_auth_config(),
                &subject,
                &roles,
                &permissions,
                &extra,
            )?;
            Ok(ZunValue::Str(token))
        }),
    );
    exports.insert(
        "verify".into(),
        Arc::new(|args: &[ZunValue], ctx: &RuntimeContext| {
            ensure_jwt_enabled(ctx)?;
            let token = intrinsic_string_arg("auth.jwt.verify", args, 0)?;
            let claims = verify_jwt(ctx.runtime_auth_config(), token)?;
            Ok(jwt_claims_to_value(&claims))
        }),
    );
    exports.insert(
        "sign_refresh".into(),
        Arc::new(|args: &[ZunValue], ctx: &RuntimeContext| {
            ensure_jwt_enabled(ctx)?;
            let payload = payload_arg(args, "auth.jwt.sign_refresh")?;
            let (subject, roles, permissions, extra) =
                jwt_payload_parts(payload, "auth.jwt.sign_refresh")?;
            let token = issue_refresh_token(ctx, &subject, &roles, &permissions, &extra)?;
            Ok(ZunValue::Str(token))
        }),
    );
    exports.insert(
        "verify_refresh".into(),
        Arc::new(|args: &[ZunValue], ctx: &RuntimeContext| {
            ensure_jwt_enabled(ctx)?;
            let token = intrinsic_string_arg("auth.jwt.verify_refresh", args, 0)?;
            let claims = verify_refresh_claims(ctx, token)?;
            Ok(jwt_claims_to_value(&claims))
        }),
    );
    exports.insert(
        "issue_pair".into(),
        Arc::new(|args: &[ZunValue], ctx: &RuntimeContext| {
            ensure_jwt_enabled(ctx)?;
            let payload = payload_arg(args, "auth.jwt.issue_pair")?;
            let (subject, roles, permissions, extra) =
                jwt_payload_parts(payload, "auth.jwt.issue_pair")?;
            Ok(token_pair_to_value(
                &sign_jwt(
                    ctx.runtime_auth_config(),
                    &subject,
                    &roles,
                    &permissions,
                    &extra,
                )?,
                &issue_refresh_token(ctx, &subject, &roles, &permissions, &extra)?,
            ))
        }),
    );
    exports.insert(
        "refresh_pair".into(),
        Arc::new(|args: &[ZunValue], ctx: &RuntimeContext| {
            ensure_jwt_enabled(ctx)?;
            let token = intrinsic_string_arg("auth.jwt.refresh_pair", args, 0)?;
            let claims = verify_refresh_claims(ctx, token)?;
            let refresh_token_id = refresh_token_id(&claims)
                .ok_or_else(|| auth_unauthorized_error("invalid refresh token"))?;
            let extra: IndexMap<String, ZunValue> = claims
                .extra
                .iter()
                .map(|(key, value)| (key.clone(), json_value_to_zun(value.clone())))
                .collect();
            ctx.with_refresh_token_store_mut(|store| store.revoke(&refresh_token_id))?;
            Ok(token_pair_to_value(
                &sign_jwt(
                    ctx.runtime_auth_config(),
                    &claims.sub,
                    &claims.roles,
                    &claims.permissions,
                    &extra,
                )?,
                &issue_refresh_token(ctx, &claims.sub, &claims.roles, &claims.permissions, &extra)?,
            ))
        }),
    );
    exports.insert(
        "revoke_refresh".into(),
        Arc::new(|args: &[ZunValue], ctx: &RuntimeContext| {
            ensure_jwt_enabled(ctx)?;
            let token = intrinsic_string_arg("auth.jwt.revoke_refresh", args, 0)?;
            let claims = verify_refresh_claims(ctx, token)?;
            let refresh_token_id = refresh_token_id(&claims)
                .ok_or_else(|| auth_unauthorized_error("invalid refresh token"))?;
            ctx.with_refresh_token_store_mut(|store| store.revoke(&refresh_token_id))?;
            Ok(ZunValue::Null)
        }),
    );
    exports.insert(
        "current".into(),
        Arc::new(|_args: &[ZunValue], ctx: &RuntimeContext| {
            let claims =
                ctx.with_request_auth_context(|auth| auth.current_jwt_claims().cloned())?;
            Ok(claims.map(ZunValue::Map).unwrap_or(ZunValue::Null))
        }),
    );
    exports.insert(
        "bearer".into(),
        Arc::new(|_args: &[ZunValue], ctx: &RuntimeContext| {
            let token = ctx.with_request_auth_context(|auth| {
                auth.current_bearer_token().map(str::to_string)
            })?;
            Ok(token.map(ZunValue::Str).unwrap_or(ZunValue::Null))
        }),
    );
    context.register_native_module("std.auth.jwt", exports);
}

fn auth_unauthorized_error(message: &str) -> Error {
    Error::Interpreter(format!("__auth_unauthorized__: {message}"))
}

fn verify_refresh_claims(
    ctx: &RuntimeContext,
    token: &str,
) -> Result<crate::runtime::auth::JwtClaims, Error> {
    let claims = verify_refresh_jwt(ctx.runtime_auth_config(), token)
        .map_err(|_| auth_unauthorized_error("invalid refresh token"))?;
    let refresh_token_id = refresh_token_id(&claims)
        .ok_or_else(|| auth_unauthorized_error("invalid refresh token"))?;
    let record = ctx.with_refresh_token_store_mut(|store| store.get(&refresh_token_id))?;
    let Some(record) = record else {
        return Err(auth_unauthorized_error("invalid refresh token"));
    };

    if record.revoked_at.is_some() || record.expires_at <= Utc::now().timestamp() {
        return Err(auth_unauthorized_error("invalid refresh token"));
    }

    Ok(claims)
}

fn issue_refresh_token(
    ctx: &RuntimeContext,
    subject: &str,
    roles: &[String],
    permissions: &[String],
    extra: &IndexMap<String, ZunValue>,
) -> Result<String, Error> {
    let token = sign_refresh_jwt(
        ctx.runtime_auth_config(),
        subject,
        roles,
        permissions,
        extra,
    )?;
    let claims = verify_refresh_jwt(ctx.runtime_auth_config(), &token)
        .map_err(|_| auth_unauthorized_error("invalid refresh token"))?;
    let record = refresh_record_from_claims(&claims)?;
    ctx.with_refresh_token_store_mut(|store| store.create(record))?;
    Ok(token)
}

fn ensure_jwt_enabled(ctx: &RuntimeContext) -> Result<(), Error> {
    if !ctx.runtime_auth_config().enabled || ctx.runtime_auth_config().jwt_enabled {
        Ok(())
    } else {
        Err(Error::Interpreter(
            "auth.jwt: jwt auth is not enabled".to_string(),
        ))
    }
}

fn string_field(
    map: &IndexMap<String, ZunValue>,
    key: &str,
    intrinsic_id: &str,
) -> Result<String, Error> {
    match map.get(key) {
        Some(ZunValue::Str(value)) => Ok(value.clone()),
        Some(other) => Err(Error::Interpreter(format!(
            "{intrinsic_id}: field '{key}' must be String, got {}",
            other.type_name()
        ))),
        None => Err(Error::Interpreter(format!(
            "{intrinsic_id}: missing required field '{key}'"
        ))),
    }
}

fn payload_arg<'a>(
    args: &'a [ZunValue],
    intrinsic_id: &str,
) -> Result<&'a IndexMap<String, ZunValue>, Error> {
    let payload = args
        .first()
        .ok_or_else(|| Error::Interpreter(format!("{intrinsic_id}: missing payload")))?;
    let (ZunValue::Map(payload) | ZunValue::Json(payload)) = payload else {
        return Err(Error::Interpreter(format!(
            "{intrinsic_id}: payload must be Map or Json"
        )));
    };
    Ok(payload)
}

fn jwt_payload_parts(
    payload: &IndexMap<String, ZunValue>,
    intrinsic_id: &str,
) -> Result<(String, Vec<String>, Vec<String>, IndexMap<String, ZunValue>), Error> {
    Ok((
        string_field(payload, "sub", intrinsic_id)?,
        string_list_field(payload, "roles"),
        string_list_field(payload, "permissions"),
        extra_claims(payload),
    ))
}

fn string_list_field(map: &IndexMap<String, ZunValue>, key: &str) -> Vec<String> {
    match map.get(key) {
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

fn extra_claims(map: &IndexMap<String, ZunValue>) -> IndexMap<String, ZunValue> {
    map.iter()
        .filter(|(key, _)| *key != "sub" && *key != "roles" && *key != "permissions")
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

fn jwt_claims_to_value(claims: &crate::runtime::auth::JwtClaims) -> ZunValue {
    let mut map = IndexMap::new();
    map.insert("sub".to_string(), ZunValue::Str(claims.sub.clone()));
    map.insert("iss".to_string(), ZunValue::Str(claims.iss.clone()));
    map.insert("aud".to_string(), ZunValue::Str(claims.aud.clone()));
    map.insert("exp".to_string(), ZunValue::Int(claims.exp as i64));
    map.insert(
        "token_use".to_string(),
        ZunValue::Str(claims.token_use.clone()),
    );
    map.insert(
        "roles".to_string(),
        ZunValue::List(
            claims
                .roles
                .iter()
                .map(|value| ZunValue::Str(value.clone()))
                .collect(),
        ),
    );
    map.insert(
        "permissions".to_string(),
        ZunValue::List(
            claims
                .permissions
                .iter()
                .map(|value| ZunValue::Str(value.clone()))
                .collect(),
        ),
    );
    for (key, value) in &claims.extra {
        map.insert(key.clone(), json_value_to_zun(value.clone()));
    }
    ZunValue::Map(map)
}

fn refresh_token_id(claims: &crate::runtime::auth::JwtClaims) -> Option<String> {
    claims
        .extra
        .get("jti")
        .and_then(|value| value.as_str())
        .map(str::to_string)
}

fn refresh_record_from_claims(
    claims: &crate::runtime::auth::JwtClaims,
) -> Result<RefreshTokenRecord, Error> {
    let token_id =
        refresh_token_id(claims).ok_or_else(|| auth_unauthorized_error("invalid refresh token"))?;
    Ok(RefreshTokenRecord {
        token_id,
        subject: claims.sub.clone(),
        expires_at: claims.exp as i64,
        revoked_at: None,
    })
}

fn token_pair_to_value(access_token: &str, refresh_token: &str) -> ZunValue {
    let mut map = IndexMap::new();
    map.insert(
        "access_token".to_string(),
        ZunValue::Str(access_token.to_string()),
    );
    map.insert(
        "refresh_token".to_string(),
        ZunValue::Str(refresh_token.to_string()),
    );
    ZunValue::Map(map)
}

fn json_value_to_zun(value: serde_json::Value) -> ZunValue {
    match value {
        serde_json::Value::Null => ZunValue::Null,
        serde_json::Value::Bool(b) => ZunValue::Bool(b),
        serde_json::Value::Number(n) => {
            if let Some(int) = n.as_i64() {
                ZunValue::Int(int)
            } else if let Some(float) = n.as_f64() {
                ZunValue::Float(float)
            } else {
                ZunValue::Null
            }
        }
        serde_json::Value::String(s) => ZunValue::Str(s),
        serde_json::Value::Array(values) => {
            ZunValue::List(values.into_iter().map(json_value_to_zun).collect())
        }
        serde_json::Value::Object(entries) => ZunValue::Map(
            entries
                .into_iter()
                .map(|(key, value)| (key, json_value_to_zun(value)))
                .collect(),
        ),
    }
}
