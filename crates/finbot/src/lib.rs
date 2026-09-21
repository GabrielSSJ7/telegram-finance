//! finbot binary internals, split out of `main.rs` so they can be tested.

pub mod cli;
pub mod commands;
pub mod config;
pub mod health;
pub mod logging;
pub mod secret;
pub mod server;
pub mod shutdown;
pub mod token_source;
pub mod wiring;
