use chrono::NaiveDate;
use domain::Cents;
use domain::invoice_cycle::{CardSchedule, InvoicePeriod};
use domain::invoice_settlement::InvoiceStatement;
use serde::{Deserialize, Serialize};

use super::{AccountId, CardId, CategoryId, InvoiceId, MemberId, PurchaseId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreditCard {
    pub id: CardId,
    pub name: String,
    pub schedule: CardSchedule,
    pub limit: Option<Cents>,
    /// Suggested account when paying this card's invoices.
    pub default_payment_account_id: Option<AccountId>,
    pub archived: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewCard {
    pub name: String,
    pub schedule: CardSchedule,
    pub limit: Option<Cents>,
    pub default_payment_account_id: Option<AccountId>,
}

/// An invoice row: its stored dates may differ from the computed ones
/// when a bank moves them around a holiday.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Invoice {
    pub id: InvoiceId,
    pub card_id: CardId,
    pub period: InvoicePeriod,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardPurchase {
    pub id: PurchaseId,
    pub card_id: CardId,
    pub description: String,
    pub category_id: CategoryId,
    pub total: Cents,
    pub installment_count: u32,
    pub first_installment_no: u32,
    pub purchased_on: NaiveDate,
    pub created_by: Option<MemberId>,
    pub deleted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewCardPurchase {
    pub card_id: CardId,
    pub description: String,
    pub category_id: CategoryId,
    pub total: Cents,
    pub installment_count: u32,
    pub first_installment_no: u32,
    pub purchased_on: NaiveDate,
    pub created_by: Option<MemberId>,
}

/// An invoice with what is owed on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceView {
    pub invoice: Invoice,
    pub statement: InvoiceStatement,
}

/// What `/fatura` and the daily report show per card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardSummary {
    pub card: CreditCard,
    /// The invoice collecting today's purchases.
    pub current: Option<InvoiceView>,
    /// The newest closed invoice, when it still has money owed.
    pub unpaid: Option<InvoiceView>,
    /// Installments already on invoices after the current one.
    pub future_committed: Cents,
}
