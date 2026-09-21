use serde::{Deserialize, Serialize};

use super::MemberId;

/// One of the people allowed to use the bot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Member {
    pub id: MemberId,
    pub telegram_user_id: i64,
    pub display_name: String,
    /// Private chat with the bot, known after `/start` in a DM.
    pub dm_chat_id: Option<i64>,
    pub receives_backups: bool,
}

/// What Telegram tells us about a user on each message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemberProfile {
    pub telegram_user_id: i64,
    pub display_name: String,
}
