//! finbot use cases. Services validate requests with the domain rules and
//! persist through the store traits in [`ports`]; adapters (Postgres,
//! Telegram, HTTP) live in other crates.

pub mod error;
pub mod model;
pub mod ports;
pub mod services;

#[cfg(feature = "test-support")]
pub mod fakes;

pub use error::{AppError, AppResult};
