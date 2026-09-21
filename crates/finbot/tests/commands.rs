// Integration-test crate: helpers panic on failure by design.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Runs the binary's commands against the test Postgres (`make test` sets
//! `DATABASE_URL` to the throwaway server from `compose.test.yml`).

use std::sync::Arc;

use app::services::AllowedUsers;
use finbot::cli::{ApiKeyCommand, Command};
use finbot::commands::run;
use finbot::config::{AppConfig, LogFormat};
use finbot::health::{ServiceHealthProbe, probe_http_health};
use finbot::server::serve_http;
use finbot::wiring::{connect_store, service_set};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

fn test_config() -> AppConfig {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must point at the test Postgres (run `make test`)");
    AppConfig {
        database_url: Some(database_url),
        http_bind: "127.0.0.1:0".parse().unwrap(),
        timezone: chrono_tz::America::Sao_Paulo,
        allowed_users: AllowedUsers::default(),
        swagger: false,
        log_format: LogFormat::Json,
    }
}

fn unique_name(prefix: &str) -> String {
    format!("{prefix}-{}", nanos_suffix())
}

fn nanos_suffix() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
        .to_string()
}

#[tokio::test]
async fn migrate_is_idempotent() {
    run(Command::Migrate, test_config()).await.unwrap();
    run(Command::Migrate, test_config()).await.unwrap();
}

#[tokio::test]
async fn api_key_create_then_revoke() {
    let name = unique_name("cli");
    run(Command::ApiKey(ApiKeyCommand::Create { name: name.clone() }), test_config())
        .await
        .unwrap();
    run(Command::ApiKey(ApiKeyCommand::Revoke { name: name.clone() }), test_config())
        .await
        .unwrap();
    let again = run(Command::ApiKey(ApiKeyCommand::Revoke { name }), test_config()).await;
    assert!(again.unwrap_err().to_string().contains("not found"));
}

#[tokio::test]
async fn missing_database_url_is_reported() {
    let config = AppConfig { database_url: None, ..test_config() };
    let error = run(Command::Migrate, config).await.unwrap_err();
    assert!(error.to_string().contains("DATABASE_URL"), "{error}");
}

#[tokio::test]
async fn serves_healthz_until_cancelled() {
    let config = test_config();
    let store = connect_store(&config).await.unwrap();
    let services = service_set(&store, &config);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let shutdown = CancellationToken::new();
    let health = Arc::new(ServiceHealthProbe::new(store));
    let server = tokio::spawn(serve_http(listener, services, health, false, shutdown.clone()));
    probe_http_health(&address).await.unwrap();
    run(Command::Healthcheck { address }, config).await.unwrap();
    shutdown.cancel();
    server.await.unwrap().unwrap();
}
