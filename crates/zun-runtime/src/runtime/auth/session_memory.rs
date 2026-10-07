use std::collections::HashMap;

use chrono::Utc;

use crate::error::Error;
use crate::runtime::gc::LazyGcWindow;

use super::{SessionRecord, SessionStore};

const SESSION_GC_INTERVAL_SECONDS: i64 = 60;

#[derive(Debug)]
pub struct MemorySessionStore {
    sessions: HashMap<String, SessionRecord>,
    gc_window: LazyGcWindow,
}

impl Default for MemorySessionStore {
    fn default() -> Self {
        Self {
            sessions: HashMap::new(),
            gc_window: LazyGcWindow::new(SESSION_GC_INTERVAL_SECONDS),
        }
    }
}

impl SessionStore for MemorySessionStore {
    fn get(&mut self, session_id: &str) -> Result<Option<SessionRecord>, Error> {
        let now = Utc::now().timestamp();
        self.maybe_prune_expired(now);
        Ok(self
            .sessions
            .get(session_id)
            .filter(|session| is_session_active(session, now))
            .cloned())
    }

    fn create(&mut self, session: SessionRecord) -> Result<(), Error> {
        self.maybe_prune_expired(Utc::now().timestamp());
        self.sessions.insert(session.session_id.clone(), session);
        Ok(())
    }

    fn update(&mut self, session: SessionRecord) -> Result<(), Error> {
        self.maybe_prune_expired(Utc::now().timestamp());
        self.sessions.insert(session.session_id.clone(), session);
        Ok(())
    }

    fn rotate(&mut self, old_session_id: &str, session: SessionRecord) -> Result<(), Error> {
        self.maybe_prune_expired(Utc::now().timestamp());
        self.sessions.remove(old_session_id);
        self.sessions.insert(session.session_id.clone(), session);
        Ok(())
    }

    fn delete(&mut self, session_id: &str) -> Result<(), Error> {
        self.sessions.remove(session_id);
        Ok(())
    }
}

impl MemorySessionStore {
    fn maybe_prune_expired(&mut self, now: i64) {
        if !self.gc_window.is_due(now) {
            return;
        }

        self.sessions
            .retain(|_, session| is_session_active(session, now));
        self.gc_window.mark_ran(now);
    }
}

fn is_session_active(session: &SessionRecord, now: i64) -> bool {
    session.expires_at > now
        && session
            .idle_timeout_at
            .map(|idle_timeout_at| idle_timeout_at > now)
            .unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use indexmap::IndexMap;

    use super::{MemorySessionStore, SessionRecord, SessionStore};

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
    fn memory_store_round_trips_session_record() {
        let mut store = MemorySessionStore::default();
        let session = sample_session("sess_1");

        store
            .create(session.clone())
            .expect("create should succeed");
        let loaded = store.get("sess_1").expect("lookup should succeed");

        assert_eq!(loaded, Some(session));
    }

    #[test]
    fn memory_store_rotate_replaces_old_session() {
        let mut store = MemorySessionStore::default();
        let original = sample_session("sess_old");
        let rotated = sample_session("sess_new");

        store.create(original).unwrap();
        store.rotate("sess_old", rotated.clone()).unwrap();

        assert!(store.get("sess_old").unwrap().is_none());
        assert_eq!(store.get("sess_new").unwrap(), Some(rotated));
    }

    #[test]
    fn memory_store_update_and_delete_session() {
        let mut store = MemorySessionStore::default();
        let mut session = sample_session("sess_update");

        store.create(session.clone()).unwrap();
        session.permissions.push("post:publish".to_string());
        store.update(session.clone()).unwrap();
        assert_eq!(store.get("sess_update").unwrap(), Some(session));

        store.delete("sess_update").unwrap();
        assert!(store.get("sess_update").unwrap().is_none());
    }

    #[test]
    fn memory_store_prunes_expired_sessions_during_activity() {
        let mut store = MemorySessionStore::default();
        let expired = SessionRecord {
            session_id: "sess_expired".to_string(),
            subject: "user_old".to_string(),
            roles: vec![],
            permissions: vec![],
            claims: IndexMap::new(),
            expires_at: 1,
            idle_timeout_at: Some(1),
        };
        let live = sample_session("sess_live");

        store.create(expired).unwrap();
        store.create(live.clone()).unwrap();

        assert!(store.get("sess_expired").unwrap().is_none());
        assert_eq!(store.get("sess_live").unwrap(), Some(live));
    }

    #[test]
    fn memory_store_defers_batch_prune_until_gc_window() {
        let mut store = MemorySessionStore::default();
        store.gc_window.set_next_gc_at_for_test(i64::MAX);
        store.sessions.insert(
            "expired".to_string(),
            SessionRecord {
                session_id: "expired".to_string(),
                subject: "user_old".to_string(),
                roles: vec![],
                permissions: vec![],
                claims: IndexMap::new(),
                expires_at: 1,
                idle_timeout_at: Some(1),
            },
        );
        let live = sample_session("live");
        store.sessions.insert("live".to_string(), live.clone());

        assert!(store.get("expired").unwrap().is_none());
        assert_eq!(store.get("live").unwrap(), Some(live));
        assert!(store.sessions.contains_key("expired"));
        assert_eq!(store.sessions.len(), 2);
    }

    #[test]
    fn memory_store_prunes_when_gc_window_opens() {
        let mut store = MemorySessionStore::default();
        store.gc_window.set_next_gc_at_for_test(0);
        store.sessions.insert(
            "expired".to_string(),
            SessionRecord {
                session_id: "expired".to_string(),
                subject: "user_old".to_string(),
                roles: vec![],
                permissions: vec![],
                claims: IndexMap::new(),
                expires_at: 1,
                idle_timeout_at: Some(1),
            },
        );
        let live = sample_session("live");
        store.sessions.insert("live".to_string(), live.clone());

        assert_eq!(store.get("live").unwrap(), Some(live));
        assert!(!store.sessions.contains_key("expired"));
        assert_eq!(store.sessions.len(), 1);
    }
}
