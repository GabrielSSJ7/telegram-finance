//! RFC 7807 problem responses. Storage failures are logged and hidden
//! from clients; everything else carries the use-case message, which names
//! the offending value and what was expected.

use app::AppError;
use axum::Json;
use axum::extract::rejection::{JsonRejection, PathRejection, QueryRejection};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug)]
pub enum ApiError {
    App(AppError),
    /// Request could not be read: 400 for syntax, 422 for wrong fields
    /// (axum's own status for each rejection is kept).
    Rejected(StatusCode, String),
    MissingCredentials,
}

/// Body of every error response.
#[derive(Debug, Serialize, ToSchema)]
pub struct Problem {
    #[schema(example = "about:blank")]
    pub r#type: &'static str,
    #[schema(example = "Not Found")]
    pub title: String,
    #[schema(example = 404)]
    pub status: u16,
    #[schema(example = "active account 0199… not found")]
    pub detail: String,
}

impl From<AppError> for ApiError {
    fn from(error: AppError) -> Self {
        ApiError::App(error)
    }
}

impl From<JsonRejection> for ApiError {
    fn from(rejection: JsonRejection) -> Self {
        ApiError::Rejected(rejection.status(), rejection.body_text())
    }
}

impl From<PathRejection> for ApiError {
    fn from(rejection: PathRejection) -> Self {
        ApiError::Rejected(rejection.status(), rejection.body_text())
    }
}

impl From<QueryRejection> for ApiError {
    fn from(rejection: QueryRejection) -> Self {
        ApiError::Rejected(rejection.status(), rejection.body_text())
    }
}

impl ApiError {
    fn status_and_detail(self) -> (StatusCode, String) {
        match self {
            ApiError::Rejected(status, detail) => (status, detail),
            ApiError::MissingCredentials => (
                StatusCode::UNAUTHORIZED,
                "missing bearer token: expected `Authorization: Bearer fbk_…`".into(),
            ),
            ApiError::App(error) => app_status_and_detail(&error),
        }
    }
}

fn app_status_and_detail(error: &AppError) -> (StatusCode, String) {
    let status = match error {
        AppError::NotFound { .. } => StatusCode::NOT_FOUND,
        AppError::Invalid { .. } => StatusCode::UNPROCESSABLE_ENTITY,
        AppError::Conflict(_) | AppError::AlreadyCommitted => StatusCode::CONFLICT,
        AppError::Unauthorized => StatusCode::UNAUTHORIZED,
        AppError::Forbidden(_) => StatusCode::FORBIDDEN,
        AppError::Storage(_) => StatusCode::SERVICE_UNAVAILABLE,
    };
    if let AppError::Storage(message) = error {
        tracing::error!(error = %message, "storage failure while serving request");
        return (status, "storage temporarily unavailable".into());
    }
    (status, error.to_string())
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, detail) = self.status_and_detail();
        let title = status.canonical_reason().unwrap_or("Error").to_owned();
        let problem = Problem { r#type: "about:blank", title, status: status.as_u16(), detail };
        let content_type = [(header::CONTENT_TYPE, "application/problem+json")];
        (status, content_type, Json(problem)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status_of(error: AppError) -> StatusCode {
        ApiError::App(error).status_and_detail().0
    }

    #[test]
    fn maps_every_app_error_to_a_status() {
        assert_eq!(status_of(AppError::not_found("entry", 1)), StatusCode::NOT_FOUND);
        assert_eq!(
            status_of(AppError::invalid("amount", -1, "positive")),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(status_of(AppError::Conflict("x".into())), StatusCode::CONFLICT);
        assert_eq!(status_of(AppError::AlreadyCommitted), StatusCode::CONFLICT);
        assert_eq!(status_of(AppError::Unauthorized), StatusCode::UNAUTHORIZED);
        assert_eq!(status_of(AppError::Forbidden(5)), StatusCode::FORBIDDEN);
    }

    #[test]
    fn hides_storage_details() {
        let (status, detail) =
            ApiError::App(AppError::Storage("password=secret".into())).status_and_detail();
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert!(!detail.contains("secret"));
    }

    #[test]
    fn response_is_problem_json() {
        let response = ApiError::Rejected(StatusCode::BAD_REQUEST, "bad".into()).into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(response.headers()[header::CONTENT_TYPE], "application/problem+json");
    }
}
