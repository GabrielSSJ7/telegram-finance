//! pt-BR message text. A flow is shown as one "card": title, answers so
//! far and the current question. Button taps edit the card in place;
//! typed answers get a fresh card below the typed message.

pub mod card;
pub mod catalog;
pub mod errors;
pub mod help;
pub mod keyboards;
pub mod notices;
pub mod report_text;
pub mod reports;

pub use card::{CardContext, CardView, card_view, committed_card};
pub use catalog::Catalog;
