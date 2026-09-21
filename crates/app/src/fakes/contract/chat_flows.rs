use chrono::{Duration, Utc};
use serde_json::json;

use crate::model::DraftId;
use crate::ports::{ChatUserKey, StoredFlow};
use crate::services::StorePorts;

fn stored(step: &str, minutes: i64) -> StoredFlow {
    StoredFlow {
        flow: json!({"form": "expense", "step": step}),
        prompt_message_id: Some(42),
        draft_id: DraftId::generate(),
        expires_at: Utc::now() + Duration::minutes(minutes),
    }
}

pub async fn chat_flow_save_load_replace_clear(stores: StorePorts) {
    let key = ChatUserKey { chat_id: -100, user_id: 7 };
    let spouse = ChatUserKey { chat_id: -100, user_id: 8 };
    assert_eq!(stores.flows.load_flow(key, Utc::now()).await.unwrap(), None);
    stores.flows.save_flow(key, &stored("amount", 30)).await.unwrap();
    let replacement = stored("category", 30);
    stores.flows.save_flow(key, &replacement).await.unwrap();
    let loaded = stores.flows.load_flow(key, Utc::now()).await.unwrap();
    assert_eq!(loaded.map(|flow| flow.flow), Some(replacement.flow));
    assert_eq!(stores.flows.load_flow(spouse, Utc::now()).await.unwrap(), None);
    stores.flows.clear_flow(key).await.unwrap();
    assert_eq!(stores.flows.load_flow(key, Utc::now()).await.unwrap(), None);
}

pub async fn chat_flow_expired_is_absent(stores: StorePorts) {
    let key = ChatUserKey { chat_id: -200, user_id: 9 };
    stores.flows.save_flow(key, &stored("amount", -1)).await.unwrap();
    assert_eq!(stores.flows.load_flow(key, Utc::now()).await.unwrap(), None);
}

pub async fn bot_offset_round_trip(stores: StorePorts) {
    assert_eq!(stores.bot_state.load_update_offset().await.unwrap(), None);
    stores.bot_state.save_update_offset(10).await.unwrap();
    stores.bot_state.save_update_offset(11).await.unwrap();
    assert_eq!(stores.bot_state.load_update_offset().await.unwrap(), Some(11));
}
