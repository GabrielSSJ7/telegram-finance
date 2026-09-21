use async_trait::async_trait;
use chrono::{DateTime, Duration, NaiveDate, Utc};

use super::{InMemoryStore, JobRunRow};
use crate::ports::{JobRunStore, StoreResult};

/// A run that started this long ago without finishing is presumed dead.
const STALE_RUN_MINUTES: i64 = 10;

#[async_trait]
impl JobRunStore for InMemoryStore {
    async fn claim_job(
        &self,
        job: &str,
        run_date: NaiveDate,
        now: DateTime<Utc>,
        max_attempts: u32,
    ) -> StoreResult<bool> {
        let mut state = self.lock();
        let key = (job.to_owned(), run_date);
        let fresh = JobRunRow { succeeded: false, running: true, attempts: 1, updated_at: now };
        let Some(row) = state.job_runs.get_mut(&key) else {
            state.job_runs.insert(key, fresh);
            return Ok(true);
        };
        if !can_retry(*row, now, max_attempts) {
            return Ok(false);
        }
        *row = JobRunRow { attempts: row.attempts + 1, ..fresh };
        Ok(true)
    }

    async fn finish_job(
        &self,
        job: &str,
        run_date: NaiveDate,
        succeeded: bool,
        now: DateTime<Utc>,
    ) -> StoreResult<()> {
        let mut state = self.lock();
        let entry = state.job_runs.entry((job.to_owned(), run_date));
        let row =
            entry.or_insert(JobRunRow { succeeded, running: false, attempts: 1, updated_at: now });
        *row = JobRunRow { succeeded, running: false, updated_at: now, ..*row };
        Ok(())
    }

    async fn last_success(&self, job: &str) -> StoreResult<Option<DateTime<Utc>>> {
        let state = self.lock();
        let successes =
            state.job_runs.iter().filter(|((name, _), row)| name == job && row.succeeded);
        Ok(successes.map(|(_, row)| row.updated_at).max())
    }
}

fn can_retry(row: JobRunRow, now: DateTime<Utc>, max_attempts: u32) -> bool {
    if row.succeeded || row.attempts >= max_attempts {
        return false;
    }
    !row.running || now - row.updated_at > Duration::minutes(STALE_RUN_MINUTES)
}
