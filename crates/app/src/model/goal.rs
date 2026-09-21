use chrono::NaiveDate;
use domain::Cents;
use serde::{Deserialize, Serialize};

use super::{Account, GoalId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalTarget {
    pub target: Cents,
    pub target_date: Option<NaiveDate>,
}

/// A savings goal and the pot account that holds its money.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Goal {
    pub id: GoalId,
    pub pot: Account,
    pub target: GoalTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalProgress {
    pub goal: Goal,
    pub saved: Cents,
    pub remaining: Cents,
    /// `saved / target` in basis points, capped at 10000.
    pub progress_bp: i64,
}
