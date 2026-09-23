//! Wire types. Amounts are integer cents (`*_cents`), dates ISO 8601.

pub mod accounts;
pub mod budgets;
pub mod cards;
pub mod categories;
pub mod entries;
pub mod goals;
pub mod installments;
pub mod members;
pub mod recurrences;
pub mod settings;

use serde::{Deserialize, Deserializer};

/// Tells a missing PATCH field from an explicit `null`: missing stays
/// `None`, `null` becomes `Some(None)` and clears the value.
///
/// ```ignore
/// #[serde(default, deserialize_with = "given_field")]
/// pub emoji: Option<Option<String>>,
/// ```
pub fn given_field<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}
