use crate::error::Error;

use super::{
    RefreshTokenRecord, RefreshTokenStoreMemory, RefreshTokenStorePostgres, RefreshTokenStoreSqlite,
};

pub trait RefreshTokenStore: Send {
    fn get(&mut self, token_id: &str) -> Result<Option<RefreshTokenRecord>, Error>;
    fn create(&mut self, record: RefreshTokenRecord) -> Result<(), Error>;
    fn revoke(&mut self, token_id: &str) -> Result<(), Error>;
    fn revoke_subject(&mut self, subject: &str) -> Result<usize, Error>;
}

pub enum RefreshTokenStoreBackend {
    Memory(RefreshTokenStoreMemory),
    Sqlite(RefreshTokenStoreSqlite),
    Postgres(RefreshTokenStorePostgres),
}

impl RefreshTokenStore for RefreshTokenStoreBackend {
    fn get(&mut self, token_id: &str) -> Result<Option<RefreshTokenRecord>, Error> {
        match self {
            Self::Memory(store) => store.get(token_id),
            Self::Sqlite(store) => store.get(token_id),
            Self::Postgres(store) => store.get(token_id),
        }
    }

    fn create(&mut self, record: RefreshTokenRecord) -> Result<(), Error> {
        match self {
            Self::Memory(store) => store.create(record),
            Self::Sqlite(store) => store.create(record),
            Self::Postgres(store) => store.create(record),
        }
    }

    fn revoke(&mut self, token_id: &str) -> Result<(), Error> {
        match self {
            Self::Memory(store) => store.revoke(token_id),
            Self::Sqlite(store) => store.revoke(token_id),
            Self::Postgres(store) => store.revoke(token_id),
        }
    }

    fn revoke_subject(&mut self, subject: &str) -> Result<usize, Error> {
        match self {
            Self::Memory(store) => store.revoke_subject(subject),
            Self::Sqlite(store) => store.revoke_subject(subject),
            Self::Postgres(store) => store.revoke_subject(subject),
        }
    }
}
