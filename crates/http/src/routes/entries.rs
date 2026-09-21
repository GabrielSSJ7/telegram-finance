use app::model::EntryId;
use app::services::EntryOrigin;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use uuid::Uuid;

use crate::dto::entries::{CreateEntryBody, EntryResponse, ListEntriesQuery, UpdateEntryBody};
use crate::error::{ApiError, Problem};
use crate::extract::{ApiJson, ApiPath, ApiQuery, IdempotencyKey};
use crate::state::ApiState;

#[utoipa::path(get, path = "/entries", tag = "entries", params(ListEntriesQuery),
    responses((status = 200, body = Vec<EntryResponse>)), security(("api_key" = [])))]
pub async fn list_entries(
    State(state): State<ApiState>,
    ApiQuery(query): ApiQuery<ListEntriesQuery>,
) -> Result<Json<Vec<EntryResponse>>, ApiError> {
    let entries = state.services.ledger.list(&query.into()).await?;
    Ok(Json(entries.into_iter().map(Into::into).collect()))
}

#[utoipa::path(post, path = "/entries", tag = "entries", request_body = CreateEntryBody,
    params(("Idempotency-Key" = Option<Uuid>, Header, description = "Retry-safe key; a repeat returns 409")),
    responses((status = 201, body = EntryResponse), (status = 404, body = Problem), (status = 409, body = Problem),
        (status = 422, body = Problem)),
    security(("api_key" = [])))]
pub async fn create_entry(
    State(state): State<ApiState>,
    IdempotencyKey(draft): IdempotencyKey,
    ApiJson(body): ApiJson<CreateEntryBody>,
) -> Result<(StatusCode, Json<EntryResponse>), ApiError> {
    let origin = EntryOrigin { created_by: None, draft };
    let entry = state.services.ledger.record(body.into(), origin).await?;
    Ok((StatusCode::CREATED, Json(entry.into())))
}

#[utoipa::path(get, path = "/entries/{id}", tag = "entries", params(("id" = Uuid, Path)),
    responses((status = 200, body = EntryResponse), (status = 404, body = Problem)), security(("api_key" = [])))]
pub async fn get_entry(
    State(state): State<ApiState>,
    ApiPath(id): ApiPath<Uuid>,
) -> Result<Json<EntryResponse>, ApiError> {
    Ok(Json(state.services.ledger.find(EntryId(id)).await?.into()))
}

#[utoipa::path(patch, path = "/entries/{id}", tag = "entries", params(("id" = Uuid, Path)),
    request_body = UpdateEntryBody,
    responses((status = 200, body = EntryResponse), (status = 404, body = Problem), (status = 422, body = Problem)),
    security(("api_key" = [])))]
pub async fn update_entry(
    State(state): State<ApiState>,
    ApiPath(id): ApiPath<Uuid>,
    ApiJson(body): ApiJson<UpdateEntryBody>,
) -> Result<Json<EntryResponse>, ApiError> {
    let entry = state.services.ledger.update(EntryId(id), body.into()).await?;
    Ok(Json(entry.into()))
}

#[utoipa::path(delete, path = "/entries/{id}", tag = "entries", params(("id" = Uuid, Path)),
    responses((status = 204), (status = 404, body = Problem)), security(("api_key" = [])))]
pub async fn delete_entry(
    State(state): State<ApiState>,
    ApiPath(id): ApiPath<Uuid>,
) -> Result<StatusCode, ApiError> {
    state.services.ledger.delete(EntryId(id)).await?;
    Ok(StatusCode::NO_CONTENT)
}
