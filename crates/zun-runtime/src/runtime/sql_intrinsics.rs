//! SQL runtime intrinsic entry point.

use crate::error::Error;
use crate::interpreter::ZunValue;
use indexmap::IndexMap;
use postgres::types::{ToSql, Type};
use postgres::{Client, NoTls, Row};
use rusqlite::params_from_iter;
use rusqlite::types::{Value as SqliteValue, ValueRef};
use std::sync::{Arc, Mutex};
use std::thread;

use super::context::RuntimeContext;
use super::intrinsics::{ids, intrinsic_string_arg};
use super::sql_registry::{PostgresClientHandle, SqlConn};

fn format_postgres_db_error_message(
    code: Option<&str>,
    message: &str,
    detail: Option<&str>,
) -> String {
    let mut parts = Vec::new();
    if let Some(code) = code {
        parts.push(code.to_string());
    }
    parts.push(message.to_string());
    if let Some(detail) = detail {
        if !detail.is_empty() {
            parts.push(format!("detail: {detail}"));
        }
    }
    parts.join(": ")
}

fn format_postgres_error(err: &postgres::Error) -> String {
    if let Some(db_error) = err.as_db_error() {
        return format_postgres_db_error_message(
            Some(db_error.code().code()),
            db_error.message(),
            db_error.detail(),
        );
    }

    err.to_string()
}

pub fn sqlite_connect(args: &[ZunValue], context: &RuntimeContext) -> Result<ZunValue, Error> {
    let path = intrinsic_string_arg(ids::SQL_SQLITE_CONNECT, args, 0)?;
    let conn = rusqlite::Connection::open(path).map_err(|err| {
        Error::Interpreter(format!(
            "{}: failed to open sqlite database '{}': {err}",
            ids::SQL_SQLITE_CONNECT,
            path
        ))
    })?;

    context.with_sql_conn_registry(|registry| {
        let id = registry.next_id("sqlite");
        registry.insert(id.clone(), SqlConn::Sqlite(Arc::new(Mutex::new(conn))));
        Ok(ZunValue::Connection {
            id,
            driver: "sqlite".to_string(),
        })
    })
}

pub fn postgres_connect(args: &[ZunValue], context: &RuntimeContext) -> Result<ZunValue, Error> {
    let url = intrinsic_string_arg(ids::SQL_POSTGRES_CONNECT, args, 0)?;
    let url = url.to_string();
    let client = thread::spawn(move || {
        Client::connect(url.as_str(), NoTls).map_err(|err| {
            Error::Interpreter(format!(
                "{}: failed to connect to postgres '{}': {err}",
                ids::SQL_POSTGRES_CONNECT,
                url
            ))
        })
    })
    .join()
    .map_err(|_| {
        Error::Interpreter(format!(
            "{}: postgres connect worker thread panicked",
            ids::SQL_POSTGRES_CONNECT
        ))
    })??;

    context.with_sql_conn_registry(|registry| {
        let id = registry.next_id("postgres");
        registry.insert(
            id.clone(),
            SqlConn::Postgres(Arc::new(Mutex::new(PostgresClientHandle::new(client)))),
        );
        Ok(ZunValue::Connection {
            id,
            driver: "postgres".to_string(),
        })
    })
}

pub fn sql_query(args: &[ZunValue], context: &RuntimeContext) -> Result<ZunValue, Error> {
    let (conn_id, driver) = connection_handle_arg(ids::SQL_QUERY, args, 0)?;
    let sql = intrinsic_string_arg(ids::SQL_QUERY, args, 1)?;
    let params = intrinsic_list_arg(ids::SQL_QUERY, args, 2)?;
    let conn = context.with_sql_conn_registry(|registry| {
        registry.get_for_handle(conn_id, driver, ids::SQL_QUERY)
    })?;

    match conn {
        SqlConn::Sqlite(conn) => sqlite_query(&conn, sql, params),
        SqlConn::Postgres(client) => postgres_query(&client, sql, params),
    }
}

pub fn sql_execute(args: &[ZunValue], context: &RuntimeContext) -> Result<ZunValue, Error> {
    let (conn_id, driver) = connection_handle_arg(ids::SQL_EXECUTE, args, 0)?;
    let sql = intrinsic_string_arg(ids::SQL_EXECUTE, args, 1)?;
    let params = intrinsic_list_arg(ids::SQL_EXECUTE, args, 2)?;
    let conn = context.with_sql_conn_registry(|registry| {
        registry.get_for_handle(conn_id, driver, ids::SQL_EXECUTE)
    })?;

    match conn {
        SqlConn::Sqlite(conn) => sqlite_execute(&conn, sql, params),
        SqlConn::Postgres(client) => postgres_execute(&client, sql, params),
    }
}

pub fn sql_close(args: &[ZunValue], context: &RuntimeContext) -> Result<ZunValue, Error> {
    let (conn_id, driver) = connection_handle_arg(ids::SQL_CLOSE, args, 0)?;
    let conn = context.with_sql_conn_registry(|registry| {
        registry.remove_for_handle(conn_id, driver, ids::SQL_CLOSE)
    })?;

    match conn {
        SqlConn::Sqlite(conn) => match sqlite_close(conn) {
            Ok(()) => Ok(ZunValue::Null),
            Err((conn, error)) => {
                context.with_sql_conn_registry(|registry| {
                    registry.insert(conn_id.to_string(), SqlConn::Sqlite(conn));
                    Ok(())
                })?;
                Err(error)
            }
        },
        SqlConn::Postgres(client) => match postgres_close(client) {
            Ok(()) => Ok(ZunValue::Null),
            Err((client, error)) => {
                context.with_sql_conn_registry(|registry| {
                    registry.insert(conn_id.to_string(), SqlConn::Postgres(client));
                    Ok(())
                })?;
                Err(error)
            }
        },
    }
}

fn connection_handle_arg<'a>(
    intrinsic_id: &str,
    args: &'a [ZunValue],
    index: usize,
) -> Result<(&'a str, &'a str), Error> {
    let Some(value) = args.get(index) else {
        return Err(Error::Interpreter(format!(
            "runtime intrinsic '{}' expects at least {} argument(s), got {}",
            intrinsic_id,
            index + 1,
            args.len()
        )));
    };

    match value {
        ZunValue::Connection { id, driver } => Ok((id.as_str(), driver.as_str())),
        other => Err(Error::Interpreter(format!(
            "runtime intrinsic '{}' expects Connection at argument {}, got {}",
            intrinsic_id,
            index,
            other.type_name()
        ))),
    }
}

fn intrinsic_list_arg<'a>(
    intrinsic_id: &str,
    args: &'a [ZunValue],
    index: usize,
) -> Result<&'a [ZunValue], Error> {
    let Some(value) = args.get(index) else {
        return Err(Error::Interpreter(format!(
            "runtime intrinsic '{}' expects at least {} argument(s), got {}",
            intrinsic_id,
            index + 1,
            args.len()
        )));
    };

    match value {
        ZunValue::List(values) => Ok(values.as_slice()),
        other => Err(Error::Interpreter(format!(
            "runtime intrinsic '{}' expects List at argument {}, got {}",
            intrinsic_id,
            index,
            other.type_name()
        ))),
    }
}

fn zun_to_sqlite(value: &ZunValue) -> Result<SqliteValue, Error> {
    match value {
        ZunValue::Int(n) => Ok(SqliteValue::Integer(*n)),
        ZunValue::Float(f) => Ok(SqliteValue::Real(*f)),
        ZunValue::Str(s) => Ok(SqliteValue::Text(s.clone())),
        ZunValue::Bool(b) => Ok(SqliteValue::Integer(if *b { 1 } else { 0 })),
        ZunValue::Null => Ok(SqliteValue::Null),
        other => Err(Error::Interpreter(format!(
            "sqlite: cannot bind value of type '{}' as SQL parameter",
            other.type_name()
        ))),
    }
}

fn sqlite_query(
    conn: &Arc<Mutex<rusqlite::Connection>>,
    sql: &str,
    params: &[ZunValue],
) -> Result<ZunValue, Error> {
    let conn = conn.lock().map_err(|_| {
        Error::Interpreter(format!(
            "{}: sqlite connection lock was poisoned",
            ids::SQL_QUERY
        ))
    })?;
    let bind_values = params
        .iter()
        .map(zun_to_sqlite)
        .collect::<Result<Vec<_>, _>>()?;
    let mut stmt = conn.prepare(sql).map_err(|err| {
        Error::Interpreter(format!("{}: sqlite prepare failed: {err}", ids::SQL_QUERY))
    })?;
    let column_names = stmt
        .column_names()
        .into_iter()
        .map(|name| name.to_string())
        .collect::<Vec<_>>();
    let mut rows = stmt
        .query(params_from_iter(bind_values.iter()))
        .map_err(|err| {
            Error::Interpreter(format!("{}: sqlite query failed: {err}", ids::SQL_QUERY))
        })?;

    let mut out = Vec::new();
    while let Some(row) = rows.next().map_err(|err| {
        Error::Interpreter(format!(
            "{}: sqlite row fetch failed: {err}",
            ids::SQL_QUERY
        ))
    })? {
        let mut mapped = IndexMap::new();
        for (idx, name) in column_names.iter().enumerate() {
            let value = sqlite_value_ref_to_zun(
                name,
                row.get_ref(idx).map_err(|err| {
                    Error::Interpreter(format!(
                        "{}: sqlite column '{}' read failed: {err}",
                        ids::SQL_QUERY,
                        name
                    ))
                })?,
            )?;
            mapped.insert(name.clone(), value);
        }
        out.push(ZunValue::Map(mapped));
    }

    Ok(ZunValue::List(out))
}

fn sqlite_execute(
    conn: &Arc<Mutex<rusqlite::Connection>>,
    sql: &str,
    params: &[ZunValue],
) -> Result<ZunValue, Error> {
    let conn = conn.lock().map_err(|_| {
        Error::Interpreter(format!(
            "{}: sqlite connection lock was poisoned",
            ids::SQL_EXECUTE
        ))
    })?;
    let bind_values = params
        .iter()
        .map(zun_to_sqlite)
        .collect::<Result<Vec<_>, _>>()?;
    let changed = conn
        .execute(sql, params_from_iter(bind_values.iter()))
        .map_err(|err| {
            Error::Interpreter(format!(
                "{}: sqlite execute failed: {err}",
                ids::SQL_EXECUTE
            ))
        })?;

    Ok(ZunValue::Int(changed as i64))
}

fn sqlite_close(
    conn: Arc<Mutex<rusqlite::Connection>>,
) -> Result<(), (Arc<Mutex<rusqlite::Connection>>, Error)> {
    let mutex = match Arc::try_unwrap(conn) {
        Ok(mutex) => mutex,
        Err(conn) => {
            return Err((
                conn,
                Error::Interpreter(format!(
                    "{}: sqlite connection is still in use and cannot be closed",
                    ids::SQL_CLOSE
                )),
            ));
        }
    };
    let conn = match mutex.into_inner() {
        Ok(conn) => conn,
        Err(poisoned) => {
            return Err((
                Arc::new(Mutex::new(poisoned.into_inner())),
                Error::Interpreter(format!(
                    "{}: sqlite connection lock was poisoned",
                    ids::SQL_CLOSE
                )),
            ));
        }
    };
    conn.close().map_err(|(conn, err)| {
        (
            Arc::new(Mutex::new(conn)),
            Error::Interpreter(format!("{}: sqlite close failed: {err}", ids::SQL_CLOSE)),
        )
    })?;

    Ok(())
}

fn postgres_close(
    client: Arc<Mutex<PostgresClientHandle>>,
) -> Result<(), (Arc<Mutex<PostgresClientHandle>>, Error)> {
    match Arc::try_unwrap(client) {
        Ok(mutex) => {
            let _client = match mutex.into_inner() {
                Ok(client) => client,
                Err(poisoned) => {
                    return Err((
                        Arc::new(Mutex::new(poisoned.into_inner())),
                        Error::Interpreter(format!(
                            "{}: postgres connection lock was poisoned",
                            ids::SQL_CLOSE
                        )),
                    ));
                }
            };
            Ok(())
        }
        Err(client) => Err((
            client,
            Error::Interpreter(format!(
                "{}: postgres connection is still in use and cannot be closed",
                ids::SQL_CLOSE
            )),
        )),
    }
}

fn sqlite_value_ref_to_zun(column_name: &str, value: ValueRef<'_>) -> Result<ZunValue, Error> {
    match value {
        ValueRef::Null => Ok(ZunValue::Null),
        ValueRef::Integer(n) => Ok(ZunValue::Int(n)),
        ValueRef::Real(f) => Ok(ZunValue::Float(f)),
        ValueRef::Text(bytes) => Ok(ZunValue::Str(String::from_utf8_lossy(bytes).into_owned())),
        ValueRef::Blob(_) => Err(Error::Interpreter(format!(
            "{}: sqlite column '{}' returned unsupported blob data",
            ids::SQL_QUERY,
            column_name
        ))),
    }
}

fn postgres_execute(
    client: &Arc<Mutex<PostgresClientHandle>>,
    sql: &str,
    params: &[ZunValue],
) -> Result<ZunValue, Error> {
    let client = Arc::clone(client);
    let sql = sql.to_string();
    let params = params.to_vec();
    let changed = thread::spawn(move || {
        let owned_params = zun_to_postgres_params(&params)?;
        let param_refs = postgres_param_refs(&owned_params);
        let mut client = client.lock().map_err(|_| {
            Error::Interpreter(format!(
                "{}: postgres connection lock was poisoned",
                ids::SQL_EXECUTE
            ))
        })?;
        client
            .client_mut()?
            .execute(sql.as_str(), &param_refs)
            .map_err(|err| {
                Error::Interpreter(format!(
                    "{}: postgres execute failed: {}",
                    ids::SQL_EXECUTE,
                    format_postgres_error(&err)
                ))
            })
    })
    .join()
    .map_err(|_| {
        Error::Interpreter(format!(
            "{}: postgres execute worker thread panicked",
            ids::SQL_EXECUTE
        ))
    })??;

    let changed = i64::try_from(changed).map_err(|_| {
        Error::Interpreter(format!(
            "{}: affected row count does not fit in Int",
            ids::SQL_EXECUTE
        ))
    })?;
    Ok(ZunValue::Int(changed))
}

fn postgres_query(
    client: &Arc<Mutex<PostgresClientHandle>>,
    sql: &str,
    params: &[ZunValue],
) -> Result<ZunValue, Error> {
    let client = Arc::clone(client);
    let sql = sql.to_string();
    let params = params.to_vec();
    let rows = thread::spawn(move || {
        let owned_params = zun_to_postgres_params(&params)?;
        let param_refs = postgres_param_refs(&owned_params);
        let mut client = client.lock().map_err(|_| {
            Error::Interpreter(format!(
                "{}: postgres connection lock was poisoned",
                ids::SQL_QUERY
            ))
        })?;
        client
            .client_mut()?
            .query(sql.as_str(), &param_refs)
            .map_err(|err| {
                Error::Interpreter(format!(
                    "{}: postgres query failed: {}",
                    ids::SQL_QUERY,
                    format_postgres_error(&err)
                ))
            })
    })
    .join()
    .map_err(|_| {
        Error::Interpreter(format!(
            "{}: postgres query worker thread panicked",
            ids::SQL_QUERY
        ))
    })??;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let mut mapped = IndexMap::new();
        for (idx, column) in row.columns().iter().enumerate() {
            mapped.insert(column.name().to_string(), postgres_value_to_zun(&row, idx)?);
        }
        out.push(ZunValue::Map(mapped));
    }

    Ok(ZunValue::List(out))
}

fn zun_to_postgres_params(params: &[ZunValue]) -> Result<Vec<Box<dyn ToSql + Sync>>, Error> {
    params
        .iter()
        .map(|value| match value {
            ZunValue::Int(n) => Ok(Box::new(*n) as Box<dyn ToSql + Sync>),
            ZunValue::Float(f) => Ok(Box::new(*f) as Box<dyn ToSql + Sync>),
            ZunValue::Str(s) => Ok(Box::new(s.clone()) as Box<dyn ToSql + Sync>),
            ZunValue::Bool(b) => Ok(Box::new(*b) as Box<dyn ToSql + Sync>),
            ZunValue::Null => Err(Error::Interpreter(
                "postgres: Null bind parameters are not supported by this runtime yet".into(),
            )),
            other => Err(Error::Interpreter(format!(
                "postgres: cannot bind value of type '{}' as SQL parameter",
                other.type_name()
            ))),
        })
        .collect()
}

fn postgres_param_refs(params: &[Box<dyn ToSql + Sync>]) -> Vec<&(dyn ToSql + Sync)> {
    params
        .iter()
        .map(|value| &**value as &(dyn ToSql + Sync))
        .collect()
}

fn postgres_value_to_zun(row: &Row, idx: usize) -> Result<ZunValue, Error> {
    let column = &row.columns()[idx];
    let ty = column.type_();

    if *ty == Type::BOOL {
        return Ok(match row.try_get::<usize, Option<bool>>(idx) {
            Ok(Some(value)) => ZunValue::Bool(value),
            Ok(None) => ZunValue::Null,
            Err(err) => {
                return Err(Error::Interpreter(format!(
                    "{}: postgres column '{}' read failed: {err}",
                    ids::SQL_QUERY,
                    column.name()
                )));
            }
        });
    }

    if *ty == Type::INT2 {
        return Ok(match row.try_get::<usize, Option<i16>>(idx) {
            Ok(Some(value)) => ZunValue::Int(i64::from(value)),
            Ok(None) => ZunValue::Null,
            Err(err) => {
                return Err(Error::Interpreter(format!(
                    "{}: postgres column '{}' read failed: {err}",
                    ids::SQL_QUERY,
                    column.name()
                )));
            }
        });
    }

    if *ty == Type::INT4 {
        return Ok(match row.try_get::<usize, Option<i32>>(idx) {
            Ok(Some(value)) => ZunValue::Int(i64::from(value)),
            Ok(None) => ZunValue::Null,
            Err(err) => {
                return Err(Error::Interpreter(format!(
                    "{}: postgres column '{}' read failed: {err}",
                    ids::SQL_QUERY,
                    column.name()
                )));
            }
        });
    }

    if *ty == Type::INT8 {
        return Ok(match row.try_get::<usize, Option<i64>>(idx) {
            Ok(Some(value)) => ZunValue::Int(value),
            Ok(None) => ZunValue::Null,
            Err(err) => {
                return Err(Error::Interpreter(format!(
                    "{}: postgres column '{}' read failed: {err}",
                    ids::SQL_QUERY,
                    column.name()
                )));
            }
        });
    }

    if *ty == Type::FLOAT4 {
        return Ok(match row.try_get::<usize, Option<f32>>(idx) {
            Ok(Some(value)) => ZunValue::Float(f64::from(value)),
            Ok(None) => ZunValue::Null,
            Err(err) => {
                return Err(Error::Interpreter(format!(
                    "{}: postgres column '{}' read failed: {err}",
                    ids::SQL_QUERY,
                    column.name()
                )));
            }
        });
    }

    if *ty == Type::FLOAT8 {
        return Ok(match row.try_get::<usize, Option<f64>>(idx) {
            Ok(Some(value)) => ZunValue::Float(value),
            Ok(None) => ZunValue::Null,
            Err(err) => {
                return Err(Error::Interpreter(format!(
                    "{}: postgres column '{}' read failed: {err}",
                    ids::SQL_QUERY,
                    column.name()
                )));
            }
        });
    }

    if matches!(*ty, Type::TEXT | Type::VARCHAR | Type::BPCHAR | Type::NAME) {
        return Ok(match row.try_get::<usize, Option<String>>(idx) {
            Ok(Some(value)) => ZunValue::Str(value),
            Ok(None) => ZunValue::Null,
            Err(err) => {
                return Err(Error::Interpreter(format!(
                    "{}: postgres column '{}' read failed: {err}",
                    ids::SQL_QUERY,
                    column.name()
                )));
            }
        });
    }

    Err(Error::Interpreter(format!(
        "{}: postgres column '{}' has unsupported type '{}'",
        ids::SQL_QUERY,
        column.name(),
        ty.name()
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{RuntimeContext, RuntimeMode};
    use std::path::PathBuf;

    fn test_context() -> RuntimeContext {
        RuntimeContext::new(RuntimeMode::Test, PathBuf::from("."))
    }

    fn connection_parts(conn: &ZunValue) -> (String, String) {
        match conn {
            ZunValue::Connection { id, driver } => (id.clone(), driver.clone()),
            other => panic!("expected Connection handle, got {}", other.type_name()),
        }
    }

    #[test]
    fn sqlite_connect_query_execute_and_close_round_trip() {
        let context = test_context();
        let conn = context
            .call_intrinsic(
                ids::SQL_SQLITE_CONNECT,
                &[ZunValue::Str(":memory:".to_string())],
            )
            .unwrap();

        context
            .call_intrinsic(
                ids::SQL_EXECUTE,
                &[
                    conn.clone(),
                    ZunValue::Str("CREATE TABLE users (id INTEGER, name TEXT)".to_string()),
                    ZunValue::List(vec![]),
                ],
            )
            .unwrap();

        context
            .call_intrinsic(
                ids::SQL_EXECUTE,
                &[
                    conn.clone(),
                    ZunValue::Str("INSERT INTO users (id, name) VALUES (?, ?)".to_string()),
                    ZunValue::List(vec![ZunValue::Int(1), ZunValue::Str("Alice".to_string())]),
                ],
            )
            .unwrap();

        let rows = context
            .call_intrinsic(
                ids::SQL_QUERY,
                &[
                    conn.clone(),
                    ZunValue::Str("SELECT id, name FROM users WHERE id = ?".to_string()),
                    ZunValue::List(vec![ZunValue::Int(1)]),
                ],
            )
            .unwrap();

        assert_eq!(format!("{rows}"), "[{id: 1, name: Alice}]");
        context.call_intrinsic(ids::SQL_CLOSE, &[conn]).unwrap();
    }

    #[test]
    fn sql_query_rejects_closed_connection_handles() {
        let context = test_context();
        let conn = context
            .call_intrinsic(
                ids::SQL_SQLITE_CONNECT,
                &[ZunValue::Str(":memory:".to_string())],
            )
            .unwrap();
        context
            .call_intrinsic(ids::SQL_CLOSE, &[conn.clone()])
            .unwrap();

        let error = context
            .call_intrinsic(
                ids::SQL_QUERY,
                &[
                    conn,
                    ZunValue::Str("SELECT 1 AS n".to_string()),
                    ZunValue::List(vec![]),
                ],
            )
            .expect_err("closed connection should fail");

        assert!(error.to_string().contains("closed or does not exist"));
    }

    #[test]
    fn sqlite_execute_rejects_unsupported_bind_types() {
        let context = test_context();
        let conn = context
            .call_intrinsic(
                ids::SQL_SQLITE_CONNECT,
                &[ZunValue::Str(":memory:".to_string())],
            )
            .unwrap();

        let error = context
            .call_intrinsic(
                ids::SQL_EXECUTE,
                &[
                    conn,
                    ZunValue::Str("SELECT ?".to_string()),
                    ZunValue::List(vec![ZunValue::List(vec![ZunValue::Int(1)])]),
                ],
            )
            .expect_err("nested list bind should fail");

        assert!(
            error
                .to_string()
                .contains("cannot bind value of type 'List")
        );
    }

    #[test]
    fn sqlite_query_rejects_blob_columns() {
        let context = test_context();
        let conn = context
            .call_intrinsic(
                ids::SQL_SQLITE_CONNECT,
                &[ZunValue::Str(":memory:".to_string())],
            )
            .unwrap();

        context
            .call_intrinsic(
                ids::SQL_EXECUTE,
                &[
                    conn.clone(),
                    ZunValue::Str("CREATE TABLE blobs (payload BLOB)".to_string()),
                    ZunValue::List(vec![]),
                ],
            )
            .unwrap();

        context
            .call_intrinsic(
                ids::SQL_EXECUTE,
                &[
                    conn.clone(),
                    ZunValue::Str("INSERT INTO blobs (payload) VALUES (X'0102')".to_string()),
                    ZunValue::List(vec![]),
                ],
            )
            .unwrap();

        let error = context
            .call_intrinsic(
                ids::SQL_QUERY,
                &[
                    conn,
                    ZunValue::Str("SELECT payload FROM blobs".to_string()),
                    ZunValue::List(vec![]),
                ],
            )
            .expect_err("blob columns should be rejected");

        assert!(error.to_string().contains("unsupported blob data"));
    }

    #[test]
    fn sql_query_rejects_driver_mismatched_handles() {
        let context = test_context();
        let conn = context
            .call_intrinsic(
                ids::SQL_SQLITE_CONNECT,
                &[ZunValue::Str(":memory:".to_string())],
            )
            .unwrap();
        let ZunValue::Connection { id, .. } = conn else {
            panic!("expected Connection handle");
        };

        let error = context
            .call_intrinsic(
                ids::SQL_QUERY,
                &[
                    ZunValue::Connection {
                        id,
                        driver: "postgres".to_string(),
                    },
                    ZunValue::Str("SELECT 1 AS n".to_string()),
                    ZunValue::List(vec![]),
                ],
            )
            .expect_err("driver mismatch should fail");

        assert!(
            error
                .to_string()
                .contains("belongs to driver 'sqlite', not 'postgres'")
        );
    }

    #[test]
    fn sql_close_failed_attempt_keeps_sqlite_handle_registered_until_holder_is_released() {
        let context = test_context();
        let conn = context
            .call_intrinsic(
                ids::SQL_SQLITE_CONNECT,
                &[ZunValue::Str(":memory:".to_string())],
            )
            .unwrap();
        let (id, driver) = connection_parts(&conn);
        let holder = context
            .with_sql_conn_registry(|registry| {
                registry.get_for_handle(id.as_str(), driver.as_str(), ids::SQL_CLOSE)
            })
            .unwrap();

        let error = context
            .call_intrinsic(ids::SQL_CLOSE, &[conn.clone()])
            .expect_err("shared sqlite handle should prevent close");

        assert!(error.to_string().contains("still in use"));
        assert!(
            context
                .with_sql_conn_registry(|registry| Ok(registry.get(id.as_str()).is_some()))
                .unwrap()
        );

        let rows = context
            .call_intrinsic(
                ids::SQL_QUERY,
                &[
                    conn.clone(),
                    ZunValue::Str("SELECT 1 AS n".to_string()),
                    ZunValue::List(vec![]),
                ],
            )
            .unwrap();
        assert_eq!(format!("{rows}"), "[{n: 1}]");

        drop(holder);
        context
            .call_intrinsic(ids::SQL_CLOSE, &[conn.clone()])
            .unwrap();
        assert!(
            context
                .with_sql_conn_registry(|registry| Ok(registry.get(id.as_str()).is_none()))
                .unwrap()
        );

        let closed_error = context
            .call_intrinsic(
                ids::SQL_QUERY,
                &[
                    conn,
                    ZunValue::Str("SELECT 1 AS n".to_string()),
                    ZunValue::List(vec![]),
                ],
            )
            .expect_err("closed connection should fail after successful retry");
        assert!(
            closed_error
                .to_string()
                .contains("closed or does not exist")
        );
    }

    #[test]
    fn postgres_null_bind_is_explicitly_rejected() {
        let error = zun_to_postgres_params(&[ZunValue::Null])
            .expect_err("postgres null bind should be rejected");

        assert!(
            error
                .to_string()
                .contains("Null bind parameters are not supported")
        );
    }

    #[test]
    fn postgres_db_error_formatter_includes_code_and_detail_when_present() {
        let formatted = format_postgres_db_error_message(
            Some("42P01"),
            "relation \"news_articles\" does not exist",
            Some("Perhaps you meant to reference a different table."),
        );

        assert_eq!(
            formatted,
            "42P01: relation \"news_articles\" does not exist: detail: Perhaps you meant to reference a different table."
        );
    }

    #[test]
    fn postgres_db_error_formatter_omits_missing_optional_fields() {
        let formatted = format_postgres_db_error_message(None, "db error", None);
        assert_eq!(formatted, "db error");
    }

    #[test]
    fn postgres_connect_smoke_test_when_url_is_present() {
        let Some(url) = std::env::var("ZUN_TEST_POSTGRES_URL").ok() else {
            return;
        };
        let context = test_context();
        let conn = context
            .call_intrinsic(ids::SQL_POSTGRES_CONNECT, &[ZunValue::Str(url)])
            .unwrap();

        let rows = context
            .call_intrinsic(
                ids::SQL_QUERY,
                &[
                    conn.clone(),
                    ZunValue::Str("SELECT 1 AS n".to_string()),
                    ZunValue::List(vec![]),
                ],
            )
            .unwrap();

        assert_eq!(format!("{rows}"), "[{n: 1}]");
        context.call_intrinsic(ids::SQL_CLOSE, &[conn]).unwrap();
    }

    #[test]
    fn postgres_close_failed_attempt_keeps_handle_registered_until_holder_is_released() {
        let Some(url) = std::env::var("ZUN_TEST_POSTGRES_URL").ok() else {
            return;
        };
        let context = test_context();
        let conn = context
            .call_intrinsic(ids::SQL_POSTGRES_CONNECT, &[ZunValue::Str(url)])
            .unwrap();
        let (id, driver) = connection_parts(&conn);
        let holder = context
            .with_sql_conn_registry(|registry| {
                registry.get_for_handle(id.as_str(), driver.as_str(), ids::SQL_CLOSE)
            })
            .unwrap();

        let error = context
            .call_intrinsic(ids::SQL_CLOSE, &[conn.clone()])
            .expect_err("shared postgres handle should prevent close");
        assert!(error.to_string().contains("still in use"));
        assert!(
            context
                .with_sql_conn_registry(|registry| Ok(registry.get(id.as_str()).is_some()))
                .unwrap()
        );

        drop(holder);
        context.call_intrinsic(ids::SQL_CLOSE, &[conn]).unwrap();
        assert!(
            context
                .with_sql_conn_registry(|registry| Ok(registry.get(id.as_str()).is_none()))
                .unwrap()
        );
    }
}
