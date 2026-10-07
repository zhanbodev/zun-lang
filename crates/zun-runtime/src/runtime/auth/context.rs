use indexmap::IndexMap;

use crate::interpreter::ZunValue;

use super::{Principal, SessionRecord};

#[derive(Debug, Clone)]
pub(crate) enum PendingSessionWrite {
    Create(SessionRecord),
    Update(SessionRecord),
    Rotate {
        old_session_id: String,
        session: SessionRecord,
    },
    Delete {
        session_id: String,
    },
}

#[derive(Debug, Clone, Default)]
pub struct RequestAuthContext {
    principal: Option<Principal>,
    current_session: Option<SessionRecord>,
    current_jwt_claims: Option<IndexMap<String, ZunValue>>,
    current_bearer_token: Option<String>,
    pending_cookie: Option<String>,
    pending_clear_cookie: bool,
    pending_session_writes: Vec<PendingSessionWrite>,
}

impl RequestAuthContext {
    pub fn principal(&self) -> Option<&Principal> {
        self.principal.as_ref()
    }

    pub fn set_principal(&mut self, principal: Option<Principal>) {
        self.principal = principal;
    }

    pub fn current_session(&self) -> Option<&SessionRecord> {
        self.current_session.as_ref()
    }

    pub fn set_current_session(&mut self, session: Option<SessionRecord>) {
        self.current_session = session;
    }

    pub fn current_jwt_claims(&self) -> Option<&IndexMap<String, ZunValue>> {
        self.current_jwt_claims.as_ref()
    }

    pub fn set_current_jwt_claims(&mut self, claims: Option<IndexMap<String, ZunValue>>) {
        self.current_jwt_claims = claims;
    }

    pub fn current_bearer_token(&self) -> Option<&str> {
        self.current_bearer_token.as_deref()
    }

    pub fn set_current_bearer_token(&mut self, token: Option<String>) {
        self.current_bearer_token = token;
    }

    pub fn pending_cookie(&self) -> Option<&str> {
        self.pending_cookie.as_deref()
    }

    pub fn set_pending_cookie(&mut self, cookie: Option<String>) {
        self.pending_cookie = cookie;
    }

    pub fn pending_clear_cookie(&self) -> bool {
        self.pending_clear_cookie
    }

    pub fn set_pending_clear_cookie(&mut self, pending_clear_cookie: bool) {
        self.pending_clear_cookie = pending_clear_cookie;
    }

    pub fn queue_session_create(&mut self, session: SessionRecord) {
        self.pending_session_writes
            .push(PendingSessionWrite::Create(session));
    }

    pub fn queue_session_update(&mut self, session: SessionRecord) {
        self.pending_session_writes
            .push(PendingSessionWrite::Update(session));
    }

    pub fn queue_session_rotate(&mut self, old_session_id: String, session: SessionRecord) {
        self.pending_session_writes
            .push(PendingSessionWrite::Rotate {
                old_session_id,
                session,
            });
    }

    pub fn queue_session_delete(&mut self, session_id: String) {
        self.pending_session_writes
            .push(PendingSessionWrite::Delete { session_id });
    }

    pub(crate) fn take_pending_session_writes(&mut self) -> Vec<PendingSessionWrite> {
        std::mem::take(&mut self.pending_session_writes)
    }
}
