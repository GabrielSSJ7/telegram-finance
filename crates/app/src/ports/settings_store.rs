use async_trait::async_trait;

use super::StoreResult;
use crate::model::{HouseholdSettings, SettingsPatch};

#[async_trait]
pub trait SettingsStore: Send + Sync {
    async fn load_settings(&self) -> StoreResult<HouseholdSettings>;
    async fn update_settings(&self, patch: SettingsPatch) -> StoreResult<HouseholdSettings>;
    async fn set_household_chat(&self, chat_id: i64) -> StoreResult<HouseholdSettings>;
}
