// Integration-test crate: helpers panic on failure by design.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Runs the shared store contract (`app::fakes::contract`) against Postgres.
//! Each case gets a fresh migrated database from `#[sqlx::test]`.

use std::sync::Arc;

use app::services::StorePorts;
use pg::PgStore;
use sqlx::PgPool;

fn pg_ports(pool: PgPool) -> StorePorts {
    StorePorts::from_single(&Arc::new(PgStore::new(pool)))
}

macro_rules! pg_case {
    ($name:ident) => {
        #[sqlx::test(migrator = "pg::MIGRATOR")]
        async fn $name(pool: PgPool) {
            app::fakes::contract::$name(pg_ports(pool)).await;
        }
    };
}

app::store_contract_cases!(pg_case);

#[sqlx::test(migrator = "pg::MIGRATOR")]
async fn ping_succeeds(pool: PgPool) {
    PgStore::new(pool).ping().await.unwrap();
}
