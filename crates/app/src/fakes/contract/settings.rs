use chrono::NaiveTime;
use domain::DayOfMonth;

use crate::model::SettingsPatch;
use crate::services::StorePorts;

pub async fn settings_update_and_bind_chat(stores: StorePorts) {
    let defaults = stores.settings.load_settings().await.unwrap();
    assert_eq!((defaults.cycle_start_day.get(), defaults.telegram_chat_id), (1, None));
    assert_eq!(defaults.daily_report_time, NaiveTime::from_hms_opt(21, 0, 0).unwrap());
    let time = NaiveTime::from_hms_opt(20, 30, 0).unwrap();
    let patch = SettingsPatch {
        cycle_start_day: Some(DayOfMonth::new(5).unwrap()),
        daily_report_time: Some(time),
    };
    let updated = stores.settings.update_settings(patch).await.unwrap();
    assert_eq!((updated.cycle_start_day.get(), updated.daily_report_time), (5, time));
    let untouched = stores.settings.update_settings(SettingsPatch::default()).await.unwrap();
    assert_eq!(untouched, updated);
    let bound = stores.settings.set_household_chat(-1_001).await.unwrap();
    assert_eq!(bound.telegram_chat_id, Some(-1_001));
    assert_eq!(stores.settings.load_settings().await.unwrap(), bound);
}
