//! Bearer-token guard for `/api/v1`.

use axum::extract::{Request, State};
use axum::http::header::AUTHORIZATION;
use axum::middleware::Next;
use axum::response::Response;

use crate::error::ApiError;
use crate::state::ApiState;

pub async fn require_api_key(
    State(state): State<ApiState>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let header = request.headers().get(AUTHORIZATION).and_then(|value| value.to_str().ok());
    let token = header
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or(ApiError::MissingCredentials)?;
    let key = state.services.api_keys.authenticate(token.trim()).await?;
    tracing::debug!(api_key = %key.name, "authenticated request");
    Ok(next.run(request).await)
}
