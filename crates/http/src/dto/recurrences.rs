use app::AppError;
use app::model::{
    AccountId, CardId, CategoryId, Recurrence, RecurrenceKind, RecurrenceMode, RecurrenceTarget,
};
use app::services::CreateRecurrence;
use chrono::NaiveDate;
use domain::recurrence::InstallmentPlan;
use domain::{Cents, DayOfMonth};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
pub struct RecurrenceResponse {
    pub id: Uuid,
    #[schema(value_type = String, example = "expense")]
    pub kind: RecurrenceKind,
    #[schema(example = 250_000)]
    pub amount_cents: i64,
    #[schema(example = "Aluguel")]
    pub description: String,
    pub category_id: Uuid,
    pub account_id: Option<Uuid>,
    pub card_id: Option<Uuid>,
    #[schema(example = 5)]
    pub day_of_month: u8,
    #[schema(value_type = String, example = "auto")]
    pub mode: RecurrenceMode,
    pub active: bool,
    pub starts_on: NaiveDate,
    pub last_generated_on: Option<NaiveDate>,
    /// Total installments of a financing; absent when it never ends.
    pub installment_count: Option<u32>,
    /// Installment paid on the first due date.
    pub first_installment_no: Option<u32>,
}

impl From<Recurrence> for RecurrenceResponse {
    fn from(recurrence: Recurrence) -> Self {
        let (account_id, card_id) = match recurrence.target {
            RecurrenceTarget::Account(id) => (Some(id.0), None),
            RecurrenceTarget::Card(id) => (None, Some(id.0)),
        };
        Self {
            id: recurrence.id.0,
            kind: recurrence.kind,
            amount_cents: recurrence.amount.value(),
            description: recurrence.description,
            category_id: recurrence.category_id.0,
            account_id,
            card_id,
            day_of_month: recurrence.day.get(),
            mode: recurrence.mode,
            active: recurrence.active,
            starts_on: recurrence.starts_on,
            last_generated_on: recurrence.last_generated_on,
            installment_count: recurrence.plan.map(|plan| plan.count),
            first_installment_no: recurrence.plan.map(|plan| plan.first_number),
        }
    }
}

/// Exactly one of `account_id` and `card_id` (cards only for expenses).
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateRecurrenceBody {
    #[schema(value_type = String, example = "expense")]
    pub kind: RecurrenceKind,
    pub amount_cents: i64,
    #[schema(example = "Aluguel")]
    pub description: String,
    pub category_id: Uuid,
    pub account_id: Option<Uuid>,
    pub card_id: Option<Uuid>,
    #[schema(example = 5)]
    pub day_of_month: u8,
    /// `auto` (default) or `confirm`.
    #[schema(value_type = Option<String>)]
    pub mode: Option<RecurrenceMode>,
    pub starts_on: Option<NaiveDate>,
    /// Makes it a financing that ends after this many installments.
    pub installment_count: Option<u32>,
    /// Installments already paid before this one; defaults to 0.
    #[serde(default)]
    pub installments_paid: u32,
}

impl TryFrom<CreateRecurrenceBody> for CreateRecurrence {
    type Error = AppError;
    fn try_from(body: CreateRecurrenceBody) -> Result<Self, Self::Error> {
        let target = target(body.account_id, body.card_id)?;
        let day = DayOfMonth::new(body.day_of_month)
            .map_err(|_| AppError::invalid("day_of_month", body.day_of_month, "1 to 31"))?;
        Ok(CreateRecurrence {
            kind: body.kind,
            amount: Cents::new(body.amount_cents),
            description: body.description,
            category_id: CategoryId(body.category_id),
            target,
            day,
            mode: body.mode.unwrap_or(RecurrenceMode::Auto),
            starts_on: body.starts_on,
            plan: body.installment_count.map(|count| InstallmentPlan {
                first_number: body.installments_paid.saturating_add(1),
                count,
            }),
        })
    }
}

fn target(account: Option<Uuid>, card: Option<Uuid>) -> Result<RecurrenceTarget, AppError> {
    match (account, card) {
        (Some(account), None) => Ok(RecurrenceTarget::Account(AccountId(account))),
        (None, Some(card)) => Ok(RecurrenceTarget::Card(CardId(card))),
        _ => Err(AppError::invalid("account_id/card_id", "both or neither", "exactly one of them")),
    }
}
