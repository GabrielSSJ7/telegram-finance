use std::sync::Arc;

use chrono::NaiveTime;

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

    /// Changes the given settings; today's summary must stay in the evening.
    ///
    /// ```ignore
    /// let evening = NaiveTime::from_hms_opt(21, 30, 0);
    /// settings.update(SettingsPatch { today_report_time: evening, ..SettingsPatch::default() }).await?;
    /// ```
    pub async fn update(&self, patch: SettingsPatch) -> AppResult<HouseholdSettings> {
        if let Some(time) = patch.today_report_time.filter(|time| !is_evening_report_time(*time)) {
            let expected = format!("{} or later", EARLIEST_TODAY_REPORT.format("%H:%M"));
            return Err(AppError::invalid("today report time", time.format("%H:%M"), expected));
        }
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

/// Today's summary goes out late enough to cover most of the day.
pub const EARLIEST_TODAY_REPORT: NaiveTime = match NaiveTime::from_hms_opt(19, 0, 0) {
    Some(time) => time,
    None => NaiveTime::MIN,
};

/// Whether `time` may be used for today's summary.
///
/// ```ignore
/// assert!(is_evening_report_time(NaiveTime::from_hms_opt(21, 0, 0).unwrap()));
/// ```
pub fn is_evening_report_time(time: NaiveTime) -> bool {
    time >= EARLIEST_TODAY_REPORT
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fakes::InMemoryStore;
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
            ..SettingsPatch::default()
        };
        let updated = service.update(patch).await.unwrap();
        assert_eq!(updated.cycle_start_day.get(), 5);
        assert_eq!(updated.today_report_time, NaiveTime::from_hms_opt(21, 0, 0).unwrap());
    }

    #[tokio::test]
    async fn today_summary_stays_in_the_evening() {
        let service = SettingsService::new(Arc::new(InMemoryStore::new()));
        let at = |hour| NaiveTime::from_hms_opt(hour, 0, 0);
        let morning = SettingsPatch { today_report_time: at(9), ..SettingsPatch::default() };
        let error = service.update(morning).await.unwrap_err().to_string();
        assert!(error.contains("09:00") && error.contains("19:00 or later"), "{error}");
        let evening = SettingsPatch {
            today_report_time: at(19),
            yesterday_report_time: at(7),
            ..SettingsPatch::default()
        };
        let updated = service.update(evening).await.unwrap();
        assert_eq!(
            (Some(updated.today_report_time), Some(updated.yesterday_report_time)),
            (at(19), at(7))
        );
    }
}
