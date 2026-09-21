use async_trait::async_trait;

use super::StoreResult;
use crate::model::{Member, MemberId, MemberProfile};

#[async_trait]
pub trait MemberStore: Send + Sync {
    /// Inserts the member or refreshes the display name.
    async fn upsert_member(&self, profile: MemberProfile) -> StoreResult<Member>;
    async fn find_member_by_telegram(&self, telegram_user_id: i64) -> StoreResult<Option<Member>>;
    async fn list_members(&self) -> StoreResult<Vec<Member>>;
    async fn set_dm_chat(&self, id: MemberId, dm_chat_id: i64) -> StoreResult<bool>;
}
