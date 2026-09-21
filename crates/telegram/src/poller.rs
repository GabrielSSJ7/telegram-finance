//! The long-poll loop: fetch updates, handle each in order, save the
//! offset after each one so a restart resumes where it stopped.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Duration;

use app::ports::BotStateStore;
use chrono::{DateTime, Utc};
use tokio_util::sync::CancellationToken;

use crate::bot::{BotContext, handle_update};
use crate::gateway::{GatewayError, IncomingUpdate};
use crate::render::help::COMMANDS;

/// Telegram holds a `getUpdates` call open up to this long.
pub const LONG_POLL: Duration = Duration::from_secs(50);
const MAX_BACKOFF: Duration = Duration::from_secs(30);
const CONFLICT_WAIT: Duration = Duration::from_secs(10);

/// Time of the last successful poll, read by the health check.
#[derive(Debug, Default)]
pub struct PollHeartbeat {
    last_success_unix: AtomicI64,
}

impl PollHeartbeat {
    pub fn beat(&self, at: DateTime<Utc>) {
        self.last_success_unix.store(at.timestamp(), Ordering::Relaxed);
    }

    pub fn last_success(&self) -> Option<DateTime<Utc>> {
        let seconds = self.last_success_unix.load(Ordering::Relaxed);
        (seconds > 0).then(|| DateTime::from_timestamp(seconds, 0)).flatten()
    }
}

/// Next offset to ask for and consecutive failures so far.
struct PollCursor {
    offset: Option<i64>,
    failures: u32,
}

pub struct Poller {
    pub context: BotContext,
    pub offsets: Arc<dyn BotStateStore>,
    pub heartbeat: Arc<PollHeartbeat>,
    pub long_poll: Duration,
}

impl Poller {
    /// Polls until `shutdown` is cancelled.
    pub async fn run(self, shutdown: CancellationToken) {
        self.prepare().await;
        let mut cursor = PollCursor { offset: self.load_offset().await, failures: 0 };
        tracing::info!(offset = ?cursor.offset, "telegram poller started");
        while let Some(polled) = self.poll(cursor.offset, &shutdown).await {
            if !self.absorb(polled, &mut cursor, &shutdown).await {
                break;
            }
        }
        tracing::info!("telegram poller stopped");
    }

    /// Handles one poll result; `false` means shutdown was requested.
    async fn absorb(
        &self,
        polled: Result<Vec<IncomingUpdate>, GatewayError>,
        cursor: &mut PollCursor,
        shutdown: &CancellationToken,
    ) -> bool {
        match polled {
            Ok(updates) => {
                cursor.failures = 0;
                self.heartbeat.beat(self.context.clock.now());
                cursor.offset = self.process(updates, cursor.offset).await;
                true
            }
            Err(error) => {
                cursor.failures = cursor.failures.saturating_add(1);
                wait_or_cancel(backoff(&error, cursor.failures), shutdown).await
            }
        }
    }

    /// Removes any webhook (webhook and polling are exclusive) and
    /// publishes the command menu. Failures are logged, not fatal.
    async fn prepare(&self) {
        if let Err(error) = self.context.gateway.delete_webhook().await {
            tracing::warn!(%error, "could not delete webhook");
        }
        if let Err(error) = self.context.gateway.set_commands(COMMANDS).await {
            tracing::warn!(%error, "could not publish command menu");
        }
    }

    async fn load_offset(&self) -> Option<i64> {
        self.offsets.load_update_offset().await.unwrap_or_else(|error| {
            tracing::error!(%error, "could not load update offset; starting from pending updates");
            None
        })
    }

    async fn poll(
        &self,
        offset: Option<i64>,
        shutdown: &CancellationToken,
    ) -> Option<Result<Vec<IncomingUpdate>, GatewayError>> {
        tokio::select! {
            () = shutdown.cancelled() => None,
            polled = self.context.gateway.get_updates(offset, self.long_poll) => Some(polled),
        }
    }

    async fn process(&self, updates: Vec<IncomingUpdate>, mut offset: Option<i64>) -> Option<i64> {
        for update in updates {
            let update_id = update.update_id;
            if let Err(error) = handle_update(&self.context, update).await {
                tracing::error!(%error, update_id, "failed to handle telegram update");
            }
            offset = Some(update_id + 1);
            if let Err(error) = self.offsets.save_update_offset(update_id + 1).await {
                tracing::error!(%error, update_id, "could not save update offset");
            }
        }
        offset
    }
}

/// How long to wait after the `failures`-th consecutive poll error.
pub fn backoff(error: &GatewayError, failures: u32) -> Duration {
    match error {
        GatewayError::RateLimited { retry_after } => *retry_after,
        GatewayError::Conflict => {
            tracing::error!(
                "another poller uses this bot token (HTTP 409); is a second finbot running?"
            );
            CONFLICT_WAIT
        }
        other => {
            tracing::warn!(error = %other, failures, "telegram poll failed");
            let exponent = failures.saturating_sub(1).min(5);
            Duration::from_secs(1u64 << exponent).min(MAX_BACKOFF)
        }
    }
}

/// Sleeps `wait` unless cancelled first; `false` means stop.
async fn wait_or_cancel(wait: Duration, shutdown: &CancellationToken) -> bool {
    tokio::select! {
        () = shutdown.cancelled() => false,
        () = tokio::time::sleep(wait) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_follows_error_kind() {
        let limited = GatewayError::RateLimited { retry_after: Duration::from_secs(7) };
        assert_eq!(backoff(&limited, 1), Duration::from_secs(7));
        assert_eq!(backoff(&GatewayError::Conflict, 1), CONFLICT_WAIT);
        let transport = GatewayError::Transport("reset".into());
        assert_eq!(backoff(&transport, 1), Duration::from_secs(1));
        assert_eq!(backoff(&transport, 3), Duration::from_secs(4));
        assert_eq!(backoff(&transport, 40), MAX_BACKOFF);
    }

    #[test]
    fn heartbeat_starts_empty_then_records() {
        let heartbeat = PollHeartbeat::default();
        assert_eq!(heartbeat.last_success(), None);
        let at = DateTime::from_timestamp(1_800_000_000, 0).unwrap();
        heartbeat.beat(at);
        assert_eq!(heartbeat.last_success(), Some(at));
    }
}
