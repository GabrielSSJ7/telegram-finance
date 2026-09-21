use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::InMemoryStore;
use crate::ports::{BotStateStore, ChatFlowStore, ChatUserKey, StoreResult, StoredFlow};

#[async_trait]
impl ChatFlowStore for InMemoryStore {
    async fn load_flow(
        &self,
        key: ChatUserKey,
        now: DateTime<Utc>,
    ) -> StoreResult<Option<StoredFlow>> {
        let state = self.lock();
        Ok(state.flows.get(&key).filter(|flow| flow.expires_at > now).cloned())
    }

    async fn save_flow(&self, key: ChatUserKey, flow: &StoredFlow) -> StoreResult<()> {
        self.lock().flows.insert(key, flow.clone());
        Ok(())
    }

    async fn clear_flow(&self, key: ChatUserKey) -> StoreResult<()> {
        self.lock().flows.remove(&key);
        Ok(())
    }
}

#[async_trait]
impl BotStateStore for InMemoryStore {
    async fn load_update_offset(&self) -> StoreResult<Option<i64>> {
        Ok(self.lock().update_offset)
    }

    async fn save_update_offset(&self, offset: i64) -> StoreResult<()> {
        self.lock().update_offset = Some(offset);
        Ok(())
    }
}
