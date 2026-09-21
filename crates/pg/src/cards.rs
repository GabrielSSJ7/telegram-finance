use app::model::{
    CardId, CardPurchase, CreditCard, DraftId, Invoice, InvoiceId, LedgerEntry, NewCard,
    NewCardPurchase, NewEntry, PurchaseId,
};
use app::ports::{CardStore, StoreResult};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::installments::InstallmentSlot;
use domain::invoice_cycle::InvoicePeriod;
use domain::invoice_settlement::InvoiceTotals;
use sqlx::PgConnection;

use crate::PgStore;
use crate::card_rows::{CardRow, InvoiceRow, InvoiceTotalsRow, PurchaseRow};
use crate::drafts::commit_draft;
use crate::entries::insert_entry;
use crate::error_mapping::store_error;

#[async_trait]
impl CardStore for PgStore {
    async fn create_card(&self, card: NewCard) -> StoreResult<CreditCard> {
        let row = sqlx::query_as!(
            CardRow,
            "insert into credit_cards (name, closing_day, due_day, closing_day_goes_next, limit_cents, default_payment_account_id)
             values ($1, $2, $3, $4, $5, $6)
             returning id, name, closing_day, due_day, closing_day_goes_next, limit_cents, default_payment_account_id, archived_at",
            card.name,
            i16::from(card.schedule.closing_day),
            i16::from(card.schedule.due_day),
            card.schedule.closing_day_goes_next,
            card.limit.map(domain::Cents::value),
            card.default_payment_account_id.map(|id| id.0),
        )
        .fetch_one(self.pool())
        .await
        .map_err(store_error)?;
        row.into_card()
    }

    async fn list_cards(&self, include_archived: bool) -> StoreResult<Vec<CreditCard>> {
        let rows = sqlx::query_as!(
            CardRow,
            "select id, name, closing_day, due_day, closing_day_goes_next, limit_cents, default_payment_account_id, archived_at
             from credit_cards where $1 or archived_at is null order by created_at, id",
            include_archived,
        )
        .fetch_all(self.pool())
        .await
        .map_err(store_error)?;
        rows.into_iter().map(CardRow::into_card).collect()
    }

    async fn find_card(&self, id: CardId) -> StoreResult<Option<CreditCard>> {
        let row = sqlx::query_as!(
            CardRow,
            "select id, name, closing_day, due_day, closing_day_goes_next, limit_cents, default_payment_account_id, archived_at
             from credit_cards where id = $1",
            id.0,
        )
        .fetch_optional(self.pool())
        .await
        .map_err(store_error)?;
        row.map(CardRow::into_card).transpose()
    }

    async fn archive_card(&self, id: CardId, at: DateTime<Utc>) -> StoreResult<bool> {
        let result = sqlx::query!(
            "update credit_cards set archived_at = $2 where id = $1 and archived_at is null",
            id.0,
            at
        )
        .execute(self.pool())
        .await
        .map_err(store_error)?;
        Ok(result.rows_affected() == 1)
    }

    async fn ensure_invoice(&self, card: CardId, period: InvoicePeriod) -> StoreResult<Invoice> {
        let mut connection = self.pool().acquire().await.map_err(store_error)?;
        ensure_invoice_on(&mut connection, card, period).await
    }

    async fn find_invoice(&self, id: InvoiceId) -> StoreResult<Option<Invoice>> {
        let row = sqlx::query_as!(
            InvoiceRow,
            "select id, card_id, reference_month, closing_date, due_date from card_invoices where id = $1",
            id.0,
        )
        .fetch_optional(self.pool())
        .await
        .map_err(store_error)?;
        Ok(row.map(InvoiceRow::into_invoice))
    }

    async fn invoice_totals(&self, card: CardId) -> StoreResult<Vec<(Invoice, InvoiceTotals)>> {
        let rows = sqlx::query_file_as!(InvoiceTotalsRow, "queries/invoice_totals.sql", card.0)
            .fetch_all(self.pool())
            .await
            .map_err(store_error)?;
        Ok(rows.into_iter().map(InvoiceTotalsRow::into_pair).collect())
    }

    async fn record_purchase(
        &self,
        purchase: NewCardPurchase,
        slots: &[InstallmentSlot],
        draft: Option<DraftId>,
    ) -> StoreResult<CardPurchase> {
        let mut transaction = self.pool().begin().await.map_err(store_error)?;
        if let Some(draft_id) = draft {
            commit_draft(&mut transaction, draft_id).await?;
        }
        let row = insert_purchase(&mut transaction, &purchase).await?;
        for slot in slots {
            insert_installment(&mut transaction, &row, slot).await?;
        }
        transaction.commit().await.map_err(store_error)?;
        row.into_purchase()
    }

    async fn find_purchase(&self, id: PurchaseId) -> StoreResult<Option<CardPurchase>> {
        let row = sqlx::query_as!(
            PurchaseRow,
            "select id, card_id, description, category_id, total_cents, installment_count, first_installment_no,
                    purchased_on, created_by, deleted_at
             from card_purchases where id = $1",
            id.0,
        )
        .fetch_optional(self.pool())
        .await
        .map_err(store_error)?;
        row.map(PurchaseRow::into_purchase).transpose()
    }

    async fn delete_purchase(&self, id: PurchaseId, at: DateTime<Utc>) -> StoreResult<bool> {
        let mut transaction = self.pool().begin().await.map_err(store_error)?;
        let purchase = sqlx::query!(
            "update card_purchases set deleted_at = $2 where id = $1 and deleted_at is null",
            id.0,
            at
        )
        .execute(&mut *transaction)
        .await
        .map_err(store_error)?;
        sqlx::query!("update ledger_entries set deleted_at = $2 where card_purchase_id = $1 and deleted_at is null", id.0, at)
            .execute(&mut *transaction)
            .await
            .map_err(store_error)?;
        transaction.commit().await.map_err(store_error)?;
        Ok(purchase.rows_affected() == 1)
    }

    async fn record_invoice_entry(
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
}

async fn ensure_invoice_on(
    connection: &mut PgConnection,
    card: CardId,
    period: InvoicePeriod,
) -> StoreResult<Invoice> {
    let row = sqlx::query_file_as!(
        InvoiceRow,
        "queries/ensure_invoice.sql",
        card.0,
        period.reference_month.first_day(),
        period.closing_date,
        period.due_date,
    )
    .fetch_one(connection)
    .await
    .map_err(store_error)?;
    Ok(row.into_invoice())
}

async fn insert_purchase(
    connection: &mut PgConnection,
    purchase: &NewCardPurchase,
) -> StoreResult<PurchaseRow> {
    let count = |value: u32| i16::try_from(value).unwrap_or(i16::MAX);
    sqlx::query_file_as!(
        PurchaseRow,
        "queries/insert_purchase.sql",
        purchase.card_id.0,
        purchase.description,
        purchase.category_id.0,
        purchase.total.value(),
        count(purchase.installment_count),
        count(purchase.first_installment_no),
        purchase.purchased_on,
        purchase.created_by.map(|id| id.0),
    )
    .fetch_one(connection)
    .await
    .map_err(store_error)
}

async fn insert_installment(
    connection: &mut PgConnection,
    purchase: &PurchaseRow,
    slot: &InstallmentSlot,
) -> StoreResult<()> {
    let invoice = ensure_invoice_on(connection, CardId(purchase.card_id), slot.invoice).await?;
    sqlx::query_file!(
        "queries/insert_installment.sql",
        slot.amount.value(),
        purchase.description,
        purchase.category_id,
        purchase.id,
        i16::try_from(slot.number).unwrap_or(i16::MAX),
        invoice.id.0,
        slot.accounting_date,
        purchase.created_by,
    )
    .execute(connection)
    .await
    .map_err(store_error)?;
    Ok(())
}
