use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("random source failed: {0}")]
pub struct TokenSourceError(pub String);

/// Source of secret random bytes for API tokens.
pub trait TokenSource: Send + Sync {
    fn secret_bytes(&self) -> Result<[u8; 32], TokenSourceError>;
}
