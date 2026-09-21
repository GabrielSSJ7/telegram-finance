//! Records the services read and write. Rules live in the `domain` crate;
//! these are plain data with typed ids.

pub mod account;
pub mod api_key;
pub mod category;
pub mod entry;
pub mod goal;
pub mod ids;
pub mod member;
pub mod settings;

pub use account::{Account, AccountBalance, BalanceSheet, NewAccount};
pub use api_key::{ApiKey, IssuedApiKey};
pub use category::{Category, CategoryKind, NewCategory};
pub use entry::{EntryFilter, EntryPatch, LedgerEntry, NewEntry};
pub use goal::{Goal, GoalProgress, GoalTarget};
pub use ids::{AccountId, ApiKeyId, CategoryId, DraftId, EntryId, GoalId, MemberId};
pub use member::{Member, MemberProfile};
pub use settings::{HouseholdSettings, SettingsPatch};
