//! Builds the production object graph: Postgres store, system clock, OS
//! randomness, the shared `ServiceSet` wiring from `app`, and the bot.

use std::sync::Arc;

use anyhow::Context;
use app::ports::{Clock, SystemClock};
use app::services::{ServiceEnvironment, ServiceSet, StorePorts};
use pg::PgStore;
use telegram::bot::BotContext;
use telegram::gateway::{FrankensteinGateway, RetryingGateway};
use telegram::poller::{LONG_POLL, PollHeartbeat, Poller};

use crate::config::AppConfig;
use crate::secret::Secret;
use crate::token_source::OsTokenSource;

/// Everything the commands run on.
pub struct Runtime {
    pub store: Arc<PgStore>,
    pub services: ServiceSet,
    pub clock: Arc<dyn Clock>,
}

/// Connects to Postgres and applies pending migrations.
pub async fn connect_store(config: &AppConfig) -> anyhow::Result<Arc<PgStore>> {
    let url = config.require_database_url()?;
    let store = PgStore::connect(url).await.context("connecting to postgres")?;
    store.migrate().await.context("applying migrations")?;
    Ok(Arc::new(store))
}

pub async fn build_runtime(config: &AppConfig) -> anyhow::Result<Runtime> {
    let store = connect_store(config).await?;
    let clock: Arc<dyn Clock> = Arc::new(SystemClock::new(config.timezone));
    let environment = ServiceEnvironment {
        clock: clock.clone(),
        tokens: Arc::new(OsTokenSource),
        allowed_users: config.allowed_users.clone(),
    };
    let services = ServiceSet::wire(StorePorts::from_single(&store), environment);
    Ok(Runtime { store, services, clock })
}

/// The long-poll loop for `token`, retrying rate-limited replies.
pub fn telegram_poller(
    runtime: &Runtime,
    config: &AppConfig,
    token: &Secret,
    heartbeat: Arc<PollHeartbeat>,
) -> Poller {
    let base = config.telegram_api_base.trim_end_matches('/');
    let api_url = format!("{base}/bot{}", token.expose());
    let gateway =
        Arc::new(RetryingGateway::new(Arc::new(FrankensteinGateway::with_api_url(&api_url))));
    let context = BotContext {
        services: runtime.services.clone(),
        flows: runtime.store.clone(),
        gateway,
        clock: runtime.clock.clone(),
    };
    Poller { context, offsets: runtime.store.clone(), heartbeat, long_poll: LONG_POLL }
}
