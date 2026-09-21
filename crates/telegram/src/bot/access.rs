//! Who is talking and where. Only allowed users are served; financial
//! commands only run in the household's own group.

use app::AppError;
use app::model::{Member, MemberProfile};

use super::BotContext;
use crate::gateway::{ChatKind, Sender};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Audience {
    /// An allowed member in the household's group.
    Household(Member),
    /// An allowed member in a group, before any group was bound.
    UnboundGroup(Member),
    /// An allowed member in a private chat with the bot.
    Private(Member),
    /// A stranger writing to the bot privately.
    Stranger,
    /// Bots, strangers in groups, other groups: say nothing.
    Ignore,
}

pub async fn identify(
    context: &BotContext,
    chat_id: i64,
    chat_kind: ChatKind,
    sender: &Sender,
) -> Result<Audience, AppError> {
    if sender.is_bot {
        return Ok(Audience::Ignore);
    }
    let Some(member) = authorized_member(context, sender).await? else {
        let stranger =
            if chat_kind == ChatKind::Private { Audience::Stranger } else { Audience::Ignore };
        return Ok(stranger);
    };
    match chat_kind {
        ChatKind::Private => Ok(Audience::Private(member)),
        ChatKind::Other => Ok(Audience::Ignore),
        ChatKind::Group => group_audience(context, chat_id, member).await,
    }
}

/// The member for `sender`, or `None` when not on the allow list.
pub async fn authorized_member(
    context: &BotContext,
    sender: &Sender,
) -> Result<Option<Member>, AppError> {
    let profile = MemberProfile {
        telegram_user_id: sender.user_id,
        display_name: sender.display_name.clone(),
    };
    match context.services.members.authorize(profile).await {
        Ok(member) => Ok(Some(member)),
        Err(AppError::Forbidden(_)) => {
            // Logged so a new spouse's id can be read from the logs and
            // added to ALLOWED_TELEGRAM_USER_IDS; the bot itself stays silent.
            tracing::info!(
                user_id = sender.user_id,
                name = %sender.display_name,
                "message from a user not in ALLOWED_TELEGRAM_USER_IDS"
            );
            Ok(None)
        }
        Err(other) => Err(other),
    }
}

async fn group_audience(
    context: &BotContext,
    chat_id: i64,
    member: Member,
) -> Result<Audience, AppError> {
    let settings = context.services.settings.get().await?;
    Ok(match settings.telegram_chat_id {
        Some(bound) if bound == chat_id => Audience::Household(member),
        Some(_) => Audience::Ignore,
        None => Audience::UnboundGroup(member),
    })
}
