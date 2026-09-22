use app::model::{InstallmentProgress, PlanSource};
use chrono::NaiveDate;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
pub struct InstallmentPlanResponse {
    #[schema(example = "Enoxaparina")]
    pub description: String,
    /// `card` or `account`.
    #[schema(example = "card")]
    pub source: &'static str,
    /// Card or account name.
    pub source_name: String,
    pub category_id: Uuid,
    pub count: u32,
    pub paid_count: u32,
    pub total_cents: i64,
    pub paid_cents: i64,
    pub remaining_cents: i64,
    pub installment_cents: i64,
    /// Basis points: 2500 = 25% paid.
    pub paid_bp: i64,
    pub last_due: NaiveDate,
}

impl From<InstallmentProgress> for InstallmentPlanResponse {
    fn from(plan: InstallmentProgress) -> Self {
        let (source, source_name) = match &plan.source {
            PlanSource::Card(name) => ("card", name.clone()),
            PlanSource::Account(name) => ("account", name.clone()),
        };
        Self {
            source,
            source_name,
            category_id: plan.category_id.0,
            count: plan.count,
            paid_count: plan.paid_count,
            total_cents: plan.total.value(),
            paid_cents: plan.paid.value(),
            remaining_cents: plan.remaining().value(),
            installment_cents: plan.installment.value(),
            paid_bp: plan.paid_bp(),
            last_due: plan.last_due,
            description: plan.description,
        }
    }
}
