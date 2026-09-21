use app::ports::{JobRunStore, StoreResult};
use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};

use crate::PgStore;
use crate::error_mapping::store_error;

#[async_trait]
impl JobRunStore for PgStore {
    async fn claim_job(
        &self,
        job: &str,
        run_date: NaiveDate,
        now: DateTime<Utc>,
        max_attempts: u32,
    ) -> StoreResult<bool> {
        let limit = i32::try_from(max_attempts).unwrap_or(i32::MAX);
        let claimed = sqlx::query_file!("queries/claim_job.sql", job, run_date, now, limit)
            .fetch_optional(self.pool())
            .await
            .map_err(store_error)?;
        Ok(claimed.is_some())
    }

    async fn finish_job(
        &self,
        job: &str,
        run_date: NaiveDate,
        succeeded: bool,
        now: DateTime<Utc>,
    ) -> StoreResult<()> {
        let status = if succeeded { "succeeded" } else { "failed" };
        sqlx::query!(
            "insert into job_runs (job, run_date, status, updated_at) values ($1, $2, $3, $4)
             on conflict (job, run_date) do update set status = excluded.status, updated_at = excluded.updated_at",
            job,
            run_date,
            status,
            now,
        )
        .execute(self.pool())
        .await
        .map_err(store_error)?;
        Ok(())
    }

    async fn last_success(&self, job: &str) -> StoreResult<Option<DateTime<Utc>>> {
        let last = sqlx::query_scalar!(
            "select max(updated_at) from job_runs where job = $1 and status = 'succeeded'",
            job
        )
        .fetch_one(self.pool())
        .await
        .map_err(store_error)?;
        Ok(last)
    }
}
