use app::model::{Account, AccountBalance, BalanceSheet};
use app::services::OpenAccount;
use chrono::NaiveDate;
use domain::{AccountKind, Cents};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
pub struct AccountResponse {
    pub id: Uuid,
    #[schema(example = "Nubank")]
    pub name: String,
    #[schema(value_type = String, example = "checking")]
    pub kind: AccountKind,
    #[schema(example = 150_000)]
    pub initial_balance_cents: i64,
    pub opened_on: NaiveDate,
    pub archived: bool,
}

impl From<Account> for AccountResponse {
    fn from(account: Account) -> Self {
        Self {
            id: account.id.0,
            name: account.name,
            kind: account.kind,
            initial_balance_cents: account.initial_balance.value(),
            opened_on: account.opened_on,
            archived: account.archived,
        }
    }
}

/// Opens a checking, savings or cash account. Pots come with goals.
#[derive(Debug, Deserialize, ToSchema)]
pub struct OpenAccountBody {
    #[schema(example = "Nubank")]
    pub name: String,
    #[schema(value_type = String, example = "checking")]
    pub kind: AccountKind,
    #[serde(default)]
    #[schema(example = 150_000)]
    pub initial_balance_cents: i64,
    /// Defaults to today.
    pub opened_on: Option<NaiveDate>,
}

impl From<OpenAccountBody> for OpenAccount {
    fn from(body: OpenAccountBody) -> Self {
        let initial_balance = Cents::new(body.initial_balance_cents);
        OpenAccount { name: body.name, kind: body.kind, initial_balance, opened_on: body.opened_on }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ListAccountsQuery {
    #[serde(default)]
    pub include_archived: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AccountBalanceResponse {
    pub account: AccountResponse,
    pub balance_cents: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BalanceSheetResponse {
    pub as_of: NaiveDate,
    pub accounts: Vec<AccountBalanceResponse>,
    /// Spendable accounts minus closed unpaid card invoices ("Disponível").
    pub available_cents: i64,
    /// Money in goal pots ("Reservado em metas").
    pub reserved_in_pots_cents: i64,
}

impl From<AccountBalance> for AccountBalanceResponse {
    fn from(item: AccountBalance) -> Self {
        Self { account: item.account.into(), balance_cents: item.balance.value() }
    }
}

impl From<BalanceSheet> for BalanceSheetResponse {
    fn from(sheet: BalanceSheet) -> Self {
        Self {
            as_of: sheet.as_of,
            accounts: sheet.accounts.into_iter().map(Into::into).collect(),
            available_cents: sheet.position.available.value(),
            reserved_in_pots_cents: sheet.position.reserved_in_pots.value(),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RenameBody {
    #[schema(example = "Nubank da Bia")]
    pub name: String,
}

/// What the bank shows for the account now; the difference becomes an
/// adjustment entry.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ReconcileBody {
    #[schema(example = 95_000)]
    pub actual_balance_cents: i64,
}
