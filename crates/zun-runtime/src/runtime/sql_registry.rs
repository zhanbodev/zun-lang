use crate::error::Error;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;

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
        registry.insert(
            id.clone(),
            SqlConn::Sqlite(Arc::new(Mutex::new(
                rusqlite::Connection::open_in_memory().unwrap(),
            ))),
        );
        assert!(matches!(registry.get(&id), Some(SqlConn::Sqlite(_))));
        registry.remove(&id);
        assert!(registry.get(&id).is_none());
    }
}

#[allow(dead_code)]
#[derive(Clone)]
pub(crate) enum SqlConn {
    Sqlite(Arc<Mutex<rusqlite::Connection>>),
    Postgres(Arc<Mutex<PostgresClientHandle>>),
}

impl std::fmt::Debug for SqlConn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sqlite(_) => f.write_str("SqlConn::Sqlite(..)"),
            Self::Postgres(_) => f.write_str("SqlConn::Postgres(..)"),
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct SqlConnRegistry {
    connections: HashMap<String, SqlConn>,
    counter: usize,
}

impl SqlConnRegistry {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn next_id(&mut self, driver: &str) -> String {
        let id = format!("conn:{driver}:{}", self.counter);
        self.counter += 1;
        id
    }

    pub(crate) fn insert(&mut self, id: String, conn: SqlConn) {
        self.connections.insert(id, conn);
    }

    #[cfg(test)]
    pub(crate) fn get(&self, id: &str) -> Option<&SqlConn> {
        self.connections.get(id)
    }

    pub(crate) fn get_for_handle(
        &self,
        id: &str,
        driver: &str,
        intrinsic_id: &str,
    ) -> Result<SqlConn, Error> {
        let conn = self.connections.get(id).ok_or_else(|| {
            Error::Interpreter(format!(
                "{intrinsic_id}: connection '{id}' for driver '{driver}' is closed or does not exist"
            ))
        })?;

        if conn.driver() != driver {
            return Err(Error::Interpreter(format!(
                "{intrinsic_id}: connection '{id}' belongs to driver '{}', not '{}'",
                conn.driver(),
                driver
            )));
        }

        Ok(conn.clone())
    }

    pub(crate) fn remove_for_handle(
        &mut self,
        id: &str,
        driver: &str,
        intrinsic_id: &str,
    ) -> Result<SqlConn, Error> {
        let conn = self.connections.remove(id).ok_or_else(|| {
            Error::Interpreter(format!(
                "{intrinsic_id}: connection '{id}' for driver '{driver}' is closed or does not exist"
            ))
        })?;

        if conn.driver() != driver {
            self.connections.insert(id.to_string(), conn);
            return Err(Error::Interpreter(format!(
                "{intrinsic_id}: connection '{id}' belongs to driver '{}', not '{}'",
                self.connections
                    .get(id)
                    .map(SqlConn::driver)
                    .unwrap_or("unknown"),
                driver
            )));
        }

        Ok(conn)
    }

    #[cfg(test)]
    pub(crate) fn remove(&mut self, id: &str) -> Option<SqlConn> {
        self.connections.remove(id)
    }
}

impl SqlConn {
    pub(crate) fn driver(&self) -> &'static str {
        match self {
            Self::Sqlite(_) => "sqlite",
            Self::Postgres(_) => "postgres",
        }
    }
}

pub(crate) struct PostgresClientHandle {
    client: Option<postgres::Client>,
}

impl PostgresClientHandle {
    pub(crate) fn new(client: postgres::Client) -> Self {
        Self {
            client: Some(client),
        }
    }

    pub(crate) fn client_mut(&mut self) -> Result<&mut postgres::Client, Error> {
        self.client.as_mut().ok_or_else(|| {
            Error::Interpreter("postgres client handle is unexpectedly unavailable".to_string())
        })
    }
}

impl Drop for PostgresClientHandle {
    fn drop(&mut self) {
        let Some(client) = self.client.take() else {
            return;
        };

        // The sync `postgres` client owns an internal Tokio runtime and may call
        // `Runtime::block_on` during drop. HTTP handlers already run inside the
        // server runtime, so dropping the client on that thread panics with
        // "Cannot start a runtime from within a runtime". Move drop onto a
        // plain worker thread instead.
        let _ = thread::spawn(move || drop(client)).join();
    }
}
