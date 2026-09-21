//! Postgres adapters for the finbot store ports. [`PgStore`] implements
//! every store trait over one connection pool; queries are checked at
//! compile time by `sqlx` (offline data lives in `/.sqlx`).

mod accounts;
mod api_keys;
mod card_rows;
mod cards;
mod categories;
mod chat_flows;
mod drafts;
mod entries;
mod error_mapping;
mod goals;
mod job_runs;
mod members;
mod recurrences;
mod reports;
mod settings;

use std::time::Duration;

use sqlx::PgPool;
use sqlx::migrate::Migrator;
use sqlx::postgres::PgPoolOptions;

/// Schema migrations embedded in the binary; applied at startup.
pub static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

#[derive(Debug, Clone)]
pub struct PgStore {
    pool: PgPool,
}

impl PgStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Connects with a small pool; the couple's load is tiny.
    ///
    /// ```ignore
    /// let store = PgStore::connect("postgres://finbot:secret@postgres/finbot").await?;
    /// ```
    pub async fn connect(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .acquire_timeout(Duration::from_secs(5))
            .connect(database_url)
            .await?;
        Ok(Self::new(pool))
    }

    pub async fn migrate(&self) -> Result<(), sqlx::migrate::MigrateError> {
        MIGRATOR.run(&self.pool).await
    }

    /// Cheap round trip for health checks.
    pub async fn ping(&self) -> Result<(), sqlx::Error> {
        sqlx::query_scalar!("select 1 as \"one!\"").fetch_one(&self.pool).await.map(|_| ())
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}
