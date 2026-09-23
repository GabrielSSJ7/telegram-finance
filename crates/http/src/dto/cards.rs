use app::AppError;
use app::model::{
    AccountId, CardEdit, CardId, CardPurchase, CardSummary, CategoryId, CreditCard, InvoiceId,
    InvoiceView,
};
use app::services::{CardCreditRequest, CardPurchaseRequest, InvoicePaymentRequest, OpenCard};
use chrono::NaiveDate;
use domain::invoice_settlement::InvoiceStatus;
use domain::{Cents, DayOfMonth};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
pub struct CardResponse {
    pub id: Uuid,
    #[schema(example = "Nubank")]
    pub name: String,
    #[schema(example = 3)]
    pub closing_day: u8,
    #[schema(example = 10)]
    pub due_day: u8,
    pub closing_day_goes_next: bool,
    pub limit_cents: Option<i64>,
    pub default_payment_account_id: Option<Uuid>,
    pub archived: bool,
}

impl From<CreditCard> for CardResponse {
    fn from(card: CreditCard) -> Self {
        Self {
            id: card.id.0,
            name: card.name,
            closing_day: card.schedule.closing_day.get(),
            due_day: card.schedule.due_day.get(),
            closing_day_goes_next: card.schedule.closing_day_goes_next,
            limit_cents: card.limit.map(Cents::value),
            default_payment_account_id: card.default_payment_account_id.map(|id| id.0),
            archived: card.archived,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct OpenCardBody {
    #[schema(example = "Nubank")]
    pub name: String,
    #[schema(example = 3)]
    pub closing_day: u8,
    #[schema(example = 10)]
    pub due_day: u8,
    /// Purchases on the closing day go to the next invoice. Default true.
    pub closing_day_goes_next: Option<bool>,
    pub limit_cents: Option<i64>,
    pub default_payment_account_id: Option<Uuid>,
}

impl TryFrom<OpenCardBody> for OpenCard {
    type Error = AppError;
    fn try_from(body: OpenCardBody) -> Result<Self, Self::Error> {
        let day = |field: &'static str, value: u8| {
            DayOfMonth::new(value).map_err(|_| AppError::invalid(field, value, "1 to 31"))
        };
        Ok(OpenCard {
            name: body.name,
            closing_day: day("closing_day", body.closing_day)?,
            due_day: day("due_day", body.due_day)?,
            closing_day_goes_next: body.closing_day_goes_next.unwrap_or(true),
            limit: body.limit_cents.map(Cents::new),
            default_payment_account_id: body.default_payment_account_id.map(AccountId),
        })
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct InvoiceResponse {
    pub id: Uuid,
    pub card_id: Uuid,
    /// Month of the due date, `YYYY-MM`.
    #[schema(example = "2026-04")]
    pub reference_month: String,
    pub closing_date: NaiveDate,
    pub due_date: NaiveDate,
    #[schema(example = "open")]
    pub status: String,
    pub charges_cents: i64,
    pub credits_cents: i64,
    pub payments_cents: i64,
    pub carried_in_cents: i64,
    pub outstanding_cents: i64,
}

impl From<InvoiceView> for InvoiceResponse {
    fn from(view: InvoiceView) -> Self {
        let (invoice, statement) = (view.invoice, view.statement);
        let month = invoice.period.reference_month;
        Self {
            id: invoice.id.0,
            card_id: invoice.card_id.0,
            reference_month: format!("{}-{:02}", month.year(), month.month()),
            closing_date: invoice.period.closing_date,
            due_date: invoice.period.due_date,
            status: status_name(statement.status).into(),
            charges_cents: statement.totals.charges.value(),
            credits_cents: statement.totals.credits.value(),
            payments_cents: statement.totals.payments.value(),
            carried_in_cents: statement.carried_in.value(),
            outstanding_cents: statement.outstanding.value(),
        }
    }
}

const fn status_name(status: InvoiceStatus) -> &'static str {
    match status {
        InvoiceStatus::Open => "open",
        InvoiceStatus::Closed => "closed",
        InvoiceStatus::Paid => "paid",
        InvoiceStatus::Rolled => "rolled",
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CardSummaryResponse {
    pub card: CardResponse,
    pub current_invoice: Option<InvoiceResponse>,
    /// Newest closed invoice with money still owed.
    pub unpaid_invoice: Option<InvoiceResponse>,
    pub future_committed_cents: i64,
}

impl From<CardSummary> for CardSummaryResponse {
    fn from(summary: CardSummary) -> Self {
        Self {
            card: summary.card.into(),
            current_invoice: summary.current.map(Into::into),
            unpaid_invoice: summary.unpaid.map(Into::into),
            future_committed_cents: summary.future_committed.value(),
        }
    }
}

/// Fields left out keep their value.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateCardBody {
    #[schema(example = "Itaú Black")]
    pub name: Option<String>,
    #[schema(example = 5)]
    pub closing_day: Option<u8>,
    #[schema(example = 15)]
    pub due_day: Option<u8>,
}

impl TryFrom<UpdateCardBody> for CardEdit {
    type Error = AppError;
    fn try_from(body: UpdateCardBody) -> Result<Self, Self::Error> {
        let day = |value: Option<u8>, field: &'static str| {
            value
                .map(|day| {
                    DayOfMonth::new(day).map_err(|_| AppError::invalid(field, day, "1 to 31"))
                })
                .transpose()
        };
        Ok(CardEdit {
            name: body.name,
            closing_day: day(body.closing_day, "closing_day")?,
            due_day: day(body.due_day, "due_day")?,
        })
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PurchaseBody {
    pub category_id: Uuid,
    #[schema(example = 30_000)]
    pub total_cents: i64,
    /// 1 to 48. Default 1.
    pub installments: Option<u32>,
    /// Imports a purchase already in progress. Default 1.
    pub first_installment_no: Option<u32>,
    #[serde(default)]
    pub description: String,
    pub purchased_on: Option<NaiveDate>,
}

impl PurchaseBody {
    pub fn into_request(self, card: Uuid) -> CardPurchaseRequest {
        CardPurchaseRequest {
            card_id: CardId(card),
            category_id: CategoryId(self.category_id),
            total: Cents::new(self.total_cents),
            installments: self.installments.unwrap_or(1),
            first_installment_no: self.first_installment_no.unwrap_or(1),
            description: self.description,
            purchased_on: self.purchased_on,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PurchaseResponse {
    pub id: Uuid,
    pub card_id: Uuid,
    pub category_id: Uuid,
    pub total_cents: i64,
    pub installment_count: u32,
    pub first_installment_no: u32,
    pub description: String,
    pub purchased_on: NaiveDate,
}

impl From<CardPurchase> for PurchaseResponse {
    fn from(purchase: CardPurchase) -> Self {
        Self {
            id: purchase.id.0,
            card_id: purchase.card_id.0,
            category_id: purchase.category_id.0,
            total_cents: purchase.total.value(),
            installment_count: purchase.installment_count,
            first_installment_no: purchase.first_installment_no,
            description: purchase.description,
            purchased_on: purchase.purchased_on,
        }
    }
}

/// A refund ("estorno") credited to the invoice open on `date`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreditBody {
    pub category_id: Uuid,
    pub amount_cents: i64,
    #[serde(default)]
    pub description: String,
    pub date: Option<NaiveDate>,
}

impl CreditBody {
    pub fn into_request(self, card: Uuid) -> CardCreditRequest {
        CardCreditRequest {
            card_id: CardId(card),
            category_id: CategoryId(self.category_id),
            amount: Cents::new(self.amount_cents),
            description: self.description,
            date: self.date,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PaymentBody {
    pub account_id: Uuid,
    pub amount_cents: i64,
    pub date: Option<NaiveDate>,
}

impl PaymentBody {
    pub fn into_request(self, invoice: Uuid) -> InvoicePaymentRequest {
        InvoicePaymentRequest {
            invoice_id: InvoiceId(invoice),
            account_id: AccountId(self.account_id),
            amount: Cents::new(self.amount_cents),
            date: self.date,
        }
    }
}
