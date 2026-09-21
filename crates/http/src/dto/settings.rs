use app::AppError;
use app::model::{HouseholdSettings, SettingsPatch};
use chrono::NaiveTime;
use domain::DayOfMonth;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct SettingsResponse {
    pub telegram_chat_id: Option<i64>,
    /// First day of the household's financial month.
    #[schema(example = 5)]
    pub cycle_start_day: u8,
    /// When the summary of the previous day goes out (any hour).
    #[schema(value_type = String, example = "09:00:00")]
    pub yesterday_report_time: NaiveTime,
    /// When the summary of the current day goes out (19:00 or later).
    #[schema(value_type = String, example = "21:00:00")]
    pub today_report_time: NaiveTime,
}

impl From<HouseholdSettings> for SettingsResponse {
    fn from(settings: HouseholdSettings) -> Self {
        Self {
            telegram_chat_id: settings.telegram_chat_id,
            cycle_start_day: settings.cycle_start_day.get(),
            yesterday_report_time: settings.yesterday_report_time,
            today_report_time: settings.today_report_time,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SettingsPatchBody {
    #[schema(example = 5)]
    pub cycle_start_day: Option<u8>,
    #[schema(value_type = Option<String>, example = "08:00:00")]
    pub yesterday_report_time: Option<NaiveTime>,
    /// 19:00 or later.
    #[schema(value_type = Option<String>, example = "20:30:00")]
    pub today_report_time: Option<NaiveTime>,
}

impl TryFrom<SettingsPatchBody> for SettingsPatch {
    type Error = AppError;
    fn try_from(body: SettingsPatchBody) -> Result<Self, Self::Error> {
        let day = body.cycle_start_day.map(DayOfMonth::new).transpose();
        let cycle_start_day =
            day.map_err(|error| AppError::invalid("cycle_start_day", error.0, "1 to 31"))?;
        Ok(SettingsPatch {
            cycle_start_day,
            yesterday_report_time: body.yesterday_report_time,
            today_report_time: body.today_report_time,
        })
    }
}
