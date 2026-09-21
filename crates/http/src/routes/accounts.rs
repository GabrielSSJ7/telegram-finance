use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use uuid::Uuid;

use crate::dto::accounts::{
    AccountResponse, BalanceSheetResponse, ListAccountsQuery, OpenAccountBody,
};
use crate::error::{ApiError, Problem};
use crate::extract::{ApiJson, ApiPath, ApiQuery};
use crate::state::ApiState;

#[utoipa::path(get, path = "/accounts", tag = "accounts", params(ListAccountsQuery),
    responses((status = 200, body = Vec<AccountResponse>)), security(("api_key" = [])))]
pub async fn list_accounts(
    State(state): State<ApiState>,
    ApiQuery(query): ApiQuery<ListAccountsQuery>,
) -> Result<Json<Vec<AccountResponse>>, ApiError> {
    let accounts = state.services.accounts.list(query.include_archived).await?;
    Ok(Json(accounts.into_iter().map(Into::into).collect()))
}

#[utoipa::path(post, path = "/accounts", tag = "accounts", request_body = OpenAccountBody,
    responses((status = 201, body = AccountResponse), (status = 409, body = Problem), (status = 422, body = Problem)),
    security(("api_key" = [])))]
pub async fn open_account(
    State(state): State<ApiState>,
    ApiJson(body): ApiJson<OpenAccountBody>,
) -> Result<(StatusCode, Json<AccountResponse>), ApiError> {
    let account = state.services.accounts.open(body.into()).await?;
    Ok((StatusCode::CREATED, Json(account.into())))
}

#[utoipa::path(delete, path = "/accounts/{id}", tag = "accounts", params(("id" = Uuid, Path)),
    responses((status = 204), (status = 404, body = Problem)), security(("api_key" = [])))]
pub async fn archive_account(
    State(state): State<ApiState>,
    ApiPath(id): ApiPath<Uuid>,
) -> Result<StatusCode, ApiError> {
    state.services.accounts.archive(app::model::AccountId(id)).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(get, path = "/accounts/balances", tag = "accounts",
    responses((status = 200, body = BalanceSheetResponse)), security(("api_key" = [])))]
pub async fn balance_sheet(
    State(state): State<ApiState>,
) -> Result<Json<BalanceSheetResponse>, ApiError> {
    let sheet = state.services.accounts.balance_sheet().await?;
    Ok(Json(sheet.into()))
}
