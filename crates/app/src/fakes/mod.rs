//! Named fakes of every port, for tests in this and other crates
//! (enable the `test-support` feature).

pub mod contract;
pub mod fake_service_set;
pub mod fixed_clock;
pub mod in_memory;
pub mod recording_notifier;
pub mod requests;
pub mod sequential_token_source;

pub use fake_service_set::FakeServiceSet;
pub use fixed_clock::FixedClock;
pub use in_memory::InMemoryStore;
pub use recording_notifier::{Notice, RecordingNotifier};
pub use sequential_token_source::SequentialTokenSource;
