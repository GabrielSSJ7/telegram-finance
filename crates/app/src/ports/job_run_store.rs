use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};

use super::StoreResult;

/// Once-per-day bookkeeping of scheduled jobs.
#[async_trait]
pub trait JobRunStore: Send + Sync {
    /// Claims `(job, run_date)`. True when this caller should run it: never
    /// ran, or failed fewer than `max_attempts` times, or a run started
    /// more than ten minutes before `now` never finished.
    async fn claim_job(
        &self,
        job: &str,
        run_date: NaiveDate,
        now: DateTime<Utc>,
        max_attempts: u32,
    ) -> StoreResult<bool>;
    async fn finish_job(
        &self,
        job: &str,
        run_date: NaiveDate,
        succeeded: bool,
        now: DateTime<Utc>,
    ) -> StoreResult<()>;
    /// When `job` last succeeded, on any date.
    async fn last_success(&self, job: &str) -> StoreResult<Option<DateTime<Utc>>>;
}
