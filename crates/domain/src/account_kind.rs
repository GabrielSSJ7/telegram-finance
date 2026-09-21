//! Account kinds. Pots ("caixinhas") hold money reserved for a goal, so it
//! is shown apart and never counted as available to spend.

use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountKind {
    Checking,
    Savings,
    Cash,
    Pot,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("unknown account kind {0:?}: expected one of checking, savings, cash, pot")]
pub struct UnknownAccountKind(pub String);

impl AccountKind {
    pub const ALL: [AccountKind; 4] = [
        AccountKind::Checking,
        AccountKind::Savings,
        AccountKind::Cash,
        AccountKind::Pot,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            AccountKind::Checking => "checking",
            AccountKind::Savings => "savings",
            AccountKind::Cash => "cash",
            AccountKind::Pot => "pot",
        }
    }

    pub const fn is_spendable(self) -> bool {
        !matches!(self, AccountKind::Pot)
    }
}

impl FromStr for AccountKind {
    type Err = UnknownAccountKind;
    fn from_str(name: &str) -> Result<Self, Self::Err> {
        AccountKind::ALL
            .into_iter()
            .find(|kind| kind.as_str() == name)
            .ok_or_else(|| UnknownAccountKind(name.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_pots_are_not_spendable() {
        let spendable: Vec<bool> = AccountKind::ALL
            .iter()
            .map(|kind| kind.is_spendable())
            .collect();
        assert_eq!(spendable, vec![true, true, true, false]);
    }

    #[test]
    fn round_trips_names() {
        for kind in AccountKind::ALL {
            assert_eq!(kind.as_str().parse::<AccountKind>(), Ok(kind));
        }
        assert!(
            "investment"
                .parse::<AccountKind>()
                .unwrap_err()
                .to_string()
                .contains("pot")
        );
    }
}
