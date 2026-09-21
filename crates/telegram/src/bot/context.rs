use std::sync::Arc;

use app::AppError;
use app::ports::{ChatFlowStore, Clock};
use app::services::ServiceSet;

use crate::gateway::{GatewayError, Keyboard, MessageEdit, OutgoingMessage, TelegramGateway};
use crate::render::errors::error_text;

/// Everything a handler needs, cheap to clone.
#[derive(Clone)]
pub struct BotContext {
    pub services: ServiceSet,
    pub flows: Arc<dyn ChatFlowStore>,
    pub gateway: Arc<dyn TelegramGateway>,
    pub clock: Arc<dyn Clock>,
}

impl BotContext {
    pub async fn reply(&self, chat_id: i64, html: impl Into<String>) -> Result<i64, GatewayError> {
        self.send(chat_id, html, None).await
    }

    pub async fn send(
        &self,
        chat_id: i64,
        html: impl Into<String>,
        keyboard: Option<Keyboard>,
    ) -> Result<i64, GatewayError> {
        let message = OutgoingMessage { chat_id, html: html.into(), keyboard };
        self.gateway.send_message(&message).await
    }

    pub async fn edit(
        &self,
        chat_id: i64,
        message_id: i64,
        html: impl Into<String>,
        keyboard: Option<Keyboard>,
    ) -> Result<(), GatewayError> {
        let edit = MessageEdit { chat_id, message_id, html: html.into(), keyboard };
        self.gateway.edit_message(&edit).await
    }

    /// Replies with the pt-BR text for `error`; storage failures are logged.
    pub async fn reply_error(&self, chat_id: i64, error: &AppError) -> Result<(), GatewayError> {
        if let AppError::Storage(detail) = error {
            tracing::error!(chat_id, error = %detail, "storage failure while handling telegram update");
        }
        self.reply(chat_id, error_text(error)).await.map(|_| ())
    }
}
