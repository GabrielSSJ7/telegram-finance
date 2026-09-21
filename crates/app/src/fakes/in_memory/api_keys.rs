use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::{ApiKeyRow, InMemoryStore, unique_violation};
use crate::model::{ApiKey, ApiKeyId};
use crate::ports::{ApiKeyStore, StoreResult};

#[async_trait]
impl ApiKeyStore for InMemoryStore {
    async fn create_api_key(&self, name: &str, token_sha256: [u8; 32]) -> StoreResult<ApiKey> {
        let mut state = self.lock();
        if state.api_keys.iter().any(|row| row.key.name == name) {
            return Err(unique_violation("api_keys_name_key"));
        }
        let key = ApiKey {
            id: ApiKeyId::generate(),
            name: name.to_owned(),
            created_at: Utc::now(),
            revoked: false,
        };
        state.api_keys.push(ApiKeyRow { key: key.clone(), token_sha256, last_used_at: None });
        Ok(key)
    }

    async fn revoke_api_key(&self, name: &str, _at: DateTime<Utc>) -> StoreResult<bool> {
        let mut state = self.lock();
        let Some(row) =
            state.api_keys.iter_mut().find(|row| row.key.name == name && !row.key.revoked)
        else {
            return Ok(false);
        };
        row.key.revoked = true;
        Ok(true)
    }

    async fn find_active_api_key(&self, token_sha256: [u8; 32]) -> StoreResult<Option<ApiKey>> {
        let state = self.lock();
        let found =
            state.api_keys.iter().find(|row| row.token_sha256 == token_sha256 && !row.key.revoked);
        Ok(found.map(|row| row.key.clone()))
    }

    async fn touch_api_key(&self, id: ApiKeyId, at: DateTime<Utc>) -> StoreResult<()> {
        let mut state = self.lock();
        if let Some(row) = state.api_keys.iter_mut().find(|row| row.key.id == id) {
            row.last_used_at = Some(at);
        }
        Ok(())
    }
}
