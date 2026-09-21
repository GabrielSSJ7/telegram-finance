//! Health: the probe behind `/healthz`, and the client side used by
//! `finbot healthcheck` (distroless images have no curl).

use std::sync::Arc;

use anyhow::{Context, bail};
use app::ports::Clock;
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use http_api::{HealthCheck, HealthProbe, HealthReport};
use pg::PgStore;
use telegram::poller::PollHeartbeat;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// A long poll lasts 50s, so three minutes without one means trouble.
pub const MAX_POLL_SILENCE_SECONDS: i64 = 180;

/// Anything that can prove the database answers.
#[async_trait]
pub trait DatabasePing: Send + Sync {
    async fn ping_database(&self) -> Result<(), String>;
}

#[async_trait]
impl DatabasePing for PgStore {
    async fn ping_database(&self) -> Result<(), String> {
        self.ping().await.map_err(|error| error.to_string())
    }
}

/// Whether the Telegram poller is alive.
pub struct TelegramLiveness {
    pub heartbeat: Arc<PollHeartbeat>,
    pub clock: Arc<dyn Clock>,
    pub started_at: DateTime<Utc>,
}

pub struct ServiceHealthProbe {
    database: Arc<dyn DatabasePing>,
    telegram: Option<TelegramLiveness>,
}

impl ServiceHealthProbe {
    pub fn new(database: Arc<dyn DatabasePing>) -> Self {
        Self { database, telegram: None }
    }

    pub fn with_telegram(self, telegram: TelegramLiveness) -> Self {
        Self { telegram: Some(telegram), ..self }
    }
}

#[async_trait]
impl HealthProbe for ServiceHealthProbe {
    async fn report(&self) -> HealthReport {
        let database = match self.database.ping_database().await {
            Ok(()) => check("database", true, "ok"),
            Err(error) => check("database", false, &error),
        };
        let telegram = self.telegram.as_ref().map(telegram_check);
        HealthReport::from_checks(std::iter::once(database).chain(telegram).collect())
    }
}

fn telegram_check(liveness: &TelegramLiveness) -> HealthCheck {
    let now = liveness.clock.now();
    let limit = Duration::seconds(MAX_POLL_SILENCE_SECONDS);
    match liveness.heartbeat.last_success() {
        Some(at) if now - at <= limit => {
            check("telegram", true, &format!("last poll {}s ago", (now - at).num_seconds()))
        }
        None if now - liveness.started_at <= limit => check("telegram", true, "starting"),
        _ => check("telegram", false, "no successful poll in the last 3 minutes"),
    }
}

fn check(name: &str, healthy: bool, detail: &str) -> HealthCheck {
    HealthCheck { name: name.into(), healthy, detail: detail.into() }
}

/// GETs `/healthz` on `address` and fails unless the status is 200.
pub async fn probe_http_health(address: &str) -> anyhow::Result<()> {
    let mut stream =
        TcpStream::connect(address).await.with_context(|| format!("connecting to {address}"))?;
    let request = format!("GET /healthz HTTP/1.0\r\nHost: {address}\r\n\r\n");
    stream.write_all(request.as_bytes()).await?;
    let mut response = String::new();
    stream.read_to_string(&mut response).await?;
    let status_line = response.lines().next().unwrap_or_default();
    let status_code = status_line.split_whitespace().nth(1);
    if status_code != Some("200") {
        bail!("unhealthy response from {address}: {status_line:?}, expected HTTP 200");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use app::fakes::FixedClock;
    use chrono::NaiveDate;

    /// Database that answers with a fixed result.
    struct ScriptedDatabase(Result<(), String>);

    #[async_trait]
    impl DatabasePing for ScriptedDatabase {
        async fn ping_database(&self) -> Result<(), String> {
            self.0.clone()
        }
    }

    fn liveness(heartbeat: &Arc<PollHeartbeat>, clock: &Arc<FixedClock>) -> TelegramLiveness {
        TelegramLiveness {
            heartbeat: heartbeat.clone(),
            clock: clock.clone(),
            started_at: clock.now(),
        }
    }

    #[tokio::test]
    async fn report_reflects_database_ping() {
        let up = ServiceHealthProbe::new(Arc::new(ScriptedDatabase(Ok(()))));
        assert!(up.report().await.healthy);
        let down = ServiceHealthProbe::new(Arc::new(ScriptedDatabase(Err("refused".into()))));
        let report = down.report().await;
        assert_eq!((report.healthy, report.checks[0].detail.as_str()), (false, "refused"));
    }

    #[tokio::test]
    async fn telegram_is_starting_then_fresh_then_stale() {
        let clock =
            Arc::new(FixedClock::at_local_noon(NaiveDate::from_ymd_opt(2026, 3, 10).unwrap()));
        let heartbeat = Arc::new(PollHeartbeat::default());
        let probe = ServiceHealthProbe::new(Arc::new(ScriptedDatabase(Ok(()))))
            .with_telegram(liveness(&heartbeat, &clock));
        assert_eq!(probe.report().await.checks[1].detail, "starting");
        heartbeat.beat(clock.now());
        clock.set(clock.now() + Duration::seconds(30));
        assert_eq!(probe.report().await.checks[1].detail, "last poll 30s ago");
        clock.set(clock.now() + Duration::minutes(5));
        assert!(!probe.report().await.healthy);
    }

    #[tokio::test]
    async fn probe_fails_when_nothing_listens() {
        let error = probe_http_health("127.0.0.1:1").await.unwrap_err();
        assert!(error.to_string().contains("127.0.0.1:1"));
    }
}
