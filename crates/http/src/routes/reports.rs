use app::model::{CycleProjection, CycleReport, DailyReport, LivingCost};
use axum::Json;
use axum::extract::State;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

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

/// The basic cost of living, with the figures the bot shows.
#[derive(Debug, Serialize, ToSchema)]
pub struct LivingCostResponse {
    #[schema(value_type = Object)]
    pub cost: LivingCost,
    pub projected_cents: i64,
    pub monthly_cost_cents: i64,
    /// Essential spending over expected income, basis points.
    pub income_share_bp: Option<i64>,
    pub reserve_target_cents: i64,
    /// Months the goal pots cover, in tenths.
    pub reserve_tenths_of_month: Option<i64>,
}

impl From<LivingCost> for LivingCostResponse {
    fn from(cost: LivingCost) -> Self {
        Self {
            projected_cents: cost.projected().value(),
            monthly_cost_cents: cost.monthly_cost().value(),
            income_share_bp: cost.income_share_bp(),
            reserve_target_cents: cost.reserve_target().value(),
            reserve_tenths_of_month: cost.reserve_tenths_of_month(),
            cost,
        }
    }
}

#[utoipa::path(get, path = "/reports/living-cost", tag = "reports", params(ReportQuery),
    responses((status = 200, description = "Essential spending of the cycle containing `date`", body = LivingCostResponse)),
    security(("api_key" = [])))]
pub async fn living_cost(
    State(state): State<ApiState>,
    ApiQuery(query): ApiQuery<ReportQuery>,
) -> Result<Json<LivingCostResponse>, ApiError> {
    let date = query.date.unwrap_or_else(|| state.services.clock.today());
    let cycle = state.services.reports.cycle_of(date).await?;
    Ok(Json(state.services.outlook.living_cost(cycle).await?.into()))
}

/// The cycle's projected end, with the figures the bot shows.
#[derive(Debug, Serialize, ToSchema)]
pub struct ProjectionResponse {
    #[schema(value_type = Object)]
    pub projection: CycleProjection,
    /// Income minus spending over the whole cycle.
    pub result_cents: i64,
    /// Result over income, basis points.
    pub saved_bp: Option<i64>,
    /// Available now plus income still to come, minus what is due.
    pub cash_at_end_cents: i64,
}

impl From<CycleProjection> for ProjectionResponse {
    fn from(projection: CycleProjection) -> Self {
        Self {
            result_cents: projection.result().value(),
            saved_bp: projection.saved_bp(),
            cash_at_end_cents: projection.cash_at_end().value(),
            projection,
        }
    }
}

#[utoipa::path(get, path = "/reports/projection", tag = "reports", params(ReportQuery),
    responses((status = 200, description = "How the cycle containing `date` should end", body = ProjectionResponse)),
    security(("api_key" = [])))]
pub async fn projection(
    State(state): State<ApiState>,
    ApiQuery(query): ApiQuery<ReportQuery>,
) -> Result<Json<ProjectionResponse>, ApiError> {
    let date = query.date.unwrap_or_else(|| state.services.clock.today());
    let cycle = state.services.reports.cycle_of(date).await?;
    Ok(Json(state.services.outlook.projection(cycle).await?.into()))
}
