use chrono::NaiveDate;
use domain::{Cents, DayOfMonth};

use super::{AllowedUsers, CreateRecurrence};
use crate::AppError;
use crate::fakes::FakeServiceSet;
use crate::fakes::requests::{open_card, open_checking};
use crate::model::{
    AccountId, CardId, CategoryId, CategoryKind, EntryFilter, RecurrenceKind, RecurrenceMode,
    RecurrenceTarget,
};

struct Setup {
    set: FakeServiceSet,
    checking: AccountId,
    card: CardId,
    salary: CategoryId,
    streaming: CategoryId,
}

fn date(month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, month, day).unwrap()
}

async fn setup() -> Setup {
    let set = FakeServiceSet::new(date(3, 1), AllowedUsers::default());
    let checking = set.services.accounts.open(open_checking("Nubank", 0)).await.unwrap().id;
    let card = set.services.cards.open(open_card("Roxinho", 3, 10)).await.unwrap().id;
    let salary = set.store.seed_category("salário", CategoryKind::Income);
    let streaming = set.store.seed_category("assinaturas", CategoryKind::Expense);
    Setup { set, checking, card, salary, streaming }
}

fn salary(setup: &Setup) -> CreateRecurrence {
    CreateRecurrence {
        kind: RecurrenceKind::Income,
        amount: Cents::new(800_000),
        description: "Salário".into(),
        category_id: setup.salary,
        target: RecurrenceTarget::Account(setup.checking),
        day: DayOfMonth::new(5).unwrap(),
        mode: RecurrenceMode::Auto,
        starts_on: None,
    }
}

#[tokio::test]
async fn income_and_card_subscription_record_once_per_date() {
    let setup = setup().await;
    let recurrences = &setup.set.services.recurrences;
    let income = recurrences.create(salary(&setup)).await.unwrap();
    let request = CreateRecurrence {
        kind: RecurrenceKind::Expense,
        category_id: setup.streaming,
        target: RecurrenceTarget::Card(setup.card),
        ..salary(&setup)
    };
    let subscription = recurrences.create(request).await.unwrap();
    assert!(recurrences.record(&income, date(3, 5), income.amount, None).await.unwrap());
    assert!(!recurrences.record(&income, date(3, 5), income.amount, None).await.unwrap());
    assert!(recurrences.record(&subscription, date(3, 5), Cents::new(3_990), None).await.unwrap());
    assert_eq!(setup.set.services.ledger.list(&EntryFilter::default()).await.unwrap().len(), 2);
}

#[tokio::test]
async fn validation_rules() {
    let setup = setup().await;
    let recurrences = &setup.set.services.recurrences;
    let zero = CreateRecurrence { amount: Cents::ZERO, ..salary(&setup) };
    assert!(matches!(
        recurrences.create(zero).await,
        Err(AppError::Invalid { field: "amount", .. })
    ));
    let wrong_category = CreateRecurrence { category_id: setup.streaming, ..salary(&setup) };
    assert!(recurrences.create(wrong_category).await.is_err());
    let income_on_card =
        CreateRecurrence { target: RecurrenceTarget::Card(setup.card), ..salary(&setup) };
    assert!(matches!(recurrences.create(income_on_card).await, Err(AppError::Invalid { .. })));
    let unknown_account = CreateRecurrence {
        target: RecurrenceTarget::Account(AccountId::generate()),
        ..salary(&setup)
    };
    assert!(matches!(recurrences.create(unknown_account).await, Err(AppError::NotFound { .. })));
}

#[tokio::test]
async fn due_upcoming_and_deactivate() {
    let setup = setup().await;
    let recurrences = &setup.set.services.recurrences;
    let created = recurrences.create(salary(&setup)).await.unwrap();
    assert_eq!(recurrences.due(date(3, 6)).await.unwrap()[0].1, vec![date(3, 5)]);
    recurrences.mark_generated(created.id, date(3, 5)).await.unwrap();
    assert!(recurrences.due(date(3, 6)).await.unwrap().is_empty());
    let upcoming = recurrences.upcoming(date(4, 3), 3).await.unwrap();
    assert_eq!(upcoming.iter().map(|(_, day)| *day).collect::<Vec<_>>(), vec![date(4, 5)]);
    recurrences.deactivate(created.id).await.unwrap();
    assert!(recurrences.deactivate(created.id).await.is_err());
    assert_eq!(recurrences.list(true).await.unwrap().len(), 1);
    assert!(!recurrences.find(created.id).await.unwrap().active);
}
