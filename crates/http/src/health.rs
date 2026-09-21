//! `/healthz`: unauthenticated liveness plus dependency checks, used by the
//! container healthcheck and uptime monitors.

use async_trait::async_trait;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::Serialize;
use utoipa::ToSchema;

use crate::state::ApiState;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct HealthCheck {
    #[schema(example = "database")]
    pub name: String,
    pub healthy: bool,
    #[schema(example = "ok")]
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct HealthReport {
    pub healthy: bool,
    pub checks: Vec<HealthCheck>,
}

impl HealthReport {
    pub fn from_checks(checks: Vec<HealthCheck>) -> Self {
        Self { healthy: checks.iter().all(|check| check.healthy), checks }
    }
}

/// Implemented by the binary: database ping, Telegram poll freshness.
#[async_trait]
pub trait HealthProbe: Send + Sync {
    async fn report(&self) -> HealthReport;
}

#[utoipa::path(
    get,
    path = "/healthz",
    tag = "health",
    responses(
        (status = 200, description = "All checks pass", body = HealthReport),
        (status = 503, description = "A dependency is failing", body = HealthReport),
    )
)]
pub async fn healthz(State(state): State<ApiState>) -> (StatusCode, Json<HealthReport>) {
    let report = state.health.report().await;
    let status = if report.healthy { StatusCode::OK } else { StatusCode::SERVICE_UNAVAILABLE };
    (status, Json(report))
}
