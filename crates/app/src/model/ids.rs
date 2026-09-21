//! Typed ids so an account id can never be passed where a category id is
//! expected. All ids are UUID v7 (time ordered).

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! define_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub Uuid);

        impl $name {
            pub fn generate() -> Self {
                Self(Uuid::now_v7())
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }

        impl FromStr for $name {
            type Err = uuid::Error;
            fn from_str(text: &str) -> Result<Self, Self::Err> {
                Uuid::parse_str(text).map(Self)
            }
        }
    };
}

define_id!(AccountId);
define_id!(CardId);
define_id!(InvoiceId);
define_id!(PurchaseId);
define_id!(ApiKeyId);
define_id!(CategoryId);
define_id!(EntryId);
define_id!(GoalId);
define_id!(MemberId);
define_id!(
    /// Idempotency key of one command: a bot flow draft or a REST
    /// `Idempotency-Key`. Committing the same draft twice is rejected.
    DraftId
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_ids_are_v7_and_round_trip_text() {
        let id = AccountId::generate();
        assert_eq!(id.0.get_version_num(), 7);
        assert_eq!(id.to_string().parse::<AccountId>().unwrap(), id);
    }

    #[test]
    fn rejects_malformed_text() {
        assert!("not-a-uuid".parse::<EntryId>().is_err());
    }
}
