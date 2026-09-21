use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::ApiKeyId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: ApiKeyId,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub revoked: bool,
}

/// Returned once at creation; only the SHA-256 of `token` is stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssuedApiKey {
    pub key: ApiKey,
    pub token: String,
}
