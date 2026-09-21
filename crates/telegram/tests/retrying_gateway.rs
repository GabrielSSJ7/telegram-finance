// Integration-test crate: helpers panic on failure by design.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;
use std::time::Duration;

use telegram::fakes::FakeTelegramGateway;
use telegram::gateway::{
    GatewayError, MessageEdit, OutgoingMessage, RetryingGateway, TelegramGateway,
};

fn message() -> OutgoingMessage {
    OutgoingMessage { chat_id: 1, html: "oi".into(), keyboard: None }
}

#[tokio::test(start_paused = true)]
async fn retries_short_rate_limits() {
    let fake = Arc::new(FakeTelegramGateway::default());
    fake.fail_next_send(GatewayError::RateLimited { retry_after: Duration::from_secs(2) });
    let gateway = RetryingGateway::new(fake.clone());
    assert_eq!(gateway.send_message(&message()).await.unwrap(), 1);
    assert_eq!(fake.sent_count(), 1);
}

#[tokio::test(start_paused = true)]
async fn gives_up_on_long_waits_and_other_errors() {
    let fake = Arc::new(FakeTelegramGateway::default());
    let gateway = RetryingGateway::new(fake.clone());
    fake.fail_next_send(GatewayError::RateLimited { retry_after: Duration::from_mins(2) });
    assert!(matches!(
        gateway.send_message(&message()).await,
        Err(GatewayError::RateLimited { .. })
    ));
    fake.fail_next_send(GatewayError::Api { code: 403, description: "blocked".into() });
    assert!(matches!(
        gateway.send_message(&message()).await,
        Err(GatewayError::Api { code: 403, .. })
    ));
}

#[tokio::test]
async fn forwards_setup_and_polling_calls() {
    let fake = Arc::new(FakeTelegramGateway::default());
    let gateway = RetryingGateway::new(fake.clone());
    gateway.delete_webhook().await.unwrap();
    gateway.set_commands(&[("saldo", "Saldo")]).await.unwrap();
    gateway.leave_chat(1).await.unwrap();
    assert!(fake.webhook_deleted() && fake.commands() == vec!["saldo".to_owned()]);
    assert_eq!(fake.left_chats(), vec![1]);
    fake.push_poll(Ok(vec![]));
    assert!(gateway.get_updates(None, Duration::from_secs(1)).await.unwrap().is_empty());
}

#[tokio::test]
async fn forwards_message_calls() {
    let fake = Arc::new(FakeTelegramGateway::default());
    let gateway = RetryingGateway::new(fake.clone());
    let id = gateway.send_message(&message()).await.unwrap();
    let edit = MessageEdit { chat_id: 1, message_id: id, html: "tchau".into(), keyboard: None };
    gateway.edit_message(&edit).await.unwrap();
    gateway.remove_keyboard(1, id).await.unwrap();
    gateway.answer_button("cb", None).await.unwrap();
    assert_eq!((fake.edit_count(), fake.last_message(1).unwrap().html.as_str()), (1, "tchau"));
    assert_eq!(fake.answers(), vec![("cb".to_owned(), None)]);
}
