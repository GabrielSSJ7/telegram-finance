//! Use cases. Each service owns one area and receives its ports through
//! `new`, so tests swap in the named fakes from `crate::fakes`.

pub mod accounts;
pub mod api_keys;
pub mod balances;
pub mod card_spending;
pub mod card_statements;
pub mod cards;
pub mod categories;
pub mod goals;
pub mod ledger;
pub mod ledger_validation;
pub mod members;
pub mod position;
pub mod service_set;
pub mod settings;
pub mod text_rules;

pub use accounts::{AccountService, OpenAccount};
pub use api_keys::ApiKeyService;
pub use card_spending::{CardCreditRequest, CardPurchaseRequest, InvoicePaymentRequest};
pub use cards::{CardService, OpenCard};
pub use categories::CategoryService;
pub use goals::{CreateGoal, GoalService, PotMove};
pub use ledger::{EntryOrigin, EntryRequest, LedgerService};
pub use members::{AllowedUsers, MemberService};
pub use position::PositionService;
pub use service_set::{ServiceEnvironment, ServiceSet, StorePorts};
pub use settings::SettingsService;

#[cfg(test)]
mod card_tests;
#[cfg(test)]
mod goals_tests;
#[cfg(test)]
mod ledger_tests;
