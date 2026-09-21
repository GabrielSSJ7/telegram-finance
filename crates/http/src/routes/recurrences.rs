use app::model::RecurrenceId;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use uuid::Uuid;

use crate::dto::recurrences::{CreateRecurrenceBody, RecurrenceResponse};
use crate::error::{ApiError, Problem};
use crate::extract::{ApiJson, ApiPath};
use crate::state::ApiState;

#[utoipa::path(get, path = "/recurrences", tag = "recurrences",
    responses((status = 200, body = Vec<RecurrenceResponse>)), security(("api_key" = [])))]
pub async fn list_recurrences(
    State(state): State<ApiState>,
) -> Result<Json<Vec<RecurrenceResponse>>, ApiError> {
    let found = state.services.recurrences.list(false).await?;
    Ok(Json(found.into_iter().map(Into::into).collect()))
}

#[utoipa::path(post, path = "/recurrences", tag = "recurrences", request_body = CreateRecurrenceBody,
    responses((status = 201, body = RecurrenceResponse), (status = 404, body = Problem), (status = 422, body = Problem)),
    security(("api_key" = [])))]
pub async fn create_recurrence(
    State(state): State<ApiState>,
    ApiJson(body): ApiJson<CreateRecurrenceBody>,
) -> Result<(StatusCode, Json<RecurrenceResponse>), ApiError> {
    let created = state.services.recurrences.create(body.try_into()?).await?;
    Ok((StatusCode::CREATED, Json(created.into())))
}

#[utoipa::path(delete, path = "/recurrences/{id}", tag = "recurrences", params(("id" = Uuid, Path)),
    responses((status = 204), (status = 404, body = Problem)), security(("api_key" = [])))]
pub async fn deactivate_recurrence(
    State(state): State<ApiState>,
    ApiPath(id): ApiPath<Uuid>,
) -> Result<StatusCode, ApiError> {
    state.services.recurrences.deactivate(RecurrenceId(id)).await?;
    Ok(StatusCode::NO_CONTENT)
}
