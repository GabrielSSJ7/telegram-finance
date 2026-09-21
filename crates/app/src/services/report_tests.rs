use chrono::NaiveDate;
use domain::{AccountKind, Cents};

use super::ledger::{AccountEntry, EntryOrigin, EntryRequest};
use super::{AllowedUsers, OpenAccount, PotMove};
use crate::fakes::FakeServiceSet;
use crate::fakes::requests::goal;
use crate::model::{AccountId, CategoryId, CategoryKind, MemberId, MemberProfile};

struct Scene {
    set: FakeServiceSet,
    checking: AccountId,
    groceries: CategoryId,
    salary: CategoryId,
    ana: MemberId,
}

fn date(month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, month, day).unwrap()
}

async fn scene() -> Scene {
    let set = FakeServiceSet::new(date(3, 10), AllowedUsers::new([11]));
    let open = OpenAccount {
        name: "Nubank".into(),
        kind: AccountKind::Checking,
        initial_balance: Cents::new(100_000),
        opened_on: None,
    };
    let checking = set.services.accounts.open(open).await.unwrap().id;
    let groceries = set.store.seed_category("mercado", CategoryKind::Expense);
    let salary = set.store.seed_category("salário", CategoryKind::Income);
    let ana = set
        .services
        .members
        .authorize(MemberProfile { telegram_user_id: 11, display_name: "Ana".into() })
        .await
        .unwrap()
        .id;
    Scene { set, checking, groceries, salary, ana }
}

impl Scene {
    async fn record(&self, income: bool, cents: i64, on: NaiveDate) {
        let category_id = if income { self.salary } else { self.groceries };
        let entry = AccountEntry {
            account_id: self.checking,
            category_id,
            amount: Cents::new(cents),
            description: String::new(),
            date: Some(on),
        };
        let request =
            if income { EntryRequest::Income(entry) } else { EntryRequest::Expense(entry) };
        let origin = EntryOrigin { created_by: Some(self.ana), draft: None };
        self.set.services.ledger.record(request, origin).await.unwrap();
    }
}

#[tokio::test]
async fn daily_report_separates_today_from_cycle() {
    let scene = scene().await;
    scene.record(true, 500_000, date(3, 5)).await;
    scene.record(false, 10_000, date(3, 9)).await;
    scene.record(false, 2_500, date(3, 10)).await;
    let report = scene.set.services.reports.daily(date(3, 10)).await.unwrap();
    assert_eq!((report.cycle.start, report.entries_today.len()), (date(3, 1), 1));
    assert_eq!(report.today.summary.spending, Cents::new(2_500));
    assert_eq!(report.cycle_to_date.summary.income, Cents::new(500_000));
    assert_eq!(report.cycle_to_date.by_member, vec![(Some(scene.ana), Cents::new(12_500))]);
    assert_eq!(report.balances.position.available, Cents::new(100_000 + 500_000 - 12_500));
}

#[tokio::test]
async fn cycle_report_compares_with_previous_and_counts_pots() {
    let scene = scene().await;
    scene.record(true, 100_000, date(2, 10)).await;
    scene.record(false, 40_000, date(3, 3)).await;
    let goal = scene.set.services.goals.create(goal("Casa", 1_000_000, 0)).await.unwrap();
    let (amount, description) = (Cents::new(30_000), String::new());
    let deposit = PotMove { goal_id: goal.id, account_id: scene.checking, amount, description };
    scene.set.services.goals.deposit(deposit, EntryOrigin::default()).await.unwrap();
    let cycle = scene.set.services.reports.cycle_of(date(3, 10)).await.unwrap();
    let report = scene.set.services.reports.cycle(cycle).await.unwrap();
    let totals = (report.totals.summary.spending, report.previous.income, report.saved_in_pots);
    assert_eq!(totals, (Cents::new(40_000), Cents::new(100_000), Cents::new(30_000)));
    assert_eq!(report.totals.by_category, vec![(Some(scene.groceries), Cents::new(40_000))]);
    assert_eq!(report.members.len(), 1);
}
