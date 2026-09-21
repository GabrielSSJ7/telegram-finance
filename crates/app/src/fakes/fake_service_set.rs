use std::sync::Arc;

use chrono::NaiveDate;

use super::{FixedClock, InMemoryStore, SequentialTokenSource};
use crate::services::{AllowedUsers, ServiceEnvironment, ServiceSet, StorePorts};

/// Production wiring over the in-memory fake, with handles to the fake
/// store and clock so tests can seed data and move time.
pub struct FakeServiceSet {
    pub store: Arc<InMemoryStore>,
    pub clock: Arc<FixedClock>,
    pub services: ServiceSet,
}

impl FakeServiceSet {
    pub fn new(today: NaiveDate, allowed_users: AllowedUsers) -> Self {
        let store = Arc::new(InMemoryStore::new());
        let clock = Arc::new(FixedClock::at_local_noon(today));
        let environment = ServiceEnvironment {
            clock: clock.clone(),
            tokens: Arc::new(SequentialTokenSource::default()),
            allowed_users,
        };
        let services = ServiceSet::wire(StorePorts::from_single(&store), environment);
        Self { store, clock, services }
    }
}
