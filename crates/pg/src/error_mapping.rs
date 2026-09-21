//! Maps driver errors to the port's `StoreError`, keeping SQL details out
//! of services.

use app::ports::StoreError;
use sqlx::error::ErrorKind;

// By value so it plugs straight into `map_err(store_error)`.
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn store_error(error: sqlx::Error) -> StoreError {
    if let sqlx::Error::Database(database_error) = &error {
        let constraint = database_error.constraint().unwrap_or("unknown").to_owned();
        match database_error.kind() {
            ErrorKind::UniqueViolation => return StoreError::UniqueViolation { constraint },
            ErrorKind::ForeignKeyViolation => return StoreError::MissingReference { constraint },
            _ => {}
        }
    }
    StoreError::Backend(error.to_string())
}

/// A text column held a value the app does not know; the schema's CHECK
/// constraints make this a sign of manual edits or a missing migration.
pub(crate) fn corrupt(column: &str, detail: impl std::fmt::Display) -> StoreError {
    StoreError::Backend(format!("unreadable {column}: {detail}"))
}
