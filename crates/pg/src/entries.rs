use app::model::{
    AccountId, CategoryId, DraftId, EntryFilter, EntryId, EntryPatch, InvoiceId, LedgerEntry,
    MemberId, NewEntry, PurchaseId,
};
use app::ports::{EntryStore, StoreResult};
use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use domain::{Cents, EntryKind};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::PgStore;
use crate::drafts::commit_draft;
use crate::error_mapping::{corrupt, store_error};

pub(crate) struct EntryRow {
    id: Uuid,
    kind: String,
    amount_cents: i64,
    description: String,
    category_id: Option<Uuid>,
    account_id: Option<Uuid>,
    counter_account_id: Option<Uuid>,
    card_purchase_id: Option<Uuid>,
    installment_no: Option<i16>,
    invoice_id: Option<Uuid>,
    accounting_date: NaiveDate,
    created_by: Option<Uuid>,
    created_at: DateTime<Utc>,
    deleted_at: Option<DateTime<Utc>>,
}

impl EntryRow {
    pub(crate) fn into_entry(self) -> StoreResult<LedgerEntry> {
        let kind = self
            .kind
            .parse::<EntryKind>()
            .map_err(|error| corrupt("ledger_entries.kind", error))?;
        Ok(LedgerEntry {
            id: EntryId(self.id),
            kind,
            amount: Cents::new(self.amount_cents),
            description: self.description,
            category_id: self.category_id.map(CategoryId),
            account_id: self.account_id.map(AccountId),
            counter_account_id: self.counter_account_id.map(AccountId),
            card_purchase_id: self.card_purchase_id.map(PurchaseId),
            installment_no: self.installment_no.and_then(|number| u32::try_from(number).ok()),
            invoice_id: self.invoice_id.map(InvoiceId),
            accounting_date: self.accounting_date,
            created_by: self.created_by.map(MemberId),
            created_at: self.created_at,
            deleted: self.deleted_at.is_some(),
        })
    }
}

pub(crate) async fn insert_entry(
    connection: &mut PgConnection,
    entry: &NewEntry,
) -> StoreResult<EntryRow> {
    sqlx::query_file_as!(
        EntryRow,
        "queries/insert_entry.sql",
        entry.kind.as_str(),
        entry.amount.value(),
        entry.description,
        entry.category_id.map(|id| id.0),
        entry.account_id.map(|id| id.0),
        entry.counter_account_id.map(|id| id.0),
        entry.invoice_id.map(|id| id.0),
        entry.accounting_date,
        entry.created_by.map(|id| id.0),
    )
    .fetch_one(connection)
    .await
    .map_err(store_error)
}

fn rows_into_entries(rows: Vec<EntryRow>) -> StoreResult<Vec<LedgerEntry>> {
    rows.into_iter().map(EntryRow::into_entry).collect()
}

#[async_trait]
impl EntryStore for PgStore {
    async fn record_entry(
        &self,
        entry: NewEntry,
        draft: Option<DraftId>,
    ) -> StoreResult<LedgerEntry> {
        let mut transaction = self.pool().begin().await.map_err(store_error)?;
        if let Some(draft_id) = draft {
            commit_draft(&mut transaction, draft_id).await?;
        }
        let row = insert_entry(&mut transaction, &entry).await?;
        transaction.commit().await.map_err(store_error)?;
        row.into_entry()
    }

    async fn find_entry(&self, id: EntryId) -> StoreResult<Option<LedgerEntry>> {
        let row = sqlx::query_as!(
            EntryRow,
            "select id, kind, amount_cents, description, category_id, account_id, counter_account_id,
                    card_purchase_id, installment_no, invoice_id, accounting_date, created_by, created_at, deleted_at
             from ledger_entries where id = $1",
            id.0,
        )
        .fetch_optional(self.pool())
        .await
        .map_err(store_error)?;
        row.map(EntryRow::into_entry).transpose()
    }

    async fn list_entries(&self, filter: &EntryFilter) -> StoreResult<Vec<LedgerEntry>> {
        let rows = sqlx::query_file_as!(
            EntryRow,
            "queries/list_entries.sql",
            filter.from,
            filter.to_inclusive,
            filter.kind.map(EntryKind::as_str),
            filter.account_id.map(|id| id.0),
            filter.category_id.map(|id| id.0),
            filter.created_by.map(|id| id.0),
            i64::from(filter.limit),
        )
        .fetch_all(self.pool())
        .await
        .map_err(store_error)?;
        rows_into_entries(rows)
    }

    async fn latest_entry_by(&self, member: MemberId) -> StoreResult<Option<LedgerEntry>> {
        let row = sqlx::query_as!(
            EntryRow,
            "select id, kind, amount_cents, description, category_id, account_id, counter_account_id,
                    card_purchase_id, installment_no, invoice_id, accounting_date, created_by, created_at, deleted_at
             from ledger_entries
             where created_by = $1 and deleted_at is null
             order by created_at desc, id desc
             limit 1",
            member.0,
        )
        .fetch_optional(self.pool())
        .await
        .map_err(store_error)?;
        row.map(EntryRow::into_entry).transpose()
    }

    async fn update_entry(
        &self,
        id: EntryId,
        patch: &EntryPatch,
    ) -> StoreResult<Option<LedgerEntry>> {
        let row = sqlx::query_file_as!(
            EntryRow,
            "queries/update_entry.sql",
            id.0,
            patch.amount.map(Cents::value),
            patch.description,
            patch.category_id.map(|category| category.0),
            patch.accounting_date,
        )
        .fetch_optional(self.pool())
        .await
        .map_err(store_error)?;
        row.map(EntryRow::into_entry).transpose()
    }

    async fn soft_delete_entry(&self, id: EntryId, at: DateTime<Utc>) -> StoreResult<bool> {
        let result = sqlx::query!(
            "update ledger_entries set deleted_at = $2 where id = $1 and deleted_at is null",
            id.0,
            at
        )
        .execute(self.pool())
        .await
        .map_err(store_error)?;
        Ok(result.rows_affected() == 1)
    }
}
