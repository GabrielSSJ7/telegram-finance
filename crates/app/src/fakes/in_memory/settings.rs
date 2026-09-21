use async_trait::async_trait;

use super::InMemoryStore;
use crate::model::{HouseholdSettings, SettingsPatch};
use crate::ports::{SettingsStore, StoreResult};

#[async_trait]
impl SettingsStore for InMemoryStore {
    async fn load_settings(&self) -> StoreResult<HouseholdSettings> {
        Ok(self.lock().settings)
    }

    async fn update_settings(&self, patch: SettingsPatch) -> StoreResult<HouseholdSettings> {
        let mut state = self.lock();
        if let Some(day) = patch.cycle_start_day {
            state.settings.cycle_start_day = day;
        }
        if let Some(time) = patch.daily_report_time {
            state.settings.daily_report_time = time;
        }
        Ok(state.settings)
    }

    async fn set_household_chat(&self, chat_id: i64) -> StoreResult<HouseholdSettings> {
        let mut state = self.lock();
        state.settings.telegram_chat_id = Some(chat_id);
        Ok(state.settings)
    }
}
