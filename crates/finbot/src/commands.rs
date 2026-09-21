//! One function per CLI command.

use std::sync::Arc;

use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

use crate::cli::{ApiKeyCommand, Command};
use crate::config::AppConfig;
use crate::health::{ServiceHealthProbe, probe_http_health};
use crate::server::serve_http;
use crate::shutdown::cancel_on_signal;
use crate::wiring::{connect_store, service_set};

pub async fn run(command: Command, config: AppConfig) -> anyhow::Result<()> {
    match command {
        Command::Serve => serve(config).await,
        Command::Migrate => {
            connect_store(&config).await.map(|_| tracing::info!("migrations applied"))
        }
        Command::Healthcheck { address } => probe_http_health(&address).await,
        Command::ApiKey(action) => api_key(action, &config).await,
    }
}

async fn serve(config: AppConfig) -> anyhow::Result<()> {
    let store = connect_store(&config).await?;
    let services = service_set(&store, &config);
    let health = Arc::new(ServiceHealthProbe::new(store));
    let shutdown = CancellationToken::new();
    cancel_on_signal(shutdown.clone())?;
    let listener = TcpListener::bind(config.http_bind).await?;
    serve_http(listener, services, health, config.swagger, shutdown).await
}

/// Prints plain text: this is operator-facing CLI output, not a log.
async fn api_key(action: ApiKeyCommand, config: &AppConfig) -> anyhow::Result<()> {
    let store = connect_store(config).await?;
    let keys = service_set(&store, config).api_keys;
    match action {
        ApiKeyCommand::Create { name } => {
            let issued = keys.issue(&name).await?;
            println!(
                "api key {:?} created; token (shown once):\n{}",
                issued.key.name, issued.token
            );
        }
        ApiKeyCommand::Revoke { name } => {
            keys.revoke(&name).await?;
            println!("api key {name:?} revoked");
        }
    }
    Ok(())
}
