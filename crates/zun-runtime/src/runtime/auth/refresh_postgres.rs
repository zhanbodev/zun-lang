use std::sync::{Arc, Mutex};

use crate::error::Error;
use crate::runtime::sql_registry::PostgresClientHandle;

use super::{RefreshTokenRecord, RefreshTokenStore, postgres_worker};

pub struct RefreshTokenStorePostgres {
    client: Arc<Mutex<PostgresClientHandle>>,
    table: String,
}

impl RefreshTokenStorePostgres {
    pub fn connect(url: &str, table: impl Into<String>) -> Result<Self, Error> {
        let client =
            postgres_worker::connect_client(url, "postgres auth refresh token store error")?;
        Ok(Self {
            client,
            table: table.into(),
        })
    }

    pub fn ensure_schema(&mut self) -> Result<(), Error> {
        let sql = format!(
            "CREATE TABLE IF NOT EXISTS {} (
                    token_id TEXT PRIMARY KEY,
                    subject TEXT NOT NULL,
                    expires_at BIGINT NOT NULL,
                    revoked_at BIGINT
                )",
            self.table
        );
        postgres_worker::with_client(
            &self.client,
            "postgres auth refresh token store ensure_schema",
            move |client| client.batch_execute(sql.as_str()).map_err(postgres_error),
        )
    }
}

impl RefreshTokenStore for RefreshTokenStorePostgres {
    fn get(&mut self, token_id: &str) -> Result<Option<RefreshTokenRecord>, Error> {
        let sql = format!(
            "SELECT token_id, subject, expires_at, revoked_at FROM {} WHERE token_id = $1",
            self.table
        );
        let token_id = token_id.to_string();
        let row = postgres_worker::with_client(
            &self.client,
            "postgres auth refresh token store get",
            move |client| {
                client
                    .query_opt(sql.as_str(), &[&token_id])
                    .map_err(postgres_error)
            },
        )?;

        Ok(row.map(|row| RefreshTokenRecord {
            token_id: row.get(0),
            subject: row.get(1),
            expires_at: row.get(2),
            revoked_at: row.get(3),
        }))
    }

    fn create(&mut self, record: RefreshTokenRecord) -> Result<(), Error> {
        let sql = format!(
            "INSERT INTO {} (token_id, subject, expires_at, revoked_at) VALUES ($1, $2, $3, $4)",
            self.table
        );
        postgres_worker::with_client(
            &self.client,
            "postgres auth refresh token store create",
            move |client| {
                client
                    .execute(
                        sql.as_str(),
                        &[
                            &record.token_id,
                            &record.subject,
                            &record.expires_at,
                            &record.revoked_at,
                        ],
                    )
                    .map_err(postgres_error)?;
                Ok(())
            },
        )
    }

    fn revoke(&mut self, token_id: &str) -> Result<(), Error> {
        let sql = format!(
            "UPDATE {} SET revoked_at = EXTRACT(EPOCH FROM NOW())::BIGINT WHERE token_id = $1 AND revoked_at IS NULL",
            self.table
        );
        let token_id = token_id.to_string();
        postgres_worker::with_client(
            &self.client,
            "postgres auth refresh token store revoke",
            move |client| {
                client
                    .execute(sql.as_str(), &[&token_id])
                    .map_err(postgres_error)?;
                Ok(())
            },
        )
    }

    fn revoke_subject(&mut self, subject: &str) -> Result<usize, Error> {
        let sql = format!(
            "UPDATE {} SET revoked_at = EXTRACT(EPOCH FROM NOW())::BIGINT WHERE subject = $1 AND revoked_at IS NULL",
            self.table
        );
        let subject = subject.to_string();
        let updated = postgres_worker::with_client(
            &self.client,
            "postgres auth refresh token store revoke_subject",
            move |client| {
                client
                    .execute(sql.as_str(), &[&subject])
                    .map_err(postgres_error)
            },
        )?;
        Ok(updated as usize)
    }
}

fn postgres_error(err: postgres::Error) -> Error {
    Error::Interpreter(format!("postgres auth refresh token store error: {err}"))
}

#[cfg(test)]
mod tests {
    use super::{RefreshTokenRecord, RefreshTokenStore, RefreshTokenStorePostgres};

    #[test]
    fn postgres_refresh_store_round_trips_and_revokes_when_url_present() {
        let Some(url) = std::env::var("ZUN_TEST_POSTGRES_URL").ok() else {
            return;
        };

        let table = format!("auth_refresh_tokens_{}", std::process::id());
        let mut store = RefreshTokenStorePostgres::connect(&url, &table).unwrap();
        store.ensure_schema().unwrap();

        let record = RefreshTokenRecord {
            token_id: "rjti_pg".to_string(),
            subject: "user_1".to_string(),
            expires_at: 1_800_000_000,
            revoked_at: None,
        };

        store.create(record.clone()).unwrap();
        assert_eq!(store.get("rjti_pg").unwrap(), Some(record));

        store.revoke("rjti_pg").unwrap();
        assert!(
            store
                .get("rjti_pg")
                .unwrap()
                .expect("record should exist")
                .revoked_at
                .is_some()
        );
        store.revoke_subject("user_1").unwrap();
    }

    #[test]
    fn postgres_refresh_store_operations_do_not_panic_inside_tokio_runtime_when_url_present() {
        let Some(url) = std::env::var("ZUN_TEST_POSTGRES_URL").ok() else {
            return;
        };

        let table = format!("auth_refresh_tokens_tokio_{}", std::process::id());
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime should build");

        rt.block_on(async {
            let mut store = RefreshTokenStorePostgres::connect(&url, &table).unwrap();
            store.ensure_schema().unwrap();

            let record = RefreshTokenRecord {
                token_id: "rjti_tokio".to_string(),
                subject: "user_1".to_string(),
                expires_at: 1_800_000_000,
                revoked_at: None,
            };

            store.create(record.clone()).unwrap();
            assert_eq!(store.get("rjti_tokio").unwrap(), Some(record));
            store.revoke("rjti_tokio").unwrap();
        });
    }
}
