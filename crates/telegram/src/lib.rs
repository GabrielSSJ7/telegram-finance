//! Telegram adapter for finbot. The couple talks to the bot in their group;
//! this crate turns updates into guided flows and service calls, and
//! service results into pt-BR messages.
//!
//! Layers: [`gateway`] wraps the Bot API client behind a project trait;
//! [`flows`] are pure form state machines; [`bot`] routes updates and runs
//! flows against the `app` services; [`poller`] drives the long-poll loop.

pub mod bot;
pub mod callback_data;
pub mod flows;
pub mod gateway;
pub mod html;
pub mod notifier;
pub mod poller;
pub mod render;

#[cfg(feature = "test-support")]
pub mod fakes;
