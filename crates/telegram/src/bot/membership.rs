//! The bot being added to a chat. Anyone outside the allow list, or any
//! group other than the household's, gets the bot to leave.

use super::BotContext;
use super::access::authorized_member;
use crate::gateway::{ChatKind, GatewayError, MembershipChange};

const OTHER_GROUP: &str = "Já estou cuidando das finanças de outro grupo. Tchau! 👋";
const GREETING: &str = "Olá! 👋 Mande /start para eu começar a registrar as finanças deste grupo.";

pub async fn on_membership(
    context: &BotContext,
    change: &MembershipChange,
) -> Result<(), GatewayError> {
    if !change.bot_is_member || change.chat_kind != ChatKind::Group {
        return Ok(());
    }
    match authorized_member(context, &change.changed_by).await {
        Ok(Some(_)) => greet_or_leave(context, change.chat_id).await,
        Ok(None) => {
            tracing::warn!(
                chat_id = change.chat_id,
                user_id = change.changed_by.user_id,
                "bot added by a stranger; leaving"
            );
            context.gateway.leave_chat(change.chat_id).await
        }
        Err(error) => context.reply_error(change.chat_id, &error).await,
    }
}

async fn greet_or_leave(context: &BotContext, chat_id: i64) -> Result<(), GatewayError> {
    let bound = match context.services.settings.get().await {
        Ok(settings) => settings.telegram_chat_id,
        Err(error) => return context.reply_error(chat_id, &error).await,
    };
    if bound.is_some_and(|existing| existing != chat_id) {
        context.reply(chat_id, OTHER_GROUP).await?;
        return context.gateway.leave_chat(chat_id).await;
    }
    context.reply(chat_id, GREETING).await.map(|_| ())
}
