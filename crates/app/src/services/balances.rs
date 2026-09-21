//! Joins accounts with their aggregated flows into balances.

use std::collections::HashMap;

use domain::balance::{AccountFlow, KindBalance, account_balance};

use crate::model::{Account, AccountBalance, AccountId};

/// Balance of each account from the flows the store aggregated.
pub fn balances_from_flows(
    accounts: Vec<Account>,
    flows: &[(AccountId, AccountFlow)],
) -> Vec<AccountBalance> {
    let mut by_account: HashMap<AccountId, Vec<AccountFlow>> = HashMap::new();
    for (account_id, flow) in flows {
        by_account.entry(*account_id).or_default().push(*flow);
    }
    accounts
        .into_iter()
        .map(|account| {
            let account_flows = by_account.get(&account.id).map_or(&[][..], Vec::as_slice);
            let balance = account_balance(account.initial_balance, account_flows);
            AccountBalance { account, balance }
        })
        .collect()
}

pub fn kind_balances(balances: &[AccountBalance]) -> Vec<KindBalance> {
    balances
        .iter()
        .map(|item| KindBalance { kind: item.account.kind, balance: item.balance })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use domain::entry_kind::AccountRole;
    use domain::{AccountKind, Cents, EntryKind};

    fn account(kind: AccountKind, initial: i64) -> Account {
        Account {
            id: AccountId::generate(),
            name: "conta".into(),
            kind,
            initial_balance: Cents::new(initial),
            opened_on: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            archived: false,
        }
    }

    #[test]
    fn applies_flows_to_matching_account_only() {
        let checking = account(AccountKind::Checking, 1000);
        let pot = account(AccountKind::Pot, 0);
        let flow = AccountFlow {
            kind: EntryKind::Transfer,
            role: AccountRole::Counter,
            total: Cents::new(300),
        };
        let balances = balances_from_flows(vec![checking.clone(), pot.clone()], &[(pot.id, flow)]);
        assert_eq!(balances[0].balance, Cents::new(1000));
        assert_eq!(balances[1].balance, Cents::new(300));
        let kinds = kind_balances(&balances);
        assert_eq!(kinds[1], KindBalance { kind: AccountKind::Pot, balance: Cents::new(300) });
    }
}
