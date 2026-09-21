use app::model::{AccountId, Goal, GoalId, GoalProgress, GoalTarget};
use app::services::{CreateGoal, PotMove};
use chrono::NaiveDate;
use domain::Cents;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
pub struct GoalResponse {
    pub id: Uuid,
    #[schema(example = "Casa própria")]
    pub name: String,
    pub pot_account_id: Uuid,
    #[schema(example = 10_000_000)]
    pub target_cents: i64,
    pub target_date: Option<NaiveDate>,
}

impl From<Goal> for GoalResponse {
    fn from(goal: Goal) -> Self {
        Self {
            id: goal.id.0,
            name: goal.pot.name,
            pot_account_id: goal.pot.id.0,
            target_cents: goal.target.target.value(),
            target_date: goal.target.target_date,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct GoalProgressResponse {
    pub goal: GoalResponse,
    #[schema(example = 2_000_000)]
    pub saved_cents: i64,
    pub remaining_cents: i64,
    /// Basis points: 2000 = 20%.
    #[schema(example = 2000)]
    pub progress_bp: i64,
}

impl From<GoalProgress> for GoalProgressResponse {
    fn from(progress: GoalProgress) -> Self {
        Self {
            goal: progress.goal.into(),
            saved_cents: progress.saved.value(),
            remaining_cents: progress.remaining.value(),
            progress_bp: progress.progress_bp,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateGoalBody {
    #[schema(example = "Casa própria")]
    pub name: String,
    #[schema(example = 10_000_000)]
    pub target_cents: i64,
    pub target_date: Option<NaiveDate>,
    /// Saved before using finbot; becomes the pot's opening balance.
    #[serde(default)]
    #[schema(example = 2_000_000)]
    pub already_saved_cents: i64,
}

impl From<CreateGoalBody> for CreateGoal {
    fn from(body: CreateGoalBody) -> Self {
        CreateGoal {
            name: body.name,
            target: Cents::new(body.target_cents),
            target_date: body.target_date,
            already_saved: Cents::new(body.already_saved_cents),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct GoalTargetBody {
    pub target_cents: i64,
    pub target_date: Option<NaiveDate>,
}

impl From<GoalTargetBody> for GoalTarget {
    fn from(body: GoalTargetBody) -> Self {
        GoalTarget { target: Cents::new(body.target_cents), target_date: body.target_date }
    }
}

/// Money moved between an account and the goal's pot.
#[derive(Debug, Deserialize, ToSchema)]
pub struct PotMoveBody {
    pub account_id: Uuid,
    pub amount_cents: i64,
    #[serde(default)]
    pub description: String,
}

impl PotMoveBody {
    pub fn into_pot_move(self, goal_id: Uuid) -> PotMove {
        PotMove {
            goal_id: GoalId(goal_id),
            account_id: AccountId(self.account_id),
            amount: Cents::new(self.amount_cents),
            description: self.description,
        }
    }
}
