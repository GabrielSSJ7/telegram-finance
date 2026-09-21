//! Builds the production object graph: Postgres store, system clock, OS
//! randomness, then the shared `ServiceSet` wiring from `app`.

use std::sync::Arc;

use anyhow::Context;
use app::ports::SystemClock;
use app::services::{ServiceEnvironment, ServiceSet, StorePorts};
use pg::PgStore;

use crate::config::AppConfig;
use crate::token_source::OsTokenSource;

/// Connects to Postgres and applies pending migrations.
pub async fn connect_store(config: &AppConfig) -> anyhow::Result<Arc<PgStore>> {
    let url = config.require_database_url()?;
    let store = PgStore::connect(url).await.context("connecting to postgres")?;
    store.migrate().await.context("applying migrations")?;
    Ok(Arc::new(store))
}

pub fn service_set(store: &Arc<PgStore>, config: &AppConfig) -> ServiceSet {
    let environment = ServiceEnvironment {
        clock: Arc::new(SystemClock::new(config.timezone)),
        tokens: Arc::new(OsTokenSource),
        allowed_users: config.allowed_users.clone(),
    };
    ServiceSet::wire(StorePorts::from_single(store), environment)
}
