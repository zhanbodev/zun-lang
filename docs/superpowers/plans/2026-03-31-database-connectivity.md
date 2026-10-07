# Database Connectivity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add working `std.sqlite` and `std.postgres` database connectivity to Zun with `connect() -> Connection` handles and `query` / `execute` / `close` methods.

**Architecture:** Keep the current `ZunValue::Connection { id, driver }` shape and route all host-side database work through runtime intrinsics first, then expose thin `std.sqlite` and `std.postgres` wrappers on top. Store live Rust connections in a shared `SqlConnRegistry` inside `RuntimeContext`, let `Connection` builtins delegate to intrinsic IDs, and use SQLite-first executable coverage with optional Postgres smoke tests behind an environment variable.

**Tech Stack:** `rusqlite 0.32` with `bundled`, `postgres 0.19` sync client, `Arc<Mutex<...>>` runtime state, spec fixtures under `tests/spec`.

---

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Create | `crates/zun-runtime/src/runtime/sql_registry.rs` | Own live SQL connections and handle allocation |
| Create | `crates/zun-runtime/src/runtime/sql_intrinsics.rs` | Implement `connect/query/execute/close` host operations and value conversion |
| Modify | `crates/zun-runtime/src/runtime/context.rs` | Add shared SQL registry to `RuntimeContext` |
| Modify | `crates/zun-runtime/src/runtime/intrinsics.rs` | Register SQL intrinsic IDs and expose shared arg helpers |
| Modify | `crates/zun-runtime/src/runtime/mod.rs` | Re-export SQL runtime modules |
| Create | `crates/zun-runtime/src/interpreter/builtins/sql_connection.rs` | Dispatch `Connection.query/execute/close` |
| Modify | `crates/zun-runtime/src/interpreter/builtins/mod.rs` | Route `ZunValue::Connection` to the new builtin module |
| Create | `crates/zun-runtime/src/stdlib_native/sqlite.rs` | Register `std.sqlite.connect` |
| Create | `crates/zun-runtime/src/stdlib_native/postgres.rs` | Register `std.postgres.connect` |
| Modify | `crates/zun-runtime/src/stdlib_native/mod.rs` | Register SQL stdlib modules |
| Modify | `docs/runtime/intrinsics.md` | Document SQL intrinsic layer as new host capability |
| Modify | `docs/spec/security-model.md` | Add database connectivity to side-effect and risk model |
| Modify | `docs/reference/stdlib-api.md` | Document public `std.sqlite` and `std.postgres` APIs |
| Modify | `docs/CHANGELOG.md` | Record the new stdlib capability |
| Create | `tests/spec/valid/stdlib/sqlite_basic_crud.zun` | SQLite happy-path executable spec |
| Create | `tests/spec/valid/stdlib/sqlite_basic_crud.zun.stdout` | Expected stdout for happy-path spec |
| Create | `tests/spec/invalid/stdlib/sqlite_query_after_close.zun` | Closed-handle regression fixture |
| Create | `tests/spec/invalid/stdlib/sqlite_query_after_close.zun.error` | Expected error snippet for closed handle |
| Create | `tests/spec/invalid/stdlib/sqlite_bind_unsupported_value.zun` | Unsupported SQL parameter regression fixture |
| Create | `tests/spec/invalid/stdlib/sqlite_bind_unsupported_value.zun.error` | Expected error snippet for unsupported bind |

## Scope Notes

- This plan intentionally supersedes the outdated dependency and value-type steps in `docs/superpowers/plans/2026-03-27-sql-stdlib.md`; `rusqlite`, `postgres`, and `ZunValue::Connection` already exist in the repo.
- Follow current Zun runtime architecture: new host capability goes through runtime intrinsics first, then `std.*`.
- Keep Postgres support in scope, but make its automated runtime verification opt-in through `ZUN_TEST_POSTGRES_URL` so the default test suite stays deterministic.

### Task 1: Add SQL Runtime State And Intrinsic IDs

**Files:**
- Create: `crates/zun-runtime/src/runtime/sql_registry.rs`
- Modify: `crates/zun-runtime/src/runtime/context.rs`
- Modify: `crates/zun-runtime/src/runtime/intrinsics.rs`
- Modify: `crates/zun-runtime/src/runtime/mod.rs`

- [ ] **Step 1: Write failing registry tests first**

Create `crates/zun-runtime/src/runtime/sql_registry.rs` with the test module before the implementation:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_id_is_driver_scoped_and_monotonic() {
        let mut registry = SqlConnRegistry::new();
        assert_eq!(registry.next_id("sqlite"), "conn:sqlite:0");
        assert_eq!(registry.next_id("postgres"), "conn:postgres:1");
    }

    #[test]
    fn insert_get_and_remove_sqlite_connection() {
        let mut registry = SqlConnRegistry::new();
        let id = registry.next_id("sqlite");
        registry.insert(id.clone(), SqlConn::Sqlite(rusqlite::Connection::open_in_memory().unwrap()));
        assert!(matches!(registry.get_mut(&id), Some(SqlConn::Sqlite(_))));
        registry.remove(&id);
        assert!(registry.get_mut(&id).is_none());
    }
}
```

- [ ] **Step 2: Run the targeted tests and confirm they fail**

Run: `cargo test -p zun-runtime sql_registry`
Expected: FAIL because `SqlConn`, `SqlConnRegistry`, and SQL runtime wiring do not exist yet.

- [ ] **Step 3: Implement `SqlConnRegistry` and expose it from the runtime**

Add the runtime state:

```rust
pub enum SqlConn {
    Sqlite(rusqlite::Connection),
    Postgres(postgres::Client),
}

pub struct SqlConnRegistry {
    connections: HashMap<String, SqlConn>,
    counter: usize,
}

impl SqlConnRegistry {
    pub fn new() -> Self {
        Self { connections: HashMap::new(), counter: 0 }
    }

    pub fn next_id(&mut self, driver: &str) -> String {
        let id = format!("conn:{driver}:{}", self.counter);
        self.counter += 1;
        id
    }

    pub fn insert(&mut self, id: String, conn: SqlConn) {
        self.connections.insert(id, conn);
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut SqlConn> {
        self.connections.get_mut(id)
    }

    pub fn remove(&mut self, id: &str) -> Option<SqlConn> {
        self.connections.remove(id)
    }
}
```

Wire `RuntimeContext` to own shared SQL state:

```rust
use crate::runtime::sql_registry::SqlConnRegistry;
use std::sync::{Arc, Mutex};

sql_conn_registry: Arc<Mutex<SqlConnRegistry>>,

sql_conn_registry: Arc::new(Mutex::new(SqlConnRegistry::new())),

pub fn sql_conn_registry(&self) -> &Arc<Mutex<SqlConnRegistry>> {
    &self.sql_conn_registry
}
```

Register intrinsic IDs in `runtime/intrinsics.rs`:

```rust
pub const SQL_SQLITE_CONNECT: &str = "sql.sqlite.connect";
pub const SQL_POSTGRES_CONNECT: &str = "sql.postgres.connect";
pub const SQL_QUERY: &str = "sql.query";
pub const SQL_EXECUTE: &str = "sql.execute";
pub const SQL_CLOSE: &str = "sql.close";
```

- [ ] **Step 4: Export the new modules from `runtime/mod.rs`**

Add:

```rust
pub mod sql_intrinsics;
pub mod sql_registry;

pub use sql_registry::{SqlConn, SqlConnRegistry};
```

- [ ] **Step 5: Re-run the runtime state tests**

Run: `cargo test -p zun-runtime sql_registry`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/zun-runtime/src/runtime/sql_registry.rs \
        crates/zun-runtime/src/runtime/context.rs \
        crates/zun-runtime/src/runtime/intrinsics.rs \
        crates/zun-runtime/src/runtime/mod.rs
git commit -m "feat: add SQL runtime state and intrinsic ids"
```

### Task 2: Implement SQL Intrinsic Handlers

**Files:**
- Create: `crates/zun-runtime/src/runtime/sql_intrinsics.rs`
- Modify: `crates/zun-runtime/src/runtime/intrinsics.rs`

- [ ] **Step 1: Add failing SQLite-first intrinsic tests**

Start `sql_intrinsics.rs` with unit tests that exercise the eventual handlers through a real `RuntimeContext`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{RuntimeContext, RuntimeMode};
    use std::path::PathBuf;

    fn test_context() -> RuntimeContext {
        RuntimeContext::new(RuntimeMode::Test, PathBuf::from("."))
    }

    #[test]
    fn sqlite_connect_query_execute_and_close_round_trip() {
        let context = test_context();
        let conn = sqlite_connect(&[ZunValue::Str(":memory:".to_string())], &context).unwrap();

        sql_execute(
            &[conn.clone(), ZunValue::Str("CREATE TABLE users (id INTEGER, name TEXT)".to_string()), ZunValue::List(vec![])],
            &context,
        ).unwrap();

        sql_execute(
            &[conn.clone(), ZunValue::Str("INSERT INTO users (id, name) VALUES (?, ?)".to_string()), ZunValue::List(vec![ZunValue::Int(1), ZunValue::Str("Alice".to_string())])],
            &context,
        ).unwrap();

        let rows = sql_query(
            &[conn.clone(), ZunValue::Str("SELECT id, name FROM users WHERE id = ?".to_string()), ZunValue::List(vec![ZunValue::Int(1)])],
            &context,
        ).unwrap();

        assert_eq!(format!("{rows}"), "[{id: 1, name: Alice}]");
        sql_close(&[conn], &context).unwrap();
    }
}
```

- [ ] **Step 2: Run the targeted intrinsic tests and confirm they fail**

Run: `cargo test -p zun-runtime sql_intrinsics`
Expected: FAIL because the SQL intrinsic handlers are not implemented yet.

- [ ] **Step 3: Implement the intrinsic handlers and conversion helpers**

Implement these public handlers in `sql_intrinsics.rs`:

```rust
pub fn sqlite_connect(args: &[ZunValue], context: &RuntimeContext) -> Result<ZunValue, Error> { /* ... */ }
pub fn postgres_connect(args: &[ZunValue], context: &RuntimeContext) -> Result<ZunValue, Error> { /* ... */ }
pub fn sql_query(args: &[ZunValue], context: &RuntimeContext) -> Result<ZunValue, Error> { /* ... */ }
pub fn sql_execute(args: &[ZunValue], context: &RuntimeContext) -> Result<ZunValue, Error> { /* ... */ }
pub fn sql_close(args: &[ZunValue], context: &RuntimeContext) -> Result<ZunValue, Error> { /* ... */ }
```

Use this argument contract so `Connection` builtins and stdlib wrappers stay thin:

```rust
// connect
[String(path_or_url)]

// query / execute / close
[Connection { id, driver }, String(sql), List(params)]
[Connection { id, driver }]
```

Use SQLite as the fully-covered implementation path:

```rust
fn zun_to_sqlite(value: &ZunValue) -> Result<rusqlite::types::Value, Error> {
    match value {
        ZunValue::Int(n) => Ok(rusqlite::types::Value::Integer(*n)),
        ZunValue::Float(f) => Ok(rusqlite::types::Value::Real(*f)),
        ZunValue::Str(s) => Ok(rusqlite::types::Value::Text(s.clone())),
        ZunValue::Bool(b) => Ok(rusqlite::types::Value::Integer(if *b { 1 } else { 0 })),
        ZunValue::Null => Ok(rusqlite::types::Value::Null),
        other => Err(Error::Interpreter(format!(
            "sqlite: cannot bind value of type '{}' as SQL parameter",
            other.type_name()
        ))),
    }
}
```

Register the handlers in `runtime/intrinsics.rs`:

```rust
self.register(ids::SQL_SQLITE_CONNECT, sql_intrinsics::sqlite_connect);
self.register(ids::SQL_POSTGRES_CONNECT, sql_intrinsics::postgres_connect);
self.register(ids::SQL_QUERY, sql_intrinsics::sql_query);
self.register(ids::SQL_EXECUTE, sql_intrinsics::sql_execute);
self.register(ids::SQL_CLOSE, sql_intrinsics::sql_close);
```

- [ ] **Step 4: Add optional Postgres smoke coverage**

Add an env-gated test in `sql_intrinsics.rs`:

```rust
#[test]
fn postgres_connect_smoke_test_when_url_is_present() {
    let Some(url) = std::env::var("ZUN_TEST_POSTGRES_URL").ok() else {
        return;
    };
    let context = test_context();
    let conn = postgres_connect(&[ZunValue::Str(url)], &context).unwrap();
    let rows = sql_query(
        &[conn, ZunValue::Str("SELECT 1 AS n".to_string()), ZunValue::List(vec![])],
        &context,
    ).unwrap();
    assert_eq!(format!("{rows}"), "[{n: 1}]");
}
```

- [ ] **Step 5: Run intrinsic tests**

Run: `cargo test -p zun-runtime sql_intrinsics`
Expected: PASS.

Run: `cargo test -p zun-runtime postgres_connect_smoke_test_when_url_is_present -- --nocapture`
Expected: PASS when `ZUN_TEST_POSTGRES_URL` is set; otherwise the test exits early without failure.

- [ ] **Step 6: Commit**

```bash
git add crates/zun-runtime/src/runtime/sql_intrinsics.rs \
        crates/zun-runtime/src/runtime/intrinsics.rs
git commit -m "feat: add SQL runtime intrinsics"
```

### Task 3: Wire `Connection` Builtins To Intrinsics

**Files:**
- Create: `crates/zun-runtime/src/interpreter/builtins/sql_connection.rs`
- Modify: `crates/zun-runtime/src/interpreter/builtins/mod.rs`

- [ ] **Step 1: Add failing builtin dispatch tests**

Add tests that prove `Connection` is no longer a dead value:

```rust
#[test]
fn dispatches_connection_query_and_close() {
    let context = RuntimeContext::new(RuntimeMode::Test, PathBuf::from("."));
    let conn = crate::runtime::sql_intrinsics::sqlite_connect(
        &[ZunValue::Str(":memory:".to_string())],
        &context,
    ).unwrap();

    dispatch(
        &conn,
        "execute",
        &[ZunValue::Str("CREATE TABLE users (id INTEGER)".to_string()), ZunValue::List(vec![])],
        &context,
    ).unwrap();

    let rows = dispatch(
        &conn,
        "query",
        &[ZunValue::Str("SELECT 1 AS id".to_string()), ZunValue::List(vec![])],
        &context,
    ).unwrap();

    assert_eq!(format!("{rows}"), "[{id: 1}]");
    dispatch(&conn, "close", &[], &context).unwrap();
}
```

- [ ] **Step 2: Run the builtin tests and confirm they fail**

Run: `cargo test -p zun-runtime dispatches_connection`
Expected: FAIL because `builtins::dispatch()` still returns `"Connection has no method"`.

- [ ] **Step 3: Implement `sql_connection.rs`**

Create the builtin dispatcher:

```rust
pub fn call(
    receiver: &ZunValue,
    method: &str,
    args: &[ZunValue],
    context: &RuntimeContext,
) -> Result<ZunValue, Error> {
    let receiver = receiver.clone();
    match method {
        "query" => context.call_intrinsic(ids::SQL_QUERY, &[receiver, args[0].clone(), args[1].clone()]),
        "execute" => context.call_intrinsic(ids::SQL_EXECUTE, &[receiver, args[0].clone(), args[1].clone()]),
        "close" => context.call_intrinsic(ids::SQL_CLOSE, &[receiver]),
        _ => Err(Error::Interpreter(format!("Connection has no method '{}'", method))),
    }
}
```

Require exact argument shapes:

```rust
query(sql: String, params: List)
execute(sql: String, params: List)
close()
```

- [ ] **Step 4: Route `ZunValue::Connection` through the new builtin**

In `builtins/mod.rs` add:

```rust
pub mod sql_connection;
```

and change the dispatch arm to:

```rust
ZunValue::Connection { .. } => sql_connection::call(receiver, method, args, context),
```

- [ ] **Step 5: Re-run the builtin tests**

Run: `cargo test -p zun-runtime dispatches_connection`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/zun-runtime/src/interpreter/builtins/sql_connection.rs \
        crates/zun-runtime/src/interpreter/builtins/mod.rs
git commit -m "feat: dispatch SQL connection methods through intrinsics"
```

### Task 4: Add `std.sqlite` And `std.postgres` Native Modules

**Files:**
- Create: `crates/zun-runtime/src/stdlib_native/sqlite.rs`
- Create: `crates/zun-runtime/src/stdlib_native/postgres.rs`
- Modify: `crates/zun-runtime/src/stdlib_native/mod.rs`

- [ ] **Step 1: Add failing module registration tests**

Write unit tests beside each module:

```rust
#[test]
fn sqlite_module_registers_connect() {
    let mut context = RuntimeContext::new(RuntimeMode::Test, PathBuf::from("."));
    register(&mut context);
    let module = context.native_module("std.sqlite").expect("module should register");
    let connect = module.get("connect").expect("connect export should exist");
    let conn = connect(&[ZunValue::Str(":memory:".to_string())], &context).unwrap();
    assert!(matches!(conn, ZunValue::Connection { driver, .. } if driver == "sqlite"));
}
```

- [ ] **Step 2: Run the SQL stdlib tests and confirm they fail**

Run: `cargo test -p zun-runtime sqlite_module_registers_connect`
Expected: FAIL because `std.sqlite` and `std.postgres` are not registered yet.

- [ ] **Step 3: Implement `sqlite.rs` and `postgres.rs` as thin intrinsic wrappers**

`sqlite.rs` should look like:

```rust
pub fn register(context: &mut RuntimeContext) {
    let mut exports = NativeFnMap::new();
    exports.insert(
        "connect".into(),
        Arc::new(|args: &[ZunValue], ctx: &RuntimeContext| {
            ctx.call_intrinsic(ids::SQL_SQLITE_CONNECT, args)
        }),
    );
    context.register_native_module("std.sqlite", exports);
}
```

`postgres.rs` should mirror it:

```rust
pub fn register(context: &mut RuntimeContext) {
    let mut exports = NativeFnMap::new();
    exports.insert(
        "connect".into(),
        Arc::new(|args: &[ZunValue], ctx: &RuntimeContext| {
            ctx.call_intrinsic(ids::SQL_POSTGRES_CONNECT, args)
        }),
    );
    context.register_native_module("std.postgres", exports);
}
```

Update `stdlib_native/mod.rs`:

```rust
mod postgres;
mod sqlite;

sqlite::register(context);
postgres::register(context);
```

- [ ] **Step 4: Run the stdlib registration tests**

Run: `cargo test -p zun-runtime sqlite_module_registers_connect`
Expected: PASS.

Run: `cargo test -p zun-runtime postgres`
Expected: PASS for unit tests; env-gated smoke coverage still depends on `ZUN_TEST_POSTGRES_URL`.

- [ ] **Step 5: Commit**

```bash
git add crates/zun-runtime/src/stdlib_native/sqlite.rs \
        crates/zun-runtime/src/stdlib_native/postgres.rs \
        crates/zun-runtime/src/stdlib_native/mod.rs
git commit -m "feat: add SQL stdlib modules"
```

### Task 5: Add Executable Spec Coverage

**Files:**
- Create: `tests/spec/valid/stdlib/sqlite_basic_crud.zun`
- Create: `tests/spec/valid/stdlib/sqlite_basic_crud.zun.stdout`
- Create: `tests/spec/invalid/stdlib/sqlite_query_after_close.zun`
- Create: `tests/spec/invalid/stdlib/sqlite_query_after_close.zun.error`
- Create: `tests/spec/invalid/stdlib/sqlite_bind_unsupported_value.zun`
- Create: `tests/spec/invalid/stdlib/sqlite_bind_unsupported_value.zun.error`

- [ ] **Step 1: Add the valid SQLite CRUD fixture**

Create `tests/spec/valid/stdlib/sqlite_basic_crud.zun`:

```zun
$mod std.sqlite;

$ conn = sqlite.connect(":memory:");
conn.execute("CREATE TABLE users (id INTEGER, name TEXT)", []);
conn.execute("INSERT INTO users (id, name) VALUES (?, ?)", [1, "Alice"]);
conn.execute("INSERT INTO users (id, name) VALUES (?, ?)", [2, "Bob"]);

$ rows = conn.query("SELECT id, name FROM users ORDER BY id", []);
$>> rows.len();
$>> rows.first();

conn.close();
```

Create `tests/spec/valid/stdlib/sqlite_basic_crud.zun.stdout`:

```text
2
{id: 1, name: Alice}
```

- [ ] **Step 2: Add invalid fixture for query-after-close**

Create `tests/spec/invalid/stdlib/sqlite_query_after_close.zun`:

```zun
$mod std.sqlite;

$ conn = sqlite.connect(":memory:");
conn.close();
conn.query("SELECT 1 AS n", []);
```

Create `tests/spec/invalid/stdlib/sqlite_query_after_close.zun.error`:

```text
not found or already closed
```

- [ ] **Step 3: Add invalid fixture for unsupported bind values**

Create `tests/spec/invalid/stdlib/sqlite_bind_unsupported_value.zun`:

```zun
$mod std.sqlite;

$ conn = sqlite.connect(":memory:");
conn.execute("CREATE TABLE users (payload TEXT)", []);
conn.execute("INSERT INTO users (payload) VALUES (?)", [{ payload: "bad" }]);
```

Create `tests/spec/invalid/stdlib/sqlite_bind_unsupported_value.zun.error`:

```text
cannot bind value of type 'Map' as SQL parameter
```

- [ ] **Step 4: Run targeted runtime tests plus the full spec suite**

Run: `cargo test -p zun-runtime sqlite`
Expected: PASS.

Run: `cargo test --test spec_suite valid_spec_samples_pass invalid_spec_samples_fail_with_expected_error`
Expected: FAIL now if fixtures are not implemented correctly; PASS once runtime behavior and stdout/error snippets match.

- [ ] **Step 5: Commit**

```bash
git add tests/spec/valid/stdlib/sqlite_basic_crud.zun \
        tests/spec/valid/stdlib/sqlite_basic_crud.zun.stdout \
        tests/spec/invalid/stdlib/sqlite_query_after_close.zun \
        tests/spec/invalid/stdlib/sqlite_query_after_close.zun.error \
        tests/spec/invalid/stdlib/sqlite_bind_unsupported_value.zun \
        tests/spec/invalid/stdlib/sqlite_bind_unsupported_value.zun.error
git commit -m "test: add SQL stdlib spec coverage"
```

### Task 6: Update Documentation And Security Model

**Files:**
- Modify: `docs/runtime/intrinsics.md`
- Modify: `docs/spec/security-model.md`
- Modify: `docs/reference/stdlib-api.md`
- Modify: `docs/CHANGELOG.md`

- [ ] **Step 1: Document the new intrinsic boundary**

In `docs/runtime/intrinsics.md`, extend the intrinsic list:

```md
- `SqlSqliteConnect`
- `SqlPostgresConnect`
- `SqlQuery`
- `SqlExecute`
- `SqlClose`
```

and add a short note:

```md
数据库连接能力先进入 runtime intrinsic 层，再由 `std.sqlite` / `std.postgres` 暴露为脚本 API。
```

- [ ] **Step 2: Update the security model**

In `docs/spec/security-model.md`, add database connectivity under side-effect entries:

```md
- 数据库连接与 SQL 查询
```

and describe the current boundary:

```md
`std.sqlite` 与 `std.postgres` 当前允许脚本使用宿主进程可达的本地数据库文件或网络数据库，不经过额外权限确认，也没有按连接目标做白名单限制。
```

- [ ] **Step 3: Document the public stdlib API**

In `docs/reference/stdlib-api.md`, add a SQL section with exact public signatures:

```md
## `std.sqlite`

- `sqlite.connect(path: String) -> Connection`
- `Connection.query(sql: String, params: List) -> List<Map>`
- `Connection.execute(sql: String, params: List) -> Int`
- `Connection.close() -> Null`

## `std.postgres`

- `postgres.connect(url: String) -> Connection`
- `Connection.query(sql: String, params: List) -> List<Map>`
- `Connection.execute(sql: String, params: List) -> Int`
- `Connection.close() -> Null`
```

- [ ] **Step 4: Add changelog entry and verify docs render cleanly**

Add a concise `docs/CHANGELOG.md` entry describing SQLite and Postgres connectivity.

Run: `cargo test --test spec_suite valid_spec_samples_pass invalid_spec_samples_fail_with_expected_error`
Expected: PASS again after docs-only edits.

- [ ] **Step 5: Commit**

```bash
git add docs/runtime/intrinsics.md \
        docs/spec/security-model.md \
        docs/reference/stdlib-api.md \
        docs/CHANGELOG.md
git commit -m "docs: document SQL stdlib and security boundary"
```

### Task 7: Final Verification

**Files:**
- Modify: none

- [ ] **Step 1: Run the targeted runtime test groups**

Run: `cargo test -p zun-runtime sql_registry sql_intrinsics dispatches_connection sqlite_module_registers_connect`
Expected: PASS.

- [ ] **Step 2: Run the default spec suite**

Run: `cargo test --test spec_suite`
Expected: PASS.

- [ ] **Step 3: Run optional Postgres smoke test if credentials exist**

Run: `cargo test -p zun-runtime postgres_connect_smoke_test_when_url_is_present -- --nocapture`
Expected: PASS when `ZUN_TEST_POSTGRES_URL` is set; otherwise exits early without failure.

- [ ] **Step 4: Create the finishing commit**

```bash
git add crates/zun-runtime/src/runtime/sql_registry.rs \
        crates/zun-runtime/src/runtime/sql_intrinsics.rs \
        crates/zun-runtime/src/runtime/context.rs \
        crates/zun-runtime/src/runtime/intrinsics.rs \
        crates/zun-runtime/src/runtime/mod.rs \
        crates/zun-runtime/src/interpreter/builtins/sql_connection.rs \
        crates/zun-runtime/src/interpreter/builtins/mod.rs \
        crates/zun-runtime/src/stdlib_native/sqlite.rs \
        crates/zun-runtime/src/stdlib_native/postgres.rs \
        crates/zun-runtime/src/stdlib_native/mod.rs \
        docs/runtime/intrinsics.md \
        docs/spec/security-model.md \
        docs/reference/stdlib-api.md \
        docs/CHANGELOG.md \
        tests/spec/valid/stdlib/sqlite_basic_crud.zun \
        tests/spec/valid/stdlib/sqlite_basic_crud.zun.stdout \
        tests/spec/invalid/stdlib/sqlite_query_after_close.zun \
        tests/spec/invalid/stdlib/sqlite_query_after_close.zun.error \
        tests/spec/invalid/stdlib/sqlite_bind_unsupported_value.zun \
        tests/spec/invalid/stdlib/sqlite_bind_unsupported_value.zun.error
git commit -m "feat: add database connectivity stdlib"
```

## Self-Review

- Spec coverage: runtime state, intrinsic boundary, stdlib modules, connection methods, executable specs, docs, and security updates are all mapped to tasks.
- Placeholder scan: no `TODO` / `TBD` placeholders remain; env-gated Postgres verification is explicit and bounded.
- Type consistency: public runtime handle stays `ZunValue::Connection { id, driver }` across intrinsics, builtins, and stdlib modules; SQLite placeholder style is `?`, Postgres placeholder style is `$1..$n`.
