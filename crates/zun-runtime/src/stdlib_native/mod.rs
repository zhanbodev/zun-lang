mod auth_csrf;
mod auth_guard;
mod auth_jwt;
mod auth_password;
mod auth_session;
mod env;
mod fs;
mod http_client;
mod json;
mod math;
mod postgres;
mod sqlite;
mod str;
mod time;
mod uuid;

use super::RuntimeContext;

pub fn register_stdlib_native_modules(context: &mut RuntimeContext) {
    auth_session::register(context);
    auth_jwt::register(context);
    auth_password::register(context);
    auth_guard::register(context);
    auth_csrf::register(context);
    fs::register(context);
    env::register(context);
    http_client::register(context);
    str::register(context);
    math::register(context);
    json::register(context);
    sqlite::register(context);
    postgres::register(context);
    time::register(context);
    uuid::register(context);
}

#[cfg(test)]
mod tests {
    use super::register_stdlib_native_modules;
    use crate::interpreter::{ZunValue, builtins};
    use crate::runtime::{ProgramState, RuntimeContext, RuntimeMode, execute_source_with_writer};
    use std::path::PathBuf;
    use std::sync::{Mutex, OnceLock};

    fn test_context() -> RuntimeContext {
        RuntimeContext::new(RuntimeMode::Test, PathBuf::from("."))
    }

    fn sample_project_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("../../../../sample/test-http-database")
    }

    fn sample_context() -> RuntimeContext {
        let root = sample_project_root();
        let mut context = RuntimeContext::new(RuntimeMode::Test, root.clone());
        register_stdlib_native_modules(&mut context);
        context.set_current_file(Some(root.join("main.zun").to_string_lossy().to_string()));
        context
    }

    fn database_url_env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn restore_database_url(previous: Option<std::ffi::OsString>) {
        match previous {
            Some(value) => unsafe { std::env::set_var("DATABASE_URL", value) },
            None => unsafe { std::env::remove_var("DATABASE_URL") },
        }
    }

    fn assert_sample_sql_registry_empty(context: &RuntimeContext, message: &str) {
        assert!(
            context
                .with_sql_conn_registry(|registry| Ok(registry.get("conn:postgres:0").is_none()))
                .unwrap(),
            "{message}"
        );
    }

    fn run_app_config_snippet() -> String {
        let mut context = sample_context();
        let mut state = ProgramState::new();
        let mut stdout = Vec::new();
        execute_source_with_writer(
            "$mod config.app_config;\n$>> app_config.database_url();\n",
            &mut state,
            &mut context,
            &mut stdout,
        )
        .expect("app config snippet should execute");
        String::from_utf8(stdout).expect("stdout should be utf-8")
    }

    #[test]
    fn aggregate_registration_exposes_sql_modules() {
        let mut context = test_context();
        register_stdlib_native_modules(&mut context);

        assert!(context.native_module("std.sqlite").is_some());
        assert!(context.native_module("std.postgres").is_some());
    }

    #[test]
    fn aggregate_registration_exposes_auth_modules() {
        let mut context = test_context();
        register_stdlib_native_modules(&mut context);

        assert!(context.native_module("std.auth.session").is_some());
        assert!(context.native_module("std.auth.jwt").is_some());
        assert!(context.native_module("std.auth.password").is_some());
        assert!(context.native_module("std.auth.guard").is_some());
        assert!(context.native_module("std.auth.csrf").is_some());
    }

    #[test]
    fn sqlite_module_connects_and_flows_into_connection_builtins() {
        let mut context = test_context();
        register_stdlib_native_modules(&mut context);

        let sqlite = context
            .native_module("std.sqlite")
            .expect("std.sqlite should be registered");
        let connect = sqlite
            .get("connect")
            .expect("std.sqlite should export connect");

        let conn = connect.as_ref()(&[ZunValue::Str(":memory:".to_string())], &context)
            .expect("sqlite connect should succeed");

        builtins::dispatch(
            &conn,
            "execute",
            &[
                ZunValue::Str("CREATE TABLE users (id INTEGER, name TEXT)".to_string()),
                ZunValue::List(vec![]),
            ],
            &context,
        )
        .expect("create table should succeed");

        builtins::dispatch(
            &conn,
            "execute",
            &[
                ZunValue::Str("INSERT INTO users (id, name) VALUES (?, ?)".to_string()),
                ZunValue::List(vec![ZunValue::Int(1), ZunValue::Str("Lin".to_string())]),
            ],
            &context,
        )
        .expect("insert should succeed");

        let rows = builtins::dispatch(
            &conn,
            "query",
            &[
                ZunValue::Str("SELECT id, name FROM users".to_string()),
                ZunValue::List(vec![]),
            ],
            &context,
        )
        .expect("query should succeed");

        assert_eq!(format!("{rows}"), "[{id: 1, name: Lin}]");
    }

    #[test]
    fn sample_app_config_database_url_uses_env_and_fallback() {
        let _guard = database_url_env_lock().lock().unwrap();
        let previous = std::env::var_os("DATABASE_URL");

        unsafe {
            std::env::set_var(
                "DATABASE_URL",
                "postgresql://env-user@127.0.0.1:6543/env-backed-database",
            );
        }
        assert_eq!(
            run_app_config_snippet(),
            "postgresql://env-user@127.0.0.1:6543/env-backed-database\n"
        );

        unsafe {
            std::env::remove_var("DATABASE_URL");
        }
        assert_eq!(
            run_app_config_snippet(),
            "host=/tmp dbname=postgres user=liangzhanbo\n"
        );

        restore_database_url(previous);
    }

    #[test]
    fn sample_safe_query_closes_postgres_connection_when_query_fails() {
        let Some(url) = std::env::var("ZUN_TEST_POSTGRES_URL").ok() else {
            return;
        };

        let _guard = database_url_env_lock().lock().unwrap();
        let previous = std::env::var_os("DATABASE_URL");

        unsafe {
            std::env::set_var("DATABASE_URL", url);
        }

        let mut success_context = sample_context();
        let mut success_state = ProgramState::new();
        let mut success_stdout = Vec::new();
        execute_source_with_writer(
            "$mod shared.db.queries;\n$>> queries.safe_query(\"SELECT 1 AS n\", []);\n",
            &mut success_state,
            &mut success_context,
            &mut success_stdout,
        )
        .expect("successful sample query snippet should execute");

        assert_eq!(
            String::from_utf8(success_stdout).expect("stdout should be utf-8"),
            "[{n: 1}]\n"
        );
        assert_sample_sql_registry_empty(
            &success_context,
            "successful query should close the postgres connection",
        );

        let mut failure_context = sample_context();
        let mut failure_state = ProgramState::new();
        let mut failure_stdout = Vec::new();
        execute_source_with_writer(
            "$mod shared.db.queries;\n\n$try {\n    queries.safe_query(\"SELECT * FROM definitely_missing_table\", []);\n} $catch err {\n    $>> \"database unavailable\";\n}\n",
            &mut failure_state,
            &mut failure_context,
            &mut failure_stdout,
        )
        .expect("sample query snippet should execute");

        assert_eq!(
            String::from_utf8(failure_stdout).expect("stdout should be utf-8"),
            "database unavailable\n"
        );
        assert_sample_sql_registry_empty(
            &failure_context,
            "failed query should close the postgres connection",
        );

        restore_database_url(previous);
    }
}
