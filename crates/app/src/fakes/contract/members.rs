use crate::model::{MemberId, MemberProfile};
use crate::services::StorePorts;

pub async fn member_upsert_find_list_dm(stores: StorePorts) {
    let profile = MemberProfile { telegram_user_id: 777_001, display_name: "Ana".into() };
    let created = stores.members.upsert_member(profile.clone()).await.unwrap();
    assert!(created.receives_backups && created.dm_chat_id.is_none());
    let renamed = MemberProfile { display_name: "Ana Maria".into(), ..profile };
    let again = stores.members.upsert_member(renamed).await.unwrap();
    assert_eq!((again.id, again.display_name.as_str()), (created.id, "Ana Maria"));
    assert!(stores.members.set_dm_chat(created.id, 555).await.unwrap());
    assert!(!stores.members.set_dm_chat(MemberId::generate(), 555).await.unwrap());
    let found = stores.members.find_member_by_telegram(777_001).await.unwrap().unwrap();
    assert_eq!(found.dm_chat_id, Some(555));
    assert_eq!(stores.members.find_member_by_telegram(1).await.unwrap(), None);
    assert!(stores.members.list_members().await.unwrap().contains(&found));
}
