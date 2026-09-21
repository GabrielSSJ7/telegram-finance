//! Extractors whose rejections become problem+json instead of axum's
//! plain-text defaults.

use app::model::DraftId;
use axum::extract::{FromRequest, FromRequestParts};
use axum::http::request::Parts;

use crate::error::ApiError;

#[derive(Debug, FromRequest)]
#[from_request(via(axum::Json), rejection(ApiError))]
pub struct ApiJson<T>(pub T);

#[derive(Debug, FromRequestParts)]
#[from_request(via(axum::extract::Path), rejection(ApiError))]
pub struct ApiPath<T>(pub T);

#[derive(Debug, FromRequestParts)]
#[from_request(via(axum::extract::Query), rejection(ApiError))]
pub struct ApiQuery<T>(pub T);

/// Optional `Idempotency-Key` (a UUID). Retrying a POST with the same key
/// returns 409 instead of saving twice.
#[derive(Debug, Clone, Copy)]
pub struct IdempotencyKey(pub Option<DraftId>);

pub const IDEMPOTENCY_HEADER: &str = "idempotency-key";

impl<S: Send + Sync> FromRequestParts<S> for IdempotencyKey {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let Some(raw) = parts.headers.get(IDEMPOTENCY_HEADER) else {
            return Ok(IdempotencyKey(None));
        };
        let text = raw.to_str().unwrap_or_default();
        let draft = text.parse::<DraftId>().map_err(|_| {
            ApiError::Rejected(
                axum::http::StatusCode::BAD_REQUEST,
                format!("invalid Idempotency-Key {text:?}: expected a UUID"),
            )
        })?;
        Ok(IdempotencyKey(Some(draft)))
    }
}
