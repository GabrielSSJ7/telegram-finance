use thiserror::Error;

/// Failures a store reports. Adapters map their driver errors here so
/// services never see SQL details.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum StoreError {
    /// The command's draft id was already committed (double tap, replay).
    #[error("draft already committed")]
    DuplicateDraft,
    /// A unique rule was broken, e.g. two active accounts with one name.
    #[error("unique rule {constraint} violated")]
    UniqueViolation { constraint: String },
    /// A referenced row does not exist.
    #[error("reference rule {constraint} violated")]
    MissingReference { constraint: String },
    #[error("storage backend failed: {0}")]
    Backend(String),
}

pub type StoreResult<T> = Result<T, StoreError>;
