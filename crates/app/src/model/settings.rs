use chrono::NaiveTime;
use domain::DayOfMonth;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HouseholdSettings {
    /// The couple's group, set by the first `/start` there.
    pub telegram_chat_id: Option<i64>,
    pub cycle_start_day: DayOfMonth,
    pub daily_report_time: NaiveTime,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingsPatch {
    pub cycle_start_day: Option<DayOfMonth>,
    pub daily_report_time: Option<NaiveTime>,
}
