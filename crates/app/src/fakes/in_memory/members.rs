use async_trait::async_trait;

use super::InMemoryStore;
use crate::model::{Member, MemberId, MemberProfile};
use crate::ports::{MemberStore, StoreResult};

#[async_trait]
impl MemberStore for InMemoryStore {
    async fn upsert_member(&self, profile: MemberProfile) -> StoreResult<Member> {
        let mut state = self.lock();
        if let Some(existing) =
            state.members.iter_mut().find(|row| row.telegram_user_id == profile.telegram_user_id)
        {
            existing.display_name = profile.display_name;
            return Ok(existing.clone());
        }
        let member = Member {
            id: MemberId::generate(),
            telegram_user_id: profile.telegram_user_id,
            display_name: profile.display_name,
            dm_chat_id: None,
            receives_backups: true,
        };
        state.members.push(member.clone());
        Ok(member)
    }

    async fn find_member_by_telegram(&self, telegram_user_id: i64) -> StoreResult<Option<Member>> {
        let state = self.lock();
        Ok(state.members.iter().find(|row| row.telegram_user_id == telegram_user_id).cloned())
    }

    async fn list_members(&self) -> StoreResult<Vec<Member>> {
        Ok(self.lock().members.clone())
    }

    async fn set_dm_chat(&self, id: MemberId, dm_chat_id: i64) -> StoreResult<bool> {
        let mut state = self.lock();
        let Some(member) = state.members.iter_mut().find(|row| row.id == id) else {
            return Ok(false);
        };
        member.dm_chat_id = Some(dm_chat_id);
        Ok(true)
    }
}
