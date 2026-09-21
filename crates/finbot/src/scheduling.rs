//! Runs the scheduler once a minute until shutdown.

use std::time::Duration;

use app::jobs::Scheduler;
use tokio_util::sync::CancellationToken;

pub const TICK_EVERY: Duration = Duration::from_mins(1);

pub async fn run_scheduler(scheduler: Scheduler, shutdown: CancellationToken) {
    let mut interval = tokio::time::interval(TICK_EVERY);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    tracing::info!("scheduler started");
    loop {
        tokio::select! {
            () = shutdown.cancelled() => break,
            _ = interval.tick() => tick_once(&scheduler).await,
        }
    }
    tracing::info!("scheduler stopped");
}

async fn tick_once(scheduler: &Scheduler) {
    match scheduler.tick().await {
        Ok(ran) => {
            for (kind, succeeded) in ran {
                tracing::info!(job = kind.name(), succeeded, "scheduled job ran");
            }
        }
        Err(error) => tracing::error!(%error, "scheduler tick failed"),
    }
}
