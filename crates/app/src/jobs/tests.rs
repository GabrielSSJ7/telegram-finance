use std::sync::Arc;

use chrono::{NaiveDate, TimeZone, Utc};

use super::{JobKind, JobRunner, Scheduler};
use crate::fakes::requests::{
    card_purchase, confirming, monthly_expense, open_card, open_checking,
};
use crate::fakes::{FakeServiceSet, Notice, RecordingNotifier};
use crate::model::{AccountId, CategoryId, CategoryKind, EntryFilter};
use crate::ports::JobRunStore;
use crate::services::{AllowedUsers, CreateRecurrence, EntryOrigin};

struct JobFixture {
    set: FakeServiceSet,
    notifier: Arc<RecordingNotifier>,
    scheduler: Scheduler,
    runner: Arc<JobRunner>,
    checking: AccountId,
    rent: CategoryId,
}

fn date(month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, month, day).unwrap()
}

/// Sets the household clock to `hour:00` local time on `on`.
fn at_local(set: &FakeServiceSet, on: NaiveDate, hour: u32) {
    let naive = on.and_hms_opt(hour, 0, 0).unwrap();
    let local = chrono_tz::America::Sao_Paulo.from_local_datetime(&naive).single().unwrap();
    set.clock.set(local.with_timezone(&Utc));
}

fn scheduler_for(set: &FakeServiceSet, runner: &Arc<JobRunner>) -> Scheduler {
    let (settings, clock) = (set.services.settings.clone(), set.clock.clone());
    Scheduler::new(runner.clone(), set.store.clone(), settings, clock, JobKind::ALL.to_vec())
}

async fn fixture() -> JobFixture {
    let set = FakeServiceSet::new(date(3, 1), AllowedUsers::default());
    let notifier = Arc::new(RecordingNotifier::default());
    let runner = Arc::new(JobRunner::new(
        set.services.clone(),
        notifier.clone(),
        set.store.clone(),
        set.clock.clone(),
    ));
    let scheduler = scheduler_for(&set, &runner);
    let checking = set.services.accounts.open(open_checking("Nubank", 1_000_000)).await.unwrap().id;
    let rent = set.store.seed_category("aluguel", CategoryKind::Expense);
    JobFixture { set, notifier, scheduler, runner, checking, rent }
}

impl JobFixture {
    fn rent(&self) -> CreateRecurrence {
        monthly_expense("aluguel", 250_000, self.rent, self.checking, 5)
    }

    async fn entries(&self) -> usize {
        self.set.services.ledger.list(&EntryFilter::default()).await.unwrap().len()
    }

    fn count(&self, matches: impl Fn(&Notice) -> bool) -> usize {
        self.notifier.notices().iter().filter(|notice| matches(notice)).count()
    }
}

#[tokio::test]
async fn nothing_runs_before_its_time() {
    let fixture = fixture().await;
    at_local(&fixture.set, date(3, 2), 5);
    assert!(fixture.scheduler.tick().await.unwrap().is_empty());
}

#[tokio::test]
async fn each_job_runs_once_per_day() {
    let fixture = fixture().await;
    at_local(&fixture.set, date(3, 2), 22);
    let ran: Vec<JobKind> =
        fixture.scheduler.tick().await.unwrap().into_iter().map(|(kind, _)| kind).collect();
    assert_eq!(
        ran,
        vec![
            JobKind::Recurrences,
            JobKind::InvoiceEvents,
            JobKind::DailyReport,
            JobKind::BackupWatch
        ]
    );
    assert!(fixture.scheduler.tick().await.unwrap().is_empty());
    assert_eq!(fixture.count(|notice| matches!(notice, Notice::Daily(_))), 1);
}

#[tokio::test]
async fn auto_recurrence_records_once_even_after_undo() {
    let fixture = fixture().await;
    fixture.set.services.recurrences.create(fixture.rent()).await.unwrap();
    at_local(&fixture.set, date(3, 5), 7);
    fixture.scheduler.tick().await.unwrap();
    assert_eq!(fixture.entries().await, 1);
    let entry = fixture.set.services.ledger.list(&EntryFilter::default()).await.unwrap()[0].id;
    fixture.set.services.ledger.delete(entry).await.unwrap();
    fixture.runner.run(JobKind::Recurrences, date(3, 5)).await.unwrap();
    assert_eq!(fixture.entries().await, 0);
    assert_eq!(
        fixture.count(
            |notice| matches!(notice, Notice::RecurrenceRecorded(_, day) if *day == date(3, 5))
        ),
        1
    );
}

#[tokio::test]
async fn confirm_recurrence_asks_instead_of_recording() {
    let fixture = fixture().await;
    fixture.set.services.recurrences.create(confirming(fixture.rent(), date(3, 1))).await.unwrap();
    fixture.runner.run(JobKind::Recurrences, date(3, 5)).await.unwrap();
    assert_eq!(fixture.entries().await, 0);
    assert!(matches!(fixture.notifier.notices()[..], [Notice::RecurrenceToConfirm(_, _)]));
}

#[tokio::test]
async fn failed_job_is_retried_on_the_next_tick() {
    let fixture = fixture().await;
    at_local(&fixture.set, date(3, 2), 22);
    fixture.notifier.set_failing(true);
    assert!(fixture.scheduler.tick().await.unwrap().contains(&(JobKind::DailyReport, false)));
    fixture.notifier.set_failing(false);
    assert!(fixture.scheduler.tick().await.unwrap().contains(&(JobKind::DailyReport, true)));
}

#[tokio::test]
async fn cycle_report_covers_the_cycle_that_ended() {
    let fixture = fixture().await;
    at_local(&fixture.set, date(4, 1), 22);
    assert!(fixture.scheduler.tick().await.unwrap().contains(&(JobKind::CycleReport, true)));
    let cycle = fixture.notifier.notices().into_iter().find_map(|notice| match notice {
        Notice::Cycle(report) => Some(report.cycle),
        _ => None,
    });
    assert_eq!(
        cycle.map(|cycle| (cycle.start, cycle.end_exclusive)),
        Some((date(3, 1), date(4, 1)))
    );
}

#[tokio::test]
async fn backup_watch_warns_only_without_recent_backup() {
    let fixture = fixture().await;
    at_local(&fixture.set, date(3, 2), 11);
    fixture.runner.run(JobKind::BackupWatch, date(3, 2)).await.unwrap();
    assert!(matches!(fixture.notifier.notices()[..], [Notice::BackupMissing(None)]));
    let now = Utc.with_ymd_and_hms(2026, 3, 2, 6, 0, 0).unwrap();
    fixture.set.store.finish_job("backup", date(3, 2), true, now).await.unwrap();
    fixture.runner.run(JobKind::BackupWatch, date(3, 2)).await.unwrap();
    assert_eq!(fixture.notifier.notices().len(), 1);
}

#[tokio::test]
async fn invoice_events_on_closing_and_due_dates() {
    let fixture = fixture().await;
    let card = fixture.set.services.cards.open(open_card("Roxinho", 10, 20)).await.unwrap().id;
    let purchase = card_purchase(card, fixture.rent, 500, 1);
    fixture.set.services.cards.purchase(purchase, EntryOrigin::default()).await.unwrap();
    for (day, notices) in [(10, 1), (17, 2), (20, 3), (21, 3)] {
        fixture.set.clock.set_local_noon(date(3, day));
        fixture.runner.run(JobKind::InvoiceEvents, date(3, day)).await.unwrap();
        assert_eq!(fixture.notifier.notices().len(), notices, "day {day}");
    }
}

#[tokio::test]
async fn recurring_rent_can_trigger_a_budget_alert() {
    let fixture = fixture().await;
    fixture.set.services.budgets.set(fixture.rent, domain::Cents::new(200_000)).await.unwrap();
    fixture.set.services.recurrences.create(fixture.rent()).await.unwrap();
    fixture.set.clock.set_local_noon(date(3, 5));
    fixture.runner.run(JobKind::Recurrences, date(3, 5)).await.unwrap();
    assert_eq!(fixture.count(|notice| matches!(notice, Notice::BudgetAlert(_, 100))), 1);
    fixture.runner.run(JobKind::DailyReport, date(3, 5)).await.unwrap();
    assert_eq!(fixture.count(|notice| matches!(notice, Notice::BudgetAlert(..))), 1);
}
