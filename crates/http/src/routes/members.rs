use axum::Json;
use axum::extract::State;

use crate::dto::members::MemberResponse;
use crate::error::ApiError;
use crate::state::ApiState;

#[utoipa::path(get, path = "/members", tag = "members",
    responses((status = 200, body = Vec<MemberResponse>)), security(("api_key" = [])))]
pub async fn list_members(
    State(state): State<ApiState>,
) -> Result<Json<Vec<MemberResponse>>, ApiError> {
    let members = state.services.members.list().await?;
    Ok(Json(members.into_iter().map(Into::into).collect()))
}
