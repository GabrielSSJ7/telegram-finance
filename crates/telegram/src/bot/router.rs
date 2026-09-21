//! Entry point for every update.

use app::AppError;
use app::model::Member;

use super::BotContext;
use super::access::{Audience, identify};
use super::commands::{household_command, parse_command};
use super::flow_runner::{Placement, continue_flow, load_session, member_key};
use super::membership::on_membership;
use super::undo::undo_button;
use crate::callback_data::{CallbackPayload, nonce_of, parse};
use crate::flows::FormInput;
use crate::gateway::{ButtonPress, GatewayError, IncomingUpdate, TextMessage, UpdateKind};
use crate::render::help::{help_text, welcome_text};

const STALE_BUTTON: &str = "Esse botão não é seu ou já expirou.";
const PRIVATE_BOT: &str = "Este bot é privado. 🔒";

/// Handles one update. Use-case failures become chat replies; only
/// Telegram transport failures are returned.
pub async fn handle_update(
    context: &BotContext,
    update: IncomingUpdate,
) -> Result<(), GatewayError> {
    match update.kind {
        UpdateKind::Text(message) => on_text(context, &message).await,
        UpdateKind::Button(press) => on_button(context, &press).await,
        UpdateKind::Membership(change) => on_membership(context, &change).await,
        UpdateKind::Ignored => Ok(()),
    }
}

async fn on_text(context: &BotContext, message: &TextMessage) -> Result<(), GatewayError> {
    let audience =
        match identify(context, message.chat_id, message.chat_kind, &message.sender).await {
            Ok(audience) => audience,
            Err(error) => return context.reply_error(message.chat_id, &error).await,
        };
    let command = parse_command(&message.text).map(|(name, _)| name);
    dispatch_text(context, message, audience, command.as_deref()).await
}

async fn dispatch_text(
    context: &BotContext,
    message: &TextMessage,
    audience: Audience,
    command: Option<&str>,
) -> Result<(), GatewayError> {
    let chat_id = message.chat_id;
    match (audience, command) {
        (Audience::Household(member), Some(command)) => {
            household_command(context, chat_id, &member, command).await
        }
        (Audience::Household(member), None) => continue_with_text(context, message, &member).await,
        (Audience::UnboundGroup(member), Some("start")) => {
            bind_group(context, chat_id, &member).await
        }
        (Audience::Private(member), Some(command)) => {
            private_command(context, chat_id, &member, command).await
        }
        (Audience::Stranger, _) => context.reply(chat_id, PRIVATE_BOT).await.map(|_| ()),
        _ => Ok(()),
    }
}

async fn continue_with_text(
    context: &BotContext,
    message: &TextMessage,
    member: &Member,
) -> Result<(), GatewayError> {
    let Some((session, state)) =
        load_session(context, member_key(message.chat_id, member), member).await
    else {
        return Ok(());
    };
    continue_flow(
        context,
        session,
        state,
        FormInput::Text(message.text.clone()),
        Placement::NewCard,
    )
    .await
}

async fn bind_group(
    context: &BotContext,
    chat_id: i64,
    member: &Member,
) -> Result<(), GatewayError> {
    match context.services.settings.bind_chat(chat_id).await {
        Ok(_) => {
            tracing::info!(chat_id, member = %member.display_name, "household bound to telegram group");
            context.reply(chat_id, welcome_text()).await.map(|_| ())
        }
        Err(AppError::Conflict(_)) => context.gateway.leave_chat(chat_id).await,
        Err(error) => context.reply_error(chat_id, &error).await,
    }
}

async fn private_command(
    context: &BotContext,
    chat_id: i64,
    member: &Member,
    command: &str,
) -> Result<(), GatewayError> {
    match command {
        "start" => match context.services.members.register_dm(member.id, chat_id).await {
            Ok(()) => context
                .reply(chat_id, "Pronto! Vou mandar os backups por aqui. 🔐")
                .await
                .map(|_| ()),
            Err(error) => context.reply_error(chat_id, &error).await,
        },
        "ajuda" | "help" => context.reply(chat_id, help_text()).await.map(|_| ()),
        _ => context.reply(chat_id, "Use os comandos no grupo de vocês. 🙂").await.map(|_| ()),
    }
}

async fn on_button(context: &BotContext, press: &ButtonPress) -> Result<(), GatewayError> {
    let chat_kind = crate::gateway::ChatKind::Group;
    let audience = identify(context, press.chat_id, chat_kind, &press.sender).await;
    let Ok(Audience::Household(member)) = audience else {
        return context.gateway.answer_button(&press.callback_id, Some(STALE_BUTTON)).await;
    };
    match parse(&press.data) {
        Some(CallbackPayload::Undo(entry_id)) => {
            undo_button(context, press, &member, entry_id).await
        }
        Some(CallbackPayload::Flow { nonce, value }) => {
            flow_button(context, press, &member, &nonce, value).await
        }
        None => context.gateway.answer_button(&press.callback_id, Some(STALE_BUTTON)).await,
    }
}

async fn flow_button(
    context: &BotContext,
    press: &ButtonPress,
    member: &Member,
    nonce: &str,
    value: crate::callback_data::ButtonValue,
) -> Result<(), GatewayError> {
    let loaded = load_session(context, member_key(press.chat_id, member), member).await;
    let Some((session, state)) = loaded.filter(|(session, _)| nonce_of(session.draft) == nonce)
    else {
        return context.gateway.answer_button(&press.callback_id, Some(STALE_BUTTON)).await;
    };
    context.gateway.answer_button(&press.callback_id, None).await?;
    continue_flow(
        context,
        session,
        state,
        FormInput::Button(value),
        Placement::EditCard(press.message_id),
    )
    .await
}
