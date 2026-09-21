use std::sync::Mutex;

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;

use crate::ports::Clock;

/// Clock frozen at a chosen instant in `America/Sao_Paulo`; tests move it
/// with [`FixedClock::set`].
#[derive(Debug)]
pub struct FixedClock {
    now: Mutex<DateTime<Utc>>,
}

impl FixedClock {
    /// Noon local time on `date`.
    pub fn at_local_noon(date: NaiveDate) -> Self {
        Self { now: Mutex::new(local_noon(date)) }
    }

    pub fn set(&self, instant: DateTime<Utc>) {
        *self.now.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = instant;
    }

    pub fn set_local_noon(&self, date: NaiveDate) {
        self.set(local_noon(date));
    }
}

fn local_noon(date: NaiveDate) -> DateTime<Utc> {
    let noon = date.and_hms_opt(12, 0, 0).unwrap_or_default();
    chrono_tz::America::Sao_Paulo
        .from_local_datetime(&noon)
        .single()
        .map_or_else(|| noon.and_utc(), |local| local.with_timezone(&Utc))
}

impl Clock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        *self.now.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn timezone(&self) -> Tz {
        chrono_tz::America::Sao_Paulo
    }
}
