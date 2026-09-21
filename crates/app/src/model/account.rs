use chrono::NaiveDate;
use domain::balance::MoneyPosition;
use domain::{AccountKind, Cents};
use serde::{Deserialize, Serialize};

use super::AccountId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    pub id: AccountId,
    pub name: String,
    pub kind: AccountKind,
    pub initial_balance: Cents,
    pub opened_on: NaiveDate,
    pub archived: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewAccount {
    pub name: String,
    pub kind: AccountKind,
    pub initial_balance: Cents,
    pub opened_on: NaiveDate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountBalance {
    pub account: Account,
    pub balance: Cents,
}

/// Every active account with its balance, plus the derived position.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BalanceSheet {
    pub as_of: NaiveDate,
    pub accounts: Vec<AccountBalance>,
    pub position: MoneyPosition,
}
