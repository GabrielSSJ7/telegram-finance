use chrono::NaiveTime;
use domain::DayOfMonth;

use crate::model::SettingsPatch;
use crate::services::StorePorts;

pub async fn settings_update_and_bind_chat(stores: StorePorts) {
    let defaults = stores.settings.load_settings().await.unwrap();
    assert_eq!((defaults.cycle_start_day.get(), defaults.telegram_chat_id), (1, None));
    let at = |hour, minute| NaiveTime::from_hms_opt(hour, minute, 0).unwrap();
    assert_eq!((defaults.yesterday_report_time, defaults.today_report_time), (at(9, 0), at(21, 0)));
    let patch = SettingsPatch {
        cycle_start_day: Some(DayOfMonth::new(5).unwrap()),
        yesterday_report_time: Some(at(7, 15)),
        today_report_time: Some(at(20, 30)),
    };
    let updated = stores.settings.update_settings(patch).await.unwrap();
    assert_eq!(updated.cycle_start_day.get(), 5);
    assert_eq!((updated.yesterday_report_time, updated.today_report_time), (at(7, 15), at(20, 30)));
    let untouched = stores.settings.update_settings(SettingsPatch::default()).await.unwrap();
    assert_eq!(untouched, updated);
    let bound = stores.settings.set_household_chat(-1_001).await.unwrap();
    assert_eq!(bound.telegram_chat_id, Some(-1_001));
    assert_eq!(stores.settings.load_settings().await.unwrap(), bound);
}
