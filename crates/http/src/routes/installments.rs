use axum::Json;
use axum::extract::State;

use crate::dto::installments::InstallmentPlanResponse;
use crate::error::ApiError;
use crate::state::ApiState;

#[utoipa::path(get, path = "/installments", tag = "installments",
    responses((status = 200, description = "Card purchases and financings still being paid, ending soonest first",
        body = Vec<InstallmentPlanResponse>)),
    security(("api_key" = [])))]
pub async fn list_installment_plans(
    State(state): State<ApiState>,
) -> Result<Json<Vec<InstallmentPlanResponse>>, ApiError> {
    let plans = state.services.installments.running().await?;
    Ok(Json(plans.into_iter().map(Into::into).collect()))
}
