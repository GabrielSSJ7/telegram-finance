use app::model::CategoryId;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use domain::Cents;
use uuid::Uuid;

use crate::dto::budgets::{BudgetBody, BudgetResponse};
use crate::error::{ApiError, Problem};
use crate::extract::{ApiJson, ApiPath};
use crate::state::ApiState;

#[utoipa::path(get, path = "/budgets", tag = "budgets",
    responses((status = 200, description = "Budgets against the current cycle", body = Vec<BudgetResponse>)),
    security(("api_key" = [])))]
pub async fn list_budgets(
    State(state): State<ApiState>,
) -> Result<Json<Vec<BudgetResponse>>, ApiError> {
    let budgets = &state.services.budgets;
    let statuses = budgets.statuses(budgets.current_cycle().await?).await?;
    Ok(Json(statuses.into_iter().map(Into::into).collect()))
}

#[utoipa::path(put, path = "/budgets/{category_id}", tag = "budgets", params(("category_id" = Uuid, Path)),
    request_body = BudgetBody,
    responses((status = 204), (status = 404, body = Problem), (status = 422, body = Problem)), security(("api_key" = [])))]
pub async fn set_budget(
    State(state): State<ApiState>,
    ApiPath(category): ApiPath<Uuid>,
    ApiJson(body): ApiJson<BudgetBody>,
) -> Result<StatusCode, ApiError> {
    state.services.budgets.set(CategoryId(category), Cents::new(body.limit_cents)).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(delete, path = "/budgets/{category_id}", tag = "budgets", params(("category_id" = Uuid, Path)),
    responses((status = 204), (status = 404, body = Problem)), security(("api_key" = [])))]
pub async fn remove_budget(
    State(state): State<ApiState>,
    ApiPath(category): ApiPath<Uuid>,
) -> Result<StatusCode, ApiError> {
    state.services.budgets.remove(CategoryId(category)).await?;
    Ok(StatusCode::NO_CONTENT)
}
