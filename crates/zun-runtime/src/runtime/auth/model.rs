use indexmap::IndexMap;

use crate::interpreter::ZunValue;

#[derive(Debug, Clone, PartialEq)]
pub struct Principal {
    pub subject: String,
    pub scheme: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub claims: IndexMap<String, ZunValue>,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SessionRecord {
    pub session_id: String,
    pub subject: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub claims: IndexMap<String, ZunValue>,
    pub expires_at: i64,
    pub idle_timeout_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefreshTokenRecord {
    pub token_id: String,
    pub subject: String,
    pub expires_at: i64,
    pub revoked_at: Option<i64>,
}
