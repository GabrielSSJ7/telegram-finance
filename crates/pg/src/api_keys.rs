use app::model::{ApiKey, ApiKeyId};
use app::ports::{ApiKeyStore, StoreResult};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::PgStore;
use crate::error_mapping::store_error;

struct ApiKeyRow {
    id: Uuid,
    name: String,
    created_at: DateTime<Utc>,
    revoked_at: Option<DateTime<Utc>>,
}

impl From<ApiKeyRow> for ApiKey {
    fn from(row: ApiKeyRow) -> Self {
        ApiKey {
            id: ApiKeyId(row.id),
            name: row.name,
            created_at: row.created_at,
            revoked: row.revoked_at.is_some(),
        }
    }
}

#[async_trait]
impl ApiKeyStore for PgStore {
    async fn create_api_key(&self, name: &str, token_sha256: [u8; 32]) -> StoreResult<ApiKey> {
        let row = sqlx::query_as!(
            ApiKeyRow,
            "insert into api_keys (name, token_sha256) values ($1, $2) returning id, name, created_at, revoked_at",
            name,
            &token_sha256[..],
        )
        .fetch_one(self.pool())
        .await
        .map_err(store_error)?;
        Ok(row.into())
    }

    async fn revoke_api_key(&self, name: &str, at: DateTime<Utc>) -> StoreResult<bool> {
        let result = sqlx::query!(
            "update api_keys set revoked_at = $2 where name = $1 and revoked_at is null",
            name,
            at
        )
        .execute(self.pool())
        .await
        .map_err(store_error)?;
        Ok(result.rows_affected() == 1)
    }

    async fn find_active_api_key(&self, token_sha256: [u8; 32]) -> StoreResult<Option<ApiKey>> {
        let row = sqlx::query_as!(
            ApiKeyRow,
            "select id, name, created_at, revoked_at from api_keys where token_sha256 = $1 and revoked_at is null",
            &token_sha256[..],
        )
        .fetch_optional(self.pool())
        .await
        .map_err(store_error)?;
        Ok(row.map(ApiKey::from))
    }

    async fn touch_api_key(&self, id: ApiKeyId, at: DateTime<Utc>) -> StoreResult<()> {
        sqlx::query!("update api_keys set last_used_at = $2 where id = $1", id.0, at)
            .execute(self.pool())
            .await
            .map_err(store_error)?;
        Ok(())
    }
}
