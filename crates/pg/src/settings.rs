use app::model::{HouseholdSettings, SettingsPatch};
use app::ports::{SettingsStore, StoreResult};
use async_trait::async_trait;
use chrono::NaiveTime;
use domain::DayOfMonth;

use crate::PgStore;
use crate::error_mapping::{corrupt, store_error};

struct HouseholdRow {
    telegram_chat_id: Option<i64>,
    cycle_start_day: i16,
    yesterday_report_time: NaiveTime,
    today_report_time: NaiveTime,
}

impl HouseholdRow {
    fn into_settings(self) -> StoreResult<HouseholdSettings> {
        let cycle_start_day = DayOfMonth::try_from(self.cycle_start_day)
            .map_err(|error| corrupt("household.cycle_start_day", error))?;
        Ok(HouseholdSettings {
            telegram_chat_id: self.telegram_chat_id,
            cycle_start_day,
            yesterday_report_time: self.yesterday_report_time,
            today_report_time: self.today_report_time,
        })
    }
}

#[async_trait]
impl SettingsStore for PgStore {
    async fn load_settings(&self) -> StoreResult<HouseholdSettings> {
        let row = sqlx::query_as!(
            HouseholdRow,
            "select telegram_chat_id, cycle_start_day, yesterday_report_time, today_report_time
             from household where id = 1",
        )
        .fetch_one(self.pool())
        .await
        .map_err(store_error)?;
        row.into_settings()
    }

    async fn update_settings(&self, patch: SettingsPatch) -> StoreResult<HouseholdSettings> {
        let row = sqlx::query_as!(
            HouseholdRow,
            "update household set
                 cycle_start_day = coalesce($1, cycle_start_day),
                 yesterday_report_time = coalesce($2, yesterday_report_time),
                 today_report_time = coalesce($3, today_report_time)
             where id = 1
             returning telegram_chat_id, cycle_start_day, yesterday_report_time, today_report_time",
            patch.cycle_start_day.map(i16::from),
            patch.yesterday_report_time,
            patch.today_report_time,
        )
        .fetch_one(self.pool())
        .await
        .map_err(store_error)?;
        row.into_settings()
    }

    async fn set_household_chat(&self, chat_id: i64) -> StoreResult<HouseholdSettings> {
        let row = sqlx::query_as!(
            HouseholdRow,
            "update household set telegram_chat_id = $1 where id = 1
             returning telegram_chat_id, cycle_start_day, yesterday_report_time, today_report_time",
            chat_id,
        )
        .fetch_one(self.pool())
        .await
        .map_err(store_error)?;
        row.into_settings()
    }
}
