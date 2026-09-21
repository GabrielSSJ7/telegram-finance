//! Time source. Every accounting date comes from here, in the household
//! timezone, so "today" is the couple's today and not the server's.

use chrono::{DateTime, NaiveDate, Utc};
use chrono_tz::Tz;

pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
    fn timezone(&self) -> Tz;

    fn local_now(&self) -> DateTime<Tz> {
        self.now().with_timezone(&self.timezone())
    }

    fn today(&self) -> NaiveDate {
        self.local_now().date_naive()
    }
}

/// Wall clock in a fixed timezone.
///
/// ```
/// use app::ports::{Clock, SystemClock};
/// let clock = SystemClock::new(chrono_tz::America::Sao_Paulo);
/// assert_eq!(clock.timezone(), chrono_tz::America::Sao_Paulo);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct SystemClock {
    timezone: Tz,
}

impl SystemClock {
    pub fn new(timezone: Tz) -> Self {
        Self { timezone }
    }
}

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }

    fn timezone(&self) -> Tz {
        self.timezone
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    struct NineThirtyPmUtcMinus3;

    impl Clock for NineThirtyPmUtcMinus3 {
        fn now(&self) -> DateTime<Utc> {
            Utc.with_ymd_and_hms(2026, 3, 11, 0, 30, 0).unwrap()
        }
        fn timezone(&self) -> Tz {
            chrono_tz::America::Sao_Paulo
        }
    }

    #[test]
    fn today_uses_household_timezone_not_utc() {
        let clock = NineThirtyPmUtcMinus3;
        assert_eq!(clock.today(), NaiveDate::from_ymd_opt(2026, 3, 10).unwrap());
    }

    #[test]
    fn system_clock_reports_its_timezone() {
        let clock = SystemClock::new(chrono_tz::Europe::Lisbon);
        assert_eq!(clock.timezone(), chrono_tz::Europe::Lisbon);
        assert!(clock.now() > Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap());
    }
}
