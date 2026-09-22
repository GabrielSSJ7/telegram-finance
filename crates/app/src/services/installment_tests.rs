use chrono::NaiveDate;
use domain::Cents;
use domain::recurrence::InstallmentPlan;

use super::AllowedUsers;
use super::ledger::EntryOrigin;
use crate::fakes::FakeServiceSet;
use crate::fakes::requests::{card_purchase, monthly_expense, open_card, open_checking};
use crate::model::{CardId, CategoryId, CategoryKind, InstallmentProgress, PlanSource};
use crate::services::{CardPurchaseRequest, CreateRecurrence};

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).unwrap()
}

/// Today is 10/03/2026.
struct Household {
    set: FakeServiceSet,
    card: CardId,
    health: CategoryId,
}

async fn household() -> Household {
    let set = FakeServiceSet::new(date(2026, 3, 10), AllowedUsers::default());
    let card = set.services.cards.open(open_card("Itau Black", 3, 10)).await.unwrap().id;
    let health = set.store.seed_category("saúde", CategoryKind::Expense);
    Household { set, card, health }
}

impl Household {
    async fn buy(&self, request: CardPurchaseRequest) {
        self.set.services.cards.purchase(request, EntryOrigin::default()).await.unwrap();
    }

    async fn running(&self) -> Vec<InstallmentProgress> {
        self.set.services.installments.running().await.unwrap()
    }
}

#[tokio::test]
async fn card_purchases_show_what_is_paid_and_what_is_left() {
    let home = household().await;
    let medicine = CardPurchaseRequest {
        description: "Enoxaparina".into(),
        purchased_on: Some(date(2026, 3, 10)),
        ..card_purchase(home.card, home.health, 410_700, 4)
    };
    home.buy(medicine).await;
    home.buy(card_purchase(home.card, home.health, 5_000, 1)).await;
    let plans = home.running().await;
    assert_eq!(plans.len(), 1, "a 1x purchase is not a plan: {plans:?}");
    let plan = &plans[0];
    assert_eq!((plan.paid_count, plan.count, plan.paid), (1, 4, Cents::new(102_675)));
    assert_eq!((plan.remaining(), plan.last_due), (Cents::new(308_025), date(2026, 6, 10)));
    assert_eq!(plan.source, PlanSource::Card("Itau Black".into()));
}

#[tokio::test]
async fn a_purchase_entered_midway_counts_the_earlier_installments_as_paid() {
    let home = household().await;
    let sofa = CardPurchaseRequest {
        first_installment_no: 3,
        purchased_on: Some(date(2026, 1, 10)),
        ..card_purchase(home.card, home.health, 150_000, 10)
    };
    home.buy(sofa).await;
    let plan = &home.running().await[0];
    assert_eq!(
        (plan.paid_count, plan.paid, plan.installment),
        (3, Cents::new(45_000), Cents::new(15_000))
    );
}

#[tokio::test]
async fn financings_from_an_account_advance_as_they_are_generated() {
    let home = household().await;
    let account = home.set.services.accounts.open(open_checking("BTG", 0)).await.unwrap().id;
    let car = CreateRecurrence {
        starts_on: Some(date(2026, 3, 1)),
        plan: Some(InstallmentPlan { first_number: 23, count: 36 }),
        ..monthly_expense("Financiamento", 120_000, home.health, account, 5)
    };
    let financing = home.set.services.recurrences.create(car).await.unwrap();
    let plan = home.running().await.remove(0);
    assert_eq!((plan.paid_count, plan.paid), (22, Cents::new(2_640_000)));
    assert_eq!((plan.last_due, plan.source), (date(2027, 4, 5), PlanSource::Account("BTG".into())));
    home.set.services.recurrences.mark_generated(financing.id, date(2026, 3, 5)).await.unwrap();
    assert_eq!(home.running().await[0].paid_count, 23);
}
