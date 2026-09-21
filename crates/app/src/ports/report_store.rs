use async_trait::async_trait;
use chrono::NaiveDate;
use domain::Cents;

use super::StoreResult;
use crate::model::PeriodFlow;

/// Aggregates for reports, computed where the data lives.
#[async_trait]
pub trait ReportStore: Send + Sync {
    /// Live entry totals with `from <= accounting_date < to_exclusive`,
    /// grouped by category, author and kind.
    async fn period_flows(
        &self,
        from: NaiveDate,
        to_exclusive: NaiveDate,
    ) -> StoreResult<Vec<PeriodFlow>>;
    /// Transfers into pot accounts minus transfers out of them.
    async fn pot_net_inflow(&self, from: NaiveDate, to_exclusive: NaiveDate) -> StoreResult<Cents>;
}
