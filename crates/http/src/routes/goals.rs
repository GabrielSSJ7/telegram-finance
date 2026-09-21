use app::model::GoalId;
use app::services::EntryOrigin;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use uuid::Uuid;

use crate::dto::entries::EntryResponse;
use crate::dto::goals::{
    CreateGoalBody, GoalProgressResponse, GoalResponse, GoalTargetBody, PotMoveBody,
};
use crate::error::{ApiError, Problem};
use crate::extract::{ApiJson, ApiPath, IdempotencyKey};
use crate::state::ApiState;

#[utoipa::path(get, path = "/goals", tag = "goals",
    responses((status = 200, body = Vec<GoalProgressResponse>)), security(("api_key" = [])))]
pub async fn list_goals(
    State(state): State<ApiState>,
) -> Result<Json<Vec<GoalProgressResponse>>, ApiError> {
    let goals = state.services.goals.list_progress().await?;
    Ok(Json(goals.into_iter().map(Into::into).collect()))
}

#[utoipa::path(post, path = "/goals", tag = "goals", request_body = CreateGoalBody,
    responses((status = 201, body = GoalResponse), (status = 409, body = Problem), (status = 422, body = Problem)),
    security(("api_key" = [])))]
pub async fn create_goal(
    State(state): State<ApiState>,
    ApiJson(body): ApiJson<CreateGoalBody>,
) -> Result<(StatusCode, Json<GoalResponse>), ApiError> {
    let goal = state.services.goals.create(body.into()).await?;
    Ok((StatusCode::CREATED, Json(goal.into())))
}

#[utoipa::path(put, path = "/goals/{id}/target", tag = "goals", params(("id" = Uuid, Path)),
    request_body = GoalTargetBody,
    responses((status = 200, body = GoalResponse), (status = 404, body = Problem), (status = 422, body = Problem)),
    security(("api_key" = [])))]
pub async fn update_goal_target(
    State(state): State<ApiState>,
    ApiPath(id): ApiPath<Uuid>,
    ApiJson(body): ApiJson<GoalTargetBody>,
) -> Result<Json<GoalResponse>, ApiError> {
    let goal = state.services.goals.update_target(GoalId(id), body.into()).await?;
    Ok(Json(goal.into()))
}

#[utoipa::path(post, path = "/goals/{id}/deposits", tag = "goals", params(("id" = Uuid, Path)),
    request_body = PotMoveBody,
    responses((status = 201, body = EntryResponse), (status = 404, body = Problem), (status = 422, body = Problem)),
    security(("api_key" = [])))]
pub async fn deposit_to_goal(
    State(state): State<ApiState>,
    ApiPath(id): ApiPath<Uuid>,
    IdempotencyKey(draft): IdempotencyKey,
    ApiJson(body): ApiJson<PotMoveBody>,
) -> Result<(StatusCode, Json<EntryResponse>), ApiError> {
    let origin = EntryOrigin { created_by: None, draft };
    let entry = state.services.goals.deposit(body.into_pot_move(id), origin).await?;
    Ok((StatusCode::CREATED, Json(entry.into())))
}

#[utoipa::path(post, path = "/goals/{id}/withdrawals", tag = "goals", params(("id" = Uuid, Path)),
    request_body = PotMoveBody,
    responses((status = 201, body = EntryResponse), (status = 404, body = Problem), (status = 422, body = Problem)),
    security(("api_key" = [])))]
pub async fn withdraw_from_goal(
    State(state): State<ApiState>,
    ApiPath(id): ApiPath<Uuid>,
    IdempotencyKey(draft): IdempotencyKey,
    ApiJson(body): ApiJson<PotMoveBody>,
) -> Result<(StatusCode, Json<EntryResponse>), ApiError> {
    let origin = EntryOrigin { created_by: None, draft };
    let entry = state.services.goals.withdraw(body.into_pot_move(id), origin).await?;
    Ok((StatusCode::CREATED, Json(entry.into())))
}
