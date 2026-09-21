//! Runs the store contract against `InMemoryStore`.

use std::sync::Arc;

use crate::fakes::InMemoryStore;
use crate::services::StorePorts;

fn memory_ports() -> StorePorts {
    StorePorts::from_single(&Arc::new(InMemoryStore::new()))
}

macro_rules! memory_case {
    ($name:ident) => {
        #[tokio::test]
        async fn $name() {
            super::$name(memory_ports()).await;
        }
    };
}

crate::store_contract_cases!(memory_case);
