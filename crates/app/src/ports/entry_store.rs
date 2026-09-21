use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::StoreResult;
use crate::model::{DraftId, EntryFilter, EntryId, EntryPatch, LedgerEntry, MemberId, NewEntry};

#[async_trait]
pub trait EntryStore: Send + Sync {
    /// Persists the entry and, when given, commits `draft` in the same
    /// transaction. A second commit of one draft fails with `DuplicateDraft`.
    async fn record_entry(
        &self,
        entry: NewEntry,
        draft: Option<DraftId>,
    ) -> StoreResult<LedgerEntry>;
    async fn find_entry(&self, id: EntryId) -> StoreResult<Option<LedgerEntry>>;
    /// Non-deleted entries matching `filter`, newest `accounting_date` first.
    async fn list_entries(&self, filter: &EntryFilter) -> StoreResult<Vec<LedgerEntry>>;
    /// The member's most recently created non-deleted entry.
    async fn latest_entry_by(&self, member: MemberId) -> StoreResult<Option<LedgerEntry>>;
    async fn update_entry(
        &self,
        id: EntryId,
        patch: &EntryPatch,
    ) -> StoreResult<Option<LedgerEntry>>;
    /// Returns false when the entry does not exist or is already deleted.
    async fn soft_delete_entry(&self, id: EntryId, at: DateTime<Utc>) -> StoreResult<bool>;
}
