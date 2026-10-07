use crate::error::Error;

use super::{MemorySessionStore, PostgresSessionStore, SessionRecord, SqliteSessionStore};

pub trait SessionStore: Send {
    fn get(&mut self, session_id: &str) -> Result<Option<SessionRecord>, Error>;
    fn create(&mut self, session: SessionRecord) -> Result<(), Error>;
    fn update(&mut self, session: SessionRecord) -> Result<(), Error>;
    fn rotate(&mut self, old_session_id: &str, session: SessionRecord) -> Result<(), Error>;
    fn delete(&mut self, session_id: &str) -> Result<(), Error>;
}

pub enum SessionStoreBackend {
    Memory(MemorySessionStore),
    Sqlite(SqliteSessionStore),
    Postgres(PostgresSessionStore),
}

impl SessionStore for SessionStoreBackend {
    fn get(&mut self, session_id: &str) -> Result<Option<SessionRecord>, Error> {
        match self {
            Self::Memory(store) => store.get(session_id),
            Self::Sqlite(store) => store.get(session_id),
            Self::Postgres(store) => store.get(session_id),
        }
    }

    fn create(&mut self, session: SessionRecord) -> Result<(), Error> {
        match self {
            Self::Memory(store) => store.create(session),
            Self::Sqlite(store) => store.create(session),
            Self::Postgres(store) => store.create(session),
        }
    }

    fn update(&mut self, session: SessionRecord) -> Result<(), Error> {
        match self {
            Self::Memory(store) => store.update(session),
            Self::Sqlite(store) => store.update(session),
            Self::Postgres(store) => store.update(session),
        }
    }

    fn rotate(&mut self, old_session_id: &str, session: SessionRecord) -> Result<(), Error> {
        match self {
            Self::Memory(store) => store.rotate(old_session_id, session),
            Self::Sqlite(store) => store.rotate(old_session_id, session),
            Self::Postgres(store) => store.rotate(old_session_id, session),
        }
    }

    fn delete(&mut self, session_id: &str) -> Result<(), Error> {
        match self {
            Self::Memory(store) => store.delete(session_id),
            Self::Sqlite(store) => store.delete(session_id),
            Self::Postgres(store) => store.delete(session_id),
        }
    }
}
