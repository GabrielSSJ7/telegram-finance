//! Row types of the card tables and their conversion to the app model.

use app::model::{
    AccountId, CardId, CardPurchase, CategoryId, CreditCard, Invoice, InvoiceId, MemberId,
    PurchaseId,
};
use app::ports::StoreResult;
use chrono::{DateTime, NaiveDate, Utc};
use domain::invoice_cycle::{CardSchedule, InvoicePeriod};
use domain::invoice_settlement::InvoiceTotals;
use domain::{Cents, DayOfMonth, YearMonth};
use uuid::Uuid;

use crate::error_mapping::corrupt;

pub(crate) struct CardRow {
    pub id: Uuid,
    pub name: String,
    pub closing_day: i16,
    pub due_day: i16,
    pub closing_day_goes_next: bool,
    pub limit_cents: Option<i64>,
    pub default_payment_account_id: Option<Uuid>,
    pub archived_at: Option<DateTime<Utc>>,
}

impl CardRow {
    pub(crate) fn into_card(self) -> StoreResult<CreditCard> {
        let day = |value: i16, column: &str| {
            DayOfMonth::try_from(value).map_err(|error| corrupt(column, error))
        };
        let schedule = CardSchedule {
            closing_day: day(self.closing_day, "credit_cards.closing_day")?,
            due_day: day(self.due_day, "credit_cards.due_day")?,
            closing_day_goes_next: self.closing_day_goes_next,
        };
        Ok(CreditCard {
            id: CardId(self.id),
            name: self.name,
            schedule,
            limit: self.limit_cents.map(Cents::new),
            default_payment_account_id: self.default_payment_account_id.map(AccountId),
            archived: self.archived_at.is_some(),
        })
    }
}

pub(crate) struct InvoiceRow {
    pub id: Uuid,
    pub card_id: Uuid,
    pub reference_month: NaiveDate,
    pub closing_date: NaiveDate,
    pub due_date: NaiveDate,
}

impl InvoiceRow {
    pub(crate) fn into_invoice(self) -> Invoice {
        let period = InvoicePeriod {
            reference_month: YearMonth::of(self.reference_month),
            closing_date: self.closing_date,
            due_date: self.due_date,
        };
        Invoice { id: InvoiceId(self.id), card_id: CardId(self.card_id), period }
    }
}

pub(crate) struct InvoiceTotalsRow {
    pub id: Uuid,
    pub card_id: Uuid,
    pub reference_month: NaiveDate,
    pub closing_date: NaiveDate,
    pub due_date: NaiveDate,
    pub charges: i64,
    pub credits: i64,
    pub payments: i64,
}

impl InvoiceTotalsRow {
    pub(crate) fn into_pair(self) -> (Invoice, InvoiceTotals) {
        let totals = InvoiceTotals {
            charges: Cents::new(self.charges),
            credits: Cents::new(self.credits),
            payments: Cents::new(self.payments),
        };
        let row = InvoiceRow {
            id: self.id,
            card_id: self.card_id,
            reference_month: self.reference_month,
            closing_date: self.closing_date,
            due_date: self.due_date,
        };
        (row.into_invoice(), totals)
    }
}

pub(crate) struct PurchaseRow {
    pub id: Uuid,
    pub card_id: Uuid,
    pub description: String,
    pub category_id: Option<Uuid>,
    pub total_cents: i64,
    pub installment_count: i16,
    pub first_installment_no: i16,
    pub purchased_on: NaiveDate,
    pub created_by: Option<Uuid>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl PurchaseRow {
    pub(crate) fn into_purchase(self) -> StoreResult<CardPurchase> {
        let category =
            self.category_id.ok_or_else(|| corrupt("card_purchases.category_id", "null"))?;
        let count =
            |value: i16, column: &str| u32::try_from(value).map_err(|error| corrupt(column, error));
        Ok(CardPurchase {
            id: PurchaseId(self.id),
            card_id: CardId(self.card_id),
            description: self.description,
            category_id: CategoryId(category),
            total: Cents::new(self.total_cents),
            installment_count: count(self.installment_count, "card_purchases.installment_count")?,
            first_installment_no: count(
                self.first_installment_no,
                "card_purchases.first_installment_no",
            )?,
            purchased_on: self.purchased_on,
            created_by: self.created_by.map(MemberId),
            deleted: self.deleted_at.is_some(),
        })
    }
}
