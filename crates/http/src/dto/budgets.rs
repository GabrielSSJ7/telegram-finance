use app::model::BudgetStatus;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
pub struct BudgetResponse {
    pub id: Uuid,
    pub category_id: Uuid,
    #[schema(example = "mercado")]
    pub category_name: String,
    #[schema(example = 150_000)]
    pub limit_cents: i64,
    /// Spent in the current cycle.
    pub spent_cents: i64,
    /// Basis points: 8500 = 85%.
    pub used_bp: i64,
}

impl From<BudgetStatus> for BudgetResponse {
    fn from(status: BudgetStatus) -> Self {
        Self {
            id: status.budget.id.0,
            category_id: status.budget.category_id.0,
            category_name: status.category.name,
            limit_cents: status.budget.limit.value(),
            spent_cents: status.spent.value(),
            used_bp: status.used_bp,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct BudgetBody {
    #[schema(example = 150_000)]
    pub limit_cents: i64,
}
