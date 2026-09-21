//! A day number 1-31 as typed by the user (card closing day, cycle start,
//! recurrence day). Months shorter than the day clamp to their last day.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub struct DayOfMonth(u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("invalid day of month {0}: expected an integer from 1 to 31")]
pub struct DayOfMonthError(pub i64);

impl DayOfMonth {
    /// Builds a validated day.
    ///
    /// ```
    /// use domain::DayOfMonth;
    /// assert_eq!(DayOfMonth::new(31).map(DayOfMonth::get), Ok(31));
    /// assert!(DayOfMonth::new(0).is_err());
    /// ```
    pub fn new(day: u8) -> Result<Self, DayOfMonthError> {
        if !(1..=31).contains(&day) {
            return Err(DayOfMonthError(i64::from(day)));
        }
        Ok(Self(day))
    }

    pub const fn get(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for DayOfMonth {
    type Error = DayOfMonthError;
    fn try_from(day: u8) -> Result<Self, Self::Error> {
        Self::new(day)
    }
}

impl TryFrom<i16> for DayOfMonth {
    type Error = DayOfMonthError;
    fn try_from(day: i16) -> Result<Self, Self::Error> {
        let narrow = u8::try_from(day).map_err(|_| DayOfMonthError(i64::from(day)))?;
        Self::new(narrow)
    }
}

impl From<DayOfMonth> for u8 {
    fn from(day: DayOfMonth) -> u8 {
        day.0
    }
}

impl From<DayOfMonth> for i16 {
    fn from(day: DayOfMonth) -> i16 {
        i16::from(day.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_one_to_thirty_one() {
        for day in 1..=31u8 {
            assert_eq!(DayOfMonth::new(day).map(u8::from), Ok(day));
        }
    }

    #[test]
    fn rejects_out_of_range_with_value_in_message() {
        assert_eq!(DayOfMonth::new(32), Err(DayOfMonthError(32)));
        assert_eq!(DayOfMonth::try_from(-1i16), Err(DayOfMonthError(-1)));
        let message = DayOfMonth::new(0).unwrap_err().to_string();
        assert!(message.contains('0') && message.contains("1 to 31"), "{message}");
    }

    #[test]
    fn converts_to_database_integer() {
        let day = DayOfMonth::try_from(15i16).unwrap();
        assert_eq!(i16::from(day), 15);
    }
}
