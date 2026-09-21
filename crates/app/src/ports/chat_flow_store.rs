use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::StoreResult;
use crate::model::DraftId;

/// One person in one chat. Flows are keyed by both, so the two spouses can
/// fill in entries at the same time in the same group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChatUserKey {
    pub chat_id: i64,
    pub user_id: i64,
}

/// A guided flow in progress. `flow` is owned by the Telegram adapter; the
/// store keeps it as opaque JSON.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoredFlow {
    pub flow: serde_json::Value,
    pub prompt_message_id: Option<i64>,
    pub draft_id: DraftId,
    pub expires_at: DateTime<Utc>,
}

#[async_trait]
pub trait ChatFlowStore: Send + Sync {
    /// The flow for `key`, or `None` when absent or expired at `now`.
    async fn load_flow(
        &self,
        key: ChatUserKey,
        now: DateTime<Utc>,
    ) -> StoreResult<Option<StoredFlow>>;
    /// Inserts or replaces the flow for `key`.
    async fn save_flow(&self, key: ChatUserKey, flow: &StoredFlow) -> StoreResult<()>;
    async fn clear_flow(&self, key: ChatUserKey) -> StoreResult<()>;
}

/// Telegram `getUpdates` offset, saved after each handled update so a
/// restart resumes where it stopped.
#[async_trait]
pub trait BotStateStore: Send + Sync {
    async fn load_update_offset(&self) -> StoreResult<Option<i64>>;
    async fn save_update_offset(&self, offset: i64) -> StoreResult<()>;
}
