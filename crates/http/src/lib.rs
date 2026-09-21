//! finbot REST API. Every route calls the same `app` services the Telegram
//! bot uses; this crate only translates HTTP to use cases and back.
//!
//! Errors are RFC 7807 `application/problem+json`. Everything under
//! `/api/v1` needs `Authorization: Bearer <api key>`.

mod auth;
mod dto;
mod error;
mod extract;
mod health;
mod openapi;
mod router;
mod routes;
mod state;

pub use error::ApiError;
pub use health::{HealthCheck, HealthProbe, HealthReport};
pub use openapi::ApiDoc;
pub use router::{RateLimit, RouterOptions, api_router};
pub use state::ApiState;
