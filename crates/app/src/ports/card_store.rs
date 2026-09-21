use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::installments::InstallmentSlot;
use domain::invoice_cycle::InvoicePeriod;
use domain::invoice_settlement::InvoiceTotals;

use super::StoreResult;
use crate::model::{
    CardId, CardPurchase, CreditCard, DraftId, Invoice, InvoiceId, LedgerEntry, NewCard,
    NewCardPurchase, NewEntry, PurchaseId,
};

#[async_trait]
pub trait CardStore: Send + Sync {
    async fn create_card(&self, card: NewCard) -> StoreResult<CreditCard>;
    async fn list_cards(&self, include_archived: bool) -> StoreResult<Vec<CreditCard>>;
    async fn find_card(&self, id: CardId) -> StoreResult<Option<CreditCard>>;
    async fn archive_card(&self, id: CardId, at: DateTime<Utc>) -> StoreResult<bool>;

    /// The invoice of `card` for `period.reference_month`, created with
    /// `period`'s dates if missing (stored dates win when it exists).
    async fn ensure_invoice(&self, card: CardId, period: InvoicePeriod) -> StoreResult<Invoice>;
    async fn find_invoice(&self, id: InvoiceId) -> StoreResult<Option<Invoice>>;
    /// Invoices of `card`, oldest closing date first, with live totals.
    async fn invoice_totals(&self, card: CardId) -> StoreResult<Vec<(Invoice, InvoiceTotals)>>;

    /// Saves the purchase and one installment row per slot (creating
    /// invoices as needed) and commits `draft`, all in one transaction.
    async fn record_purchase(
        &self,
        purchase: NewCardPurchase,
        slots: &[InstallmentSlot],
        draft: Option<DraftId>,
    ) -> StoreResult<CardPurchase>;
    async fn find_purchase(&self, id: PurchaseId) -> StoreResult<Option<CardPurchase>>;
    /// Soft-deletes the purchase and all its installments.
    async fn delete_purchase(&self, id: PurchaseId, at: DateTime<Utc>) -> StoreResult<bool>;

    /// Saves a card credit or invoice payment (`entry.invoice_id` set) and
    /// commits `draft` in one transaction.
    async fn record_invoice_entry(
        &self,
        entry: NewEntry,
        draft: Option<DraftId>,
    ) -> StoreResult<LedgerEntry>;
}
