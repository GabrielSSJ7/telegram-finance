use std::collections::HashSet;
use std::sync::Arc;

use crate::model::{Member, MemberId, MemberProfile};
use crate::ports::MemberStore;
use crate::{AppError, AppResult};

/// Telegram user ids allowed to use the bot (`ALLOWED_TELEGRAM_USER_IDS`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AllowedUsers(HashSet<i64>);

impl AllowedUsers {
    pub fn new(ids: impl IntoIterator<Item = i64>) -> Self {
        Self(ids.into_iter().collect())
    }

    pub fn contains(&self, telegram_user_id: i64) -> bool {
        self.0.contains(&telegram_user_id)
    }
}

pub struct MemberService {
    members: Arc<dyn MemberStore>,
    allowed: AllowedUsers,
}

impl MemberService {
    pub fn new(members: Arc<dyn MemberStore>, allowed: AllowedUsers) -> Self {
        Self { members, allowed }
    }

    /// The member for this Telegram user, registered on first contact.
    /// Anyone not in the allow list is rejected.
    pub async fn authorize(&self, profile: MemberProfile) -> AppResult<Member> {
        if !self.allowed.contains(profile.telegram_user_id) {
            return Err(AppError::Forbidden(profile.telegram_user_id));
        }
        Ok(self.members.upsert_member(profile).await?)
    }

    pub async fn list(&self) -> AppResult<Vec<Member>> {
        Ok(self.members.list_members().await?)
    }

    /// Remembers the member's private chat so backups can be sent there.
    pub async fn register_dm(&self, id: MemberId, dm_chat_id: i64) -> AppResult<()> {
        if !self.members.set_dm_chat(id, dm_chat_id).await? {
            return Err(AppError::not_found("member", id));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fakes::InMemoryStore;

    fn profile(id: i64, name: &str) -> MemberProfile {
        MemberProfile { telegram_user_id: id, display_name: name.into() }
    }

    #[tokio::test]
    async fn authorizes_allowed_users_and_refreshes_name() {
        let service =
            MemberService::new(Arc::new(InMemoryStore::new()), AllowedUsers::new([10, 20]));
        let first = service.authorize(profile(10, "Gabriel")).await.unwrap();
        let again = service.authorize(profile(10, "Gabi")).await.unwrap();
        assert_eq!((first.id, again.display_name.as_str()), (again.id, "Gabi"));
        assert_eq!(service.list().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn rejects_strangers_and_unknown_dm_member() {
        let service = MemberService::new(Arc::new(InMemoryStore::new()), AllowedUsers::new([10]));
        assert_eq!(service.authorize(profile(99, "x")).await, Err(AppError::Forbidden(99)));
        assert!(service.register_dm(MemberId::generate(), 5).await.is_err());
        let member = service.authorize(profile(10, "Ana")).await.unwrap();
        service.register_dm(member.id, 777).await.unwrap();
        assert_eq!(service.list().await.unwrap()[0].dm_chat_id, Some(777));
    }
}
