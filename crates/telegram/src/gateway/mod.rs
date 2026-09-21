//! The Bot API as finbot needs it. [`TelegramGateway`] is the only thing
//! the rest of the crate knows about Telegram's client library.

mod frankenstein_gateway;
mod retrying_gateway;
mod update_mapping;

use std::time::Duration;

use async_trait::async_trait;
use thiserror::Error;

pub use frankenstein_gateway::FrankensteinGateway;
pub use retrying_gateway::RetryingGateway;
pub use update_mapping::map_update;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatKind {
    Private,
    Group,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sender {
    pub user_id: i64,
    pub display_name: String,
    pub is_bot: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextMessage {
    pub chat_id: i64,
    pub chat_kind: ChatKind,
    pub message_id: i64,
    pub sender: Sender,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ButtonPress {
    pub callback_id: String,
    pub chat_id: i64,
    pub message_id: i64,
    pub sender: Sender,
    pub data: String,
}

/// The bot itself joined or left a chat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipChange {
    pub chat_id: i64,
    pub chat_kind: ChatKind,
    pub changed_by: Sender,
    pub bot_is_member: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateKind {
    Text(TextMessage),
    Button(ButtonPress),
    Membership(MembershipChange),
    /// Anything finbot does not handle (stickers, edits, photos...).
    Ignored,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncomingUpdate {
    pub update_id: i64,
    pub kind: UpdateKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Button {
    pub label: String,
    pub data: String,
}

/// Rows of inline buttons.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Keyboard {
    pub rows: Vec<Vec<Button>>,
}

/// A message in Telegram's HTML parse mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutgoingMessage {
    pub chat_id: i64,
    pub html: String,
    pub keyboard: Option<Keyboard>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageEdit {
    pub chat_id: i64,
    pub message_id: i64,
    pub html: String,
    /// `None` removes the buttons.
    pub keyboard: Option<Keyboard>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum GatewayError {
    #[error("telegram rate limit; retry after {retry_after:?}")]
    RateLimited { retry_after: Duration },
    #[error("another getUpdates poller is running for this bot (HTTP 409)")]
    Conflict,
    #[error("telegram api error {code}: {description}")]
    Api { code: u64, description: String },
    #[error("telegram transport error: {0}")]
    Transport(String),
}

#[async_trait]
pub trait TelegramGateway: Send + Sync {
    async fn delete_webhook(&self) -> Result<(), GatewayError>;
    async fn set_commands(&self, commands: &[(&str, &str)]) -> Result<(), GatewayError>;
    async fn get_updates(
        &self,
        offset: Option<i64>,
        timeout: Duration,
    ) -> Result<Vec<IncomingUpdate>, GatewayError>;
    /// Returns the id of the sent message.
    async fn send_message(&self, message: &OutgoingMessage) -> Result<i64, GatewayError>;
    async fn edit_message(&self, edit: &MessageEdit) -> Result<(), GatewayError>;
    /// Drops the inline buttons of a message, keeping its text.
    async fn remove_keyboard(&self, chat_id: i64, message_id: i64) -> Result<(), GatewayError>;
    /// Stops the button's loading spinner, optionally with a short toast.
    async fn answer_button(
        &self,
        callback_id: &str,
        toast: Option<&str>,
    ) -> Result<(), GatewayError>;
    async fn leave_chat(&self, chat_id: i64) -> Result<(), GatewayError>;
}
