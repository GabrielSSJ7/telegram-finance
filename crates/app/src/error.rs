use thiserror::Error;

use crate::ports::StoreError;

/// Errors a use case returns. Messages name the offending value and what
/// was expected, because they reach the couple in chat and API clients.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AppError {
    #[error("{what} {id} not found")]
    NotFound { what: &'static str, id: String },
    #[error("invalid {field} {value:?}: expected {expected}")]
    Invalid { field: &'static str, value: String, expected: String },
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("this command was already saved")]
    AlreadyCommitted,
    #[error("unknown or revoked credentials")]
    Unauthorized,
    #[error("telegram user {0} is not allowed to use this bot")]
    Forbidden(i64),
    #[error("storage failed: {0}")]
    Storage(String),
}

pub type AppResult<T> = Result<T, AppError>;

// Constructors take values by design so call sites stay short
// (`AppError::not_found("account", id)`).
#[allow(clippy::needless_pass_by_value)]
impl AppError {
    pub fn not_found(what: &'static str, id: impl ToString) -> Self {
        AppError::NotFound { what, id: id.to_string() }
    }

    pub fn invalid(field: &'static str, value: impl ToString, expected: impl Into<String>) -> Self {
        AppError::Invalid { field, value: value.to_string(), expected: expected.into() }
    }
}

impl From<StoreError> for AppError {
    fn from(error: StoreError) -> Self {
        match error {
            StoreError::DuplicateDraft => AppError::AlreadyCommitted,
            StoreError::UniqueViolation { constraint } => {
                AppError::Conflict(format!("a record with the same {constraint} already exists"))
            }
            StoreError::MissingReference { constraint } => {
                AppError::invalid("reference", constraint, "an existing record")
            }
            StoreError::Backend(message) => AppError::Storage(message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_store_errors() {
        assert_eq!(AppError::from(StoreError::DuplicateDraft), AppError::AlreadyCommitted);
        let conflict = AppError::from(StoreError::UniqueViolation { constraint: "name".into() });
        assert!(conflict.to_string().contains("same name"));
        let storage = AppError::from(StoreError::Backend("timeout".into()));
        assert_eq!(storage, AppError::Storage("timeout".into()));
    }

    #[test]
    fn invalid_message_has_value_and_expectation() {
        let error = AppError::invalid("amount", -5, "a positive number of cents");
        assert_eq!(error.to_string(), "invalid amount \"-5\": expected a positive number of cents");
    }
}
