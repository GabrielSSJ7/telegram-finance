use app::model::{CardId, PurchaseId};
use app::services::EntryOrigin;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use uuid::Uuid;

use crate::dto::cards::{
    CardResponse, CardSummaryResponse, CreditBody, InvoiceResponse, OpenCardBody, PaymentBody,
    PurchaseBody, PurchaseResponse, UpdateCardBody,
};
use crate::dto::entries::EntryResponse;
use crate::error::{ApiError, Problem};
use crate::extract::{ApiJson, ApiPath, IdempotencyKey};
use crate::state::ApiState;

#[utoipa::path(get, path = "/cards", tag = "cards", responses((status = 200, body = Vec<CardResponse>)), security(("api_key" = [])))]
pub async fn list_cards(
    State(state): State<ApiState>,
) -> Result<Json<Vec<CardResponse>>, ApiError> {
    let cards = state.services.cards.list().await?;
    Ok(Json(cards.into_iter().map(Into::into).collect()))
}

#[utoipa::path(post, path = "/cards", tag = "cards", request_body = OpenCardBody,
    responses((status = 201, body = CardResponse), (status = 409, body = Problem), (status = 422, body = Problem)),
    security(("api_key" = [])))]
pub async fn open_card(
    State(state): State<ApiState>,
    ApiJson(body): ApiJson<OpenCardBody>,
) -> Result<(StatusCode, Json<CardResponse>), ApiError> {
    let card = state.services.cards.open(body.try_into()?).await?;
    Ok((StatusCode::CREATED, Json(card.into())))
}

#[utoipa::path(delete, path = "/cards/{id}", tag = "cards", params(("id" = Uuid, Path)),
    responses((status = 204), (status = 404, body = Problem)), security(("api_key" = [])))]
pub async fn archive_card(
    State(state): State<ApiState>,
    ApiPath(id): ApiPath<Uuid>,
) -> Result<StatusCode, ApiError> {
    state.services.cards.archive(CardId(id)).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(get, path = "/cards/summaries", tag = "cards",
    responses((status = 200, body = Vec<CardSummaryResponse>)), security(("api_key" = [])))]
pub async fn card_summaries(
    State(state): State<ApiState>,
) -> Result<Json<Vec<CardSummaryResponse>>, ApiError> {
    let summaries = state.services.cards.summaries().await?;
    Ok(Json(summaries.into_iter().map(Into::into).collect()))
}

#[utoipa::path(get, path = "/cards/{id}/invoices", tag = "cards", params(("id" = Uuid, Path)),
    responses((status = 200, body = Vec<InvoiceResponse>), (status = 404, body = Problem)), security(("api_key" = [])))]
pub async fn card_invoices(
    State(state): State<ApiState>,
    ApiPath(id): ApiPath<Uuid>,
) -> Result<Json<Vec<InvoiceResponse>>, ApiError> {
    let invoices = state.services.cards.invoices(CardId(id)).await?;
    Ok(Json(invoices.into_iter().map(Into::into).collect()))
}

#[utoipa::path(post, path = "/cards/{id}/purchases", tag = "cards", params(("id" = Uuid, Path)), request_body = PurchaseBody,
    responses((status = 201, body = PurchaseResponse), (status = 404, body = Problem), (status = 409, body = Problem),
        (status = 422, body = Problem)),
    security(("api_key" = [])))]
pub async fn create_purchase(
    State(state): State<ApiState>,
    ApiPath(id): ApiPath<Uuid>,
    IdempotencyKey(draft): IdempotencyKey,
    ApiJson(body): ApiJson<PurchaseBody>,
) -> Result<(StatusCode, Json<PurchaseResponse>), ApiError> {
    let origin = EntryOrigin { created_by: None, draft };
    let purchase = state.services.cards.purchase(body.into_request(id), origin).await?;
    Ok((StatusCode::CREATED, Json(purchase.into())))
}

#[utoipa::path(delete, path = "/card-purchases/{id}", tag = "cards", params(("id" = Uuid, Path)),
    responses((status = 204), (status = 404, body = Problem)), security(("api_key" = [])))]
pub async fn delete_purchase(
    State(state): State<ApiState>,
    ApiPath(id): ApiPath<Uuid>,
) -> Result<StatusCode, ApiError> {
    state.services.cards.delete_purchase(PurchaseId(id)).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(post, path = "/cards/{id}/credits", tag = "cards", params(("id" = Uuid, Path)), request_body = CreditBody,
    responses((status = 201, body = EntryResponse), (status = 404, body = Problem), (status = 422, body = Problem)),
    security(("api_key" = [])))]
pub async fn create_credit(
    State(state): State<ApiState>,
    ApiPath(id): ApiPath<Uuid>,
    IdempotencyKey(draft): IdempotencyKey,
    ApiJson(body): ApiJson<CreditBody>,
) -> Result<(StatusCode, Json<EntryResponse>), ApiError> {
    let origin = EntryOrigin { created_by: None, draft };
    let entry = state.services.cards.credit(body.into_request(id), origin).await?;
    Ok((StatusCode::CREATED, Json(entry.into())))
}

#[utoipa::path(post, path = "/invoices/{id}/payments", tag = "cards", params(("id" = Uuid, Path)), request_body = PaymentBody,
    responses((status = 201, body = EntryResponse), (status = 404, body = Problem), (status = 422, body = Problem)),
    security(("api_key" = [])))]
pub async fn pay_invoice(
    State(state): State<ApiState>,
    ApiPath(id): ApiPath<Uuid>,
    IdempotencyKey(draft): IdempotencyKey,
    ApiJson(body): ApiJson<PaymentBody>,
) -> Result<(StatusCode, Json<EntryResponse>), ApiError> {
    let origin = EntryOrigin { created_by: None, draft };
    let entry = state.services.cards.pay_invoice(body.into_request(id), origin).await?;
    Ok((StatusCode::CREATED, Json(entry.into())))
}

#[utoipa::path(patch, path = "/cards/{id}", tag = "cards", params(("id" = Uuid, Path)),
    request_body = UpdateCardBody,
    responses((status = 200, body = CardResponse), (status = 404, body = Problem),
        (status = 409, body = Problem), (status = 422, body = Problem)),
    security(("api_key" = [])))]
pub async fn update_card(
    State(state): State<ApiState>,
    ApiPath(id): ApiPath<Uuid>,
    ApiJson(body): ApiJson<UpdateCardBody>,
) -> Result<Json<CardResponse>, ApiError> {
    let card = state.services.cards.update(CardId(id), body.try_into()?).await?;
    Ok(Json(card.into()))
}
