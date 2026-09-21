use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::InMemoryStore;
use crate::model::{DraftId, EntryFilter, EntryId, EntryPatch, LedgerEntry, MemberId, NewEntry};
use crate::ports::{EntryStore, StoreError, StoreResult};

#[async_trait]
impl EntryStore for InMemoryStore {
    async fn record_entry(
        &self,
        entry: NewEntry,
        draft: Option<DraftId>,
    ) -> StoreResult<LedgerEntry> {
        let mut state = self.lock();
        if let Some(draft_id) = draft
            && !state.drafts.insert(draft_id)
        {
            return Err(StoreError::DuplicateDraft);
        }
        let recorded = ledger_entry(entry);
        state.entries.push(recorded.clone());
        Ok(recorded)
    }

    async fn find_entry(&self, id: EntryId) -> StoreResult<Option<LedgerEntry>> {
        Ok(self.lock().entries.iter().find(|entry| entry.id == id).cloned())
    }

    async fn list_entries(&self, filter: &EntryFilter) -> StoreResult<Vec<LedgerEntry>> {
        let state = self.lock();
        let mut found: Vec<LedgerEntry> =
            state.entries.iter().filter(|entry| matches_filter(entry, filter)).cloned().collect();
        found.sort_by(|left, right| {
            (right.accounting_date, right.id).cmp(&(left.accounting_date, left.id))
        });
        found.truncate(usize::try_from(filter.limit).unwrap_or(usize::MAX));
        Ok(found)
    }

    async fn latest_entry_by(&self, member: MemberId) -> StoreResult<Option<LedgerEntry>> {
        let state = self.lock();
        let mut mine = state.entries.iter().rev();
        Ok(mine.find(|entry| !entry.deleted && entry.created_by == Some(member)).cloned())
    }

    async fn update_entry(
        &self,
        id: EntryId,
        patch: &EntryPatch,
    ) -> StoreResult<Option<LedgerEntry>> {
        let mut state = self.lock();
        let Some(entry) = state.entries.iter_mut().find(|entry| entry.id == id && !entry.deleted)
        else {
            return Ok(None);
        };
        apply_patch(entry, patch);
        Ok(Some(entry.clone()))
    }

    async fn soft_delete_entry(&self, id: EntryId, _at: DateTime<Utc>) -> StoreResult<bool> {
        let mut state = self.lock();
        let Some(entry) = state.entries.iter_mut().find(|entry| entry.id == id && !entry.deleted)
        else {
            return Ok(false);
        };
        entry.deleted = true;
        Ok(true)
    }
}

fn ledger_entry(entry: NewEntry) -> LedgerEntry {
    LedgerEntry {
        id: EntryId::generate(),
        kind: entry.kind,
        amount: entry.amount,
        description: entry.description,
        category_id: entry.category_id,
        account_id: entry.account_id,
        counter_account_id: entry.counter_account_id,
        accounting_date: entry.accounting_date,
        created_by: entry.created_by,
        created_at: Utc::now(),
        deleted: false,
    }
}

fn matches_filter(entry: &LedgerEntry, filter: &EntryFilter) -> bool {
    let touches_account = |id| entry.account_id == Some(id) || entry.counter_account_id == Some(id);
    !entry.deleted
        && filter.from.is_none_or(|from| entry.accounting_date >= from)
        && filter.to_inclusive.is_none_or(|to| entry.accounting_date <= to)
        && filter.kind.is_none_or(|kind| entry.kind == kind)
        && filter.account_id.is_none_or(touches_account)
        && filter.category_id.is_none_or(|id| entry.category_id == Some(id))
        && filter.created_by.is_none_or(|id| entry.created_by == Some(id))
}

fn apply_patch(entry: &mut LedgerEntry, patch: &EntryPatch) {
    if let Some(amount) = patch.amount {
        entry.amount = amount;
    }
    if let Some(description) = &patch.description {
        entry.description.clone_from(description);
    }
    if let Some(category_id) = patch.category_id {
        entry.category_id = Some(category_id);
    }
    if let Some(date) = patch.accounting_date {
        entry.accounting_date = date;
    }
}
