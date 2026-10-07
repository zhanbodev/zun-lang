use std::collections::HashMap;

use chrono::Utc;

use crate::error::Error;
use crate::runtime::gc::LazyGcWindow;

use super::{RefreshTokenRecord, RefreshTokenStore};

const REFRESH_GC_INTERVAL_SECONDS: i64 = 60;
const REVOKED_RETENTION_SECONDS: i64 = 300;

pub struct RefreshTokenStoreMemory {
    tokens: HashMap<String, RefreshTokenRecord>,
    gc_window: LazyGcWindow,
    revoked_retention_seconds: i64,
}

impl Default for RefreshTokenStoreMemory {
    fn default() -> Self {
        Self {
            tokens: HashMap::new(),
            gc_window: LazyGcWindow::new(REFRESH_GC_INTERVAL_SECONDS),
            revoked_retention_seconds: REVOKED_RETENTION_SECONDS,
        }
    }
}

impl RefreshTokenStore for RefreshTokenStoreMemory {
    fn get(&mut self, token_id: &str) -> Result<Option<RefreshTokenRecord>, Error> {
        let now = Utc::now().timestamp();
        self.maybe_prune_stale(now);
        Ok(self
            .tokens
            .get(token_id)
            .filter(|record| should_retain_record(record, now, self.revoked_retention_seconds))
            .cloned())
    }

    fn create(&mut self, record: RefreshTokenRecord) -> Result<(), Error> {
        self.maybe_prune_stale(Utc::now().timestamp());
        self.tokens.insert(record.token_id.clone(), record);
        Ok(())
    }

    fn revoke(&mut self, token_id: &str) -> Result<(), Error> {
        let now = Utc::now().timestamp();
        self.maybe_prune_stale(now);
        if let Some(record) = self.tokens.get_mut(token_id) {
            if record.revoked_at.is_none() {
                record.revoked_at = Some(now);
            }
        }
        Ok(())
    }

    fn revoke_subject(&mut self, subject: &str) -> Result<usize, Error> {
        let now = Utc::now().timestamp();
        self.maybe_prune_stale(now);
        let mut count = 0;
        for record in self.tokens.values_mut() {
            if record.subject == subject && record.revoked_at.is_none() {
                record.revoked_at = Some(now);
                count += 1;
            }
        }
        Ok(count)
    }
}

impl RefreshTokenStoreMemory {
    fn maybe_prune_stale(&mut self, now: i64) {
        if !self.gc_window.is_due(now) {
            return;
        }

        let revoked_retention_seconds = self.revoked_retention_seconds;
        self.tokens
            .retain(|_, record| should_retain_record(record, now, revoked_retention_seconds));
        self.gc_window.mark_ran(now);
    }
}

fn should_retain_record(
    record: &RefreshTokenRecord,
    now: i64,
    revoked_retention_seconds: i64,
) -> bool {
    record.expires_at > now
        && record
            .revoked_at
            .map(|revoked_at| revoked_at >= now - revoked_retention_seconds)
            .unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::{RefreshTokenRecord, RefreshTokenStore, RefreshTokenStoreMemory};

    #[test]
    fn memory_refresh_store_round_trips_and_revokes() {
        let mut store = RefreshTokenStoreMemory::default();
        let record = RefreshTokenRecord {
            token_id: "rjti_1".to_string(),
            subject: "user_1".to_string(),
            expires_at: 1_800_000_000,
            revoked_at: None,
        };

        store.create(record.clone()).unwrap();
        assert_eq!(store.get("rjti_1").unwrap(), Some(record.clone()));

        store.revoke("rjti_1").unwrap();
        assert!(
            store
                .get("rjti_1")
                .unwrap()
                .expect("record should exist")
                .revoked_at
                .is_some()
        );
    }

    #[test]
    fn memory_refresh_store_prunes_expired_and_stale_revoked_tokens() {
        let mut store = RefreshTokenStoreMemory::default();
        store
            .create(RefreshTokenRecord {
                token_id: "expired".to_string(),
                subject: "user_1".to_string(),
                expires_at: 1,
                revoked_at: None,
            })
            .unwrap();
        store
            .create(RefreshTokenRecord {
                token_id: "revoked_old".to_string(),
                subject: "user_1".to_string(),
                expires_at: i64::MAX,
                revoked_at: Some(1),
            })
            .unwrap();
        store
            .create(RefreshTokenRecord {
                token_id: "live".to_string(),
                subject: "user_2".to_string(),
                expires_at: i64::MAX,
                revoked_at: None,
            })
            .unwrap();

        assert!(store.get("expired").unwrap().is_none());
        assert!(store.get("revoked_old").unwrap().is_none());
        assert_eq!(
            store
                .get("live")
                .unwrap()
                .expect("live token should remain")
                .subject,
            "user_2"
        );
    }

    #[test]
    fn memory_refresh_store_defers_batch_prune_until_gc_window() {
        let mut store = RefreshTokenStoreMemory::default();
        store.gc_window.set_next_gc_at_for_test(i64::MAX);
        store.tokens.insert(
            "expired".to_string(),
            RefreshTokenRecord {
                token_id: "expired".to_string(),
                subject: "user_1".to_string(),
                expires_at: 1,
                revoked_at: None,
            },
        );
        store.tokens.insert(
            "revoked_old".to_string(),
            RefreshTokenRecord {
                token_id: "revoked_old".to_string(),
                subject: "user_1".to_string(),
                expires_at: i64::MAX,
                revoked_at: Some(1),
            },
        );
        store.tokens.insert(
            "live".to_string(),
            RefreshTokenRecord {
                token_id: "live".to_string(),
                subject: "user_2".to_string(),
                expires_at: i64::MAX,
                revoked_at: None,
            },
        );

        assert!(store.get("expired").unwrap().is_none());
        assert!(store.get("revoked_old").unwrap().is_none());
        assert!(store.tokens.contains_key("expired"));
        assert!(store.tokens.contains_key("revoked_old"));
        assert_eq!(
            store
                .get("live")
                .unwrap()
                .expect("live token should remain")
                .subject,
            "user_2"
        );
        assert_eq!(store.tokens.len(), 3);
    }

    #[test]
    fn memory_refresh_store_prunes_when_gc_window_opens() {
        let mut store = RefreshTokenStoreMemory::default();
        store.gc_window.set_next_gc_at_for_test(0);
        store.tokens.insert(
            "expired".to_string(),
            RefreshTokenRecord {
                token_id: "expired".to_string(),
                subject: "user_1".to_string(),
                expires_at: 1,
                revoked_at: None,
            },
        );
        store.tokens.insert(
            "revoked_old".to_string(),
            RefreshTokenRecord {
                token_id: "revoked_old".to_string(),
                subject: "user_1".to_string(),
                expires_at: i64::MAX,
                revoked_at: Some(1),
            },
        );
        store.tokens.insert(
            "live".to_string(),
            RefreshTokenRecord {
                token_id: "live".to_string(),
                subject: "user_2".to_string(),
                expires_at: i64::MAX,
                revoked_at: None,
            },
        );

        assert_eq!(
            store
                .get("live")
                .unwrap()
                .expect("live token should remain")
                .subject,
            "user_2"
        );
        assert!(!store.tokens.contains_key("expired"));
        assert!(!store.tokens.contains_key("revoked_old"));
        assert_eq!(store.tokens.len(), 1);
    }
}
