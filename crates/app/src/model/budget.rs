use domain::Cents;
use serde::{Deserialize, Serialize};

use super::{BudgetId, Category, CategoryId};

/// A monthly spending limit for one expense category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Budget {
    pub id: BudgetId,
    pub category_id: CategoryId,
    pub limit: Cents,
}

/// A budget against what was spent in a cycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetStatus {
    pub budget: Budget,
    pub category: Category,
    pub spent: Cents,
    /// `spent / limit` in basis points (100% = 10000).
    pub used_bp: i64,
}

/// A threshold (80 or 100%) crossed for the first time this cycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetAlert {
    pub status: BudgetStatus,
    pub threshold: u8,
}
