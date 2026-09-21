use std::str::FromStr;

use chrono::NaiveDate;
use domain::{Cents, DayOfMonth};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{AccountId, CardId, CategoryId, RecurrenceId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecurrenceKind {
    Income,
    Expense,
}

/// `Auto` records on the day; `Confirm` asks first (bills that vary).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecurrenceMode {
    Auto,
    Confirm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", content = "id", rename_all = "snake_case")]
pub enum RecurrenceTarget {
    Account(AccountId),
    /// Only for expenses (subscriptions on the card).
    Card(CardId),
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("unknown {what} {value:?}: expected {expected}")]
pub struct UnknownRecurrenceValue {
    pub what: &'static str,
    pub value: String,
    pub expected: &'static str,
}

impl RecurrenceKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            RecurrenceKind::Income => "income",
            RecurrenceKind::Expense => "expense",
        }
    }
}

impl FromStr for RecurrenceKind {
    type Err = UnknownRecurrenceValue;
    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name {
            "income" => Ok(RecurrenceKind::Income),
            "expense" => Ok(RecurrenceKind::Expense),
            other => Err(UnknownRecurrenceValue {
                what: "recurrence kind",
                value: other.into(),
                expected: "income or expense",
            }),
        }
    }
}

impl RecurrenceMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            RecurrenceMode::Auto => "auto",
            RecurrenceMode::Confirm => "confirm",
        }
    }
}

impl FromStr for RecurrenceMode {
    type Err = UnknownRecurrenceValue;
    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name {
            "auto" => Ok(RecurrenceMode::Auto),
            "confirm" => Ok(RecurrenceMode::Confirm),
            other => Err(UnknownRecurrenceValue {
                what: "recurrence mode",
                value: other.into(),
                expected: "auto or confirm",
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recurrence {
    pub id: RecurrenceId,
    pub kind: RecurrenceKind,
    pub amount: Cents,
    pub description: String,
    pub category_id: CategoryId,
    pub target: RecurrenceTarget,
    pub day: DayOfMonth,
    pub mode: RecurrenceMode,
    pub active: bool,
    pub starts_on: NaiveDate,
    pub last_generated_on: Option<NaiveDate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewRecurrence {
    pub kind: RecurrenceKind,
    pub amount: Cents,
    pub description: String,
    pub category_id: CategoryId,
    pub target: RecurrenceTarget,
    pub day: DayOfMonth,
    pub mode: RecurrenceMode,
    pub starts_on: NaiveDate,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_and_modes_round_trip_names() {
        for kind in [RecurrenceKind::Income, RecurrenceKind::Expense] {
            assert_eq!(kind.as_str().parse::<RecurrenceKind>(), Ok(kind));
        }
        for mode in [RecurrenceMode::Auto, RecurrenceMode::Confirm] {
            assert_eq!(mode.as_str().parse::<RecurrenceMode>(), Ok(mode));
        }
        let error = "weekly".parse::<RecurrenceMode>().unwrap_err().to_string();
        assert!(error.contains("\"weekly\"") && error.contains("auto or confirm"), "{error}");
        assert!("gift".parse::<RecurrenceKind>().is_err());
    }
}
