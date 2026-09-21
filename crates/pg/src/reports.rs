use app::model::{CategoryId, MemberId, PeriodFlow};
use app::ports::{ReportStore, StoreResult};
use async_trait::async_trait;
use chrono::NaiveDate;
use domain::{Cents, EntryKind};
use uuid::Uuid;

use crate::PgStore;
use crate::error_mapping::{corrupt, store_error};

struct PeriodFlowRow {
    category_id: Option<Uuid>,
    created_by: Option<Uuid>,
    kind: String,
    total: i64,
}

impl PeriodFlowRow {
    fn into_flow(self) -> StoreResult<PeriodFlow> {
        Ok(PeriodFlow {
            category_id: self.category_id.map(CategoryId),
            created_by: self.created_by.map(MemberId),
            kind: self
                .kind
                .parse::<EntryKind>()
                .map_err(|error| corrupt("ledger_entries.kind", error))?,
            total: Cents::new(self.total),
        })
    }
}

#[async_trait]
impl ReportStore for PgStore {
    async fn period_flows(
        &self,
        from: NaiveDate,
        to_exclusive: NaiveDate,
    ) -> StoreResult<Vec<PeriodFlow>> {
        let rows =
            sqlx::query_file_as!(PeriodFlowRow, "queries/period_flows.sql", from, to_exclusive)
                .fetch_all(self.pool())
                .await
                .map_err(store_error)?;
        rows.into_iter().map(PeriodFlowRow::into_flow).collect()
    }

    async fn pot_net_inflow(&self, from: NaiveDate, to_exclusive: NaiveDate) -> StoreResult<Cents> {
        let net = sqlx::query_file_scalar!("queries/pot_net_inflow.sql", from, to_exclusive)
            .fetch_one(self.pool())
            .await
            .map_err(store_error)?;
        Ok(Cents::new(net))
    }
}
