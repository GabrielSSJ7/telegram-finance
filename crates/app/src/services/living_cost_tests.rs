use chrono::NaiveDate;
use domain::Cents;

use super::AllowedUsers;
use super::categories::CreateCategory;
use super::ledger::{AccountEntry, EntryOrigin, EntryRequest};
use crate::fakes::FakeServiceSet;
use crate::fakes::requests::{goal, monthly_expense, open_checking};
use crate::model::{AccountId, Category, CategoryKind, RecurrenceKind};
use crate::services::OpenAccount;

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).unwrap()
}

struct Household {
    set: FakeServiceSet,
    account: AccountId,
    home: Category,
    fun: Category,
}

/// Today is 10/03/2026 and cycles start on the 1st. The account exists
/// since December, so December to February count as tracked cycles.
async fn household() -> Household {
    let set = FakeServiceSet::new(date(2026, 3, 10), AllowedUsers::default());
    let opened = OpenAccount { opened_on: Some(date(2025, 12, 1)), ..open_checking("Nubank", 0) };
    let account = set.services.accounts.open(opened).await.unwrap().id;
    let category = |name: &str, essential| CreateCategory {
        name: name.into(),
        kind: CategoryKind::Expense,
        emoji: None,
        essential,
    };
    let home = set.services.categories.create(category("casa", true)).await.unwrap();
    let fun = set.services.categories.create(category("lazer", false)).await.unwrap();
    Household { set, account, home, fun }
}

impl Household {
    async fn spend(&self, category: &Category, cents: i64, day: NaiveDate) {
        let entry = AccountEntry {
            account_id: self.account,
            category_id: category.id,
            amount: Cents::new(cents),
            description: String::new(),
            date: Some(day),
        };
        let request = EntryRequest::Expense(entry);
        self.set.services.ledger.record(request, EntryOrigin::default()).await.unwrap();
    }

    /// An R$ 8.000 salary recurrence on `day`.
    async fn expect_salary_on(&self, day: u8) {
        let income = CreateCategory {
            name: "salário".into(),
            kind: CategoryKind::Income,
            emoji: None,
            essential: false,
        };
        let salary = self.set.services.categories.create(income).await.unwrap();
        let request = super::CreateRecurrence {
            kind: RecurrenceKind::Income,
            ..monthly_expense("salário", 800_000, salary.id, self.account, day)
        };
        self.set.services.recurrences.create(request).await.unwrap();
    }

    async fn cost(&self) -> crate::model::LivingCost {
        let services = &self.set.services;
        let cycle = services.reports.cycle_of(date(2026, 3, 10)).await.unwrap();
        services.living_costs.for_cycle(cycle).await.unwrap()
    }
}

#[tokio::test]
async fn counts_only_essential_spending_and_the_bills_still_due() {
    let home = household().await;
    home.spend(&home.home, 100_000, date(2026, 3, 5)).await;
    home.spend(&home.fun, 50_000, date(2026, 3, 6)).await;
    let rent = monthly_expense("aluguel", 200_000, home.home.id, home.account, 25);
    home.set.services.recurrences.create(rent).await.unwrap();
    home.expect_salary_on(28).await;
    let cost = home.cost().await;
    assert_eq!((cost.spent, cost.still_coming), (Cents::new(100_000), Cents::new(200_000)));
    assert_eq!(cost.by_category, vec![(home.home.clone(), Cents::new(300_000))]);
    assert_eq!(cost.expected_income, Cents::new(800_000));
    assert_eq!(cost.income_share_bp(), Some(3_750));
}

#[tokio::test]
async fn averages_tracked_cycles_and_reads_the_reserve() {
    let home = household().await;
    home.spend(&home.home, 300_000, date(2026, 2, 10)).await;
    home.spend(&home.home, 150_000, date(2026, 1, 10)).await;
    home.set.services.goals.create(goal("reserva", 5_000_000, 900_000)).await.unwrap();
    let cost = home.cost().await;
    assert_eq!((cost.recent_average, cost.averaged_cycles), (Some(Cents::new(150_000)), 3));
    assert_eq!(cost.reserved, Cents::new(900_000));
    assert_eq!(cost.reserve_tenths_of_month(), Some(60));
}

#[tokio::test]
async fn cycles_before_tracking_started_are_not_averaged() {
    let set = FakeServiceSet::new(date(2026, 3, 10), AllowedUsers::default());
    set.services.accounts.open(open_checking("Nubank", 0)).await.unwrap();
    let cycle = set.services.reports.cycle_of(date(2026, 3, 10)).await.unwrap();
    let cost = set.services.living_costs.for_cycle(cycle).await.unwrap();
    assert_eq!((cost.recent_average, cost.averaged_cycles), (None, 0));
    assert_eq!(cost.projected(), Cents::ZERO);
}
