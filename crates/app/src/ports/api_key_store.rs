use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::StoreResult;
use crate::model::{ApiKey, ApiKeyId};

#[async_trait]
pub trait ApiKeyStore: Send + Sync {
    async fn create_api_key(&self, name: &str, token_sha256: [u8; 32]) -> StoreResult<ApiKey>;
    /// Returns false when no active key has this name.
    async fn revoke_api_key(&self, name: &str, at: DateTime<Utc>) -> StoreResult<bool>;
    async fn find_active_api_key(&self, token_sha256: [u8; 32]) -> StoreResult<Option<ApiKey>>;
    async fn touch_api_key(&self, id: ApiKeyId, at: DateTime<Utc>) -> StoreResult<()>;
}
