//! Money moving through cards: purchases (with installments), refunds
//! credited to an invoice, and invoice payments from an account.

use chrono::NaiveDate;
use domain::installments::{InstallmentPlan, schedule_installments};
use domain::{Cents, EntryKind};

use super::CardService;
use super::ledger::EntryOrigin;
use super::ledger_validation::ensure_positive_amount;
use super::text_rules::clean_description;
use crate::model::{
    AccountId, CardId, CardPurchase, CategoryId, CategoryKind, InvoiceId, LedgerEntry,
    NewCardPurchase, NewEntry, PurchaseId,
};
use crate::{AppError, AppResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardPurchaseRequest {
    pub card_id: CardId,
    pub category_id: CategoryId,
    pub total: Cents,
    pub installments: u32,
    /// Above 1 imports a purchase already in progress.
    pub first_installment_no: u32,
    pub description: String,
    pub purchased_on: Option<NaiveDate>,
}

/// A refund ("estorno") credited to the invoice open on `date`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardCreditRequest {
    pub card_id: CardId,
    pub category_id: CategoryId,
    pub amount: Cents,
    pub description: String,
    pub date: Option<NaiveDate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvoicePaymentRequest {
    pub invoice_id: InvoiceId,
    pub account_id: AccountId,
    pub amount: Cents,
    pub date: Option<NaiveDate>,
}

impl CardService {
    /// Records a purchase split into installments across invoices.
    pub async fn purchase(
        &self,
        request: CardPurchaseRequest,
        origin: EntryOrigin,
    ) -> AppResult<CardPurchase> {
        let card = self.require_active(request.card_id).await?;
        self.categories.require_kind(request.category_id, CategoryKind::Expense).await?;
        let purchased_on = request.purchased_on.unwrap_or_else(|| self.clock.today());
        let plan = InstallmentPlan {
            total: request.total,
            count: request.installments,
            first_number: request.first_installment_no,
            purchase_date: purchased_on,
        };
        let slots = schedule_installments(plan, card.schedule).map_err(|error| {
            AppError::invalid("installments", request.installments, error.to_string())
        })?;
        let purchase =
            new_purchase(&request, purchased_on, clean_description(&request.description)?, origin);
        Ok(self.cards.record_purchase(purchase, &slots, origin.draft).await?)
    }

    pub async fn credit(
        &self,
        request: CardCreditRequest,
        origin: EntryOrigin,
    ) -> AppResult<LedgerEntry> {
        ensure_positive_amount(request.amount)?;
        let card = self.require_active(request.card_id).await?;
        self.categories.require_kind(request.category_id, CategoryKind::Expense).await?;
        let date = request.date.unwrap_or_else(|| self.clock.today());
        let invoice =
            self.cards.ensure_invoice(card.id, card.schedule.period_for_purchase(date)).await?;
        let entry = invoice_entry(EntryKind::CardCredit, request.amount, invoice.id, date, origin);
        let description = clean_description(&request.description)?;
        let entry = NewEntry { category_id: Some(request.category_id), description, ..entry };
        Ok(self.cards.record_invoice_entry(entry, origin.draft).await?)
    }

    /// Pays (part of) an invoice from a spendable account. Paying more
    /// than owed is allowed: the rest becomes a credit on the next invoice.
    pub async fn pay_invoice(
        &self,
        request: InvoicePaymentRequest,
        origin: EntryOrigin,
    ) -> AppResult<LedgerEntry> {
        ensure_positive_amount(request.amount)?;
        let invoice = self.cards.find_invoice(request.invoice_id).await?;
        let invoice = invoice.ok_or_else(|| AppError::not_found("invoice", request.invoice_id))?;
        let account = self.accounts.require_active(request.account_id).await?;
        if !account.kind.is_spendable() {
            return Err(AppError::invalid(
                "payment account",
                &account.name,
                "a checking, savings or cash account",
            ));
        }
        let date = request.date.unwrap_or_else(|| self.clock.today());
        let entry =
            invoice_entry(EntryKind::InvoicePayment, request.amount, invoice.id, date, origin);
        let entry = NewEntry { account_id: Some(account.id), ..entry };
        Ok(self.cards.record_invoice_entry(entry, origin.draft).await?)
    }

    /// A purchase that has not been deleted.
    pub async fn find_purchase(&self, id: PurchaseId) -> AppResult<CardPurchase> {
        let purchase = self.cards.find_purchase(id).await?.filter(|purchase| !purchase.deleted);
        purchase.ok_or_else(|| AppError::not_found("card purchase", id))
    }

    /// Removes a purchase and every installment of it.
    pub async fn delete_purchase(&self, id: PurchaseId) -> AppResult<CardPurchase> {
        let purchase = self.find_purchase(id).await?;
        if !self.cards.delete_purchase(id, self.clock.now()).await? {
            return Err(AppError::not_found("card purchase", id));
        }
        Ok(purchase)
    }
}

/// A ledger row tied to an invoice; callers fill account or category.
fn invoice_entry(
    kind: EntryKind,
    amount: Cents,
    invoice: InvoiceId,
    date: NaiveDate,
    origin: EntryOrigin,
) -> NewEntry {
    NewEntry {
        kind,
        amount,
        description: String::new(),
        category_id: None,
        account_id: None,
        counter_account_id: None,
        invoice_id: Some(invoice),
        accounting_date: date,
        created_by: origin.created_by,
    }
}

fn new_purchase(
    request: &CardPurchaseRequest,
    purchased_on: NaiveDate,
    description: String,
    origin: EntryOrigin,
) -> NewCardPurchase {
    NewCardPurchase {
        card_id: request.card_id,
        description,
        category_id: request.category_id,
        total: request.total,
        installment_count: request.installments,
        first_installment_no: request.first_installment_no,
        purchased_on,
        created_by: origin.created_by,
    }
}
