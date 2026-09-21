//! Health: the probe behind `/healthz`, and the client side used by
//! `finbot healthcheck` (distroless images have no curl).

use std::sync::Arc;

use anyhow::{Context, bail};
use async_trait::async_trait;
use http_api::{HealthCheck, HealthProbe, HealthReport};
use pg::PgStore;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

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

pub struct ServiceHealthProbe {
    database: Arc<dyn DatabasePing>,
}

impl ServiceHealthProbe {
    pub fn new(database: Arc<dyn DatabasePing>) -> Self {
        Self { database }
    }
}

#[async_trait]
impl HealthProbe for ServiceHealthProbe {
    async fn report(&self) -> HealthReport {
        let database = match self.database.ping_database().await {
            Ok(()) => HealthCheck { name: "database".into(), healthy: true, detail: "ok".into() },
            Err(error) => HealthCheck { name: "database".into(), healthy: false, detail: error },
        };
        HealthReport::from_checks(vec![database])
    }
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

    /// Database that answers with a fixed result.
    struct ScriptedDatabase(Result<(), String>);

    #[async_trait]
    impl DatabasePing for ScriptedDatabase {
        async fn ping_database(&self) -> Result<(), String> {
            self.0.clone()
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
    async fn probe_fails_when_nothing_listens() {
        let error = probe_http_health("127.0.0.1:1").await.unwrap_err();
        assert!(error.to_string().contains("127.0.0.1:1"));
    }
}
