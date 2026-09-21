//! One function per CLI command.

use std::sync::Arc;

use telegram::poller::PollHeartbeat;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::cli::{ApiKeyCommand, Command};
use crate::config::AppConfig;
use crate::health::{ServiceHealthProbe, TelegramLiveness, probe_http_health};
use crate::server::serve_http;
use crate::shutdown::cancel_on_signal;
use crate::wiring::{Runtime, build_runtime, connect_store, telegram_poller};

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
    let runtime = build_runtime(&config).await?;
    let shutdown = CancellationToken::new();
    cancel_on_signal(shutdown.clone())?;
    let listener = TcpListener::bind(config.http_bind).await?;
    serve_until_shutdown(&runtime, &config, listener, shutdown).await
}

/// Runs the API and, when a bot token is set, the Telegram poller, until
/// `shutdown` is cancelled.
pub async fn serve_until_shutdown(
    runtime: &Runtime,
    config: &AppConfig,
    listener: TcpListener,
    shutdown: CancellationToken,
) -> anyhow::Result<()> {
    let bot = start_bot(runtime, config, &shutdown);
    let health = health_probe(runtime, bot.as_ref().map(|(_, heartbeat)| heartbeat));
    let served = serve_http(
        listener,
        runtime.services.clone(),
        Arc::new(health),
        config.swagger,
        shutdown.clone(),
    )
    .await;
    shutdown.cancel();
    if let Some((task, _)) = bot {
        task.await?;
    }
    served
}

fn health_probe(runtime: &Runtime, heartbeat: Option<&Arc<PollHeartbeat>>) -> ServiceHealthProbe {
    let probe = ServiceHealthProbe::new(runtime.store.clone());
    let Some(heartbeat) = heartbeat else {
        return probe;
    };
    let clock = runtime.clock.clone();
    probe.with_telegram(TelegramLiveness {
        heartbeat: heartbeat.clone(),
        started_at: clock.now(),
        clock,
    })
}

fn start_bot(
    runtime: &Runtime,
    config: &AppConfig,
    shutdown: &CancellationToken,
) -> Option<(JoinHandle<()>, Arc<PollHeartbeat>)> {
    let Some(token) = &config.telegram_bot_token else {
        tracing::warn!("TELEGRAM_BOT_TOKEN not set; running the REST API only");
        return None;
    };
    let heartbeat = Arc::new(PollHeartbeat::default());
    let poller = telegram_poller(runtime, config, token, heartbeat.clone());
    Some((tokio::spawn(poller.run(shutdown.clone())), heartbeat))
}

/// Prints plain text: this is operator-facing CLI output, not a log.
async fn api_key(action: ApiKeyCommand, config: &AppConfig) -> anyhow::Result<()> {
    let keys = build_runtime(config).await?.services.api_keys;
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
