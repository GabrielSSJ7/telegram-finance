use std::sync::Arc;

use crate::model::{HouseholdSettings, SettingsPatch};
use crate::ports::SettingsStore;
use crate::{AppError, AppResult};

pub struct SettingsService {
    settings: Arc<dyn SettingsStore>,
}

impl SettingsService {
    pub fn new(settings: Arc<dyn SettingsStore>) -> Self {
        Self { settings }
    }

    pub async fn get(&self) -> AppResult<HouseholdSettings> {
        Ok(self.settings.load_settings().await?)
    }

    pub async fn update(&self, patch: SettingsPatch) -> AppResult<HouseholdSettings> {
        Ok(self.settings.update_settings(patch).await?)
    }

    /// Binds the household to its Telegram group. The first group wins;
    /// another group trying later is a conflict.
    pub async fn bind_chat(&self, chat_id: i64) -> AppResult<HouseholdSettings> {
        let current = self.settings.load_settings().await?;
        match current.telegram_chat_id {
            Some(existing) if existing == chat_id => Ok(current),
            Some(existing) => Err(AppError::Conflict(format!(
                "household already uses chat {existing}; refusing chat {chat_id}"
            ))),
            None => Ok(self.settings.set_household_chat(chat_id).await?),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fakes::InMemoryStore;
    use chrono::NaiveTime;
    use domain::DayOfMonth;

    #[tokio::test]
    async fn first_chat_wins() {
        let service = SettingsService::new(Arc::new(InMemoryStore::new()));
        assert_eq!(service.bind_chat(-100).await.unwrap().telegram_chat_id, Some(-100));
        assert!(service.bind_chat(-100).await.is_ok());
        let error = service.bind_chat(-200).await.unwrap_err().to_string();
        assert!(error.contains("-100") && error.contains("-200"), "{error}");
    }

    #[tokio::test]
    async fn update_changes_only_given_fields() {
        let service = SettingsService::new(Arc::new(InMemoryStore::new()));
        let patch = SettingsPatch {
            cycle_start_day: Some(DayOfMonth::new(5).unwrap()),
            daily_report_time: None,
        };
        let updated = service.update(patch).await.unwrap();
        assert_eq!(updated.cycle_start_day.get(), 5);
        assert_eq!(updated.daily_report_time, NaiveTime::from_hms_opt(21, 0, 0).unwrap());
    }
}
