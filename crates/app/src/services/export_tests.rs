use chrono::NaiveDate;
use domain::Cents;

use super::AllowedUsers;
use super::ledger::{AccountEntry, EntryOrigin, EntryRequest};
use crate::fakes::FakeServiceSet;
use crate::fakes::requests::{card_purchase, open_card, open_checking};
use crate::model::{AccountId, CategoryId, CategoryKind, MemberProfile};

fn date(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 3, day).unwrap()
}

#[tokio::test]
async fn export_lists_entries_with_names_oldest_first() {
    let set = FakeServiceSet::new(date(10), AllowedUsers::new([11]));
    let checking = set.services.accounts.open(open_checking("Nubank", 0)).await.unwrap().id;
    let groceries = set.store.seed_category("mercado", CategoryKind::Expense);
    let ana = set.services.members.authorize(ana_profile()).await.unwrap().id;
    let origin = EntryOrigin { created_by: Some(ana), draft: None };
    for (cents, day) in [(1050, 5), (200, 2)] {
        let expense = EntryRequest::Expense(feira(checking, groceries, cents, date(day)));
        set.services.ledger.record(expense, origin).await.unwrap();
    }
    let csv = set.services.exports.entries_csv(date(1), date(31)).await.unwrap();
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines.len(), 3, "{csv}");
    assert_eq!(lines[1], "02/03/2026;Gasto;feira;mercado;Nubank;;;;2,00;Ana");
    assert_eq!(lines[2], "05/03/2026;Gasto;feira;mercado;Nubank;;;;10,50;Ana");
}

fn ana_profile() -> MemberProfile {
    MemberProfile { telegram_user_id: 11, display_name: "Ana".into() }
}

fn feira(
    account_id: AccountId,
    category_id: CategoryId,
    cents: i64,
    day: NaiveDate,
) -> AccountEntry {
    AccountEntry {
        account_id,
        category_id,
        amount: Cents::new(cents),
        description: "feira".into(),
        date: Some(day),
    }
}

#[tokio::test]
async fn export_names_the_card_of_installments() {
    let set = FakeServiceSet::new(date(10), AllowedUsers::default());
    let card = set.services.cards.open(open_card("Roxinho", 3, 10)).await.unwrap().id;
    let groceries = set.store.seed_category("mercado", CategoryKind::Expense);
    set.services
        .cards
        .purchase(card_purchase(card, groceries, 900, 3), EntryOrigin::default())
        .await
        .unwrap();
    let csv = set
        .services
        .exports
        .entries_csv(date(1), NaiveDate::from_ymd_opt(2026, 6, 30).unwrap())
        .await
        .unwrap();
    assert!(
        csv.contains("10/03/2026;Parcela de cartão;;mercado;;;Roxinho;1;3,00;automático"),
        "{csv}"
    );
    assert_eq!(csv.lines().count(), 4);
}
