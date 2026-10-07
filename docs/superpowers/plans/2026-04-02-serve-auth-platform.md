# Serve Auth Platform Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a runtime-native auth/authz platform for Zun `serve` mode with `package.toml` configuration, request-scoped principals, session stores (`memory` / `sqlite` / `postgres`), JWT support, auth stdlib modules, and real HTTP integration coverage.

**Architecture:** Extend `ProjectConfig` with `serve.auth` settings, introduce a dedicated runtime auth slice under `crates/zun-runtime/src/runtime/auth/`, and keep request execution centered around a unified `principal` model. Host-sensitive work stays in runtime intrinsics and Axum integration, while Zun-facing behavior is exposed through `std.auth.session`, `std.auth.jwt`, `std.auth.password`, and `std.auth.guard`.

**Tech Stack:** Rust, Serde/TOML config parsing, Zun runtime + stdlib native modules, Axum, `cookie`, `jsonwebtoken`, `argon2`, `uuid`, `rusqlite`, `postgres`, Rust unit tests, Rust integration tests

---

## Scope Check

This spec touches multiple surfaces, but they are not independent subsystems. `package.toml` config, request principal resolution, session stores, stdlib modules, and HTTP integration all depend on the same auth core and unified data model. Splitting them into separate plans would create interface drift, so this stays one implementation plan with incremental, testable slices.

## File Structure

### Create

- `crates/zun-runtime/src/runtime/auth/mod.rs`
- `crates/zun-runtime/src/runtime/auth/config.rs`
- `crates/zun-runtime/src/runtime/auth/model.rs`
- `crates/zun-runtime/src/runtime/auth/context.rs`
- `crates/zun-runtime/src/runtime/auth/errors.rs`
- `crates/zun-runtime/src/runtime/auth/session_store.rs`
- `crates/zun-runtime/src/runtime/auth/session_memory.rs`
- `crates/zun-runtime/src/runtime/auth/session_sqlite.rs`
- `crates/zun-runtime/src/runtime/auth/session_postgres.rs`
- `crates/zun-runtime/src/runtime/auth/jwt.rs`
- `crates/zun-runtime/src/runtime/auth/password.rs`
- `crates/zun-runtime/src/runtime/auth/pipeline.rs`
- `crates/zun-runtime/src/stdlib_native/auth_session.rs`
- `crates/zun-runtime/src/stdlib_native/auth_jwt.rs`
- `crates/zun-runtime/src/stdlib_native/auth_password.rs`
- `crates/zun-runtime/src/stdlib_native/auth_guard.rs`
- `tests/spec/valid/stdlib/auth_session_flow.zun`
- `tests/spec/valid/stdlib/auth_session_flow.zun.stdout`
- `tests/spec/valid/stdlib/auth_jwt_flow.zun`
- `tests/spec/valid/stdlib/auth_jwt_flow.zun.stdout`
- `tests/spec/valid/stdlib/auth_guard_flow.zun`
- `tests/spec/valid/stdlib/auth_guard_flow.zun.stdout`
- `tests/spec/invalid/stdlib/auth_guard_require_role_fails.zun`
- `tests/spec/invalid/stdlib/auth_guard_require_role_fails.zun.error`

### Modify

- `crates/zun-runtime/src/module/manifest.rs`
- `crates/zun-runtime/src/config.rs`
- `crates/zun-runtime/src/runtime/mod.rs`
- `crates/zun-runtime/src/runtime/context.rs`
- `crates/zun-runtime/src/runtime/http.rs`
- `crates/zun-runtime/src/runtime/intrinsics.rs`
- `crates/zun-runtime/src/stdlib_native/mod.rs`
- `crates/zun-runtime/Cargo.toml`
- `crates/zun-cli/src/backends/axum_backend.rs`
- `tests/integration_suite.rs`
- `docs/spec/security-model.md`
- `docs/reference/http.md`
- `docs/reference/stdlib-api.md`
- `docs/CHANGELOG.md`

### Responsibilities

- `module/manifest.rs`: parse `package.toml` auth config into strongly typed structs
- `runtime/auth/config.rs`: normalize parsed auth config into runtime-usable defaults
- `runtime/auth/model.rs`: define `Principal`, `SessionRecord`, `RouteAuthRule`, and request auth state
- `runtime/auth/context.rs`: carry request-scoped auth state and pending auth side effects
- `runtime/auth/session_store.rs`: define the session store trait and driver selection
- `runtime/auth/session_*`: implement `memory`, `sqlite`, and `postgres` session drivers
- `runtime/auth/jwt.rs`: sign and verify JWTs against config
- `runtime/auth/password.rs`: hash and verify passwords
- `runtime/auth/pipeline.rs`: resolve cookie/bearer identity, build principal, and authorize routes
- `runtime/context.rs`: own shared registries and request-clone auth state
- `runtime/http.rs`: run auth pipeline before handler execution
- `axum_backend.rs`: parse cookies/body/headers into `HandlerInput`, map auth failures to `401`/`403`, and emit `Set-Cookie`
- `stdlib_native/auth_*`: expose `std.auth.*` modules
- `tests/integration_suite.rs`: cover startup validation, live HTTP auth, and store-specific flows

### Task 1: Extend `package.toml` With Serve Auth Configuration

**Files:**
- Modify: `crates/zun-runtime/src/module/manifest.rs`
- Modify: `crates/zun-runtime/src/config.rs`
- Test: `crates/zun-runtime/src/module/manifest.rs`

- [ ] **Step 1: Write the failing config parse tests**

```rust
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
    assert_eq!(config.server.auth.identity_sources, vec!["cookie", "bearer"]);
    assert_eq!(config.server.auth.session.store.driver, "postgres");
}
```

```rust
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
    assert_eq!(config.server.auth.identity_sources, vec!["cookie"]);
}
```

- [ ] **Step 2: Run the focused manifest tests and confirm they fail**

Run: `cargo test -p zun-runtime parse_project_config -- --nocapture`
Expected: FAIL because `ServerConfig` has no `auth` field and `ProjectConfig::parse_toml` cannot deserialize nested auth settings.

- [ ] **Step 3: Add minimal auth config structs and defaults**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub port: u16,
    pub host: String,
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
pub struct SessionStoreConfig {
    pub driver: String,
    pub sqlite: SqliteSessionStoreConfig,
    pub postgres: PostgresSessionStoreConfig,
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
```

```rust
// crates/zun-runtime/src/config.rs
pub use crate::module::manifest::{
    AuthConfig, AuthorizationConfig, JwtConfig, PostgresSessionStoreConfig, ProjectConfig,
    ServerConfig, SessionConfig, SessionStoreConfig, SqliteSessionStoreConfig,
};
```

- [ ] **Step 4: Re-run the focused tests and confirm they pass**

Run: `cargo test -p zun-runtime parse_project_config -- --nocapture`
Expected: PASS for explicit auth config parsing and disabled-by-default fallback behavior.

- [ ] **Step 5: Commit the config schema slice**

```bash
git add crates/zun-runtime/src/module/manifest.rs crates/zun-runtime/src/config.rs
git commit -m "feat: add serve auth config schema"
```

### Task 2: Add Runtime Auth Models, Validation, And Request-Scoped Context

**Files:**
- Create: `crates/zun-runtime/src/runtime/auth/mod.rs`
- Create: `crates/zun-runtime/src/runtime/auth/config.rs`
- Create: `crates/zun-runtime/src/runtime/auth/model.rs`
- Create: `crates/zun-runtime/src/runtime/auth/context.rs`
- Create: `crates/zun-runtime/src/runtime/auth/errors.rs`
- Modify: `crates/zun-runtime/src/runtime/mod.rs`
- Modify: `crates/zun-runtime/src/runtime/context.rs`
- Test: `crates/zun-runtime/src/runtime/context.rs`
- Test: `crates/zun-runtime/src/runtime/auth/config.rs`

- [ ] **Step 1: Write the failing runtime auth tests**

```rust
#[test]
fn clone_for_request_execution_carries_empty_auth_state() {
    let context = RuntimeContext::new(RuntimeMode::Serve, PathBuf::from("."));
    let cloned = context.clone_for_request_execution();

    assert!(cloned.request_auth_context().principal().is_none());
    assert!(cloned.request_auth_context().pending_cookie().is_none());
}
```

```rust
#[test]
fn validate_auth_config_rejects_missing_jwt_secret() {
    let config = RuntimeAuthConfig {
        enabled: true,
        jwt_enabled: true,
        jwt_secret: None,
        ..RuntimeAuthConfig::default()
    };

    let error = validate_runtime_auth_config(&config).expect_err("config should fail");
    assert!(error.contains("JWT secret"));
}
```

- [ ] **Step 2: Run the focused auth-core tests and confirm they fail**

Run: `cargo test -p zun-runtime auth_config -- --nocapture`
Expected: FAIL because no runtime auth module or request auth context exists yet.

- [ ] **Step 3: Implement the minimal auth model and request context shell**

```rust
// crates/zun-runtime/src/runtime/auth/model.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Principal {
    pub subject: String,
    pub scheme: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub claims: indexmap::IndexMap<String, crate::interpreter::ZunValue>,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRecord {
    pub session_id: String,
    pub subject: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub claims: indexmap::IndexMap<String, crate::interpreter::ZunValue>,
    pub expires_at: i64,
    pub idle_timeout_at: Option<i64>,
}
```

```rust
// crates/zun-runtime/src/runtime/auth/context.rs
#[derive(Debug, Clone, Default)]
pub struct RequestAuthContext {
    principal: Option<Principal>,
    current_session: Option<SessionRecord>,
    current_jwt_claims: Option<indexmap::IndexMap<String, crate::interpreter::ZunValue>>,
    pending_cookie: Option<String>,
    pending_clear_cookie: bool,
}
```

```rust
// crates/zun-runtime/src/runtime/context.rs
pub struct RuntimeContext {
    // existing fields...
    request_auth_context: crate::runtime::auth::RequestAuthContext,
}

pub fn request_auth_context(&self) -> &crate::runtime::auth::RequestAuthContext {
    &self.request_auth_context
}

pub fn request_auth_context_mut(&mut self) -> &mut crate::runtime::auth::RequestAuthContext {
    &mut self.request_auth_context
}
```

- [ ] **Step 4: Re-run the focused auth-core tests and confirm they pass**

Run: `cargo test -p zun-runtime auth_config -- --nocapture`
Expected: PASS for request clone auth state initialization and startup validation rules.

- [ ] **Step 5: Commit the runtime auth-core scaffold**

```bash
git add crates/zun-runtime/src/runtime/mod.rs crates/zun-runtime/src/runtime/context.rs crates/zun-runtime/src/runtime/auth/mod.rs crates/zun-runtime/src/runtime/auth/config.rs crates/zun-runtime/src/runtime/auth/model.rs crates/zun-runtime/src/runtime/auth/context.rs crates/zun-runtime/src/runtime/auth/errors.rs
git commit -m "feat: add runtime auth core scaffold"
```

### Task 3: Implement The Session Store Trait And Memory Driver

**Files:**
- Create: `crates/zun-runtime/src/runtime/auth/session_store.rs`
- Create: `crates/zun-runtime/src/runtime/auth/session_memory.rs`
- Modify: `crates/zun-runtime/src/runtime/auth/mod.rs`
- Modify: `crates/zun-runtime/src/runtime/context.rs`
- Test: `crates/zun-runtime/src/runtime/auth/session_memory.rs`

- [ ] **Step 1: Write the failing session store tests**

```rust
#[test]
fn memory_store_round_trips_session_record() {
    let mut store = MemorySessionStore::default();
    let session = SessionRecord {
        session_id: "sess_1".to_string(),
        subject: "user_1".to_string(),
        roles: vec!["admin".to_string()],
        permissions: vec!["post:create".to_string()],
        claims: indexmap::IndexMap::new(),
        expires_at: 1_800_000_000,
        idle_timeout_at: Some(1_800_000_600),
    };

    store.create(session.clone()).expect("create should succeed");
    let loaded = store.get("sess_1").expect("lookup should succeed");
    assert_eq!(loaded, Some(session));
}
```

```rust
#[test]
fn memory_store_rotate_replaces_old_session() {
    let mut store = MemorySessionStore::default();
    let original = sample_session("sess_old");
    let rotated = sample_session("sess_new");

    store.create(original).unwrap();
    store.rotate("sess_old", rotated.clone()).unwrap();

    assert!(store.get("sess_old").unwrap().is_none());
    assert_eq!(store.get("sess_new").unwrap(), Some(rotated));
}
```

- [ ] **Step 2: Run the focused session-memory tests and confirm they fail**

Run: `cargo test -p zun-runtime memory_store -- --nocapture`
Expected: FAIL because no session store trait or memory driver exists.

- [ ] **Step 3: Implement the trait and in-memory driver**

```rust
pub trait SessionStore: Send + Sync {
    fn get(&mut self, session_id: &str) -> Result<Option<SessionRecord>, Error>;
    fn create(&mut self, session: SessionRecord) -> Result<(), Error>;
    fn update(&mut self, session: SessionRecord) -> Result<(), Error>;
    fn rotate(&mut self, old_session_id: &str, session: SessionRecord) -> Result<(), Error>;
    fn delete(&mut self, session_id: &str) -> Result<(), Error>;
}
```

```rust
#[derive(Debug, Default)]
pub struct MemorySessionStore {
    sessions: std::collections::HashMap<String, SessionRecord>,
}

impl SessionStore for MemorySessionStore {
    fn get(&mut self, session_id: &str) -> Result<Option<SessionRecord>, Error> {
        Ok(self.sessions.get(session_id).cloned())
    }

    fn create(&mut self, session: SessionRecord) -> Result<(), Error> {
        self.sessions.insert(session.session_id.clone(), session);
        Ok(())
    }

    fn update(&mut self, session: SessionRecord) -> Result<(), Error> {
        self.sessions.insert(session.session_id.clone(), session);
        Ok(())
    }

    fn rotate(&mut self, old_session_id: &str, session: SessionRecord) -> Result<(), Error> {
        self.sessions.remove(old_session_id);
        self.sessions.insert(session.session_id.clone(), session);
        Ok(())
    }

    fn delete(&mut self, session_id: &str) -> Result<(), Error> {
        self.sessions.remove(session_id);
        Ok(())
    }
}
```

- [ ] **Step 4: Re-run the focused session-memory tests and confirm they pass**

Run: `cargo test -p zun-runtime memory_store -- --nocapture`
Expected: PASS for create/get/update/rotate/delete semantics in the memory driver.

- [ ] **Step 5: Commit the memory store slice**

```bash
git add crates/zun-runtime/src/runtime/auth/mod.rs crates/zun-runtime/src/runtime/auth/session_store.rs crates/zun-runtime/src/runtime/auth/session_memory.rs crates/zun-runtime/src/runtime/context.rs
git commit -m "feat: add in-memory auth session store"
```

### Task 4: Add SQLite And Postgres Session Store Drivers

**Files:**
- Create: `crates/zun-runtime/src/runtime/auth/session_sqlite.rs`
- Create: `crates/zun-runtime/src/runtime/auth/session_postgres.rs`
- Modify: `crates/zun-runtime/src/runtime/auth/session_store.rs`
- Modify: `crates/zun-runtime/src/runtime/context.rs`
- Modify: `crates/zun-runtime/Cargo.toml`
- Test: `crates/zun-runtime/src/runtime/auth/session_sqlite.rs`
- Test: `crates/zun-runtime/src/runtime/auth/session_postgres.rs`

- [ ] **Step 1: Write the failing persistent store tests**

```rust
#[test]
fn sqlite_store_round_trips_session_record() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    let mut store = SqliteSessionStore::new(conn, "auth_sessions");
    store.ensure_schema().unwrap();

    let session = sample_session("sess_sqlite");
    store.create(session.clone()).unwrap();

    assert_eq!(store.get("sess_sqlite").unwrap(), Some(session));
}
```

```rust
#[test]
fn postgres_store_round_trips_session_record_when_url_present() {
    let Some(url) = std::env::var("ZUN_TEST_POSTGRES_URL").ok() else {
        return;
    };

    let mut store = PostgresSessionStore::connect(&url, "auth_sessions_plan").unwrap();
    store.ensure_schema().unwrap();

    let session = sample_session("sess_pg");
    store.create(session.clone()).unwrap();

    assert_eq!(store.get("sess_pg").unwrap(), Some(session));
    store.delete("sess_pg").unwrap();
}
```

- [ ] **Step 2: Run the focused persistent-store tests and confirm they fail**

Run: `cargo test -p zun-runtime sqlite_store_round_trips_session_record -- --nocapture`
Expected: FAIL because the SQLite driver and auth session schema helpers do not exist.

Run: `env ZUN_TEST_POSTGRES_URL=host=/tmp dbname=postgres user=liangzhanbo cargo test -p zun-runtime postgres_store_round_trips_session_record_when_url_present -- --nocapture`
Expected: FAIL because the Postgres auth session driver does not exist.

- [ ] **Step 3: Implement the SQLite and Postgres drivers with a shared row shape**

```rust
const CREATE_AUTH_SESSIONS_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS auth_sessions (
    session_id TEXT PRIMARY KEY,
    subject TEXT NOT NULL,
    roles_json TEXT NOT NULL,
    permissions_json TEXT NOT NULL,
    claims_json TEXT NOT NULL,
    expires_at BIGINT NOT NULL,
    idle_timeout_at BIGINT,
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL
)
"#;
```

```rust
pub struct SqliteSessionStore {
    conn: rusqlite::Connection,
    table: String,
}

impl SqliteSessionStore {
    pub fn ensure_schema(&self) -> Result<(), Error> {
        self.conn.execute_batch(&format!(
            "CREATE TABLE IF NOT EXISTS {} (...)",
            self.table
        ))?;
        Ok(())
    }
}
```

```rust
pub struct PostgresSessionStore {
    client: postgres::Client,
    table: String,
}

impl PostgresSessionStore {
    pub fn ensure_schema(&mut self) -> Result<(), Error> {
        self.client.batch_execute(&format!(
            "CREATE TABLE IF NOT EXISTS {} (...)",
            self.table
        ))?;
        Ok(())
    }
}
```

- [ ] **Step 4: Re-run the focused persistent-store tests and confirm they pass**

Run: `cargo test -p zun-runtime sqlite_store_round_trips_session_record -- --nocapture`
Expected: PASS for SQLite auth session persistence.

Run: `env ZUN_TEST_POSTGRES_URL=host=/tmp dbname=postgres user=liangzhanbo cargo test -p zun-runtime postgres_store_round_trips_session_record_when_url_present -- --nocapture`
Expected: PASS for Postgres auth session persistence when the URL is present.

- [ ] **Step 5: Commit the persistent store slice**

```bash
git add crates/zun-runtime/src/runtime/auth/session_store.rs crates/zun-runtime/src/runtime/auth/session_sqlite.rs crates/zun-runtime/src/runtime/auth/session_postgres.rs crates/zun-runtime/Cargo.toml
git commit -m "feat: add sqlite and postgres auth session stores"
```

### Task 5: Add JWT And Password Helpers In The Runtime Auth Layer

**Files:**
- Create: `crates/zun-runtime/src/runtime/auth/jwt.rs`
- Create: `crates/zun-runtime/src/runtime/auth/password.rs`
- Modify: `crates/zun-runtime/src/runtime/auth/mod.rs`
- Modify: `crates/zun-runtime/Cargo.toml`
- Test: `crates/zun-runtime/src/runtime/auth/jwt.rs`
- Test: `crates/zun-runtime/src/runtime/auth/password.rs`

- [ ] **Step 1: Write the failing JWT and password tests**

```rust
#[test]
fn jwt_sign_and_verify_round_trip_principal_claims() {
    let config = sample_runtime_auth_config_with_secret("super-secret");
    let token = sign_jwt(
        &config,
        "user_1",
        &["admin".to_string()],
        &["post:create".to_string()],
        &indexmap::IndexMap::new(),
    )
    .expect("token should sign");

    let claims = verify_jwt(&config, &token).expect("token should verify");
    assert_eq!(claims.sub, "user_1");
    assert_eq!(claims.roles, vec!["admin".to_string()]);
}
```

```rust
#[test]
fn password_hash_and_verify_round_trip() {
    let hash = hash_password("correct horse battery staple").expect("hash should succeed");
    assert!(verify_password("correct horse battery staple", &hash).unwrap());
    assert!(!verify_password("wrong password", &hash).unwrap());
}
```

- [ ] **Step 2: Run the focused JWT/password tests and confirm they fail**

Run: `cargo test -p zun-runtime jwt_sign_and_verify_round_trip_principal_claims -- --nocapture`
Expected: FAIL because no JWT helper exists.

Run: `cargo test -p zun-runtime password_hash_and_verify_round_trip -- --nocapture`
Expected: FAIL because no password helper exists.

- [ ] **Step 3: Implement the minimal JWT and password helpers**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String,
    pub iss: String,
    pub aud: String,
    pub exp: usize,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    #[serde(flatten)]
    pub extra: std::collections::BTreeMap<String, serde_json::Value>,
}
```

```rust
pub fn sign_jwt(
    config: &RuntimeAuthConfig,
    subject: &str,
    roles: &[String],
    permissions: &[String],
    claims: &indexmap::IndexMap<String, crate::interpreter::ZunValue>,
) -> Result<String, Error> {
    let claims = JwtClaims::new(config, subject, roles, permissions, claims)?;
    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(config.jwt_secret.as_deref().unwrap().as_bytes()),
    )
    .map_err(|err| Error::Interpreter(format!("failed to sign jwt: {err}")))
}
```

```rust
pub fn hash_password(password: &str) -> Result<String, Error> {
    let salt = argon2::password_hash::SaltString::generate(&mut rand::thread_rng());
    Ok(argon2::Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|err| Error::Interpreter(format!("failed to hash password: {err}")))?
        .to_string())
}
```

- [ ] **Step 4: Re-run the focused JWT/password tests and confirm they pass**

Run: `cargo test -p zun-runtime jwt_sign_and_verify_round_trip_principal_claims -- --nocapture`
Expected: PASS for signing and verification.

Run: `cargo test -p zun-runtime password_hash_and_verify_round_trip -- --nocapture`
Expected: PASS for password hash/verify.

- [ ] **Step 5: Commit the JWT/password helpers**

```bash
git add crates/zun-runtime/src/runtime/auth/mod.rs crates/zun-runtime/src/runtime/auth/jwt.rs crates/zun-runtime/src/runtime/auth/password.rs crates/zun-runtime/Cargo.toml
git commit -m "feat: add runtime auth jwt and password helpers"
```

### Task 6: Expose `std.auth.session`, `std.auth.jwt`, `std.auth.password`, And `std.auth.guard`

**Files:**
- Create: `crates/zun-runtime/src/stdlib_native/auth_session.rs`
- Create: `crates/zun-runtime/src/stdlib_native/auth_jwt.rs`
- Create: `crates/zun-runtime/src/stdlib_native/auth_password.rs`
- Create: `crates/zun-runtime/src/stdlib_native/auth_guard.rs`
- Modify: `crates/zun-runtime/src/stdlib_native/mod.rs`
- Test: `crates/zun-runtime/src/stdlib_native/mod.rs`
- Test: `tests/spec/valid/stdlib/auth_session_flow.zun`
- Test: `tests/spec/valid/stdlib/auth_jwt_flow.zun`
- Test: `tests/spec/valid/stdlib/auth_guard_flow.zun`
- Test: `tests/spec/invalid/stdlib/auth_guard_require_role_fails.zun`

- [ ] **Step 1: Write the failing stdlib fixture and registration tests**

```rust
#[test]
fn aggregate_registration_exposes_auth_modules() {
    let mut context = RuntimeContext::new(RuntimeMode::Test, PathBuf::from("."));
    register_stdlib_native_modules(&mut context);

    assert!(context.native_module("std.auth.session").is_some());
    assert!(context.native_module("std.auth.jwt").is_some());
    assert!(context.native_module("std.auth.password").is_some());
    assert!(context.native_module("std.auth.guard").is_some());
}
```

```zun
$mod std.auth.jwt;

$ token = jwt.sign({
    "sub": "user_1",
    "roles": ["admin"],
    "permissions": ["post:create"]
});

$ claims = jwt.verify(token);
$>> claims["sub"];
$>> claims["roles"][0];
```

- [ ] **Step 2: Run the focused stdlib tests and confirm they fail**

Run: `cargo test -p zun-runtime aggregate_registration_exposes_auth_modules -- --nocapture`
Expected: FAIL because `std.auth.*` modules are not registered.

Run: `cargo test --test integration_suite auth_ -- --nocapture`
Expected: FAIL because the new stdlib fixtures and modules do not exist yet.

- [ ] **Step 3: Implement the minimal auth stdlib modules**

```rust
// crates/zun-runtime/src/stdlib_native/auth_guard.rs
pub fn register(context: &mut RuntimeContext) {
    let mut exports = NativeFnMap::new();
    exports.insert(
        "principal".to_string(),
        Arc::new(|_, context| {
            let principal = context
                .request_auth_context()
                .principal()
                .cloned()
                .ok_or_else(|| Error::Interpreter("no authenticated principal".to_string()))?;
            Ok(principal_to_zun_value(&principal))
        }),
    );
    context.register_native_module("std.auth.guard", exports);
}
```

```rust
// crates/zun-runtime/src/stdlib_native/mod.rs
mod auth_guard;
mod auth_jwt;
mod auth_password;
mod auth_session;

pub fn register_stdlib_native_modules(context: &mut RuntimeContext) {
    // existing modules...
    auth_session::register(context);
    auth_jwt::register(context);
    auth_password::register(context);
    auth_guard::register(context);
}
```

- [ ] **Step 4: Re-run the focused stdlib tests and confirm they pass**

Run: `cargo test -p zun-runtime aggregate_registration_exposes_auth_modules -- --nocapture`
Expected: PASS for module registration.

Run: `cargo test --test integration_suite auth_ -- --nocapture`
Expected: PASS for fixture-based stdlib session/jwt/guard flows.

- [ ] **Step 5: Commit the auth stdlib surface**

```bash
git add crates/zun-runtime/src/stdlib_native/mod.rs crates/zun-runtime/src/stdlib_native/auth_session.rs crates/zun-runtime/src/stdlib_native/auth_jwt.rs crates/zun-runtime/src/stdlib_native/auth_password.rs crates/zun-runtime/src/stdlib_native/auth_guard.rs tests/spec/valid/stdlib/auth_session_flow.zun tests/spec/valid/stdlib/auth_session_flow.zun.stdout tests/spec/valid/stdlib/auth_jwt_flow.zun tests/spec/valid/stdlib/auth_jwt_flow.zun.stdout tests/spec/valid/stdlib/auth_guard_flow.zun tests/spec/valid/stdlib/auth_guard_flow.zun.stdout tests/spec/invalid/stdlib/auth_guard_require_role_fails.zun tests/spec/invalid/stdlib/auth_guard_require_role_fails.zun.error
git commit -m "feat: add auth stdlib modules"
```

### Task 7: Integrate The Auth Pipeline Into Request Execution And Axum

**Files:**
- Create: `crates/zun-runtime/src/runtime/auth/pipeline.rs`
- Modify: `crates/zun-runtime/src/runtime/http.rs`
- Modify: `crates/zun-cli/src/backends/axum_backend.rs`
- Modify: `tests/integration_suite.rs`
- Test: `tests/integration_suite.rs`

- [ ] **Step 1: Write the failing live HTTP auth integration tests**

```rust
#[test]
fn live_http_session_protected_route_returns_401_without_cookie() {
    let project_dir = write_temp_project(
        r#"
name = "auth-session"
version = "0.1.0"
entry = "main.zun"

[server]
host = "127.0.0.1"
port = 0

[server.auth]
enabled = true
default_scheme = "session"
identity_sources = ["cookie"]

[server.auth.session]
enabled = true
cookie_name = "zun_session"
ttl_seconds = 86400

[server.auth.session.store]
driver = "memory"

[server.auth.authorization]
enabled = true
default = "public"

[[server.auth.authorization.rules]]
method = "GET"
path = "/admin"
require = "authenticated"
"#,
        r#"
$GET("/login") login() -> JSON {
    std.auth.session.create({
        subject: "user_1",
        roles: ["admin"],
        permissions: []
    });
    $# {"ok": true};
}

$GET("/admin") admin() -> JSON {
    $# {"ok": true};
}
"#,
    );

    let outcome = run_program_at_path(&project_dir, RuntimeMode::Serve);
    let base_url = start_live_http_server_from_context(outcome.context);
    let response = http_get(&base_url, "/admin");
    assert_eq!(response.status, 401);
}
```

```rust
#[test]
fn live_http_login_sets_cookie_and_allows_protected_route() {
    // same project as above
    let login = http_get(&base_url, "/login");
    let session_cookie = header_value(&login.headers, "set-cookie").unwrap();
    let admin = http_request_with_headers("GET", &base_url, "/admin", &[("Cookie", session_cookie)]);

    assert_eq!(login.status, 200);
    assert_eq!(admin.status, 200);
}
```

- [ ] **Step 2: Run the focused live HTTP auth tests and confirm they fail**

Run: `cargo test --test integration_suite live_http_session_ -- --nocapture`
Expected: FAIL because request execution does not currently parse cookies, build principals, or enforce auth rules.

- [ ] **Step 3: Implement the auth pipeline and HTTP response hooks**

```rust
pub fn execute_http_route(
    route: &HttpRoute,
    input: &HandlerInput,
    context: &RuntimeContext,
) -> (bool, Option<ZunValue>, Option<String>) {
    let mut runtime_context = context.clone_for_request_execution();
    let mut state = ProgramState::new();

    if let Err(error) = apply_request_auth(route, input, &mut runtime_context) {
        return (true, None, Some(error.to_string()));
    }

    // existing handler setup...
    let (should_continue, result, error) =
        exec_http_handler(&route.body, &mut state, &mut runtime_context);

    if error.is_none() {
        if let Err(error) = commit_request_auth_side_effects(&mut runtime_context) {
            return (true, None, Some(error.to_string()));
        }
    }

    (should_continue, result, error)
}
```

```rust
fn append_auth_cookie_headers(
    mut response: axum::response::Response,
    context: &RuntimeContext,
) -> axum::response::Response {
    if let Some(cookie) = context.request_auth_context().pending_cookie() {
        response.headers_mut().append(
            axum::http::header::SET_COOKIE,
            HeaderValue::from_str(cookie).unwrap(),
        );
    }
    if context.request_auth_context().pending_clear_cookie() {
        response.headers_mut().append(
            axum::http::header::SET_COOKIE,
            HeaderValue::from_static("zun_session=; Max-Age=0; Path=/; HttpOnly"),
        );
    }
    response
}
```

- [ ] **Step 4: Re-run the focused live HTTP auth tests and confirm they pass**

Run: `cargo test --test integration_suite live_http_session_ -- --nocapture`
Expected: PASS for session login, cookie emission, protected route `401`, and authenticated route success.

- [ ] **Step 5: Commit the HTTP auth integration**

```bash
git add crates/zun-runtime/src/runtime/auth/pipeline.rs crates/zun-runtime/src/runtime/http.rs crates/zun-cli/src/backends/axum_backend.rs tests/integration_suite.rs
git commit -m "feat: integrate auth into serve request pipeline"
```

### Task 8: Add Route Authorization Rules, Bearer JWT Support, And Store-Specific Integration Coverage

**Files:**
- Modify: `crates/zun-runtime/src/runtime/auth/pipeline.rs`
- Modify: `crates/zun-cli/src/backends/axum_backend.rs`
- Modify: `tests/integration_suite.rs`
- Test: `tests/integration_suite.rs`

- [ ] **Step 1: Write the failing authorization and bearer tests**

```rust
#[test]
fn live_http_bearer_token_allows_protected_route() {
    let token = issue_test_token("user_1", &["admin"], &["post:create"]);
    let response = http_request_with_headers(
        "GET",
        &base_url,
        "/admin",
        &[("Authorization", &format!("Bearer {token}"))],
    );

    assert_eq!(response.status, 200);
}
```

```rust
#[test]
fn live_http_role_guard_returns_403_for_authenticated_but_unauthorized_user() {
    let token = issue_test_token("user_2", &["viewer"], &[]);
    let response = http_request_with_headers(
        "GET",
        &base_url,
        "/admin",
        &[("Authorization", &format!("Bearer {token}"))],
    );

    assert_eq!(response.status, 403);
}
```

```rust
#[test]
fn live_http_sqlite_session_store_persists_login_flow() {
    // start a temp project with [server.auth.session.store] driver = "sqlite"
    // assert login writes a cookie and protected route accepts it.
}
```

- [ ] **Step 2: Run the focused authz/JWT/store tests and confirm they fail**

Run: `cargo test --test integration_suite live_http_bearer_ -- --nocapture`
Expected: FAIL because bearer token resolution and role/permission checks are not wired into the route pipeline.

Run: `cargo test --test integration_suite live_http_sqlite_session_store_persists_login_flow -- --nocapture`
Expected: FAIL because the runtime does not yet select the SQLite store from config.

- [ ] **Step 3: Implement route auth rules, bearer resolution, and config-driven store selection**

```rust
pub fn authorize_request(
    route: &HttpRoute,
    principal: Option<&Principal>,
    config: &RuntimeAuthConfig,
) -> Result<(), AuthError> {
    let rule = config.match_rule(&route.method, &route.path);
    if !rule.requires_authentication() {
        return Ok(());
    }

    let principal = principal.ok_or(AuthError::Unauthorized)?;
    if !rule.allows_roles(&principal.roles) {
        return Err(AuthError::Forbidden);
    }
    if !rule.allows_permissions(&principal.permissions) {
        return Err(AuthError::Forbidden);
    }
    Ok(())
}
```

```rust
pub fn resolve_identity_from_request(
    input: &HandlerInput,
    context: &mut RuntimeContext,
) -> Result<Option<Principal>, AuthError> {
    for source in context.runtime_auth_config().identity_sources.iter() {
        match source.as_str() {
            "cookie" => if let Some(principal) = resolve_session_cookie(input, context)? {
                return Ok(Some(principal));
            },
            "bearer" => if let Some(principal) = resolve_bearer_token(input, context)? {
                return Ok(Some(principal));
            },
            _ => {}
        }
    }
    Ok(None)
}
```

- [ ] **Step 4: Re-run the focused authz/JWT/store tests and confirm they pass**

Run: `cargo test --test integration_suite live_http_bearer_ -- --nocapture`
Expected: PASS for bearer token auth and role-based `403`.

Run: `cargo test --test integration_suite live_http_sqlite_session_store_persists_login_flow -- --nocapture`
Expected: PASS for the SQLite-backed session flow.

- [ ] **Step 5: Commit the authz and bearer slice**

```bash
git add crates/zun-runtime/src/runtime/auth/pipeline.rs crates/zun-cli/src/backends/axum_backend.rs tests/integration_suite.rs
git commit -m "feat: add route auth rules and bearer token auth"
```

### Task 9: Sync User-Facing Documentation And Security Notes

**Files:**
- Modify: `docs/spec/security-model.md`
- Modify: `docs/reference/http.md`
- Modify: `docs/reference/stdlib-api.md`
- Modify: `docs/CHANGELOG.md`

- [ ] **Step 1: Write the failing doc sync checklist**

```text
- security model documents request-scoped auth context and sensitive auth config
- HTTP reference documents serve auth config, 401 vs 403 behavior, and cookie/bearer identity flow
- stdlib API reference documents std.auth.session, std.auth.jwt, std.auth.password, std.auth.guard
- changelog records the new auth platform capability
```

Expected: the current docs do not contain any of the above auth platform behavior.

- [ ] **Step 2: Run a targeted grep to confirm the docs are missing these entries**

Run: `rg -n "std\\.auth|session store|401|403|bearer token|principal" docs/spec/security-model.md docs/reference/http.md docs/reference/stdlib-api.md docs/CHANGELOG.md`
Expected: either no matches or only unrelated existing mentions.

- [ ] **Step 3: Add the minimal auth documentation updates**

```md
## Serve Auth

`serve` mode can now enforce auth rules from `package.toml` before handler execution.

- unauthenticated protected requests return `401`
- authenticated but unauthorized requests return `403`
- identity can come from session cookies or bearer tokens
```

```md
### `std.auth.guard`

| API | Return | Description | Stability |
| --- | --- | --- | --- |
| `guard.principal()` | `Map` | Returns the current authenticated principal | Preview |
| `guard.require_role(name)` | `Null` | Throws an authz failure when the role is missing | Preview |
```

- [ ] **Step 4: Re-run the targeted grep and confirm the new auth docs are present**

Run: `rg -n "std\\.auth|session store|401|403|bearer token|principal" docs/spec/security-model.md docs/reference/http.md docs/reference/stdlib-api.md docs/CHANGELOG.md`
Expected: PASS with matches in all four files.

- [ ] **Step 5: Commit the documentation sync**

```bash
git add docs/spec/security-model.md docs/reference/http.md docs/reference/stdlib-api.md docs/CHANGELOG.md
git commit -m "docs: add serve auth platform references"
```

### Task 10: Run Full Verification Before Declaring The Auth Platform Ready

**Files:**
- Test only: `crates/zun-runtime/src/module/manifest.rs`
- Test only: `crates/zun-runtime/src/runtime/auth/*.rs`
- Test only: `crates/zun-runtime/src/stdlib_native/*.rs`
- Test only: `tests/integration_suite.rs`

- [ ] **Step 1: Run focused runtime unit tests**

Run: `cargo test -p zun-runtime auth -- --nocapture`
Expected: PASS for config validation, session store drivers, JWT helpers, password helpers, and stdlib auth module registration.

- [ ] **Step 2: Run focused integration coverage**

Run: `cargo test --test integration_suite auth_ -- --nocapture`
Expected: PASS for stdlib fixtures and live HTTP auth flows.

- [ ] **Step 3: Run the full integration suite**

Run: `cargo test --test integration_suite -- --nocapture`
Expected: PASS with the new auth coverage coexisting with existing HTTP, SQL, and runtime behavior.

- [ ] **Step 4: Run the whole workspace test suite if time permits**

Run: `cargo test --workspace -- --nocapture`
Expected: PASS, or if a pre-existing unrelated failure remains, capture it explicitly before claiming completion.

- [ ] **Step 5: Commit the final verification checkpoint**

```bash
git status --short
```

Expected: the worktree only shows the intended auth platform changes and no unverified surprises remain.

## Self-Review

- Spec coverage:
  - `package.toml` configuration: Task 1
  - unified `principal` model and request context: Task 2
  - `memory` / `sqlite` / `postgres` session stores: Tasks 3 and 4
  - JWT and password helpers: Task 5
  - `std.auth.*` modules: Task 6
  - request lifecycle integration and `401` / `403`: Tasks 7 and 8
  - docs and security sync: Task 9
  - verification: Task 10
- Placeholder scan:
  - Removed generic deferred-work language; every task contains concrete files, code, commands, and expected outcomes.
- Type consistency:
  - The plan uses `ProjectConfig.server.auth`, `RuntimeAuthConfig`, `Principal`, `SessionRecord`, `RequestAuthContext`, and `SessionStore` consistently across later tasks.
