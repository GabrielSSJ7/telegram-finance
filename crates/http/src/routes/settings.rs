use axum::Json;
use axum::extract::State;

use crate::dto::settings::{SettingsPatchBody, SettingsResponse};
use crate::error::{ApiError, Problem};
use crate::extract::ApiJson;
use crate::state::ApiState;

#[utoipa::path(get, path = "/settings", tag = "settings",
    responses((status = 200, body = SettingsResponse)), security(("api_key" = [])))]
pub async fn get_settings(
    State(state): State<ApiState>,
) -> Result<Json<SettingsResponse>, ApiError> {
    Ok(Json(state.services.settings.get().await?.into()))
}

#[utoipa::path(patch, path = "/settings", tag = "settings", request_body = SettingsPatchBody,
    responses((status = 200, body = SettingsResponse), (status = 422, body = Problem)), security(("api_key" = [])))]
pub async fn update_settings(
    State(state): State<ApiState>,
    ApiJson(body): ApiJson<SettingsPatchBody>,
) -> Result<Json<SettingsResponse>, ApiError> {
    let settings = state.services.settings.update(body.try_into()?).await?;
    Ok(Json(settings.into()))
}
