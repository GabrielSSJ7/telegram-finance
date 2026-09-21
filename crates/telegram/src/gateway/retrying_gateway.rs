//! Decorator that waits out Telegram rate limits (HTTP 429) on outgoing
//! calls, so a busy minute delays a reply instead of dropping it.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;

use super::{GatewayError, IncomingUpdate, MessageEdit, OutgoingMessage, TelegramGateway};

/// Waits longer than this are not worth holding the update loop for.
const MAX_WAIT: Duration = Duration::from_secs(30);
const ATTEMPTS: u32 = 3;

pub struct RetryingGateway {
    inner: Arc<dyn TelegramGateway>,
}

impl RetryingGateway {
    pub fn new(inner: Arc<dyn TelegramGateway>) -> Self {
        Self { inner }
    }
}

/// Runs `call` again after each short rate-limit wait, up to `ATTEMPTS`.
async fn with_retry<T, F, Fut>(mut call: F) -> Result<T, GatewayError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, GatewayError>>,
{
    let mut attempt = 1;
    loop {
        match call().await {
            Err(GatewayError::RateLimited { retry_after })
                if attempt < ATTEMPTS && retry_after <= MAX_WAIT =>
            {
                tracing::warn!(?retry_after, attempt, "telegram rate limit; waiting");
                tokio::time::sleep(retry_after).await;
                attempt += 1;
            }
            other => return other,
        }
    }
}

#[async_trait]
impl TelegramGateway for RetryingGateway {
    async fn delete_webhook(&self) -> Result<(), GatewayError> {
        with_retry(|| self.inner.delete_webhook()).await
    }

    async fn set_commands(&self, commands: &[(&str, &str)]) -> Result<(), GatewayError> {
        with_retry(|| self.inner.set_commands(commands)).await
    }

    /// Not retried here: the poller has its own backoff.
    async fn get_updates(
        &self,
        offset: Option<i64>,
        timeout: Duration,
    ) -> Result<Vec<IncomingUpdate>, GatewayError> {
        self.inner.get_updates(offset, timeout).await
    }

    async fn send_message(&self, message: &OutgoingMessage) -> Result<i64, GatewayError> {
        with_retry(|| self.inner.send_message(message)).await
    }

    async fn edit_message(&self, edit: &MessageEdit) -> Result<(), GatewayError> {
        with_retry(|| self.inner.edit_message(edit)).await
    }

    async fn remove_keyboard(&self, chat_id: i64, message_id: i64) -> Result<(), GatewayError> {
        with_retry(|| self.inner.remove_keyboard(chat_id, message_id)).await
    }

    async fn answer_button(
        &self,
        callback_id: &str,
        toast: Option<&str>,
    ) -> Result<(), GatewayError> {
        with_retry(|| self.inner.answer_button(callback_id, toast)).await
    }

    async fn leave_chat(&self, chat_id: i64) -> Result<(), GatewayError> {
        with_retry(|| self.inner.leave_chat(chat_id)).await
    }
}
