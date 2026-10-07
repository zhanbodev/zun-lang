use crate::config::ProjectConfig;

use super::errors::AuthConfigError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeAuthorizationRule {
    pub declaration_index: usize,
    pub method: String,
    pub path: String,
    pub require_authenticated: bool,
    pub roles_any: Vec<String>,
    pub roles_all: Vec<String>,
    pub permissions_any: Vec<String>,
    pub permissions_all: Vec<String>,
    pub claims_all: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeAuthConfig {
    pub enabled: bool,
    pub default_scheme: String,
    pub identity_sources: Vec<String>,
    pub session_enabled: bool,
    pub jwt_enabled: bool,
    pub jwt_secret: Option<String>,
    pub jwt_algorithm: String,
    pub issuer: String,
    pub audience: String,
    pub access_ttl_seconds: i64,
    pub refresh_ttl_seconds: i64,
    pub session_cookie_name: String,
    pub session_cookie_secure: bool,
    pub session_cookie_http_only: bool,
    pub session_cookie_same_site: String,
    pub session_cookie_path: String,
    pub session_ttl_seconds: i64,
    pub session_idle_timeout_seconds: i64,
    pub session_rotation: String,
    pub session_store_driver: String,
    pub authorization_enabled: bool,
    pub authorization_default: String,
    pub authorization_rules: Vec<RuntimeAuthorizationRule>,
}

impl Default for RuntimeAuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            default_scheme: "session".to_string(),
            identity_sources: vec!["cookie".to_string()],
            session_enabled: false,
            jwt_enabled: false,
            jwt_secret: Some("zun-dev-secret".to_string()),
            jwt_algorithm: "HS256".to_string(),
            issuer: "zun".to_string(),
            audience: "zun".to_string(),
            access_ttl_seconds: 3_600,
            refresh_ttl_seconds: 2_592_000,
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
}

impl RuntimeAuthConfig {
    pub fn from_project_config(project_config: Option<&ProjectConfig>) -> Self {
        let Some(project_config) = project_config else {
            return Self::default();
        };

        let auth = &project_config.server.auth;
        let jwt_secret = if !auth.jwt.secret.is_empty() {
            Some(auth.jwt.secret.clone())
        } else if !auth.jwt.secret_env.is_empty() {
            std::env::var(&auth.jwt.secret_env).ok()
        } else {
            None
        };

        Self {
            enabled: auth.enabled,
            default_scheme: auth.default_scheme.clone(),
            identity_sources: auth.identity_sources.clone(),
            session_enabled: auth.session.enabled,
            jwt_enabled: auth.jwt.enabled,
            jwt_secret,
            jwt_algorithm: auth.jwt.algorithm.clone(),
            issuer: auth.jwt.issuer.clone(),
            audience: auth.jwt.audience.clone(),
            access_ttl_seconds: auth.jwt.access_ttl_seconds,
            refresh_ttl_seconds: auth.jwt.refresh_ttl_seconds,
            session_cookie_name: auth.session.cookie_name.clone(),
            session_cookie_secure: auth.session.cookie_secure,
            session_cookie_http_only: auth.session.cookie_http_only,
            session_cookie_same_site: auth.session.cookie_same_site.clone(),
            session_cookie_path: auth.session.cookie_path.clone(),
            session_ttl_seconds: auth.session.ttl_seconds,
            session_idle_timeout_seconds: auth.session.idle_timeout_seconds,
            session_rotation: auth.session.rotation.clone(),
            session_store_driver: auth.session.store.driver.clone(),
            authorization_enabled: auth.authorization.enabled,
            authorization_default: auth.authorization.default.clone(),
            authorization_rules: auth
                .authorization
                .rules
                .iter()
                .enumerate()
                .map(|(index, rule)| RuntimeAuthorizationRule {
                    declaration_index: index,
                    method: rule.method.clone(),
                    path: rule.path.clone(),
                    require_authenticated: rule.require == "authenticated",
                    roles_any: rule.roles_any.clone(),
                    roles_all: rule.roles_all.clone(),
                    permissions_any: rule.permissions_any.clone(),
                    permissions_all: rule.permissions_all.clone(),
                    claims_all: rule.claims_all.clone(),
                })
                .collect(),
        }
    }
}

pub fn validate_runtime_auth_config(config: &RuntimeAuthConfig) -> Result<(), AuthConfigError> {
    if config.jwt_enabled && config.jwt_secret.as_deref().unwrap_or("").is_empty() {
        return Err(AuthConfigError::MissingJwtSecret);
    }

    if config.jwt_enabled && !matches!(config.jwt_algorithm.as_str(), "HS256" | "HS384" | "HS512") {
        return Err(AuthConfigError::UnsupportedJwtAlgorithm(
            config.jwt_algorithm.clone(),
        ));
    }

    match config.default_scheme.as_str() {
        "session" => {}
        "bearer" => {
            if config.enabled && !config.jwt_enabled {
                return Err(AuthConfigError::DefaultSchemeRequiresJwt);
            }
        }
        other => return Err(AuthConfigError::InvalidDefaultScheme(other.to_string())),
    }

    if config.session_enabled {
        let same_site = validated_same_site(&config.session_cookie_same_site)?;
        if same_site == "None" && !config.session_cookie_secure {
            return Err(AuthConfigError::SameSiteNoneRequiresSecure);
        }

        if !matches!(
            config.session_rotation.as_str(),
            "off" | "on_login" | "always"
        ) {
            return Err(AuthConfigError::InvalidSessionRotation(
                config.session_rotation.clone(),
            ));
        }
    }

    Ok(())
}

impl RuntimeAuthConfig {
    pub fn matching_rule(&self, method: &str, path: &str) -> Option<&RuntimeAuthorizationRule> {
        self.authorization_rules
            .iter()
            .filter(|rule| rule_matches(rule, method, path))
            .max_by_key(|rule| rule_priority(rule))
    }

    pub fn requires_authentication(&self, method: &str, path: &str) -> bool {
        if !self.enabled || !self.authorization_enabled {
            return false;
        }

        self.matching_rule(method, path)
            .map(|rule| rule.require_authenticated)
            .unwrap_or(self.authorization_default == "authenticated")
    }

    pub fn cookie_header_value(&self, session_id: &str) -> String {
        let mut attributes = vec![
            format!("{}={}", self.session_cookie_name, session_id),
            format!("Path={}", self.session_cookie_path),
            format!("Max-Age={}", self.session_ttl_seconds.max(0)),
            format!(
                "SameSite={}",
                normalize_same_site(&self.session_cookie_same_site)
            ),
        ];

        if self.session_cookie_http_only {
            attributes.push("HttpOnly".to_string());
        }
        if self.session_cookie_secure {
            attributes.push("Secure".to_string());
        }

        attributes.join("; ")
    }

    pub fn clear_cookie_header_value(&self) -> String {
        let mut attributes = vec![
            format!("{}=", self.session_cookie_name),
            "Max-Age=0".to_string(),
            format!("Path={}", self.session_cookie_path),
            format!(
                "SameSite={}",
                normalize_same_site(&self.session_cookie_same_site)
            ),
        ];

        if self.session_cookie_http_only {
            attributes.push("HttpOnly".to_string());
        }
        if self.session_cookie_secure {
            attributes.push("Secure".to_string());
        }

        attributes.join("; ")
    }
}

fn rule_matches(rule: &RuntimeAuthorizationRule, method: &str, path: &str) -> bool {
    method_matches(rule, method) && path_matches(rule, path)
}

fn method_matches(rule: &RuntimeAuthorizationRule, method: &str) -> bool {
    rule.method == "*" || rule.method.eq_ignore_ascii_case(method)
}

fn path_matches(rule: &RuntimeAuthorizationRule, path: &str) -> bool {
    if let Some(prefix) = rule.path.strip_suffix('*') {
        path.starts_with(prefix)
    } else {
        rule.path == path
    }
}

fn rule_priority(rule: &RuntimeAuthorizationRule) -> (usize, usize, std::cmp::Reverse<usize>) {
    let path_specificity = rule.path.trim_end_matches('*').len();
    let exact_bonus = usize::from(!rule.path.ends_with('*'));
    (
        path_specificity,
        exact_bonus,
        std::cmp::Reverse(rule.declaration_index),
    )
}

fn normalize_same_site(value: &str) -> &'static str {
    validated_same_site(value).unwrap_or("Lax")
}

fn validated_same_site(value: &str) -> Result<&'static str, AuthConfigError> {
    if value.eq_ignore_ascii_case("lax") {
        Ok("Lax")
    } else if value.eq_ignore_ascii_case("strict") {
        Ok("Strict")
    } else if value.eq_ignore_ascii_case("none") {
        Ok("None")
    } else {
        Err(AuthConfigError::InvalidSessionCookieSameSite(
            value.to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{RuntimeAuthConfig, validate_runtime_auth_config};

    #[test]
    fn validate_auth_config_rejects_missing_jwt_secret() {
        let config = RuntimeAuthConfig {
            enabled: true,
            jwt_enabled: true,
            jwt_secret: None,
            ..RuntimeAuthConfig::default()
        };

        let error = validate_runtime_auth_config(&config).expect_err("config should fail");
        assert!(error.to_string().contains("JWT secret"));
    }

    #[test]
    fn validate_auth_config_rejects_invalid_same_site_value() {
        let config = RuntimeAuthConfig {
            enabled: true,
            session_enabled: true,
            session_cookie_same_site: "invalid".to_string(),
            ..RuntimeAuthConfig::default()
        };

        let error = validate_runtime_auth_config(&config).expect_err("config should fail");
        assert!(error.to_string().contains("same_site"));
    }

    #[test]
    fn validate_auth_config_rejects_same_site_none_without_secure_cookie() {
        let config = RuntimeAuthConfig {
            enabled: true,
            session_enabled: true,
            session_cookie_same_site: "none".to_string(),
            session_cookie_secure: false,
            ..RuntimeAuthConfig::default()
        };

        let error = validate_runtime_auth_config(&config).expect_err("config should fail");
        assert!(error.to_string().contains("SameSite=None"));
    }

    #[test]
    fn validate_auth_config_rejects_invalid_session_rotation() {
        let config = RuntimeAuthConfig {
            enabled: true,
            session_enabled: true,
            session_rotation: "sometimes".to_string(),
            ..RuntimeAuthConfig::default()
        };

        let error = validate_runtime_auth_config(&config).expect_err("config should fail");
        assert!(error.to_string().contains("session rotation"));
    }
}
