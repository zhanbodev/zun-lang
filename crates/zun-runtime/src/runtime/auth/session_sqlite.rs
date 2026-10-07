use indexmap::IndexMap;
use rusqlite::{Connection, params};

use crate::error::Error;
use crate::interpreter::{ZunValue, value::value_to_json};

use super::{SessionRecord, SessionStore};

pub struct SqliteSessionStore {
    conn: Connection,
    table: String,
}

impl SqliteSessionStore {
    pub fn new(conn: Connection, table: impl Into<String>) -> Self {
        Self {
            conn,
            table: table.into(),
        }
    }

    pub fn ensure_schema(&self) -> Result<(), Error> {
        self.conn
            .execute_batch(&format!(
                "CREATE TABLE IF NOT EXISTS {} (
                    session_id TEXT PRIMARY KEY,
                    subject TEXT NOT NULL,
                    roles_json TEXT NOT NULL,
                    permissions_json TEXT NOT NULL,
                    claims_json TEXT NOT NULL,
                    expires_at INTEGER NOT NULL,
                    idle_timeout_at INTEGER
                )",
                self.table
            ))
            .map_err(sqlite_error)?;
        Ok(())
    }
}

impl SessionStore for SqliteSessionStore {
    fn get(&mut self, session_id: &str) -> Result<Option<SessionRecord>, Error> {
        let mut stmt = self
            .conn
            .prepare(&format!(
                "SELECT session_id, subject, roles_json, permissions_json, claims_json, expires_at, idle_timeout_at FROM {} WHERE session_id = ?1",
                self.table
            ))
            .map_err(sqlite_error)?;

        let row = stmt.query_row([session_id], |row| {
            Ok(SessionRecord {
                session_id: row.get(0)?,
                subject: row.get(1)?,
                roles: decode_string_list(&row.get::<_, String>(2)?),
                permissions: decode_string_list(&row.get::<_, String>(3)?),
                claims: decode_claims(&row.get::<_, String>(4)?),
                expires_at: row.get(5)?,
                idle_timeout_at: row.get(6)?,
            })
        });

        match row {
            Ok(session) => Ok(Some(session)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(err) => Err(sqlite_error(err)),
        }
    }

    fn create(&mut self, session: SessionRecord) -> Result<(), Error> {
        self.conn
            .execute(
                &format!(
                    "INSERT INTO {} (session_id, subject, roles_json, permissions_json, claims_json, expires_at, idle_timeout_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    self.table
                ),
                params![
                    session.session_id,
                    session.subject,
                    encode_string_list(&session.roles),
                    encode_string_list(&session.permissions),
                    encode_claims(&session.claims),
                    session.expires_at,
                    session.idle_timeout_at,
                ],
            )
            .map_err(sqlite_error)?;
        Ok(())
    }

    fn update(&mut self, session: SessionRecord) -> Result<(), Error> {
        self.conn
            .execute(
                &format!(
                    "UPDATE {} SET subject = ?2, roles_json = ?3, permissions_json = ?4, claims_json = ?5, expires_at = ?6, idle_timeout_at = ?7 WHERE session_id = ?1",
                    self.table
                ),
                params![
                    session.session_id,
                    session.subject,
                    encode_string_list(&session.roles),
                    encode_string_list(&session.permissions),
                    encode_claims(&session.claims),
                    session.expires_at,
                    session.idle_timeout_at,
                ],
            )
            .map_err(sqlite_error)?;
        Ok(())
    }

    fn rotate(&mut self, old_session_id: &str, session: SessionRecord) -> Result<(), Error> {
        self.delete(old_session_id)?;
        self.create(session)
    }

    fn delete(&mut self, session_id: &str) -> Result<(), Error> {
        self.conn
            .execute(
                &format!("DELETE FROM {} WHERE session_id = ?1", self.table),
                [session_id],
            )
            .map_err(sqlite_error)?;
        Ok(())
    }
}

fn sqlite_error(err: rusqlite::Error) -> Error {
    Error::Interpreter(format!("sqlite auth session store error: {err}"))
}

fn encode_string_list(values: &[String]) -> String {
    serde_json::to_string(values).expect("string list should serialize")
}

fn decode_string_list(raw: &str) -> Vec<String> {
    serde_json::from_str(raw).unwrap_or_default()
}

fn encode_claims(claims: &IndexMap<String, ZunValue>) -> String {
    let value = value_to_json(&ZunValue::Map(claims.clone()));
    serde_json::to_string(&value).expect("claims should serialize")
}

fn decode_claims(raw: &str) -> IndexMap<String, ZunValue> {
    match serde_json::from_str::<serde_json::Value>(raw) {
        Ok(serde_json::Value::Object(map)) => map
            .into_iter()
            .map(|(key, value)| (key, json_value_to_zun(value)))
            .collect(),
        _ => IndexMap::new(),
    }
}

fn json_value_to_zun(value: serde_json::Value) -> ZunValue {
    match value {
        serde_json::Value::Null => ZunValue::Null,
        serde_json::Value::Bool(b) => ZunValue::Bool(b),
        serde_json::Value::Number(n) => {
            if let Some(int) = n.as_i64() {
                ZunValue::Int(int)
            } else if let Some(float) = n.as_f64() {
                ZunValue::Float(float)
            } else {
                ZunValue::Null
            }
        }
        serde_json::Value::String(s) => ZunValue::Str(s),
        serde_json::Value::Array(values) => {
            ZunValue::List(values.into_iter().map(json_value_to_zun).collect())
        }
        serde_json::Value::Object(entries) => ZunValue::Map(
            entries
                .into_iter()
                .map(|(key, value)| (key, json_value_to_zun(value)))
                .collect(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use indexmap::IndexMap;
    use rusqlite::Connection;

    use super::{SessionRecord, SessionStore, SqliteSessionStore};

    fn sample_session(session_id: &str) -> SessionRecord {
        SessionRecord {
            session_id: session_id.to_string(),
            subject: "user_1".to_string(),
            roles: vec!["admin".to_string()],
            permissions: vec!["post:create".to_string()],
            claims: IndexMap::new(),
            expires_at: 1_800_000_000,
            idle_timeout_at: Some(1_800_000_600),
        }
    }

    #[test]
    fn sqlite_store_round_trips_session_record() {
        let conn = Connection::open_in_memory().unwrap();
        let mut store = SqliteSessionStore::new(conn, "auth_sessions");
        store.ensure_schema().unwrap();

        let session = sample_session("sess_sqlite");
        store.create(session.clone()).unwrap();

        assert_eq!(store.get("sess_sqlite").unwrap(), Some(session));
    }
}
