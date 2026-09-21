// Integration-test crate: helpers panic on failure by design.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Runs the binary's commands against the test Postgres (`make test` sets
//! `DATABASE_URL` to the throwaway server from `compose.test.yml`).

use app::services::AllowedUsers;
use finbot::cli::{ApiKeyCommand, Command};
use finbot::commands::run;
use finbot::commands::serve_until_shutdown;
use finbot::config::{AppConfig, LogFormat};
use finbot::health::probe_http_health;
use finbot::secret::Secret;
use finbot::wiring::build_runtime;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

fn test_config() -> AppConfig {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must point at the test Postgres (run `make test`)");
    AppConfig {
        database_url: Some(finbot::secret::Secret::new(database_url)),
        telegram_bot_token: None,
        // Nothing listens on port 1: Telegram calls fail fast, offline.
        telegram_api_base: "http://127.0.0.1:1".into(),
        http_bind: "127.0.0.1:0".parse().unwrap(),
        timezone: chrono_tz::America::Sao_Paulo,
        allowed_users: AllowedUsers::default(),
        swagger: false,
        log_format: LogFormat::Json,
        backup_watch: false,
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
async fn run_job_validates_name_and_token() {
    let unknown =
        run(Command::RunJob { job: "sleep".into(), date: None }, test_config()).await.unwrap_err();
    assert!(unknown.to_string().contains("daily-report"), "{unknown}");
    let no_token = run(Command::RunJob { job: "daily-report".into(), date: None }, test_config())
        .await
        .unwrap_err();
    assert!(no_token.to_string().contains("TELEGRAM_BOT_TOKEN"), "{no_token}");
}

#[tokio::test]
async fn run_job_without_group_drops_message_quietly() {
    let config = AppConfig { telegram_bot_token: Some(Secret::new("0:fake")), ..test_config() };
    let date = chrono::NaiveDate::from_ymd_opt(2026, 3, 1);
    // No group is bound in a fresh database, so the report is dropped quietly.
    run(Command::RunJob { job: "daily-report".into(), date }, config).await.unwrap();
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
    let runtime = build_runtime(&config).await.unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let shutdown = CancellationToken::new();
    let server = tokio::spawn(async move {
        serve_until_shutdown(&runtime, &config, listener, shutdown.clone()).await
    });
    probe_http_health(&address).await.unwrap();
    run(Command::Healthcheck { address: address.clone() }, test_config()).await.unwrap();
    server.abort();
}

#[tokio::test]
async fn serves_with_bot_enabled_and_stops_cleanly() {
    // The poller cannot reach the (unreachable) API base and keeps backing
    // off, which must not block shutdown or the health endpoint.
    let config = AppConfig { telegram_bot_token: Some(Secret::new("0:fake")), ..test_config() };
    let runtime = build_runtime(&config).await.unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let shutdown = CancellationToken::new();
    let stopper = shutdown.clone();
    let server =
        tokio::spawn(
            async move { serve_until_shutdown(&runtime, &config, listener, shutdown).await },
        );
    probe_http_health(&address).await.unwrap();
    stopper.cancel();
    tokio::time::timeout(std::time::Duration::from_secs(10), server)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}
