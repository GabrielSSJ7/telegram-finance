use chrono::Utc;
use domain::balance::{AccountFlow, account_balance};
use domain::{AccountKind, Cents, EntryKind};

use super::{day, open_account};
use crate::model::{AccountId, NewEntry};
use crate::ports::StoreError;
use crate::services::StorePorts;

pub async fn account_create_find_list_archive(stores: StorePorts) {
    let account = open_account(&stores, "Contrato Conta", AccountKind::Checking, 1234).await;
    let found = stores.accounts.find_account(account.id).await.unwrap();
    assert_eq!(found.as_ref(), Some(&account));
    assert!(stores.accounts.list_accounts(false).await.unwrap().contains(&account));
    assert!(stores.accounts.archive_account(account.id, Utc::now()).await.unwrap());
    assert!(!stores.accounts.archive_account(account.id, Utc::now()).await.unwrap());
    assert!(
        !stores.accounts.list_accounts(false).await.unwrap().iter().any(|row| row.id == account.id)
    );
    let archived = stores.accounts.list_accounts(true).await.unwrap();
    assert!(archived.iter().any(|row| row.id == account.id && row.archived));
    assert_eq!(stores.accounts.find_account(AccountId::generate()).await.unwrap(), None);
}

pub async fn account_active_names_are_unique(stores: StorePorts) {
    let first = open_account(&stores, "Nome Único", AccountKind::Cash, 0).await;
    let clash = super::NewAccount {
        name: "nome único".into(),
        kind: AccountKind::Savings,
        initial_balance: Cents::ZERO,
        opened_on: day(1, 1),
    };
    let error = stores.accounts.create_account(clash.clone()).await.unwrap_err();
    assert!(matches!(error, StoreError::UniqueViolation { .. }), "{error:?}");
    stores.accounts.archive_account(first.id, Utc::now()).await.unwrap();
    assert!(stores.accounts.create_account(clash).await.is_ok());
}

fn entry(
    kind: EntryKind,
    account: AccountId,
    counter: Option<AccountId>,
    cents: i64,
    on: u32,
) -> NewEntry {
    NewEntry {
        kind,
        amount: Cents::new(cents),
        description: String::new(),
        category_id: None,
        account_id: Some(account),
        counter_account_id: counter,
        invoice_id: None,
        accounting_date: day(3, on),
        created_by: None,
    }
}

async fn record_flow_rows(stores: &StorePorts, checking: AccountId, pot: AccountId) {
    let rows = [
        entry(EntryKind::AdjustIn, checking, None, 5_000, 1),
        entry(EntryKind::AdjustOut, checking, None, 700, 2),
        entry(EntryKind::Transfer, checking, Some(pot), 2_000, 3),
        entry(EntryKind::AdjustOut, checking, None, 999, 20),
    ];
    for row in rows {
        stores.entries.record_entry(row, None).await.unwrap();
    }
    let deleted =
        stores.entries.record_entry(entry(EntryKind::AdjustIn, checking, None, 50, 1), None);
    stores.entries.soft_delete_entry(deleted.await.unwrap().id, Utc::now()).await.unwrap();
}

fn balance_from_flows(flows: &[(AccountId, AccountFlow)], id: AccountId) -> Cents {
    let own: Vec<AccountFlow> =
        flows.iter().filter(|(account, _)| *account == id).map(|(_, flow)| *flow).collect();
    account_balance(Cents::ZERO, &own)
}

pub async fn account_flows_follow_entries(stores: StorePorts) {
    let checking = open_account(&stores, "Fluxo Corrente", AccountKind::Checking, 10_000).await;
    let pot = open_account(&stores, "Fluxo Pote", AccountKind::Pot, 0).await;
    record_flow_rows(&stores, checking.id, pot.id).await;
    let flows = stores.accounts.account_flows(day(3, 10)).await.unwrap();
    assert_eq!(balance_from_flows(&flows, checking.id), Cents::new(5_000 - 700 - 2_000));
    assert_eq!(balance_from_flows(&flows, pot.id), Cents::new(2_000));
}

pub async fn account_rename_keeps_names_unique(stores: StorePorts) {
    let account = open_account(&stores, "Renomear", AccountKind::Checking, 0).await;
    let other = open_account(&stores, "Ocupado", AccountKind::Checking, 0).await;
    let renamed = stores.accounts.rename_account(account.id, "Renomeada").await.unwrap();
    assert_eq!(renamed.map(|row| row.name), Some("Renomeada".to_owned()));
    let clash = stores.accounts.rename_account(account.id, &other.name).await.unwrap_err();
    assert!(matches!(clash, StoreError::UniqueViolation { .. }), "{clash:?}");
    assert!(stores.accounts.archive_account(account.id, Utc::now()).await.unwrap());
    assert_eq!(stores.accounts.rename_account(account.id, "Depois").await.unwrap(), None);
}
