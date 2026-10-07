use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::ast::TypeExpr;
use crate::config::ProjectConfig;
use crate::error::Error;
use crate::interpreter::{CorsConfig, HttpRoute, ModuleNamespace, StaticRoute};
use crate::runtime::auth::{
    MemorySessionStore, PendingSessionWrite, PostgresSessionStore, RefreshTokenStoreBackend,
    RefreshTokenStoreMemory, RefreshTokenStorePostgres, RefreshTokenStoreSqlite,
    RequestAuthContext, RuntimeAuthConfig, SessionStore, SessionStoreBackend, SqliteSessionStore,
};

use std::collections::HashMap;

/// A single field in a registered type shape.
#[derive(Debug, Clone)]
pub struct TypeField {
    pub name: String,
    pub type_expr: TypeExpr,
    pub hidden: bool,
}

/// Shape descriptor registered by `$Type` declarations.
#[derive(Debug, Clone)]
pub struct TypeShape {
    pub name: String,
    pub fields: Vec<TypeField>,
}

use super::intrinsics::{
    IntrinsicRegistry, NativeFn, NativeFnMap, NativeModuleRegistry, RuntimePolicy,
};
use super::sql_registry::SqlConnRegistry;

#[derive(Debug, Clone)]
pub enum ModuleNamespaceCacheEntry {
    Loading { module_path: String },
    Ready(Arc<ModuleNamespace>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeMode {
    Repl,
    Run,
    Serve,
    Test,
}

pub struct RuntimeContext {
    mode: RuntimeMode,
    project_root: PathBuf,
    current_file: Option<String>,
    project_config: Option<ProjectConfig>,
    intrinsic_registry: IntrinsicRegistry,
    runtime_policy: RuntimePolicy,
    global_cors: Option<CorsConfig>,
    http_routes: Arc<Vec<HttpRoute>>,
    static_routes: Arc<Vec<StaticRoute>>,
    sql_conn_registry: Arc<Mutex<SqlConnRegistry>>,
    /// Callable native functions accessible directly by name from Zun code.
    native_fn_registry: Arc<HashMap<String, NativeFn>>,
    /// Native modules importable via `$mod path;`.
    native_module_registry: Arc<NativeModuleRegistry>,
    /// Type shapes registered by `$Type` declarations.
    type_registry: Arc<HashMap<String, TypeShape>>,
    module_namespace_cache: Arc<Mutex<HashMap<String, ModuleNamespaceCacheEntry>>>,
    runtime_auth_config: RuntimeAuthConfig,
    request_auth_context: Arc<Mutex<RequestAuthContext>>,
    session_store: Arc<Mutex<SessionStoreBackend>>,
    refresh_token_store: Arc<Mutex<RefreshTokenStoreBackend>>,
}

impl RuntimeContext {
    pub fn new(mode: RuntimeMode, project_root: PathBuf) -> Self {
        Self {
            mode,
            project_root,
            current_file: None,
            project_config: None,
            intrinsic_registry: IntrinsicRegistry::with_defaults(),
            runtime_policy: RuntimePolicy::allow_all(),
            global_cors: None,
            http_routes: Arc::new(Vec::new()),
            static_routes: Arc::new(Vec::new()),
            sql_conn_registry: Arc::new(Mutex::new(SqlConnRegistry::new())),
            native_fn_registry: Arc::new(HashMap::new()),
            native_module_registry: Arc::new(NativeModuleRegistry::new()),
            type_registry: Arc::new(HashMap::new()),
            module_namespace_cache: Arc::new(Mutex::new(HashMap::new())),
            runtime_auth_config: RuntimeAuthConfig::default(),
            request_auth_context: Arc::new(Mutex::new(RequestAuthContext::default())),
            session_store: Arc::new(Mutex::new(SessionStoreBackend::Memory(
                MemorySessionStore::default(),
            ))),
            refresh_token_store: Arc::new(Mutex::new(RefreshTokenStoreBackend::Memory(
                RefreshTokenStoreMemory::default(),
            ))),
        }
    }

    pub fn mode(&self) -> &RuntimeMode {
        &self.mode
    }

    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    pub fn current_file(&self) -> Option<&str> {
        self.current_file.as_deref()
    }

    pub fn set_current_file(&mut self, filename: Option<String>) {
        self.current_file = filename;
    }

    pub fn set_project_config(&mut self, project_config: Option<ProjectConfig>) {
        self.project_config = project_config;
        self.runtime_auth_config =
            RuntimeAuthConfig::from_project_config(self.project_config.as_ref());
        self.session_store = Arc::new(Mutex::new(self.build_session_store()));
        self.refresh_token_store = Arc::new(Mutex::new(self.build_refresh_token_store()));
    }

    pub fn is_serve_mode(&self) -> bool {
        matches!(self.mode, RuntimeMode::Serve)
    }

    pub fn project_config(&self) -> Option<&ProjectConfig> {
        self.project_config.as_ref()
    }

    pub fn set_intrinsic_registry(&mut self, intrinsic_registry: IntrinsicRegistry) {
        self.intrinsic_registry = intrinsic_registry;
    }

    pub fn set_runtime_policy(&mut self, runtime_policy: RuntimePolicy) {
        self.runtime_policy = runtime_policy;
    }

    #[cfg(test)]
    pub(crate) fn sql_conn_registry(&self) -> &Arc<Mutex<SqlConnRegistry>> {
        &self.sql_conn_registry
    }

    pub(crate) fn with_sql_conn_registry<T>(
        &self,
        f: impl FnOnce(&mut SqlConnRegistry) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let mut registry = self.sql_conn_registry.lock().map_err(|_| {
            Error::Interpreter("runtime SQL connection registry lock was poisoned".to_string())
        })?;
        f(&mut registry)
    }

    pub fn clone_for_request_execution(&self) -> Self {
        self.clone_with_sql_conn_registry(Arc::new(Mutex::new(SqlConnRegistry::new())))
    }

    pub(crate) fn clone_for_isolated_execution(&self) -> Self {
        self.clone_with_sql_conn_registry(Arc::new(Mutex::new(SqlConnRegistry::new())))
    }

    fn clone_with_sql_conn_registry(&self, sql_conn_registry: Arc<Mutex<SqlConnRegistry>>) -> Self {
        Self {
            mode: self.mode.clone(),
            project_root: self.project_root.clone(),
            current_file: self.current_file.clone(),
            project_config: self.project_config.clone(),
            intrinsic_registry: self.intrinsic_registry.clone(),
            runtime_policy: self.runtime_policy.clone(),
            global_cors: self.global_cors.clone(),
            http_routes: Arc::clone(&self.http_routes),
            static_routes: Arc::clone(&self.static_routes),
            sql_conn_registry,
            native_fn_registry: Arc::clone(&self.native_fn_registry),
            native_module_registry: Arc::clone(&self.native_module_registry),
            type_registry: Arc::clone(&self.type_registry),
            module_namespace_cache: Arc::new(Mutex::new(HashMap::new())),
            runtime_auth_config: self.runtime_auth_config.clone(),
            request_auth_context: Arc::new(Mutex::new(RequestAuthContext::default())),
            session_store: Arc::clone(&self.session_store),
            refresh_token_store: Arc::clone(&self.refresh_token_store),
        }
    }

    pub fn runtime_auth_config(&self) -> &RuntimeAuthConfig {
        &self.runtime_auth_config
    }

    pub fn cached_module_namespace(
        &self,
        path: &str,
    ) -> Result<Option<ModuleNamespaceCacheEntry>, Error> {
        let cache = self.module_namespace_cache.lock().map_err(|_| {
            Error::Interpreter("module namespace cache lock was poisoned".to_string())
        })?;
        Ok(cache.get(path).cloned())
    }

    pub fn mark_module_namespace_loading(
        &self,
        path: String,
        module_path: String,
    ) -> Result<(), Error> {
        let mut cache = self.module_namespace_cache.lock().map_err(|_| {
            Error::Interpreter("module namespace cache lock was poisoned".to_string())
        })?;
        cache.insert(path, ModuleNamespaceCacheEntry::Loading { module_path });
        Ok(())
    }

    pub fn cache_module_namespace(
        &self,
        path: String,
        state: Arc<ModuleNamespace>,
    ) -> Result<(), Error> {
        let mut cache = self.module_namespace_cache.lock().map_err(|_| {
            Error::Interpreter("module namespace cache lock was poisoned".to_string())
        })?;
        cache.insert(path, ModuleNamespaceCacheEntry::Ready(state));
        Ok(())
    }

    pub fn clear_module_namespace_cache_entry(&self, path: &str) -> Result<(), Error> {
        let mut cache = self.module_namespace_cache.lock().map_err(|_| {
            Error::Interpreter("module namespace cache lock was poisoned".to_string())
        })?;
        cache.remove(path);
        Ok(())
    }

    pub fn with_request_auth_context<T>(
        &self,
        f: impl FnOnce(&RequestAuthContext) -> T,
    ) -> Result<T, Error> {
        let auth_context = self.request_auth_context.lock().map_err(|_| {
            Error::Interpreter("request auth context lock was poisoned".to_string())
        })?;
        Ok(f(&auth_context))
    }

    pub fn with_request_auth_context_mut<T>(
        &self,
        f: impl FnOnce(&mut RequestAuthContext) -> T,
    ) -> Result<T, Error> {
        let mut auth_context = self.request_auth_context.lock().map_err(|_| {
            Error::Interpreter("request auth context lock was poisoned".to_string())
        })?;
        Ok(f(&mut auth_context))
    }

    pub fn with_session_store_mut<T>(
        &self,
        f: impl FnOnce(&mut SessionStoreBackend) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let mut session_store = self
            .session_store
            .lock()
            .map_err(|_| Error::Interpreter("auth session store lock was poisoned".to_string()))?;
        f(&mut session_store)
    }

    pub fn commit_pending_auth_side_effects(&self) -> Result<(), Error> {
        let writes =
            self.with_request_auth_context_mut(|auth| auth.take_pending_session_writes())?;
        if writes.is_empty() {
            return Ok(());
        }

        self.with_session_store_mut(|store| {
            for write in writes {
                match write {
                    PendingSessionWrite::Create(session) => {
                        store.create(session)?;
                    }
                    PendingSessionWrite::Update(session) => {
                        store.update(session)?;
                    }
                    PendingSessionWrite::Rotate {
                        old_session_id,
                        session,
                    } => {
                        store.rotate(&old_session_id, session)?;
                    }
                    PendingSessionWrite::Delete { session_id } => {
                        store.delete(&session_id)?;
                    }
                }
            }
            Ok(())
        })
    }

    pub fn with_refresh_token_store_mut<T>(
        &self,
        f: impl FnOnce(&mut RefreshTokenStoreBackend) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let mut refresh_token_store = self.refresh_token_store.lock().map_err(|_| {
            Error::Interpreter("auth refresh token store lock was poisoned".to_string())
        })?;
        f(&mut refresh_token_store)
    }

    pub fn call_intrinsic(
        &self,
        id: &str,
        args: &[crate::interpreter::ZunValue],
    ) -> Result<crate::interpreter::ZunValue, crate::error::Error> {
        self.runtime_policy.check(id)?;
        self.intrinsic_registry.call(id, args, self)
    }

    /// Register a native (Rust) function callable from Zun by name.
    pub fn register_native_fn(&mut self, name: impl Into<String>, f: NativeFn) {
        Arc::make_mut(&mut self.native_fn_registry).insert(name.into(), f);
    }

    /// Look up a registered native function by name.
    pub fn get_native_fn(&self, name: &str) -> Option<&NativeFn> {
        self.native_fn_registry.get(name)
    }

    /// Register a native module importable via `$mod path;`.
    pub fn register_native_module(&mut self, path: &str, exports: NativeFnMap) {
        Arc::make_mut(&mut self.native_module_registry).register(path, exports);
    }

    /// Look up a registered native module by path.
    pub fn native_module(&self, path: &str) -> Option<&NativeFnMap> {
        self.native_module_registry.get(path)
    }

    /// Register a type shape from a `$Type` declaration.
    pub fn register_type(&mut self, name: impl Into<String>, shape: TypeShape) {
        Arc::make_mut(&mut self.type_registry).insert(name.into(), shape);
    }

    /// Look up a registered type shape by name.
    pub fn get_type(&self, name: &str) -> Option<&TypeShape> {
        self.type_registry.get(name)
    }

    pub(crate) fn merge_types_from(&mut self, other: &RuntimeContext) {
        Arc::make_mut(&mut self.type_registry).extend(
            other
                .type_registry
                .iter()
                .map(|(name, shape)| (name.clone(), shape.clone())),
        );
    }

    pub fn clear_routes(&mut self) {
        Arc::make_mut(&mut self.http_routes).clear();
        Arc::make_mut(&mut self.static_routes).clear();
    }

    pub fn set_global_cors(&mut self, cors: Option<CorsConfig>) {
        self.global_cors = cors;
    }

    pub fn global_cors(&self) -> Option<&CorsConfig> {
        self.global_cors.as_ref()
    }

    pub fn register_http_route(&mut self, route: HttpRoute) {
        Arc::make_mut(&mut self.http_routes).push(route);
    }

    pub fn extend_http_routes(&mut self, routes: Vec<HttpRoute>) {
        Arc::make_mut(&mut self.http_routes).extend(routes);
    }

    pub fn register_static_route(&mut self, route: StaticRoute) {
        Arc::make_mut(&mut self.static_routes).push(route);
    }

    pub fn routes(&self) -> &[HttpRoute] {
        &self.http_routes
    }

    pub fn static_routes(&self) -> &[StaticRoute] {
        &self.static_routes
    }

    pub fn server_host(&self) -> &str {
        self.project_config
            .as_ref()
            .map(|cfg| cfg.server.host.as_str())
            .unwrap_or("127.0.0.1")
    }

    pub fn server_port(&self) -> u16 {
        self.project_config
            .as_ref()
            .map(|cfg| cfg.server.port)
            .unwrap_or(8080)
    }

    pub fn server_upload_max_size(&self) -> u64 {
        self.project_config
            .as_ref()
            .map(|cfg| cfg.server.upload_max_size * 1024 * 1024)
            .unwrap_or(50 * 1024 * 1024)
    }

    pub fn server_upload_max_file_size(&self) -> u64 {
        self.project_config
            .as_ref()
            .map(|cfg| cfg.server.upload_max_file_size * 1024 * 1024)
            .unwrap_or(10 * 1024 * 1024)
    }

    fn build_session_store(&self) -> SessionStoreBackend {
        match self.runtime_auth_config.session_store_driver.as_str() {
            "sqlite" => {
                let path = self
                    .project_config
                    .as_ref()
                    .map(|config| config.server.auth.session.store.sqlite.path.clone())
                    .unwrap_or_else(|| ".zun/auth.sqlite3".to_string());
                let table = self
                    .project_config
                    .as_ref()
                    .map(|config| config.server.auth.session.store.sqlite.table.clone())
                    .unwrap_or_else(|| "auth_sessions".to_string());
                let conn =
                    rusqlite::Connection::open(path).expect("sqlite session store should open");
                let store = SqliteSessionStore::new(conn, table);
                store
                    .ensure_schema()
                    .expect("sqlite session store schema should initialize");
                SessionStoreBackend::Sqlite(store)
            }
            "postgres" => {
                let config = self
                    .project_config
                    .as_ref()
                    .expect("project config should exist");
                let url = if !config.server.auth.session.store.postgres.url.is_empty() {
                    config.server.auth.session.store.postgres.url.clone()
                } else if !config.server.auth.session.store.postgres.url_env.is_empty() {
                    std::env::var(&config.server.auth.session.store.postgres.url_env)
                        .expect("postgres session store env should exist")
                } else {
                    panic!("postgres session store requires url or url_env");
                };
                let table = config.server.auth.session.store.postgres.table.clone();
                let mut store = PostgresSessionStore::connect(&url, table)
                    .expect("postgres session store should connect");
                store
                    .ensure_schema()
                    .expect("postgres session store schema should initialize");
                SessionStoreBackend::Postgres(store)
            }
            _ => SessionStoreBackend::Memory(MemorySessionStore::default()),
        }
    }

    fn build_refresh_token_store(&self) -> RefreshTokenStoreBackend {
        match self.runtime_auth_config.session_store_driver.as_str() {
            "sqlite" => {
                let (path, table) = self
                    .project_config
                    .as_ref()
                    .map(|config| {
                        (
                            config.server.auth.session.store.sqlite.path.clone(),
                            config
                                .server
                                .auth
                                .session
                                .store
                                .sqlite
                                .refresh_table
                                .clone(),
                        )
                    })
                    .unwrap_or_else(|| {
                        (
                            ".zun/auth.sqlite3".to_string(),
                            "auth_refresh_tokens".to_string(),
                        )
                    });
                let conn = rusqlite::Connection::open(path)
                    .expect("sqlite refresh token store should open");
                let store = RefreshTokenStoreSqlite::new(conn, table);
                store
                    .ensure_schema()
                    .expect("sqlite refresh token store schema should initialize");
                RefreshTokenStoreBackend::Sqlite(store)
            }
            "postgres" => {
                let config = self
                    .project_config
                    .as_ref()
                    .expect("project config should exist");
                let url = if !config.server.auth.session.store.postgres.url.is_empty() {
                    config.server.auth.session.store.postgres.url.clone()
                } else if !config.server.auth.session.store.postgres.url_env.is_empty() {
                    std::env::var(&config.server.auth.session.store.postgres.url_env)
                        .expect("postgres refresh token store env should exist")
                } else {
                    panic!("postgres refresh token store requires url or url_env");
                };
                let table = config
                    .server
                    .auth
                    .session
                    .store
                    .postgres
                    .refresh_table
                    .clone();
                let mut store = RefreshTokenStorePostgres::connect(&url, table)
                    .expect("postgres refresh token store should connect");
                store
                    .ensure_schema()
                    .expect("postgres refresh token store schema should initialize");
                RefreshTokenStoreBackend::Postgres(store)
            }
            _ => RefreshTokenStoreBackend::Memory(RefreshTokenStoreMemory::default()),
        }
    }
}

impl std::fmt::Debug for RuntimeContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuntimeContext")
            .field("mode", &self.mode)
            .field("project_root", &self.project_root)
            .field("current_file", &self.current_file)
            .field("native_fns", &self.native_fn_registry.len())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use crate::runtime::auth::validate_runtime_auth_config;

    fn test_context() -> RuntimeContext {
        RuntimeContext::new(RuntimeMode::Test, PathBuf::from("."))
    }

    #[test]
    fn request_clone_gets_fresh_sql_registry() {
        let context = test_context();
        let cloned = context.clone_for_request_execution();

        assert!(!Arc::ptr_eq(
            context.sql_conn_registry(),
            cloned.sql_conn_registry()
        ));
    }

    #[test]
    fn isolated_clone_gets_fresh_sql_registry() {
        let context = test_context();
        let cloned = context.clone_for_isolated_execution();

        assert!(!Arc::ptr_eq(
            context.sql_conn_registry(),
            cloned.sql_conn_registry()
        ));
    }

    #[test]
    fn request_and_isolated_clones_do_not_see_parent_registry_entries() {
        let context = test_context();
        let request_clone = context.clone_for_request_execution();
        let isolated_clone = context.clone_for_isolated_execution();
        let id = {
            let mut registry = context.sql_conn_registry().lock().unwrap();
            let id = registry.next_id("sqlite");
            registry.insert(
                id.clone(),
                crate::runtime::sql_registry::SqlConn::Sqlite(Arc::new(Mutex::new(
                    rusqlite::Connection::open_in_memory().unwrap(),
                ))),
            );
            id
        };

        assert!(
            request_clone
                .sql_conn_registry()
                .lock()
                .unwrap()
                .get(&id)
                .is_none()
        );
        assert!(
            isolated_clone
                .sql_conn_registry()
                .lock()
                .unwrap()
                .get(&id)
                .is_none()
        );
    }

    #[test]
    fn clone_for_request_execution_carries_empty_auth_state() {
        let context = RuntimeContext::new(RuntimeMode::Serve, PathBuf::from("."));
        let cloned = context.clone_for_request_execution();

        assert!(
            cloned
                .with_request_auth_context(|auth| auth.principal().is_none())
                .unwrap()
        );
        assert!(
            cloned
                .with_request_auth_context(|auth| auth.pending_cookie().is_none())
                .unwrap()
        );
        assert!(
            !cloned
                .with_request_auth_context(|auth| auth.pending_clear_cookie())
                .unwrap()
        );
    }

    #[test]
    fn request_clone_shares_route_and_type_registries() {
        let mut context = test_context();
        context.register_http_route(HttpRoute {
            method: "GET".to_string(),
            path: "/health".to_string(),
            name: "health".to_string(),
            params: Vec::new(),
            variadic_param: None,
            return_type: Some(crate::ast::TypeExpr::Named("String".to_string())),
            cors: None,
            parent_cors: None,
            response_headers: Vec::new(),
            body: Vec::new(),
            module_state: Arc::new(crate::interpreter::RouteModuleState::default()),
        });
        context.register_type(
            "User",
            TypeShape {
                name: "User".to_string(),
                fields: Vec::new(),
            },
        );

        let cloned = context.clone_for_request_execution();

        assert!(Arc::ptr_eq(&context.http_routes, &cloned.http_routes));
        assert!(Arc::ptr_eq(&context.type_registry, &cloned.type_registry));
        assert_eq!(cloned.routes().len(), 1);
        assert!(cloned.get_type("User").is_some());
    }

    #[test]
    fn runtime_auth_config_validation_rejects_missing_jwt_secret() {
        let config = crate::runtime::auth::RuntimeAuthConfig {
            enabled: true,
            jwt_enabled: true,
            jwt_secret: None,
            ..crate::runtime::auth::RuntimeAuthConfig::default()
        };

        let error = validate_runtime_auth_config(&config).expect_err("config should fail");
        assert!(error.to_string().contains("JWT secret"));
    }

    #[test]
    fn sqlite_refresh_store_uses_configured_refresh_table() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        let sqlite_path = std::env::temp_dir().join(format!("zun-auth-refresh-{unique}.sqlite3"));
        let sqlite_path_string = sqlite_path.to_string_lossy().to_string();
        let manifest = format!(
            r#"
name = "auth-demo"
version = "0.1.0"
entry = "main.zun"

[server.auth]
enabled = true

[server.auth.session]
enabled = true

[server.auth.session.store]
driver = "sqlite"

[server.auth.session.store.sqlite]
path = "{sqlite_path_string}"
table = "auth_sessions_custom"
refresh_table = "auth_refresh_tokens_custom"
"#
        );
        let config = ProjectConfig::parse_toml(&manifest).expect("config should parse");
        let mut context = RuntimeContext::new(RuntimeMode::Test, PathBuf::from("."));

        context.set_project_config(Some(config));

        let conn = rusqlite::Connection::open(&sqlite_path).expect("sqlite db should open");
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name = ?1")
            .expect("sqlite_master query should prepare");
        let table_name: String = stmt
            .query_row(["auth_refresh_tokens_custom"], |row| row.get(0))
            .expect("custom refresh token table should exist");

        assert_eq!(table_name, "auth_refresh_tokens_custom");

        let _ = fs::remove_file(sqlite_path);
    }

    #[test]
    fn sqlite_refresh_store_defaults_to_auth_refresh_tokens() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        let sqlite_path =
            std::env::temp_dir().join(format!("zun-auth-refresh-default-{unique}.sqlite3"));
        let sqlite_path_string = sqlite_path.to_string_lossy().to_string();
        let manifest = format!(
            r#"
name = "auth-demo"
version = "0.1.0"
entry = "main.zun"

[server.auth]
enabled = true

[server.auth.session]
enabled = true

[server.auth.session.store]
driver = "sqlite"

[server.auth.session.store.sqlite]
path = "{sqlite_path_string}"
table = "auth_sessions_custom"
"#
        );
        let config = ProjectConfig::parse_toml(&manifest).expect("config should parse");
        let mut context = RuntimeContext::new(RuntimeMode::Test, PathBuf::from("."));

        context.set_project_config(Some(config));

        let conn = rusqlite::Connection::open(&sqlite_path).expect("sqlite db should open");
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name = ?1")
            .expect("sqlite_master query should prepare");
        let table_name: String = stmt
            .query_row(["auth_refresh_tokens"], |row| row.get(0))
            .expect("default refresh token table should exist");

        assert_eq!(table_name, "auth_refresh_tokens");

        let _ = fs::remove_file(sqlite_path);
    }
}
