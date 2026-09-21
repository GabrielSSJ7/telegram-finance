//! Interfaces the services depend on. Adapters implement them: `pg` for the
//! stores, the binary for the clock and token source, `fakes` for tests.

pub mod account_store;
pub mod api_key_store;
pub mod card_store;
pub mod category_store;
pub mod chat_flow_store;
pub mod clock;
pub mod entry_store;
pub mod goal_store;
pub mod member_store;
pub mod settings_store;
pub mod store_error;
pub mod token_source;

pub use account_store::AccountStore;
pub use api_key_store::ApiKeyStore;
pub use card_store::CardStore;
pub use category_store::CategoryStore;
pub use chat_flow_store::{BotStateStore, ChatFlowStore, ChatUserKey, StoredFlow};
pub use clock::{Clock, SystemClock};
pub use entry_store::EntryStore;
pub use goal_store::GoalStore;
pub use member_store::MemberStore;
pub use settings_store::SettingsStore;
pub use store_error::{StoreError, StoreResult};
pub use token_source::{TokenSource, TokenSourceError};
