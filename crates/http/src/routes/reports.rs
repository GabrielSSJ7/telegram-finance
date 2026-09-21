use app::model::{CycleReport, DailyReport};
use axum::Json;
use axum::extract::State;
use chrono::NaiveDate;
use serde::Deserialize;
use utoipa::IntoParams;

use crate::error::ApiError;
use crate::extract::ApiQuery;
use crate::state::ApiState;

#[derive(Debug, Deserialize, IntoParams)]
pub struct ReportQuery {
    /// Defaults to today (household timezone). For the cycle report, any
    /// date inside the wanted cycle.
    pub date: Option<NaiveDate>,
}

#[utoipa::path(get, path = "/reports/daily", tag = "reports", params(ReportQuery),
    responses((status = 200, description = "The daily report (same data the bot sends)", body = Object)),
    security(("api_key" = [])))]
pub async fn daily_report(
    State(state): State<ApiState>,
    ApiQuery(query): ApiQuery<ReportQuery>,
) -> Result<Json<DailyReport>, ApiError> {
    let date = query.date.unwrap_or_else(|| state.services.clock.today());
    Ok(Json(state.services.reports.daily(date).await?))
}

#[utoipa::path(get, path = "/reports/cycle", tag = "reports", params(ReportQuery),
    responses((status = 200, description = "Totals of the cycle containing `date`", body = Object)),
    security(("api_key" = [])))]
pub async fn cycle_report(
    State(state): State<ApiState>,
    ApiQuery(query): ApiQuery<ReportQuery>,
) -> Result<Json<CycleReport>, ApiError> {
    let date = query.date.unwrap_or_else(|| state.services.clock.today());
    let cycle = state.services.reports.cycle_of(date).await?;
    Ok(Json(state.services.reports.cycle(cycle).await?))
}
