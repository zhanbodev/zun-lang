use std::collections::BTreeMap;

use chrono::Utc;
use indexmap::IndexMap;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::Error;
use crate::interpreter::{ZunValue, value::value_to_json};

use super::RuntimeAuthConfig;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JwtClaims {
    pub sub: String,
    pub iss: String,
    pub aud: String,
    pub exp: usize,
    pub token_use: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

impl JwtClaims {
    fn new(
        config: &RuntimeAuthConfig,
        subject: &str,
        roles: &[String],
        permissions: &[String],
        claims: &IndexMap<String, ZunValue>,
        token_use: &str,
        ttl_seconds: i64,
    ) -> Result<Self, Error> {
        let mut extra = BTreeMap::new();
        for (key, value) in claims {
            extra.insert(key.clone(), value_to_json(value));
        }

        Ok(Self {
            sub: subject.to_string(),
            iss: config.issuer.clone(),
            aud: config.audience.clone(),
            exp: (Utc::now().timestamp() + ttl_seconds) as usize,
            token_use: token_use.to_string(),
            roles: roles.to_vec(),
            permissions: permissions.to_vec(),
            extra,
        })
    }
}

pub fn sign_jwt(
    config: &RuntimeAuthConfig,
    subject: &str,
    roles: &[String],
    permissions: &[String],
    claims: &IndexMap<String, ZunValue>,
) -> Result<String, Error> {
    sign_jwt_with_ttl(
        config,
        subject,
        roles,
        permissions,
        claims,
        "access",
        config.access_ttl_seconds,
    )
}

pub fn sign_refresh_jwt(
    config: &RuntimeAuthConfig,
    subject: &str,
    roles: &[String],
    permissions: &[String],
    claims: &IndexMap<String, ZunValue>,
) -> Result<String, Error> {
    let mut refresh_claims = claims.clone();
    refresh_claims.insert(
        "jti".to_string(),
        ZunValue::Str(format!("rjti_{}", Uuid::new_v4().simple())),
    );

    sign_jwt_with_ttl(
        config,
        subject,
        roles,
        permissions,
        &refresh_claims,
        "refresh",
        config.refresh_ttl_seconds,
    )
}

fn sign_jwt_with_ttl(
    config: &RuntimeAuthConfig,
    subject: &str,
    roles: &[String],
    permissions: &[String],
    claims: &IndexMap<String, ZunValue>,
    token_use: &str,
    ttl_seconds: i64,
) -> Result<String, Error> {
    let claims = JwtClaims::new(
        config,
        subject,
        roles,
        permissions,
        claims,
        token_use,
        ttl_seconds,
    )?;
    let secret = config
        .jwt_secret
        .as_deref()
        .ok_or_else(|| Error::Interpreter("missing JWT secret".to_string()))?;
    let algorithm = jwt_algorithm(config)?;

    jsonwebtoken::encode(
        &jsonwebtoken::Header::new(algorithm),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|err| Error::Interpreter(format!("failed to sign jwt: {err}")))
}

pub fn verify_jwt(config: &RuntimeAuthConfig, token: &str) -> Result<JwtClaims, Error> {
    verify_jwt_with_use(config, token, "access")
}

pub fn verify_refresh_jwt(config: &RuntimeAuthConfig, token: &str) -> Result<JwtClaims, Error> {
    verify_jwt_with_use(config, token, "refresh")
}

fn verify_jwt_with_use(
    config: &RuntimeAuthConfig,
    token: &str,
    expected_token_use: &str,
) -> Result<JwtClaims, Error> {
    let secret = config
        .jwt_secret
        .as_deref()
        .ok_or_else(|| Error::Interpreter("missing JWT secret".to_string()))?;
    let algorithm = jwt_algorithm(config)?;

    let mut validation = Validation::new(algorithm);
    validation.set_issuer(&[config.issuer.as_str()]);
    validation.set_audience(&[config.audience.as_str()]);

    let claims = jsonwebtoken::decode::<JwtClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map(|data| data.claims)
    .map_err(|err| Error::Interpreter(format!("failed to verify jwt: {err}")))?;

    if claims.token_use != expected_token_use {
        return Err(Error::Interpreter(format!(
            "failed to verify jwt: expected {expected_token_use} token but got {}",
            claims.token_use
        )));
    }

    Ok(claims)
}

fn jwt_algorithm(config: &RuntimeAuthConfig) -> Result<Algorithm, Error> {
    match config.jwt_algorithm.as_str() {
        "HS256" => Ok(Algorithm::HS256),
        "HS384" => Ok(Algorithm::HS384),
        "HS512" => Ok(Algorithm::HS512),
        other => Err(Error::Interpreter(format!(
            "unsupported JWT algorithm '{other}'"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use indexmap::IndexMap;
    use jsonwebtoken::Algorithm;

    use super::{sign_jwt, sign_refresh_jwt, verify_jwt, verify_refresh_jwt};
    use crate::runtime::auth::RuntimeAuthConfig;

    fn sample_runtime_auth_config_with_secret(secret: &str) -> RuntimeAuthConfig {
        RuntimeAuthConfig {
            enabled: true,
            default_scheme: "session".to_string(),
            identity_sources: vec!["bearer".to_string()],
            session_enabled: false,
            jwt_enabled: true,
            jwt_secret: Some(secret.to_string()),
            jwt_algorithm: "HS256".to_string(),
            issuer: "demo-issuer".to_string(),
            audience: "demo-audience".to_string(),
            access_ttl_seconds: 3600,
            refresh_ttl_seconds: 86400,
            session_cookie_name: "zun_session".to_string(),
            session_cookie_secure: false,
            session_cookie_http_only: true,
            session_cookie_same_site: "lax".to_string(),
            session_cookie_path: "/".to_string(),
            session_ttl_seconds: 86_400,
            session_idle_timeout_seconds: 7_200,
            session_rotation: "on_login".to_string(),
            session_store_driver: "memory".to_string(),
            authorization_enabled: false,
            authorization_default: "public".to_string(),
            authorization_rules: Vec::new(),
        }
    }

    #[test]
    fn jwt_sign_and_verify_round_trip_principal_claims() {
        let config = sample_runtime_auth_config_with_secret("super-secret");
        let token = sign_jwt(
            &config,
            "user_1",
            &["admin".to_string()],
            &["post:create".to_string()],
            &IndexMap::new(),
        )
        .expect("token should sign");

        let claims = verify_jwt(&config, &token).expect("token should verify");
        assert_eq!(claims.sub, "user_1");
        assert_eq!(claims.token_use, "access");
        assert_eq!(claims.roles, vec!["admin".to_string()]);
        assert_eq!(claims.permissions, vec!["post:create".to_string()]);
    }

    #[test]
    fn refresh_jwt_round_trip_and_rejects_access_verifier() {
        let config = sample_runtime_auth_config_with_secret("super-secret");
        let token = sign_refresh_jwt(
            &config,
            "user_1",
            &["admin".to_string()],
            &["post:create".to_string()],
            &IndexMap::new(),
        )
        .expect("refresh token should sign");

        let claims = verify_refresh_jwt(&config, &token).expect("refresh token should verify");
        assert_eq!(claims.token_use, "refresh");

        let error =
            verify_jwt(&config, &token).expect_err("refresh token should fail access verifier");
        assert!(error.to_string().contains("expected access token"));
    }

    #[test]
    fn jwt_sign_and_verify_respects_hs512_algorithm_config() {
        let mut config = sample_runtime_auth_config_with_secret("super-secret");
        config.jwt_algorithm = "HS512".to_string();

        let token = sign_jwt(
            &config,
            "user_42",
            &["admin".to_string()],
            &[],
            &IndexMap::new(),
        )
        .expect("token should sign");

        let header = jsonwebtoken::decode_header(&token).expect("header should decode");
        assert_eq!(header.alg, Algorithm::HS512);

        let claims = verify_jwt(&config, &token).expect("token should verify");
        assert_eq!(claims.sub, "user_42");
    }
}
