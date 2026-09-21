//! [`TelegramGateway`] over the `frankenstein` Bot API client (reqwest +
//! rustls). The client strips the URL, which holds the bot token, from its
//! transport errors, so error text is safe to log.

use std::time::Duration;

use async_trait::async_trait;
use frankenstein::AsyncTelegramApi;
use frankenstein::ParseMode;
use frankenstein::client_reqwest::Bot;
use frankenstein::methods::{
    AnswerCallbackQueryParams, DeleteWebhookParams, EditMessageReplyMarkupParams,
    EditMessageTextParams, GetUpdatesParams, LeaveChatParams, SendMessageParams,
    SetMyCommandsParams,
};
use frankenstein::response::ErrorResponse;
use frankenstein::types::{
    AllowedUpdate, BotCommand, InlineKeyboardButton, InlineKeyboardMarkup, LinkPreviewOptions,
    ReplyMarkup,
};

use super::{
    GatewayError, IncomingUpdate, Keyboard, MessageEdit, OutgoingMessage, TelegramGateway,
    map_update,
};

/// Default wait when Telegram answers 429 without `retry_after`.
const DEFAULT_RETRY_AFTER: Duration = Duration::from_secs(5);

#[derive(Clone)]
pub struct FrankensteinGateway {
    bot: Bot,
}

/// The API URL embeds the bot token, so it is never printed.
impl std::fmt::Debug for FrankensteinGateway {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("FrankensteinGateway").field("bot", &"<redacted>").finish()
    }
}

impl FrankensteinGateway {
    pub fn new(bot_token: &str) -> Self {
        Self { bot: Bot::new(bot_token) }
    }

    /// Points at another Bot API server (tests use a local fake server).
    pub fn with_api_url(api_url: &str) -> Self {
        Self { bot: Bot::new_url(api_url) }
    }
}

#[async_trait]
impl TelegramGateway for FrankensteinGateway {
    async fn delete_webhook(&self) -> Result<(), GatewayError> {
        let params = DeleteWebhookParams::builder().drop_pending_updates(false).build();
        self.bot.delete_webhook(&params).await.map(|_| ()).map_err(gateway_error)
    }

    async fn set_commands(&self, commands: &[(&str, &str)]) -> Result<(), GatewayError> {
        let commands = commands
            .iter()
            .map(|(command, description)| {
                BotCommand::builder().command(*command).description(*description).build()
            })
            .collect();
        let params = SetMyCommandsParams::builder().commands(commands).build();
        self.bot.set_my_commands(&params).await.map(|_| ()).map_err(gateway_error)
    }

    async fn get_updates(
        &self,
        offset: Option<i64>,
        timeout: Duration,
    ) -> Result<Vec<IncomingUpdate>, GatewayError> {
        let allowed =
            vec![AllowedUpdate::Message, AllowedUpdate::CallbackQuery, AllowedUpdate::MyChatMember];
        let seconds = u32::try_from(timeout.as_secs()).unwrap_or(50);
        let params = GetUpdatesParams::builder()
            .maybe_offset(offset)
            .timeout(seconds)
            .allowed_updates(allowed)
            .build();
        let response = self.bot.get_updates(&params).await.map_err(gateway_error)?;
        Ok(response.result.into_iter().map(map_update).collect())
    }

    async fn send_message(&self, message: &OutgoingMessage) -> Result<i64, GatewayError> {
        let params = SendMessageParams::builder()
            .chat_id(message.chat_id)
            .text(message.html.clone())
            .parse_mode(ParseMode::Html)
            .link_preview_options(LinkPreviewOptions::builder().is_disabled(true).build())
            .maybe_reply_markup(
                message
                    .keyboard
                    .as_ref()
                    .map(|keyboard| ReplyMarkup::InlineKeyboardMarkup(markup(keyboard))),
            )
            .build();
        let sent = self.bot.send_message(&params).await.map_err(gateway_error)?;
        Ok(i64::from(sent.result.message_id))
    }

    async fn edit_message(&self, edit: &MessageEdit) -> Result<(), GatewayError> {
        let params = EditMessageTextParams::builder()
            .chat_id(edit.chat_id)
            .message_id(i32::try_from(edit.message_id).unwrap_or(i32::MAX))
            .text(edit.html.clone())
            .parse_mode(ParseMode::Html)
            .maybe_reply_markup(edit.keyboard.as_ref().map(markup))
            .build();
        ignore_not_modified(
            self.bot.edit_message_text(&params).await.map(|_| ()).map_err(gateway_error),
        )
    }

    async fn remove_keyboard(&self, chat_id: i64, message_id: i64) -> Result<(), GatewayError> {
        let params = EditMessageReplyMarkupParams::builder()
            .chat_id(chat_id)
            .message_id(i32::try_from(message_id).unwrap_or(i32::MAX))
            .build();
        ignore_not_modified(
            self.bot.edit_message_reply_markup(&params).await.map(|_| ()).map_err(gateway_error),
        )
    }

    async fn answer_button(
        &self,
        callback_id: &str,
        toast: Option<&str>,
    ) -> Result<(), GatewayError> {
        let params = AnswerCallbackQueryParams::builder()
            .callback_query_id(callback_id)
            .maybe_text(toast.map(str::to_owned))
            .build();
        self.bot.answer_callback_query(&params).await.map(|_| ()).map_err(gateway_error)
    }

    async fn leave_chat(&self, chat_id: i64) -> Result<(), GatewayError> {
        let params = LeaveChatParams::builder().chat_id(chat_id).build();
        self.bot.leave_chat(&params).await.map(|_| ()).map_err(gateway_error)
    }
}

fn markup(keyboard: &Keyboard) -> InlineKeyboardMarkup {
    let rows = keyboard
        .rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|button| {
                    InlineKeyboardButton::builder()
                        .text(button.label.clone())
                        .callback_data(button.data.clone())
                        .build()
                })
                .collect()
        })
        .collect();
    InlineKeyboardMarkup::builder().inline_keyboard(rows).build()
}

/// Editing a message into what it already shows is not a failure.
fn ignore_not_modified(result: Result<(), GatewayError>) -> Result<(), GatewayError> {
    match result {
        Err(GatewayError::Api { description, .. })
            if description.contains("message is not modified") =>
        {
            Ok(())
        }
        other => other,
    }
}

fn gateway_error(error: frankenstein::Error) -> GatewayError {
    match error {
        frankenstein::Error::Api(response) => api_error(response),
        other => GatewayError::Transport(other.to_string()),
    }
}

fn api_error(response: ErrorResponse) -> GatewayError {
    let retry_after = response.parameters.as_ref().and_then(|parameters| parameters.retry_after);
    match response.error_code {
        429 => GatewayError::RateLimited {
            retry_after: retry_after
                .map_or(DEFAULT_RETRY_AFTER, |seconds| Duration::from_secs(u64::from(seconds))),
        },
        409 => GatewayError::Conflict,
        code => GatewayError::Api { code, description: response.description },
    }
}
