use app::AppError;
use axum::extract::State;
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use chrono::NaiveDate;
use serde::Deserialize;
use utoipa::IntoParams;

use crate::error::{ApiError, Problem};
use crate::extract::ApiQuery;
use crate::state::ApiState;

#[derive(Debug, Deserialize, IntoParams)]
pub struct ExportQuery {
    /// Any date inside the wanted cycle; defaults to today. Ignored when
    /// `from` and `to` are given.
    pub date: Option<NaiveDate>,
    /// First day to export (inclusive); needs `to`.
    pub from: Option<NaiveDate>,
    /// Last day to export (inclusive); needs `from`.
    pub to: Option<NaiveDate>,
}

type CsvResponse = ([(axum::http::HeaderName, String); 2], String);

/// Spreadsheet-friendly CSV (`;`, decimal comma, UTF-8 with BOM).
#[utoipa::path(get, path = "/exports/entries.csv", tag = "exports", params(ExportQuery),
    responses((status = 200, description = "Entries of the period, oldest first", content_type = "text/csv", body = String),
        (status = 422, body = Problem)),
    security(("api_key" = [])))]
pub async fn export_entries(
    State(state): State<ApiState>,
    ApiQuery(query): ApiQuery<ExportQuery>,
) -> Result<CsvResponse, ApiError> {
    let (from, to) = export_range(&state, &query).await?;
    let csv = state.services.exports.entries_csv(from, to).await?;
    let disposition = format!("attachment; filename=\"finbot-{from}_{to}.csv\"");
    let headers =
        [(CONTENT_TYPE, "text/csv; charset=utf-8".to_owned()), (CONTENT_DISPOSITION, disposition)];
    Ok((headers, csv))
}

/// `from..=to` when both are given, else the cycle containing `date`.
async fn export_range(
    state: &ApiState,
    query: &ExportQuery,
) -> Result<(NaiveDate, NaiveDate), ApiError> {
    match (query.from, query.to) {
        (Some(from), Some(to)) if from <= to => Ok((from, to)),
        (Some(from), Some(to)) => {
            Err(AppError::invalid("to", to, format!("a date on or after from ({from})")).into())
        }
        (None, None) => {
            let date = query.date.unwrap_or_else(|| state.services.clock.today());
            let cycle = state.services.reports.cycle_of(date).await?;
            Ok((cycle.start, cycle.last_day()))
        }
        (from, to) => Err(AppError::invalid(
            "from/to",
            format!("{from:?}/{to:?}"),
            "both from and to, or neither",
        )
        .into()),
    }
}
