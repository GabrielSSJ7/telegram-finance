//! "Disponível": what the couple can spend, which needs both account
//! balances and the card invoices already closed and not yet paid.

use std::sync::Arc;

use domain::balance::money_position;

use super::balances::kind_balances;
use super::{AccountService, CardService};
use crate::AppResult;
use crate::model::BalanceSheet;
use crate::ports::Clock;

pub struct PositionService {
    accounts: Arc<AccountService>,
    cards: Arc<CardService>,
    clock: Arc<dyn Clock>,
}

impl PositionService {
    pub fn new(
        accounts: Arc<AccountService>,
        cards: Arc<CardService>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self { accounts, cards, clock }
    }

    /// Balances as of today plus the money position.
    pub async fn balance_sheet(&self) -> AppResult<BalanceSheet> {
        let accounts = self.accounts.balances().await?;
        let owed = self.cards.owed().await?;
        let position =
            money_position(&kind_balances(&accounts), owed.closed_unpaid, owed.all_invoices);
        Ok(BalanceSheet { as_of: self.clock.today(), accounts, position })
    }
}
