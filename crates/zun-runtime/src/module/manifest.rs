use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectConfig {
    pub name: String,
    pub version: String,
    pub entry: String,
    pub env: HashMap<String, String>,
    pub server: ServerConfig,
    pub dependencies: HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct ManifestFile {
    pub project: ProjectSection,
    pub name: String,
    pub version: String,
    pub entry: String,
    pub env: HashMap<String, String>,
    pub server: ServerConfig,
    pub dependencies: HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct ProjectSection {
    pub name: String,
    pub version: String,
    pub entry: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub port: u16,
    pub host: String,
    pub upload_max_size: u64,
    pub upload_max_file_size: u64,
    pub auth: AuthConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AuthConfig {
    pub enabled: bool,
    pub default_scheme: String,
    pub identity_sources: Vec<String>,
    pub session: SessionConfig,
    pub jwt: JwtConfig,
    pub authorization: AuthorizationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SessionConfig {
    pub enabled: bool,
    pub cookie_name: String,
    pub cookie_secure: bool,
    pub cookie_http_only: bool,
    pub cookie_same_site: String,
    pub cookie_path: String,
    pub ttl_seconds: i64,
    pub idle_timeout_seconds: i64,
    pub rotation: String,
    pub store: SessionStoreConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SessionStoreConfig {
    pub driver: String,
    pub sqlite: SqliteSessionStoreConfig,
    pub postgres: PostgresSessionStoreConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SqliteSessionStoreConfig {
    pub path: String,
    pub table: String,
    pub refresh_table: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PostgresSessionStoreConfig {
    pub url: String,
    pub url_env: String,
    pub table: String,
    pub refresh_table: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct JwtConfig {
    pub enabled: bool,
    pub issuer: String,
    pub audience: String,
    pub algorithm: String,
    pub secret: String,
    pub secret_env: String,
    pub access_ttl_seconds: i64,
    pub refresh_ttl_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AuthorizationConfig {
    pub enabled: bool,
    pub default: String,
    pub rules: Vec<AuthorizationRuleConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AuthorizationRuleConfig {
    pub method: String,
    pub path: String,
    pub require: String,
    pub roles_any: Vec<String>,
    pub roles_all: Vec<String>,
    pub permissions_any: Vec<String>,
    pub permissions_all: Vec<String>,
    pub claims_all: HashMap<String, String>,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            version: String::new(),
            entry: "main.zun".to_string(),
            env: HashMap::new(),
            server: ServerConfig::default(),
            dependencies: HashMap::new(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 8080,
            host: "127.0.0.1".to_string(),
            upload_max_size: 50,
            upload_max_file_size: 10,
            auth: AuthConfig::default(),
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            default_scheme: "session".to_string(),
            identity_sources: vec!["cookie".to_string()],
            session: SessionConfig::default(),
            jwt: JwtConfig::default(),
            authorization: AuthorizationConfig::default(),
        }
    }
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            cookie_name: "zun_session".to_string(),
            cookie_secure: false,
            cookie_http_only: true,
            cookie_same_site: "lax".to_string(),
            cookie_path: "/".to_string(),
            ttl_seconds: 86_400,
            idle_timeout_seconds: 7_200,
            rotation: "on_login".to_string(),
            store: SessionStoreConfig::default(),
        }
    }
}

impl Default for SessionStoreConfig {
    fn default() -> Self {
        Self {
            driver: "memory".to_string(),
            sqlite: SqliteSessionStoreConfig::default(),
            postgres: PostgresSessionStoreConfig::default(),
        }
    }
}

impl Default for SqliteSessionStoreConfig {
    fn default() -> Self {
        Self {
            path: ".zun/auth.sqlite3".to_string(),
            table: "auth_sessions".to_string(),
            refresh_table: "auth_refresh_tokens".to_string(),
        }
    }
}

impl Default for PostgresSessionStoreConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            url_env: String::new(),
            table: "auth_sessions".to_string(),
            refresh_table: "auth_refresh_tokens".to_string(),
        }
    }
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            issuer: String::new(),
            audience: String::new(),
            algorithm: "HS256".to_string(),
            secret: String::new(),
            secret_env: String::new(),
            access_ttl_seconds: 3_600,
            refresh_ttl_seconds: 2_592_000,
        }
    }
}

impl Default for AuthorizationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            default: "public".to_string(),
            rules: Vec::new(),
        }
    }
}

impl Default for AuthorizationRuleConfig {
    fn default() -> Self {
        Self {
            method: String::new(),
            path: String::new(),
            require: String::new(),
            roles_any: Vec::new(),
            roles_all: Vec::new(),
            permissions_any: Vec::new(),
            permissions_all: Vec::new(),
            claims_all: HashMap::new(),
        }
    }
}

impl ProjectConfig {
    pub fn load_from_dir(dir: &Path) -> Option<Self> {
        let config_path = super::paths::resolve_manifest_path(dir);
        if !config_path.exists() {
            return None;
        }

        let content = fs::read_to_string(&config_path).ok()?;
        Self::parse_toml(&content)
    }

    pub fn parse_toml(content: &str) -> Option<Self> {
        let parsed = toml::from_str::<ManifestFile>(content).ok()?;
        let config = ProjectConfig {
            name: first_non_empty(&parsed.project.name, &parsed.name),
            version: first_non_empty(&parsed.project.version, &parsed.version),
            entry: first_non_empty_or_default(&parsed.project.entry, &parsed.entry, "main.zun"),
            env: parsed.env,
            server: parsed.server,
            dependencies: parsed.dependencies,
        };
        if config.name.is_empty() {
            return None;
        }
        Some(config)
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.env.get(key).map(|s| s.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::ProjectConfig;

    #[test]
    fn parse_project_config_supports_project_section() {
        let config = ProjectConfig::parse_toml(
            r#"
[project]
name = "auth-demo"
version = "0.1.0"
entry = "app/main.zun"

[server]
host = "127.0.0.1"
port = 8080
"#,
        )
        .expect("config should parse");

        assert_eq!(config.name, "auth-demo");
        assert_eq!(config.version, "0.1.0");
        assert_eq!(config.entry, "app/main.zun");
        assert_eq!(config.server.host, "127.0.0.1");
    }

    #[test]
    fn parse_project_config_with_auth_settings() {
        let config = ProjectConfig::parse_toml(
            r#"
name = "auth-demo"
version = "0.1.0"
entry = "main.zun"

[server]
host = "127.0.0.1"
port = 8080

[server.auth]
enabled = true
default_scheme = "session"
identity_sources = ["cookie", "bearer"]

[server.auth.session]
enabled = true
cookie_name = "zun_session"
ttl_seconds = 86400
idle_timeout_seconds = 7200

[server.auth.session.store]
driver = "postgres"

[server.auth.session.store.postgres]
url_env = "SESSION_DATABASE_URL"
table = "auth_sessions"
refresh_table = "auth_refresh_tokens_custom"

[server.auth.jwt]
enabled = true
issuer = "demo"
audience = "api"
algorithm = "HS256"
secret_env = "JWT_SECRET"
access_ttl_seconds = 3600
refresh_ttl_seconds = 86400
"#,
        )
        .expect("config should parse");

        assert!(config.server.auth.enabled);
        assert_eq!(config.server.auth.default_scheme, "session");
        assert_eq!(
            config.server.auth.identity_sources,
            vec!["cookie", "bearer"]
        );
        assert_eq!(config.server.auth.session.store.driver, "postgres");
        assert_eq!(
            config.server.auth.session.store.postgres.refresh_table,
            "auth_refresh_tokens_custom"
        );
        assert_eq!(config.server.auth.jwt.secret_env, "JWT_SECRET");
    }

    #[test]
    fn parse_project_config_defaults_auth_to_disabled() {
        let config = ProjectConfig::parse_toml(
            r#"
name = "plain-http"
version = "0.1.0"
entry = "main.zun"
"#,
        )
        .expect("config should parse");

        assert!(!config.server.auth.enabled);
        assert_eq!(config.server.auth.default_scheme, "session");
        assert_eq!(config.server.auth.identity_sources, vec!["cookie"]);
        assert_eq!(config.server.auth.session.store.driver, "memory");
        assert_eq!(
            config.server.auth.session.store.sqlite.refresh_table,
            "auth_refresh_tokens"
        );
        assert_eq!(
            config.server.auth.session.store.postgres.refresh_table,
            "auth_refresh_tokens"
        );
    }

    #[test]
    fn parse_project_config_prefers_project_section_over_legacy_top_level_fields() {
        let config = ProjectConfig::parse_toml(
            r#"
name = "legacy"
version = "0.1.0"
entry = "main.zun"

[project]
name = "new-style"
version = "0.2.0"
entry = "app/main.zun"
"#,
        )
        .expect("config should parse");

        assert_eq!(config.name, "new-style");
        assert_eq!(config.version, "0.2.0");
        assert_eq!(config.entry, "app/main.zun");
    }
}

fn first_non_empty(preferred: &str, fallback: &str) -> String {
    if !preferred.is_empty() {
        preferred.to_string()
    } else {
        fallback.to_string()
    }
}

fn first_non_empty_or_default(preferred: &str, fallback: &str, default: &str) -> String {
    if !preferred.is_empty() {
        preferred.to_string()
    } else if !fallback.is_empty() {
        fallback.to_string()
    } else {
        default.to_string()
    }
}
