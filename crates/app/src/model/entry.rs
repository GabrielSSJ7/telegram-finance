use chrono::{DateTime, NaiveDate, Utc};
use domain::{Cents, EntryKind};
use serde::{Deserialize, Serialize};

use super::{AccountId, CategoryId, EntryId, MemberId};

/// One row of the ledger. Card fields arrive with the cards phase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub id: EntryId,
    pub kind: EntryKind,
    pub amount: Cents,
    pub description: String,
    pub category_id: Option<CategoryId>,
    pub account_id: Option<AccountId>,
    pub counter_account_id: Option<AccountId>,
    pub accounting_date: NaiveDate,
    pub created_by: Option<MemberId>,
    pub created_at: DateTime<Utc>,
    pub deleted: bool,
}

/// A validated entry ready to persist.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewEntry {
    pub kind: EntryKind,
    pub amount: Cents,
    pub description: String,
    pub category_id: Option<CategoryId>,
    pub account_id: Option<AccountId>,
    pub counter_account_id: Option<AccountId>,
    pub accounting_date: NaiveDate,
    pub created_by: Option<MemberId>,
}

/// Fields that may change after an entry is recorded.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntryPatch {
    pub amount: Option<Cents>,
    pub description: Option<String>,
    pub category_id: Option<CategoryId>,
    pub accounting_date: Option<NaiveDate>,
}

impl EntryPatch {
    pub fn is_empty(&self) -> bool {
        self == &EntryPatch::default()
    }
}

/// Filters for listing entries; `None` means "any". Newest first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntryFilter {
    pub from: Option<NaiveDate>,
    pub to_inclusive: Option<NaiveDate>,
    pub kind: Option<EntryKind>,
    pub account_id: Option<AccountId>,
    pub category_id: Option<CategoryId>,
    pub created_by: Option<MemberId>,
    pub limit: u32,
}

impl Default for EntryFilter {
    fn default() -> Self {
        Self {
            from: None,
            to_inclusive: None,
            kind: None,
            account_id: None,
            category_id: None,
            created_by: None,
            limit: 100,
        }
    }
}
