use std::sync::Arc;

use sha2::{Digest, Sha256};

use super::text_rules::clean_name;
use crate::model::{ApiKey, IssuedApiKey};
use crate::ports::{ApiKeyStore, Clock, TokenSource};
use crate::{AppError, AppResult};

/// Prefix makes leaked tokens easy to grep for in logs and repos.
pub const TOKEN_PREFIX: &str = "fbk_";

pub struct ApiKeyService {
    keys: Arc<dyn ApiKeyStore>,
    tokens: Arc<dyn TokenSource>,
    clock: Arc<dyn Clock>,
}

impl ApiKeyService {
    pub fn new(
        keys: Arc<dyn ApiKeyStore>,
        tokens: Arc<dyn TokenSource>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self { keys, tokens, clock }
    }

    /// Creates a named key. The token is returned once; only its SHA-256
    /// is stored.
    pub async fn issue(&self, name: &str) -> AppResult<IssuedApiKey> {
        let name = clean_name("api key name", name, 64)?;
        let bytes =
            self.tokens.secret_bytes().map_err(|error| AppError::Storage(error.to_string()))?;
        let token = format!("{TOKEN_PREFIX}{}", hex::encode(bytes));
        let key = self.keys.create_api_key(&name, token_digest(&token)).await?;
        Ok(IssuedApiKey { key, token })
    }

    /// The active key for `token`. Tokens carry 256 random bits, so a
    /// lookup by hash leaks nothing useful through timing.
    pub async fn authenticate(&self, token: &str) -> AppResult<ApiKey> {
        if !token.starts_with(TOKEN_PREFIX) {
            return Err(AppError::Unauthorized);
        }
        let found = self.keys.find_active_api_key(token_digest(token)).await?;
        let key = found.ok_or(AppError::Unauthorized)?;
        self.keys.touch_api_key(key.id, self.clock.now()).await?;
        Ok(key)
    }

    pub async fn revoke(&self, name: &str) -> AppResult<()> {
        if !self.keys.revoke_api_key(name, self.clock.now()).await? {
            return Err(AppError::not_found("active api key", name));
        }
        Ok(())
    }
}

fn token_digest(token: &str) -> [u8; 32] {
    Sha256::digest(token.as_bytes()).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fakes::{FixedClock, InMemoryStore, SequentialTokenSource};
    use chrono::NaiveDate;

    fn service() -> ApiKeyService {
        let clock = FixedClock::at_local_noon(NaiveDate::from_ymd_opt(2026, 3, 10).unwrap());
        ApiKeyService::new(
            Arc::new(InMemoryStore::new()),
            Arc::new(SequentialTokenSource::default()),
            Arc::new(clock),
        )
    }

    #[tokio::test]
    async fn issued_token_authenticates_until_revoked() {
        let service = service();
        let issued = service.issue("dashboard").await.unwrap();
        assert_eq!(issued.token, format!("fbk_{}", "01".repeat(32)));
        assert_eq!(service.authenticate(&issued.token).await.unwrap().name, "dashboard");
        service.revoke("dashboard").await.unwrap();
        assert_eq!(service.authenticate(&issued.token).await, Err(AppError::Unauthorized));
        assert!(service.revoke("dashboard").await.is_err());
    }

    #[tokio::test]
    async fn rejects_unknown_or_malformed_tokens() {
        let service = service();
        service.issue("cli").await.unwrap();
        assert_eq!(service.authenticate("Bearer abc").await, Err(AppError::Unauthorized));
        assert_eq!(
            service.authenticate(&format!("fbk_{}", "ff".repeat(32))).await,
            Err(AppError::Unauthorized)
        );
    }

    #[tokio::test]
    async fn names_are_unique() {
        let service = service();
        service.issue("cli").await.unwrap();
        assert!(matches!(service.issue("cli").await, Err(AppError::Conflict(_))));
    }
}
