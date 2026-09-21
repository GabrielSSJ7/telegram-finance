// Integration-test crate: helpers panic on failure by design.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::sync::Arc;
use std::time::Duration;

use app::ports::BotStateStore;
use common::{ANA, BotHarness, GROUP, sender};
use telegram::gateway::{ChatKind, GatewayError, IncomingUpdate, TextMessage, UpdateKind};
use telegram::poller::{PollHeartbeat, Poller};
use tokio_util::sync::CancellationToken;

fn text_update(update_id: i64, text: &str) -> IncomingUpdate {
    let message = TextMessage {
        chat_id: GROUP,
        chat_kind: ChatKind::Group,
        message_id: 1,
        sender: sender(ANA),
        text: text.into(),
    };
    IncomingUpdate { update_id, kind: UpdateKind::Text(message) }
}

fn poller(harness: &BotHarness, heartbeat: &Arc<PollHeartbeat>) -> Poller {
    Poller {
        context: harness.context.clone(),
        offsets: harness.set.store.clone(),
        heartbeat: heartbeat.clone(),
        long_poll: Duration::from_secs(1),
    }
}

async fn run_until_drained(harness: &BotHarness, heartbeat: &Arc<PollHeartbeat>) {
    let shutdown = CancellationToken::new();
    let task = tokio::spawn(poller(harness, heartbeat).run(shutdown.clone()));
    harness.gateway.wait_until_drained().await;
    shutdown.cancel();
    task.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn handles_updates_saves_offsets_and_survives_errors() {
    let harness = BotHarness::new();
    harness.gateway.push_poll(Ok(vec![text_update(5, "/start"), text_update(6, "/ajuda")]));
    harness
        .gateway
        .push_poll(Err(GatewayError::RateLimited { retry_after: Duration::from_secs(3) }));
    harness.gateway.push_poll(Err(GatewayError::Transport("connection reset".into())));
    harness.gateway.push_poll(Err(GatewayError::Conflict));
    harness.gateway.push_poll(Ok(vec![text_update(7, "/saldo")]));
    let heartbeat = Arc::new(PollHeartbeat::default());
    run_until_drained(&harness, &heartbeat).await;
    assert_eq!(harness.set.store.load_update_offset().await.unwrap(), Some(8));
    assert_eq!(harness.gateway.offsets_requested().first(), Some(&None));
    assert_eq!(harness.gateway.offsets_requested().last(), Some(&Some(8)));
    assert!(
        harness.gateway.webhook_deleted()
            && harness.gateway.commands().contains(&"gasto".to_owned())
    );
    assert!(heartbeat.last_success().is_some());
    assert_eq!(harness.set.services.settings.get().await.unwrap().telegram_chat_id, Some(GROUP));
}

#[tokio::test(start_paused = true)]
async fn resumes_from_saved_offset_and_skips_failed_sends() {
    let harness = BotHarness::new();
    harness.set.store.save_update_offset(41).await.unwrap();
    harness
        .gateway
        .fail_next_send(GatewayError::Api { code: 403, description: "bot was kicked".into() });
    harness.gateway.push_poll(Ok(vec![text_update(41, "/start")]));
    let heartbeat = Arc::new(PollHeartbeat::default());
    run_until_drained(&harness, &heartbeat).await;
    assert_eq!(harness.gateway.offsets_requested(), vec![Some(41), Some(42)]);
    assert_eq!(harness.set.store.load_update_offset().await.unwrap(), Some(42));
}

#[tokio::test]
async fn stops_promptly_while_waiting_for_updates() {
    let harness = BotHarness::new();
    let shutdown = CancellationToken::new();
    let task =
        tokio::spawn(poller(&harness, &Arc::new(PollHeartbeat::default())).run(shutdown.clone()));
    harness.gateway.wait_until_drained().await;
    shutdown.cancel();
    tokio::time::timeout(Duration::from_secs(2), task).await.unwrap().unwrap();
}
