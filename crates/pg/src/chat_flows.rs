use app::model::DraftId;
use app::ports::{BotStateStore, ChatFlowStore, ChatUserKey, StoreResult, StoredFlow};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::PgStore;
use crate::error_mapping::store_error;

const UPDATE_OFFSET_KEY: &str = "telegram_update_offset";

#[async_trait]
impl ChatFlowStore for PgStore {
    async fn load_flow(
        &self,
        key: ChatUserKey,
        now: DateTime<Utc>,
    ) -> StoreResult<Option<StoredFlow>> {
        let row = sqlx::query!(
            "select flow, prompt_message_id, draft_id, expires_at from chat_flows
             where chat_id = $1 and user_id = $2 and expires_at > $3",
            key.chat_id,
            key.user_id,
            now,
        )
        .fetch_optional(self.pool())
        .await
        .map_err(store_error)?;
        Ok(row.map(|row| StoredFlow {
            flow: row.flow,
            prompt_message_id: row.prompt_message_id,
            draft_id: DraftId(row.draft_id),
            expires_at: row.expires_at,
        }))
    }

    async fn save_flow(&self, key: ChatUserKey, flow: &StoredFlow) -> StoreResult<()> {
        sqlx::query_file!(
            "queries/save_flow.sql",
            key.chat_id,
            key.user_id,
            flow.flow,
            flow.prompt_message_id,
            flow.draft_id.0,
            flow.expires_at,
        )
        .execute(self.pool())
        .await
        .map_err(store_error)?;
        Ok(())
    }

    async fn clear_flow(&self, key: ChatUserKey) -> StoreResult<()> {
        sqlx::query!(
            "delete from chat_flows where chat_id = $1 and user_id = $2",
            key.chat_id,
            key.user_id
        )
        .execute(self.pool())
        .await
        .map_err(store_error)?;
        Ok(())
    }
}

#[async_trait]
impl BotStateStore for PgStore {
    async fn load_update_offset(&self) -> StoreResult<Option<i64>> {
        let value =
            sqlx::query_scalar!("select value from bot_state where key = $1", UPDATE_OFFSET_KEY)
                .fetch_optional(self.pool())
                .await
                .map_err(store_error)?;
        Ok(value)
    }

    async fn save_update_offset(&self, offset: i64) -> StoreResult<()> {
        sqlx::query!(
            "insert into bot_state (key, value) values ($1, $2) on conflict (key) do update set value = excluded.value",
            UPDATE_OFFSET_KEY,
            offset,
        )
        .execute(self.pool())
        .await
        .map_err(store_error)?;
        Ok(())
    }
}
