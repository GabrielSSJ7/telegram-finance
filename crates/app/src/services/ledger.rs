use std::sync::Arc;

use super::ledger_validation::{
    EntryShape, ensure_distinct_accounts, ensure_editable_kind, ensure_positive_amount,
    shape_to_new_entry,
};
use super::text_rules::clean_description;
use super::{AccountService, CategoryService};
use crate::model::{
    CategoryKind, DraftId, EntryFilter, EntryId, EntryPatch, LedgerEntry, MemberId, NewEntry,
};
use crate::ports::{Clock, EntryStore};
use crate::{AppError, AppResult};

pub use super::ledger_validation::{AccountEntry, AdjustmentEntry, EntryRequest, TransferEntry};

/// Who asked for a command and its idempotency key.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EntryOrigin {
    pub created_by: Option<MemberId>,
    pub draft: Option<DraftId>,
}

pub struct LedgerService {
    entries: Arc<dyn EntryStore>,
    accounts: Arc<AccountService>,
    categories: Arc<CategoryService>,
    clock: Arc<dyn Clock>,
}

impl LedgerService {
    pub fn new(
        entries: Arc<dyn EntryStore>,
        accounts: Arc<AccountService>,
        categories: Arc<CategoryService>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self { entries, accounts, categories, clock }
    }

    /// Validates and saves one entry. Saving the same draft twice fails with
    /// `AlreadyCommitted`.
    ///
    /// ```ignore
    /// let entry = ledger.record(EntryRequest::Expense(AccountEntry { account_id, category_id,
    ///     amount: Cents::new(1050), description: "mercado".into(), date: None }), origin).await?;
    /// ```
    pub async fn record(
        &self,
        request: EntryRequest,
        origin: EntryOrigin,
    ) -> AppResult<LedgerEntry> {
        let entry = self.validate(request.into_shape(), origin.created_by).await?;
        Ok(self.entries.record_entry(entry, origin.draft).await?)
    }

    pub async fn find(&self, id: EntryId) -> AppResult<LedgerEntry> {
        match self.entries.find_entry(id).await? {
            Some(entry) if !entry.deleted => Ok(entry),
            _ => Err(AppError::not_found("entry", id)),
        }
    }

    pub async fn list(&self, filter: &EntryFilter) -> AppResult<Vec<LedgerEntry>> {
        Ok(self.entries.list_entries(filter).await?)
    }

    /// Changes amount, description, category or date of an account entry.
    pub async fn update(&self, id: EntryId, patch: EntryPatch) -> AppResult<LedgerEntry> {
        if patch.is_empty() {
            return Err(AppError::invalid("patch", "{}", "at least one field to change"));
        }
        let entry = self.find(id).await?;
        ensure_editable_kind(entry.kind)?;
        let patch = self.validate_patch(&entry, patch).await?;
        let updated = self.entries.update_entry(id, &patch).await?;
        updated.ok_or_else(|| AppError::not_found("entry", id))
    }

    pub async fn delete(&self, id: EntryId) -> AppResult<LedgerEntry> {
        let entry = self.find(id).await?;
        if !self.entries.soft_delete_entry(id, self.clock.now()).await? {
            return Err(AppError::not_found("entry", id));
        }
        Ok(entry)
    }

    /// Deletes the member's most recent entry (`/desfazer`).
    pub async fn undo_last(&self, member: MemberId) -> AppResult<LedgerEntry> {
        let latest = self.entries.latest_entry_by(member).await?;
        let entry = latest.ok_or_else(|| AppError::not_found("entry created by member", member))?;
        self.delete(entry.id).await
    }

    async fn validate(
        &self,
        shape: EntryShape,
        created_by: Option<MemberId>,
    ) -> AppResult<NewEntry> {
        ensure_positive_amount(shape.amount)?;
        ensure_distinct_accounts(&shape)?;
        self.accounts.require_active(shape.account_id).await?;
        if let Some(counter) = shape.counter_account_id {
            self.accounts.require_active(counter).await?;
        }
        if let Some(category_id) = shape.category_id {
            self.categories.require_kind(category_id, expected_category_kind(&shape)).await?;
        }
        shape_to_new_entry(&shape, self.clock.today(), created_by)
    }

    async fn validate_patch(
        &self,
        entry: &LedgerEntry,
        mut patch: EntryPatch,
    ) -> AppResult<EntryPatch> {
        if let Some(amount) = patch.amount {
            ensure_positive_amount(amount)?;
        }
        if let Some(description) = &patch.description {
            patch.description = Some(clean_description(description)?);
        }
        let Some(category_id) = patch.category_id else {
            return Ok(patch);
        };
        let Some(expected) = CategoryKind::required_for(entry.kind) else {
            return Err(AppError::invalid(
                "category",
                category_id,
                format!("no category on {} entries", entry.kind),
            ));
        };
        self.categories.require_kind(category_id, expected).await?;
        Ok(patch)
    }
}

fn expected_category_kind(shape: &EntryShape) -> CategoryKind {
    CategoryKind::required_for(shape.kind).unwrap_or(CategoryKind::Expense)
}
