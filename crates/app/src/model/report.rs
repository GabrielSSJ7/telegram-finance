//! Reports the scheduler sends and the API returns. They carry the names
//! (categories, members) needed to render them without more lookups.

use chrono::NaiveDate;
use domain::cycle::Cycle;
use domain::spend::PeriodSummary;
use domain::{Cents, EntryKind};
use serde::{Deserialize, Serialize};

use super::{
    BalanceSheet, BudgetStatus, CardSummary, Category, CategoryId, GoalProgress, LedgerEntry,
    Member, MemberId, Recurrence,
};

/// Live entry totals for one (category, author, kind) inside a period.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeriodFlow {
    pub category_id: Option<CategoryId>,
    pub created_by: Option<MemberId>,
    pub kind: EntryKind,
    pub total: Cents,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeriodTotals {
    pub summary: PeriodSummary,
    /// Net spending per category, largest first.
    pub by_category: Vec<(Option<CategoryId>, Cents)>,
    /// Net spending per person who recorded it, largest first.
    pub by_member: Vec<(Option<MemberId>, Cents)>,
}

/// Which day a daily summary describes, relative to when it is sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportDay {
    Today,
    Yesterday,
}

/// A day's summary: that day, the cycle up to it, and what is coming.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DailyReport {
    pub date: NaiveDate,
    pub cycle: Cycle,
    pub entries_today: Vec<LedgerEntry>,
    pub today: PeriodTotals,
    pub cycle_to_date: PeriodTotals,
    pub balances: BalanceSheet,
    pub cards: Vec<CardSummary>,
    pub goals: Vec<GoalProgress>,
    pub upcoming: Vec<(Recurrence, NaiveDate)>,
    pub budgets: Vec<BudgetStatus>,
    pub categories: Vec<Category>,
    pub members: Vec<Member>,
}

/// The closing of a financial month.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CycleReport {
    pub cycle: Cycle,
    pub totals: PeriodTotals,
    pub previous: PeriodSummary,
    /// Net money moved into goal pots during the cycle.
    pub saved_in_pots: Cents,
    pub balances: BalanceSheet,
    pub cards: Vec<CardSummary>,
    pub goals: Vec<GoalProgress>,
    pub budgets: Vec<BudgetStatus>,
    pub categories: Vec<Category>,
    pub members: Vec<Member>,
}
