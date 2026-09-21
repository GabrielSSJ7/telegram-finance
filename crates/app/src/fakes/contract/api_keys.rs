use chrono::Utc;

use crate::ports::StoreError;
use crate::services::StorePorts;

pub async fn api_key_create_find_revoke(stores: StorePorts) {
    let hash = [7u8; 32];
    let key = stores.api_keys.create_api_key("contrato", hash).await.unwrap();
    assert_eq!((key.name.as_str(), key.revoked), ("contrato", false));
    let clash = stores.api_keys.create_api_key("contrato", [8u8; 32]).await.unwrap_err();
    assert!(matches!(clash, StoreError::UniqueViolation { .. }), "{clash:?}");
    assert_eq!(stores.api_keys.find_active_api_key(hash).await.unwrap(), Some(key.clone()));
    stores.api_keys.touch_api_key(key.id, Utc::now()).await.unwrap();
    assert!(stores.api_keys.revoke_api_key("contrato", Utc::now()).await.unwrap());
    assert!(!stores.api_keys.revoke_api_key("contrato", Utc::now()).await.unwrap());
    assert_eq!(stores.api_keys.find_active_api_key(hash).await.unwrap(), None);
}
