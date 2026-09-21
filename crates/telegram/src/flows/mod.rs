//! Guided flows as pure form state machines. A flow asks one field at a
//! time, then a confirmation, then yields a [`FormCommand`]. Nothing here
//! talks to Telegram or the database; `bot::flow_runner` does that.

pub mod answers;
pub mod command;
pub mod dates;
pub mod engine;
pub mod form;
pub mod interpret;

pub use answers::{Answer, Answers};
pub use command::{FormCommand, build_command};
pub use engine::{Advance, Awaiting, FormInput, FormState};
pub use form::{Field, FormKind};
