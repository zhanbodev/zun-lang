use rusqlite::{Connection, params};

use crate::error::Error;

use super::{RefreshTokenRecord, RefreshTokenStore};

pub struct RefreshTokenStoreSqlite {
    conn: Connection,
    table: String,
}

impl RefreshTokenStoreSqlite {
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
                    token_id TEXT PRIMARY KEY,
                    subject TEXT NOT NULL,
                    expires_at INTEGER NOT NULL,
                    revoked_at INTEGER
                )",
                self.table
            ))
            .map_err(sqlite_error)?;
        Ok(())
    }
}

impl RefreshTokenStore for RefreshTokenStoreSqlite {
    fn get(&mut self, token_id: &str) -> Result<Option<RefreshTokenRecord>, Error> {
        let mut stmt = self
            .conn
            .prepare(&format!(
                "SELECT token_id, subject, expires_at, revoked_at FROM {} WHERE token_id = ?1",
                self.table
            ))
            .map_err(sqlite_error)?;

        let row = stmt.query_row([token_id], |row| {
            Ok(RefreshTokenRecord {
                token_id: row.get(0)?,
                subject: row.get(1)?,
                expires_at: row.get(2)?,
                revoked_at: row.get(3)?,
            })
        });

        match row {
            Ok(record) => Ok(Some(record)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(err) => Err(sqlite_error(err)),
        }
    }

    fn create(&mut self, record: RefreshTokenRecord) -> Result<(), Error> {
        self.conn
            .execute(
                &format!(
                    "INSERT INTO {} (token_id, subject, expires_at, revoked_at) VALUES (?1, ?2, ?3, ?4)",
                    self.table
                ),
                params![record.token_id, record.subject, record.expires_at, record.revoked_at],
            )
            .map_err(sqlite_error)?;
        Ok(())
    }

    fn revoke(&mut self, token_id: &str) -> Result<(), Error> {
        self.conn
            .execute(
                &format!(
                    "UPDATE {} SET revoked_at = unixepoch() WHERE token_id = ?1 AND revoked_at IS NULL",
                    self.table
                ),
                [token_id],
            )
            .map_err(sqlite_error)?;
        Ok(())
    }

    fn revoke_subject(&mut self, subject: &str) -> Result<usize, Error> {
        let updated = self
            .conn
            .execute(
                &format!(
                    "UPDATE {} SET revoked_at = unixepoch() WHERE subject = ?1 AND revoked_at IS NULL",
                    self.table
                ),
                [subject],
            )
            .map_err(sqlite_error)?;
        Ok(updated)
    }
}

fn sqlite_error(err: rusqlite::Error) -> Error {
    Error::Interpreter(format!("sqlite auth refresh token store error: {err}"))
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::{RefreshTokenRecord, RefreshTokenStore, RefreshTokenStoreSqlite};

    #[test]
    fn sqlite_refresh_store_round_trips_and_revokes() {
        let conn = Connection::open_in_memory().unwrap();
        let mut store = RefreshTokenStoreSqlite::new(conn, "auth_refresh_tokens");
        store.ensure_schema().unwrap();

        let record = RefreshTokenRecord {
            token_id: "rjti_sqlite".to_string(),
            subject: "user_1".to_string(),
            expires_at: 1_800_000_000,
            revoked_at: None,
        };

        store.create(record.clone()).unwrap();
        assert_eq!(store.get("rjti_sqlite").unwrap(), Some(record));

        store.revoke("rjti_sqlite").unwrap();
        assert!(
            store
                .get("rjti_sqlite")
                .unwrap()
                .expect("record should exist")
                .revoked_at
                .is_some()
        );
    }
}
