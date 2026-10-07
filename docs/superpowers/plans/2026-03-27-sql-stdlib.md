# SQL Stdlib Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `std.sqlite` and `std.postgres` modules to zun's standard library, giving zun scripts a connection-object API for SQL queries.

**Architecture:** Two independent native modules (`sqlite.rs`, `postgres.rs`) follow the existing `NativeFnMap` registration pattern. `connect()` stores the live Rust connection in a shared `SqlConnRegistry` (held as `Arc<Mutex<...>>` in `RuntimeContext`) and returns a new `ZunValue::Connection { id, driver }` handle. Method calls on that handle (`query`, `execute`, `close`) are dispatched through a new `builtins/sql_connection.rs` file.

**Tech Stack:** `rusqlite 0.32` (bundled, sync), `postgres 0.19` (sync wrapper over tokio-postgres), standard library `Arc<Mutex<>>` for interior mutability.

---

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `crates/zun-runtime/Cargo.toml` | Add rusqlite, postgres deps |
| Modify | `crates/zun-runtime/src/interpreter/value.rs` | Add `Connection` variant |
| Create | `crates/zun-runtime/src/runtime/sql_registry.rs` | Connection registry type |
| Modify | `crates/zun-runtime/src/runtime/mod.rs` | Expose sql_registry |
| Modify | `crates/zun-runtime/src/runtime/context.rs` | Add `sql_conn_registry` field |
| Create | `crates/zun-runtime/src/interpreter/builtins/sql_connection.rs` | `query`/`execute`/`close` builtins |
| Modify | `crates/zun-runtime/src/interpreter/builtins/mod.rs` | Dispatch Connection variant |
| Create | `crates/zun-runtime/src/stdlib_native/sqlite.rs` | `std.sqlite` connect |
| Create | `crates/zun-runtime/src/stdlib_native/postgres.rs` | `std.postgres` connect |
| Modify | `crates/zun-runtime/src/stdlib_native/mod.rs` | Register both modules |

---

## Task 1: Add Dependencies

**Files:**
- Modify: `crates/zun-runtime/Cargo.toml`

- [ ] **Step 1: Add rusqlite and postgres to Cargo.toml**

Replace the `[dependencies]` block in `crates/zun-runtime/Cargo.toml`:

```toml
[package]
name = "zun-runtime"
version = "0.1.0"
edition = "2024"

[dependencies]
zun-frontend = { path = "../zun-frontend" }
indexmap = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
rand = "0.8"
chrono = { version = "0.4", features = ["clock"] }
uuid = { version = "1", features = ["v4"] }
reqwest = { version = "0.12", features = ["blocking", "json"] }
rusqlite = { version = "0.32", features = ["bundled"] }
postgres = "0.19"
```

- [ ] **Step 2: Verify compilation**

```bash
cd /Users/liangzhanbo/CodeStudio/zun
cargo check -p zun-runtime
```

Expected: compiles without errors (warnings about unused imports are OK at this stage).

- [ ] **Step 3: Commit**

```bash
git add crates/zun-runtime/Cargo.toml
git commit -m "chore: add rusqlite and postgres dependencies"
```

---

## Task 2: Add ZunValue::Connection Variant

**Files:**
- Modify: `crates/zun-runtime/src/interpreter/value.rs`

- [ ] **Step 1: Write a failing test confirming Connection doesn't exist yet**

Add to the bottom of `value.rs` (inside the existing or a new `#[cfg(test)]` block):

```rust
#[cfg(test)]
mod connection_tests {
    use super::*;

    #[test]
    fn connection_value_has_correct_type_name() {
        let conn = ZunValue::Connection {
            id: "conn:sqlite:0".to_string(),
            driver: "sqlite".to_string(),
        };
        assert_eq!(conn.type_name(), "Connection");
    }

    #[test]
    fn connection_value_is_truthy() {
        let conn = ZunValue::Connection {
            id: "conn:sqlite:0".to_string(),
            driver: "sqlite".to_string(),
        };
        assert!(conn.is_truthy());
    }

    #[test]
    fn connection_display() {
        let conn = ZunValue::Connection {
            id: "conn:sqlite:0".to_string(),
            driver: "sqlite".to_string(),
        };
        assert_eq!(format!("{}", conn), "Connection(sqlite:conn:sqlite:0)");
    }
}
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cargo test -p zun-runtime connection_value 2>&1 | head -20
```

Expected: compile error — `Connection` variant not found.

- [ ] **Step 3: Add the Connection variant to the enum**

In `value.rs`, add after the `Null` variant (before the closing `}`):

```rust
    Connection {
        id: String,
        driver: String, // "sqlite" or "postgres"
    },
```

- [ ] **Step 4: Update PartialEq**

In the `fn eq` match block, add before the final `_ => false`:

```rust
            (Self::Connection { id: a, .. }, Self::Connection { id: b, .. }) => a == b,
```

- [ ] **Step 5: Update Display**

In the `impl fmt::Display` match block, add before the `Self::Null` arm:

```rust
            Self::Connection { id, driver } => {
                write!(f, "Connection({}:{})", driver, id)
            }
```

- [ ] **Step 6: Update type_name()**

In `pub fn type_name()`, add before `Self::Null =>`:

```rust
            Self::Connection { .. } => Cow::Borrowed("Connection"),
```

- [ ] **Step 7: Update is_truthy()**

In `pub fn is_truthy()`, add before `Self::Null =>`:

```rust
            Self::Connection { .. } => true,
```

- [ ] **Step 8: Run the tests**

```bash
cargo test -p zun-runtime connection_value
```

Expected: all 3 tests PASS.

- [ ] **Step 9: Run full test suite to check for regressions**

```bash
cargo test -p zun-runtime
```

Expected: all existing tests pass.

- [ ] **Step 10: Commit**

```bash
git add crates/zun-runtime/src/interpreter/value.rs
git commit -m "feat: add ZunValue::Connection variant"
```

---

## Task 3: Add SqlConnRegistry to RuntimeContext

**Files:**
- Create: `crates/zun-runtime/src/runtime/sql_registry.rs`
- Modify: `crates/zun-runtime/src/runtime/mod.rs`
- Modify: `crates/zun-runtime/src/runtime/context.rs`

- [ ] **Step 1: Write failing tests for the registry**

Create `crates/zun-runtime/src/runtime/sql_registry.rs` with the tests first:

```rust
use std::collections::HashMap;

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
        Self {
            connections: HashMap::new(),
            counter: 0,
        }
    }

    /// Allocate a new unique connection ID for the given driver name.
    pub fn next_id(&mut self, driver: &str) -> String {
        let id = format!("conn:{}:{}", driver, self.counter);
        self.counter += 1;
        id
    }

    /// Store a connection under the given ID.
    pub fn insert(&mut self, id: String, conn: SqlConn) {
        self.connections.insert(id, conn);
    }

    /// Borrow a connection mutably by ID.
    pub fn get_mut(&mut self, id: &str) -> Option<&mut SqlConn> {
        self.connections.get_mut(id)
    }

    /// Remove and drop a connection.
    pub fn remove(&mut self, id: &str) {
        self.connections.remove(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_id_increments() {
        let mut reg = SqlConnRegistry::new();
        let a = reg.next_id("sqlite");
        let b = reg.next_id("sqlite");
        assert_eq!(a, "conn:sqlite:0");
        assert_eq!(b, "conn:sqlite:1");
    }

    #[test]
    fn insert_and_get() {
        let mut reg = SqlConnRegistry::new();
        let id = reg.next_id("sqlite");
        let db = rusqlite::Connection::open_in_memory().unwrap();
        reg.insert(id.clone(), SqlConn::Sqlite(db));
        assert!(reg.get_mut(&id).is_some());
    }

    #[test]
    fn remove_drops_connection() {
        let mut reg = SqlConnRegistry::new();
        let id = reg.next_id("sqlite");
        let db = rusqlite::Connection::open_in_memory().unwrap();
        reg.insert(id.clone(), SqlConn::Sqlite(db));
        reg.remove(&id);
        assert!(reg.get_mut(&id).is_none());
    }
}
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cargo test -p zun-runtime sql_registry 2>&1 | head -20
```

Expected: compile error — module not found.

- [ ] **Step 3: Expose sql_registry from runtime/mod.rs**

Open `crates/zun-runtime/src/runtime/mod.rs` and add:

```rust
pub mod sql_registry;
```

- [ ] **Step 4: Run registry tests**

```bash
cargo test -p zun-runtime sql_registry
```

Expected: all 3 tests PASS.

- [ ] **Step 5: Add sql_conn_registry field to RuntimeContext**

In `context.rs`, add the import at the top:

```rust
use std::sync::{Arc, Mutex};
use crate::runtime::sql_registry::SqlConnRegistry;
```

Add the field to the `RuntimeContext` struct (after `visible_user_types`):

```rust
    /// Shared SQL connection registry (Arc+Mutex for Clone-safety).
    sql_conn_registry: Arc<Mutex<SqlConnRegistry>>,
```

In `RuntimeContext::new()`, add the field initialization:

```rust
            sql_conn_registry: Arc::new(Mutex::new(SqlConnRegistry::new())),
```

Add a public accessor method at the end of `impl RuntimeContext`:

```rust
    /// Access the shared SQL connection registry.
    pub fn sql_conn_registry(&self) -> &Arc<Mutex<SqlConnRegistry>> {
        &self.sql_conn_registry
    }
```

- [ ] **Step 6: Verify compilation**

```bash
cargo check -p zun-runtime
```

Expected: no errors.

- [ ] **Step 7: Run full tests**

```bash
cargo test -p zun-runtime
```

Expected: all tests pass.

- [ ] **Step 8: Commit**

```bash
git add crates/zun-runtime/src/runtime/sql_registry.rs \
        crates/zun-runtime/src/runtime/mod.rs \
        crates/zun-runtime/src/runtime/context.rs
git commit -m "feat: add SqlConnRegistry to RuntimeContext"
```

---

## Task 4: Implement sql_connection Builtins (query, execute, close)

**Files:**
- Create: `crates/zun-runtime/src/interpreter/builtins/sql_connection.rs`
- Modify: `crates/zun-runtime/src/interpreter/builtins/mod.rs`

- [ ] **Step 1: Write failing tests**

Create `crates/zun-runtime/src/interpreter/builtins/sql_connection.rs`:

```rust
use std::path::PathBuf;

use indexmap::IndexMap;

use crate::error::Error;
use crate::interpreter::ZunValue;
use crate::runtime::sql_registry::{SqlConn, SqlConnRegistry};
use crate::runtime::{RuntimeContext, RuntimeMode};

pub fn call(
    receiver: &ZunValue,
    method: &str,
    args: &[ZunValue],
    context: &RuntimeContext,
) -> Result<ZunValue, Error> {
    let (id, driver) = match receiver {
        ZunValue::Connection { id, driver } => (id.as_str(), driver.as_str()),
        _ => unreachable!("sql_connection::call called on non-Connection value"),
    };

    match method {
        "query" => sql_query(id, driver, args, context),
        "execute" => sql_execute(id, driver, args, context),
        "close" => sql_close(id, context),
        _ => Err(Error::Interpreter(format!(
            "Connection has no method '{}'",
            method
        ))),
    }
}

fn sql_query(
    conn_id: &str,
    driver: &str,
    args: &[ZunValue],
    context: &RuntimeContext,
) -> Result<ZunValue, Error> {
    let sql = match args.get(0) {
        Some(ZunValue::Str(s)) => s.clone(),
        _ => {
            return Err(Error::Interpreter(
                "conn.query: first argument must be a SQL string".to_string(),
            ))
        }
    };
    let params = match args.get(1) {
        Some(ZunValue::List(list)) => list.clone(),
        Some(_) => {
            return Err(Error::Interpreter(
                "conn.query: second argument must be a List of parameters".to_string(),
            ))
        }
        None => vec![],
    };

    let mut registry = context
        .sql_conn_registry()
        .lock()
        .map_err(|_| Error::Interpreter("sql: connection registry lock poisoned".to_string()))?;

    match driver {
        "sqlite" => sqlite_query(conn_id, &sql, &params, &mut registry),
        "postgres" => postgres_query(conn_id, &sql, &params, &mut registry),
        _ => Err(Error::Interpreter(format!("unknown driver: {}", driver))),
    }
}

fn sql_execute(
    conn_id: &str,
    driver: &str,
    args: &[ZunValue],
    context: &RuntimeContext,
) -> Result<ZunValue, Error> {
    let sql = match args.get(0) {
        Some(ZunValue::Str(s)) => s.clone(),
        _ => {
            return Err(Error::Interpreter(
                "conn.execute: first argument must be a SQL string".to_string(),
            ))
        }
    };
    let params = match args.get(1) {
        Some(ZunValue::List(list)) => list.clone(),
        Some(_) => {
            return Err(Error::Interpreter(
                "conn.execute: second argument must be a List of parameters".to_string(),
            ))
        }
        None => vec![],
    };

    let mut registry = context
        .sql_conn_registry()
        .lock()
        .map_err(|_| Error::Interpreter("sql: connection registry lock poisoned".to_string()))?;

    match driver {
        "sqlite" => sqlite_execute(conn_id, &sql, &params, &mut registry),
        "postgres" => postgres_execute(conn_id, &sql, &params, &mut registry),
        _ => Err(Error::Interpreter(format!("unknown driver: {}", driver))),
    }
}

fn sql_close(conn_id: &str, context: &RuntimeContext) -> Result<ZunValue, Error> {
    let mut registry = context
        .sql_conn_registry()
        .lock()
        .map_err(|_| Error::Interpreter("sql: connection registry lock poisoned".to_string()))?;
    registry.remove(conn_id);
    Ok(ZunValue::Null)
}

// ── SQLite helpers ──────────────────────────────────────────────────────────

fn sqlite_query(
    conn_id: &str,
    sql: &str,
    params: &[ZunValue],
    registry: &mut SqlConnRegistry,
) -> Result<ZunValue, Error> {
    let conn = match registry.get_mut(conn_id) {
        Some(SqlConn::Sqlite(c)) => c,
        _ => {
            return Err(Error::Interpreter(format!(
                "sqlite: connection '{}' not found or already closed",
                conn_id
            )))
        }
    };

    let mut stmt = conn.prepare(sql).map_err(|e| {
        Error::Interpreter(format!("sqlite.query: failed to prepare SQL: {}", e))
    })?;

    let col_names: Vec<String> = stmt
        .column_names()
        .into_iter()
        .map(|s| s.to_string())
        .collect();

    let rusqlite_params: Vec<rusqlite::types::Value> = params
        .iter()
        .map(zun_to_rusqlite)
        .collect::<Result<_, _>>()?;

    let rows_iter = stmt
        .query_map(
            rusqlite::params_from_iter(rusqlite_params.iter()),
            |row| {
                let mut map = IndexMap::new();
                for (i, name) in col_names.iter().enumerate() {
                    let val: rusqlite::types::Value = row.get(i)?;
                    map.insert(name.clone(), rusqlite_to_zun(val));
                }
                Ok(ZunValue::Map(map))
            },
        )
        .map_err(|e| Error::Interpreter(format!("sqlite.query: {}", e)))?;

    let mut results = Vec::new();
    for row in rows_iter {
        results.push(row.map_err(|e| Error::Interpreter(format!("sqlite.query: {}", e)))?);
    }
    Ok(ZunValue::List(results))
}

fn sqlite_execute(
    conn_id: &str,
    sql: &str,
    params: &[ZunValue],
    registry: &mut SqlConnRegistry,
) -> Result<ZunValue, Error> {
    let conn = match registry.get_mut(conn_id) {
        Some(SqlConn::Sqlite(c)) => c,
        _ => {
            return Err(Error::Interpreter(format!(
                "sqlite: connection '{}' not found or already closed",
                conn_id
            )))
        }
    };

    let rusqlite_params: Vec<rusqlite::types::Value> = params
        .iter()
        .map(zun_to_rusqlite)
        .collect::<Result<_, _>>()?;

    let affected = conn
        .execute(sql, rusqlite::params_from_iter(rusqlite_params.iter()))
        .map_err(|e| Error::Interpreter(format!("sqlite.execute: {}", e)))?;

    Ok(ZunValue::Int(affected as i64))
}

fn zun_to_rusqlite(v: &ZunValue) -> Result<rusqlite::types::Value, Error> {
    match v {
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

fn rusqlite_to_zun(v: rusqlite::types::Value) -> ZunValue {
    match v {
        rusqlite::types::Value::Integer(n) => ZunValue::Int(n),
        rusqlite::types::Value::Real(f) => ZunValue::Float(f),
        rusqlite::types::Value::Text(s) => ZunValue::Str(s),
        rusqlite::types::Value::Blob(b) => {
            ZunValue::Str(String::from_utf8_lossy(&b).into_owned())
        }
        rusqlite::types::Value::Null => ZunValue::Null,
    }
}

// ── PostgreSQL helpers ──────────────────────────────────────────────────────

fn postgres_query(
    conn_id: &str,
    sql: &str,
    params: &[ZunValue],
    registry: &mut SqlConnRegistry,
) -> Result<ZunValue, Error> {
    let client = match registry.get_mut(conn_id) {
        Some(SqlConn::Postgres(c)) => c,
        _ => {
            return Err(Error::Interpreter(format!(
                "postgres: connection '{}' not found or already closed",
                conn_id
            )))
        }
    };

    let pg_params: Vec<Box<dyn postgres::types::ToSql + Sync>> = params
        .iter()
        .map(zun_to_postgres)
        .collect::<Result<_, _>>()?;

    let param_refs: Vec<&(dyn postgres::types::ToSql + Sync)> =
        pg_params.iter().map(|p| p.as_ref()).collect();

    let rows = client
        .query(sql, param_refs.as_slice())
        .map_err(|e| Error::Interpreter(format!("postgres.query: {}", e)))?;

    let result: Vec<ZunValue> = rows
        .iter()
        .map(|row| {
            let mut map = IndexMap::new();
            for col in row.columns() {
                let val = postgres_col_to_zun(row, col.name());
                map.insert(col.name().to_string(), val);
            }
            ZunValue::Map(map)
        })
        .collect();

    Ok(ZunValue::List(result))
}

fn postgres_execute(
    conn_id: &str,
    sql: &str,
    params: &[ZunValue],
    registry: &mut SqlConnRegistry,
) -> Result<ZunValue, Error> {
    let client = match registry.get_mut(conn_id) {
        Some(SqlConn::Postgres(c)) => c,
        _ => {
            return Err(Error::Interpreter(format!(
                "postgres: connection '{}' not found or already closed",
                conn_id
            )))
        }
    };

    let pg_params: Vec<Box<dyn postgres::types::ToSql + Sync>> = params
        .iter()
        .map(zun_to_postgres)
        .collect::<Result<_, _>>()?;

    let param_refs: Vec<&(dyn postgres::types::ToSql + Sync)> =
        pg_params.iter().map(|p| p.as_ref()).collect();

    let affected = client
        .execute(sql, param_refs.as_slice())
        .map_err(|e| Error::Interpreter(format!("postgres.execute: {}", e)))?;

    Ok(ZunValue::Int(affected as i64))
}

fn zun_to_postgres(
    v: &ZunValue,
) -> Result<Box<dyn postgres::types::ToSql + Sync>, Error> {
    match v {
        ZunValue::Int(n) => Ok(Box::new(*n)),
        ZunValue::Float(f) => Ok(Box::new(*f)),
        ZunValue::Str(s) => Ok(Box::new(s.clone())),
        ZunValue::Bool(b) => Ok(Box::new(*b)),
        ZunValue::Null => Ok(Box::new(Option::<String>::None)),
        other => Err(Error::Interpreter(format!(
            "postgres: cannot bind value of type '{}' as SQL parameter",
            other.type_name()
        ))),
    }
}

fn postgres_col_to_zun(row: &postgres::Row, col_name: &str) -> ZunValue {
    // Try types in preference order; fall back to Null on any error.
    if let Ok(v) = row.try_get::<_, Option<i64>>(col_name) {
        return v.map(ZunValue::Int).unwrap_or(ZunValue::Null);
    }
    if let Ok(v) = row.try_get::<_, Option<f64>>(col_name) {
        return v.map(ZunValue::Float).unwrap_or(ZunValue::Null);
    }
    if let Ok(v) = row.try_get::<_, Option<bool>>(col_name) {
        return v.map(ZunValue::Bool).unwrap_or(ZunValue::Null);
    }
    if let Ok(v) = row.try_get::<_, Option<String>>(col_name) {
        return v.map(ZunValue::Str).unwrap_or(ZunValue::Null);
    }
    ZunValue::Null
}

// ── Unit tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_context() -> RuntimeContext {
        RuntimeContext::new(RuntimeMode::Test, PathBuf::from("."))
    }

    fn open_sqlite_conn(context: &RuntimeContext) -> ZunValue {
        let db = rusqlite::Connection::open_in_memory().unwrap();
        let mut reg = context.sql_conn_registry().lock().unwrap();
        let id = reg.next_id("sqlite");
        reg.insert(id.clone(), SqlConn::Sqlite(db));
        ZunValue::Connection { id, driver: "sqlite".to_string() }
    }

    #[test]
    fn sqlite_create_table_and_insert() {
        let ctx = make_context();
        let conn = open_sqlite_conn(&ctx);

        // CREATE TABLE
        let result = call(
            &conn,
            "execute",
            &[
                ZunValue::Str(
                    "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)".to_string(),
                ),
                ZunValue::List(vec![]),
            ],
            &ctx,
        );
        assert!(result.is_ok(), "{:?}", result);

        // INSERT
        let n = call(
            &conn,
            "execute",
            &[
                ZunValue::Str("INSERT INTO users (name) VALUES (?)".to_string()),
                ZunValue::List(vec![ZunValue::Str("Alice".to_string())]),
            ],
            &ctx,
        )
        .unwrap();
        assert_eq!(n, ZunValue::Int(1));

        // SELECT
        let rows = call(
            &conn,
            "query",
            &[
                ZunValue::Str("SELECT name FROM users WHERE id = ?".to_string()),
                ZunValue::List(vec![ZunValue::Int(1)]),
            ],
            &ctx,
        )
        .unwrap();

        if let ZunValue::List(list) = rows {
            assert_eq!(list.len(), 1);
            if let ZunValue::Map(map) = &list[0] {
                assert_eq!(
                    map.get("name"),
                    Some(&ZunValue::Str("Alice".to_string()))
                );
            } else {
                panic!("row is not a Map");
            }
        } else {
            panic!("result is not a List");
        }
    }

    #[test]
    fn sqlite_close_removes_connection() {
        let ctx = make_context();
        let conn = open_sqlite_conn(&ctx);
        let conn_id = match &conn {
            ZunValue::Connection { id, .. } => id.clone(),
            _ => panic!("not a Connection"),
        };

        call(&conn, "close", &[], &ctx).unwrap();

        let reg = ctx.sql_conn_registry().lock().unwrap();
        assert!(reg.get_mut_by_id(&conn_id).is_none());
    }

    #[test]
    fn unknown_method_returns_error() {
        let ctx = make_context();
        let conn = open_sqlite_conn(&ctx);
        let result = call(&conn, "nonexistent", &[], &ctx);
        assert!(result.is_err());
    }
}
```

> **Note:** The test `sqlite_close_removes_connection` calls `reg.get_mut_by_id()`; add that alias to `SqlConnRegistry` in Task 3's `sql_registry.rs`:
>
> ```rust
> pub fn get_mut_by_id(&mut self, id: &str) -> Option<&mut SqlConn> {
>     self.get_mut(id)
> }
> ```

- [ ] **Step 2: Run failing tests**

```bash
cargo test -p zun-runtime sql_connection 2>&1 | head -20
```

Expected: compile error — module not declared.

- [ ] **Step 3: Declare sql_connection in builtins/mod.rs**

Add to the top of `mod.rs`:

```rust
pub mod sql_connection;
```

Add to the `dispatch()` function, before the `ZunValue::Null` arm:

```rust
        ZunValue::Connection { .. } => sql_connection::call(receiver, method, args, context),
```

- [ ] **Step 4: Run sql_connection tests**

```bash
cargo test -p zun-runtime sql_connection
```

Expected: all tests PASS.

- [ ] **Step 5: Run full tests**

```bash
cargo test -p zun-runtime
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/zun-runtime/src/interpreter/builtins/sql_connection.rs \
        crates/zun-runtime/src/interpreter/builtins/mod.rs \
        crates/zun-runtime/src/runtime/sql_registry.rs
git commit -m "feat: implement sql_connection builtins (query, execute, close)"
```

---

## Task 5: Implement std.sqlite Module

**Files:**
- Create: `crates/zun-runtime/src/stdlib_native/sqlite.rs`
- Modify: `crates/zun-runtime/src/stdlib_native/mod.rs`

- [ ] **Step 1: Create sqlite.rs**

Create `crates/zun-runtime/src/stdlib_native/sqlite.rs`:

```rust
use std::sync::Arc;

use crate::error::Error;
use crate::interpreter::ZunValue;
use crate::runtime::sql_registry::{SqlConn, SqlConnRegistry};
use crate::runtime::{NativeFnMap, RuntimeContext};

pub fn register(context: &mut RuntimeContext) {
    let mut exports = NativeFnMap::new();

    exports.insert(
        "connect".into(),
        Arc::new(|args: &[ZunValue], ctx: &RuntimeContext| {
            let path = match args.get(0) {
                Some(ZunValue::Str(s)) => s.clone(),
                _ => {
                    return Err(Error::Interpreter(
                        "sqlite.connect: expected a file path string".to_string(),
                    ))
                }
            };

            let db = rusqlite::Connection::open(&path).map_err(|e| {
                Error::Interpreter(format!("sqlite.connect: failed to open '{}': {}", path, e))
            })?;

            let mut registry = ctx
                .sql_conn_registry()
                .lock()
                .map_err(|_| Error::Interpreter("sqlite: registry lock poisoned".to_string()))?;

            let id = registry.next_id("sqlite");
            registry.insert(id.clone(), SqlConn::Sqlite(db));

            Ok(ZunValue::Connection {
                id,
                driver: "sqlite".to_string(),
            })
        }),
    );

    context.register_native_module("std.sqlite", exports);
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::runtime::{RuntimeContext, RuntimeMode};

    #[test]
    fn connect_in_memory_returns_connection() {
        let mut ctx = RuntimeContext::new(RuntimeMode::Test, PathBuf::from("."));
        register(&mut ctx);

        let module = ctx.native_module("std.sqlite").unwrap();
        let connect = module.get("connect").unwrap().clone();

        let result = connect(
            &[ZunValue::Str(":memory:".to_string())],
            &ctx,
        );

        assert!(result.is_ok(), "{:?}", result);
        assert!(matches!(
            result.unwrap(),
            ZunValue::Connection { driver, .. } if driver == "sqlite"
        ));
    }

    #[test]
    fn connect_bad_path_returns_error() {
        let mut ctx = RuntimeContext::new(RuntimeMode::Test, PathBuf::from("."));
        register(&mut ctx);

        let module = ctx.native_module("std.sqlite").unwrap();
        let connect = module.get("connect").unwrap().clone();

        // /nonexistent/path/db.sqlite — parent dir doesn't exist
        let result = connect(
            &[ZunValue::Str("/nonexistent/path/db.sqlite".to_string())],
            &ctx,
        );
        assert!(result.is_err());
    }
}
```

- [ ] **Step 2: Register in mod.rs**

In `crates/zun-runtime/src/stdlib_native/mod.rs`:

```rust
mod env;
mod fs;
mod http_client;
mod json;
mod math;
mod sqlite;
mod str;
mod time;
mod uuid;

use super::RuntimeContext;

pub fn register_stdlib_native_modules(context: &mut RuntimeContext) {
    fs::register(context);
    env::register(context);
    http_client::register(context);
    str::register(context);
    math::register(context);
    json::register(context);
    time::register(context);
    uuid::register(context);
    sqlite::register(context);
}
```

- [ ] **Step 3: Run sqlite module tests**

```bash
cargo test -p zun-runtime sqlite
```

Expected: all tests PASS.

- [ ] **Step 4: Run full tests**

```bash
cargo test -p zun-runtime
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/zun-runtime/src/stdlib_native/sqlite.rs \
        crates/zun-runtime/src/stdlib_native/mod.rs
git commit -m "feat: add std.sqlite native module"
```

---

## Task 6: Implement std.postgres Module

**Files:**
- Create: `crates/zun-runtime/src/stdlib_native/postgres.rs`
- Modify: `crates/zun-runtime/src/stdlib_native/mod.rs`

- [ ] **Step 1: Create postgres.rs**

Create `crates/zun-runtime/src/stdlib_native/postgres.rs`:

```rust
use std::sync::Arc;

use crate::error::Error;
use crate::interpreter::ZunValue;
use crate::runtime::sql_registry::{SqlConn, SqlConnRegistry};
use crate::runtime::{NativeFnMap, RuntimeContext};

pub fn register(context: &mut RuntimeContext) {
    let mut exports = NativeFnMap::new();

    exports.insert(
        "connect".into(),
        Arc::new(|args: &[ZunValue], ctx: &RuntimeContext| {
            let url = match args.get(0) {
                Some(ZunValue::Str(s)) => s.clone(),
                _ => {
                    return Err(Error::Interpreter(
                        "postgres.connect: expected a connection URL string".to_string(),
                    ))
                }
            };

            let client = postgres::Client::connect(&url, postgres::NoTls).map_err(|e| {
                Error::Interpreter(format!("postgres.connect: failed to connect: {}", e))
            })?;

            let mut registry = ctx
                .sql_conn_registry()
                .lock()
                .map_err(|_| {
                    Error::Interpreter("postgres: registry lock poisoned".to_string())
                })?;

            let id = registry.next_id("postgres");
            registry.insert(id.clone(), SqlConn::Postgres(client));

            Ok(ZunValue::Connection {
                id,
                driver: "postgres".to_string(),
            })
        }),
    );

    context.register_native_module("std.postgres", exports);
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::runtime::{RuntimeContext, RuntimeMode};

    /// Skips if ZUN_TEST_POSTGRES_URL env var is not set.
    /// Set it to run: ZUN_TEST_POSTGRES_URL=postgres://user:pass@localhost/testdb cargo test
    fn postgres_url() -> Option<String> {
        std::env::var("ZUN_TEST_POSTGRES_URL").ok()
    }

    #[test]
    fn connect_returns_connection_value() {
        let Some(url) = postgres_url() else {
            eprintln!("Skipping postgres test: ZUN_TEST_POSTGRES_URL not set");
            return;
        };

        let mut ctx = RuntimeContext::new(RuntimeMode::Test, PathBuf::from("."));
        register(&mut ctx);

        let module = ctx.native_module("std.postgres").unwrap();
        let connect = module.get("connect").unwrap().clone();

        let result = connect(&[ZunValue::Str(url)], &ctx);
        assert!(result.is_ok(), "{:?}", result);
        assert!(matches!(
            result.unwrap(),
            ZunValue::Connection { driver, .. } if driver == "postgres"
        ));
    }

    #[test]
    fn connect_bad_url_returns_error() {
        let mut ctx = RuntimeContext::new(RuntimeMode::Test, PathBuf::from("."));
        register(&mut ctx);

        let module = ctx.native_module("std.postgres").unwrap();
        let connect = module.get("connect").unwrap().clone();

        let result = connect(
            &[ZunValue::Str("postgres://invalid:5432/nodb".to_string())],
            &ctx,
        );
        assert!(result.is_err());
    }
}
```

- [ ] **Step 2: Register in mod.rs**

```rust
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
    fs::register(context);
    env::register(context);
    http_client::register(context);
    str::register(context);
    math::register(context);
    json::register(context);
    time::register(context);
    uuid::register(context);
    sqlite::register(context);
    postgres::register(context);
}
```

- [ ] **Step 3: Run postgres tests (SQLite-only; skip postgres if no URL)**

```bash
cargo test -p zun-runtime postgres
```

Expected: `connect_bad_url_returns_error` PASSES; `connect_returns_connection_value` is skipped (no env var).

- [ ] **Step 4: Run full test suite**

```bash
cargo test -p zun-runtime
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/zun-runtime/src/stdlib_native/postgres.rs \
        crates/zun-runtime/src/stdlib_native/mod.rs
git commit -m "feat: add std.postgres native module"
```

---

## Task 7: End-to-End Zun Integration Test (SQLite)

**Files:**
- Create: `tests/spec/valid/sqlite/basic_crud.zun`
- Modify: `tests/spec/mod.rs` (or relevant integration test runner)

- [ ] **Step 1: Write the zun test file**

Create `tests/spec/valid/sqlite/basic_crud.zun`:

```
$mod std.sqlite;

$ conn = sqlite.connect(":memory:");

conn.execute("CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT NOT NULL)", []);

conn.execute("INSERT INTO items (name) VALUES (?)", ["apple"]);
conn.execute("INSERT INTO items (name) VALUES (?)", ["banana"]);

$ rows = conn.query("SELECT name FROM items ORDER BY id", []);

$assert rows.len() == 2;
$assert rows[0]["name"] == "apple";
$assert rows[1]["name"] == "banana";

$ n = conn.execute("DELETE FROM items WHERE name = ?", ["apple"]);
$assert n == 1;

$ remaining = conn.query("SELECT COUNT(*) as cnt FROM items", []);
$assert remaining[0]["cnt"] == 1;

conn.close();
```

- [ ] **Step 2: Run the integration test**

```bash
cargo test -p zun-cli -- sqlite 2>&1
```

Expected: test PASSES without errors.

- [ ] **Step 3: Commit**

```bash
git add tests/spec/valid/sqlite/basic_crud.zun
git commit -m "test: add sqlite basic CRUD integration test"
```

---

## Self-Review

**Spec coverage:**
- ✅ Two independent modules: `std.sqlite`, `std.postgres`
- ✅ Connection object style (`connect()` → handle, methods on handle)
- ✅ `query()` returns `List<Map>` (row per element, field names as keys)
- ✅ `execute()` returns `Int` (affected row count)
- ✅ `close()` drops the connection
- ✅ SQLite placeholder `?`, PostgreSQL placeholder `$1`
- ✅ Error handling via existing `Error::Interpreter`
- ✅ No TCP layer needed (rusqlite = embedded; postgres crate = synchronous wrapper)

**Placeholder scan:** No TBD/TODO in any step.

**Type consistency:**
- `SqlConn::Sqlite` / `SqlConn::Postgres` used consistently across `sql_registry.rs`, `sql_connection.rs`, `sqlite.rs`, `postgres.rs`
- `ZunValue::Connection { id, driver }` used consistently
- `context.sql_conn_registry()` returns `&Arc<Mutex<SqlConnRegistry>>` — consistent across all files
