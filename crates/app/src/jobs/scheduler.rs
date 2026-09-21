use std::sync::Arc;

use super::{JobKind, JobRunner};
use crate::AppResult;
use crate::ports::{Clock, JobRunStore};
use crate::services::SettingsService;

/// A job that keeps failing stops retrying for that date after this.
pub const MAX_JOB_ATTEMPTS: u32 = 5;

pub struct Scheduler {
    runner: Arc<JobRunner>,
    job_runs: Arc<dyn JobRunStore>,
    settings: Arc<SettingsService>,
    clock: Arc<dyn Clock>,
    jobs: Vec<JobKind>,
}

impl Scheduler {
    pub fn new(
        runner: Arc<JobRunner>,
        job_runs: Arc<dyn JobRunStore>,
        settings: Arc<SettingsService>,
        clock: Arc<dyn Clock>,
        jobs: Vec<JobKind>,
    ) -> Self {
        Self { runner, job_runs, settings, clock, jobs }
    }

    /// Runs every job that is due now and not yet done today. Returns the
    /// jobs it ran and whether each succeeded.
    ///
    /// ```ignore
    /// loop { scheduler.tick().await?; tokio::time::sleep(Duration::from_secs(60)).await; }
    /// ```
    pub async fn tick(&self) -> AppResult<Vec<(JobKind, bool)>> {
        let settings = self.settings.get().await?;
        let local = self.clock.local_now();
        let (date, time) = (local.date_naive(), local.time());
        let mut ran = Vec::new();
        for kind in self.jobs.iter().copied() {
            let due = kind.applies_on(date, &settings) && time >= kind.due_time(&settings);
            if due
                && self
                    .job_runs
                    .claim_job(kind.name(), date, self.clock.now(), MAX_JOB_ATTEMPTS)
                    .await?
            {
                ran.push((kind, self.run_claimed(kind, date).await?));
            }
        }
        Ok(ran)
    }

    async fn run_claimed(&self, kind: JobKind, date: chrono::NaiveDate) -> AppResult<bool> {
        let result = self.runner.run(kind, date).await;
        if let Err(error) = &result {
            tracing::error!(job = kind.name(), %date, %error, "scheduled job failed");
        }
        self.job_runs.finish_job(kind.name(), date, result.is_ok(), self.clock.now()).await?;
        Ok(result.is_ok())
    }
}
