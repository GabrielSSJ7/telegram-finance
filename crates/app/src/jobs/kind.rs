use chrono::{NaiveDate, NaiveTime};
use domain::cycle::Cycle;

use crate::model::HouseholdSettings;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JobKind {
    /// Records due salaries, rents, subscriptions (06:00).
    Recurrences,
    /// Invoice closed today, or due in 3 days or today (09:00).
    InvoiceEvents,
    /// The evening summary (household report time).
    DailyReport,
    /// Closing of the financial month, on the first day of a new cycle.
    CycleReport,
    /// Warns when the nightly backup has not succeeded for a day (10:00).
    BackupWatch,
}

impl JobKind {
    pub const ALL: [JobKind; 5] = [
        JobKind::Recurrences,
        JobKind::InvoiceEvents,
        JobKind::DailyReport,
        JobKind::CycleReport,
        JobKind::BackupWatch,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            JobKind::Recurrences => "recurrences",
            JobKind::InvoiceEvents => "invoice_events",
            JobKind::DailyReport => "daily_report",
            JobKind::CycleReport => "cycle_report",
            JobKind::BackupWatch => "backup_watch",
        }
    }

    /// Accepts `daily_report` or `daily-report`.
    pub fn from_name(name: &str) -> Option<JobKind> {
        let normalized = name.replace('-', "_");
        JobKind::ALL.into_iter().find(|kind| kind.name() == normalized)
    }

    /// Local time from which the job may run on a date.
    pub fn due_time(self, settings: &HouseholdSettings) -> NaiveTime {
        let at = |hour| NaiveTime::from_hms_opt(hour, 0, 0).unwrap_or_default();
        match self {
            JobKind::Recurrences => at(6),
            JobKind::InvoiceEvents => at(9),
            JobKind::BackupWatch => at(10),
            JobKind::DailyReport | JobKind::CycleReport => settings.daily_report_time,
        }
    }

    /// The cycle report only runs on the first day of a cycle.
    pub fn applies_on(self, date: NaiveDate, settings: &HouseholdSettings) -> bool {
        match self {
            JobKind::CycleReport => Cycle::containing(date, settings.cycle_start_day).start == date,
            _ => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::DayOfMonth;

    fn settings(cycle_day: u8) -> HouseholdSettings {
        HouseholdSettings {
            cycle_start_day: DayOfMonth::new(cycle_day).unwrap(),
            ..HouseholdSettings::default()
        }
    }

    #[test]
    fn names_round_trip_with_dashes() {
        for kind in JobKind::ALL {
            assert_eq!(JobKind::from_name(kind.name()), Some(kind));
            assert_eq!(JobKind::from_name(&kind.name().replace('_', "-")), Some(kind));
        }
        assert_eq!(JobKind::from_name("nope"), None);
    }

    #[test]
    fn reports_follow_settings_time() {
        let settings = settings(1);
        assert_eq!(
            JobKind::DailyReport.due_time(&settings),
            NaiveTime::from_hms_opt(21, 0, 0).unwrap()
        );
        assert_eq!(
            JobKind::Recurrences.due_time(&settings),
            NaiveTime::from_hms_opt(6, 0, 0).unwrap()
        );
    }

    #[test]
    fn cycle_report_only_on_cycle_start() {
        let settings = settings(5);
        let on = |day| NaiveDate::from_ymd_opt(2026, 3, day).unwrap();
        assert!(JobKind::CycleReport.applies_on(on(5), &settings));
        assert!(!JobKind::CycleReport.applies_on(on(6), &settings));
        assert!(JobKind::DailyReport.applies_on(on(6), &settings));
    }
}
