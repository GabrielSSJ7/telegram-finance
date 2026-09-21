//! One function per CLI command.

use std::sync::Arc;

use anyhow::{Context, anyhow};
use app::jobs::JobKind;
use chrono::NaiveDate;
use telegram::poller::PollHeartbeat;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::cli::{ApiKeyCommand, Command};
use crate::config::AppConfig;
use crate::health::{ServiceHealthProbe, TelegramLiveness, probe_http_health};
use crate::scheduling::run_scheduler;
use crate::server::serve_http;
use crate::shutdown::cancel_on_signal;
use crate::wiring::{
    Runtime, build_runtime, connect_store, job_runner, scheduler, telegram_gateway, telegram_poller,
};

pub async fn run(command: Command, config: AppConfig) -> anyhow::Result<()> {
    match command {
        Command::Serve => serve(config).await,
        Command::Migrate => {
            connect_store(&config).await.map(|_| tracing::info!("migrations applied"))
        }
        Command::Healthcheck { address } => probe_http_health(&address).await,
        Command::ApiKey(action) => api_key(action, &config).await,
        Command::RunJob { job, date } => run_job(&job, date, &config).await,
    }
}

async fn serve(config: AppConfig) -> anyhow::Result<()> {
    let runtime = build_runtime(&config).await?;
    let shutdown = CancellationToken::new();
    cancel_on_signal(shutdown.clone())?;
    let listener = TcpListener::bind(config.http_bind).await?;
    serve_until_shutdown(&runtime, &config, listener, shutdown).await
}

/// Background tasks of the bot, all stopped by the same token.
struct BotTasks {
    handles: Vec<JoinHandle<()>>,
    heartbeat: Arc<PollHeartbeat>,
}

/// Runs the API and, when a bot token is set, the Telegram poller and the
/// scheduler, until `shutdown` is cancelled.
pub async fn serve_until_shutdown(
    runtime: &Runtime,
    config: &AppConfig,
    listener: TcpListener,
    shutdown: CancellationToken,
) -> anyhow::Result<()> {
    let bot = start_bot(runtime, config, &shutdown);
    let health = health_probe(runtime, bot.as_ref().map(|tasks| &tasks.heartbeat));
    let served = serve_http(
        listener,
        runtime.services.clone(),
        Arc::new(health),
        config.swagger,
        shutdown.clone(),
    )
    .await;
    shutdown.cancel();
    for handle in bot.map(|tasks| tasks.handles).unwrap_or_default() {
        handle.await?;
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
) -> Option<BotTasks> {
    let Some(token) = &config.telegram_bot_token else {
        tracing::warn!("TELEGRAM_BOT_TOKEN not set; running the REST API only");
        return None;
    };
    let gateway = telegram_gateway(config, token);
    let heartbeat = Arc::new(PollHeartbeat::default());
    let poller = telegram_poller(runtime, gateway.clone(), heartbeat.clone());
    let scheduler = scheduler(runtime, job_runner(runtime, gateway), config);
    let handles = vec![
        tokio::spawn(poller.run(shutdown.clone())),
        tokio::spawn(run_scheduler(scheduler, shutdown.clone())),
    ];
    Some(BotTasks { handles, heartbeat })
}

/// Runs one job immediately, bypassing the once-a-day bookkeeping.
async fn run_job(name: &str, date: Option<NaiveDate>, config: &AppConfig) -> anyhow::Result<()> {
    let kind = JobKind::from_name(name)
        .ok_or_else(|| anyhow!("unknown job {name:?}: expected one of {}", JobKind::cli_names()))?;
    let token = config
        .telegram_bot_token
        .as_ref()
        .context("run-job sends Telegram messages: set TELEGRAM_BOT_TOKEN")?;
    let runtime = build_runtime(config).await?;
    let date = date.unwrap_or_else(|| runtime.clock.today());
    job_runner(&runtime, telegram_gateway(config, token)).run(kind, date).await?;
    println!("job {} ran for {date}", kind.name());
    Ok(())
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
