//! Turns updates into actions: who is talking ([`access`]), what they
//! asked ([`router`], [`commands`]) and running guided flows
//! ([`flow_runner`]) against the `app` services ([`executor`]).

pub mod access;
pub mod budget_alerts;
pub mod category_statement;
pub mod commands;
pub mod context;
pub mod entry_actions;
pub mod executor;
pub mod export;
pub mod flow_runner;
pub mod membership;
pub mod outlook;
pub mod periods;
pub mod recurrence_buttons;
pub mod router;
pub mod undo;

pub use context::BotContext;
pub use router::handle_update;
