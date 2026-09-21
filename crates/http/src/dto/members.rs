use app::model::Member;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
pub struct MemberResponse {
    pub id: Uuid,
    pub telegram_user_id: i64,
    #[schema(example = "Ana")]
    pub display_name: String,
    /// Whether the encrypted daily backup goes to this member's DM.
    pub receives_backups: bool,
    /// False until the member sends `/start` to the bot in private.
    pub has_private_chat: bool,
}

impl From<Member> for MemberResponse {
    fn from(member: Member) -> Self {
        Self {
            id: member.id.0,
            telegram_user_id: member.telegram_user_id,
            display_name: member.display_name,
            receives_backups: member.receives_backups,
            has_private_chat: member.dm_chat_id.is_some(),
        }
    }
}
