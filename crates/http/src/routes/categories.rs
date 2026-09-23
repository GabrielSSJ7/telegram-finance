use app::model::{CategoryEdit, CategoryId};
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use uuid::Uuid;

use crate::dto::categories::{
    CategoryResponse, CreateCategoryBody, ListCategoriesQuery, UpdateCategoryBody,
};
use crate::error::{ApiError, Problem};
use crate::extract::{ApiJson, ApiPath, ApiQuery};
use crate::state::ApiState;

#[utoipa::path(get, path = "/categories", tag = "categories", params(ListCategoriesQuery),
    responses((status = 200, body = Vec<CategoryResponse>)), security(("api_key" = [])))]
pub async fn list_categories(
    State(state): State<ApiState>,
    ApiQuery(query): ApiQuery<ListCategoriesQuery>,
) -> Result<Json<Vec<CategoryResponse>>, ApiError> {
    let categories = state.services.categories.list(query.kind).await?;
    Ok(Json(categories.into_iter().map(Into::into).collect()))
}

#[utoipa::path(post, path = "/categories", tag = "categories", request_body = CreateCategoryBody,
    responses((status = 201, body = CategoryResponse), (status = 409, body = Problem), (status = 422, body = Problem)),
    security(("api_key" = [])))]
pub async fn create_category(
    State(state): State<ApiState>,
    ApiJson(body): ApiJson<CreateCategoryBody>,
) -> Result<(StatusCode, Json<CategoryResponse>), ApiError> {
    let category = state.services.categories.create(body.into()).await?;
    Ok((StatusCode::CREATED, Json(category.into())))
}

#[utoipa::path(delete, path = "/categories/{id}", tag = "categories", params(("id" = Uuid, Path)),
    responses((status = 204), (status = 404, body = Problem)), security(("api_key" = [])))]
pub async fn archive_category(
    State(state): State<ApiState>,
    ApiPath(id): ApiPath<Uuid>,
) -> Result<StatusCode, ApiError> {
    state.services.categories.archive(CategoryId(id)).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(patch, path = "/categories/{id}", tag = "categories", params(("id" = Uuid, Path)),
    request_body = UpdateCategoryBody,
    responses((status = 200, body = CategoryResponse), (status = 404, body = Problem), (status = 422, body = Problem)),
    security(("api_key" = [])))]
pub async fn update_category(
    State(state): State<ApiState>,
    ApiPath(id): ApiPath<Uuid>,
    ApiJson(body): ApiJson<UpdateCategoryBody>,
) -> Result<Json<CategoryResponse>, ApiError> {
    let categories = &state.services.categories;
    let id = CategoryId(id);
    let edit = CategoryEdit { name: body.name.clone(), emoji: body.emoji_change() };
    let mut category = categories.update(id, edit).await?;
    if let Some(essential) = body.essential {
        category = categories.set_essential(id, essential).await?;
    }
    Ok(Json(category.into()))
}
