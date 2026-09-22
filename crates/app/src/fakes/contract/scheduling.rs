use chrono::{DateTime, Duration, TimeZone, Utc};
use domain::{AccountKind, Cents, DayOfMonth, EntryKind};

use super::{day, member, new_category, open_account};
use crate::model::{
    AccountId, CategoryKind, NewEntry, NewRecurrence, PeriodFlow, RecurrenceKind, RecurrenceMode,
    RecurrenceTarget,
};
use crate::services::StorePorts;

async fn rent(stores: &StorePorts) -> NewRecurrence {
    let account = open_account(stores, "Conta aluguel", AccountKind::Checking, 0).await.id;
    let category = new_category(stores, "aluguel", CategoryKind::Expense).await.id;
    NewRecurrence {
        kind: RecurrenceKind::Expense,
        amount: Cents::new(250_000),
        description: "aluguel".into(),
        category_id: category,
        target: RecurrenceTarget::Account(account),
        day: DayOfMonth::new(5).unwrap(),
        mode: RecurrenceMode::Auto,
        starts_on: day(1, 1),
        plan: None,
    }
}

pub async fn recurrence_keeps_its_installment_plan(stores: StorePorts) {
    let plan = Some(domain::recurrence::InstallmentPlan { first_number: 23, count: 36 });
    let financing = NewRecurrence { plan, ..rent(&stores).await };
    let created = stores.recurrences.create_recurrence(financing).await.unwrap();
    assert_eq!(created.plan, plan);
    let found = stores.recurrences.find_recurrence(created.id).await.unwrap().unwrap();
    assert_eq!(found.plan, plan);
}

pub async fn recurrence_create_find_list_deactivate(stores: StorePorts) {
    let created = stores.recurrences.create_recurrence(rent(&stores).await).await.unwrap();
    assert!(created.active && created.last_generated_on.is_none());
    assert_eq!(
        stores.recurrences.find_recurrence(created.id).await.unwrap().as_ref(),
        Some(&created)
    );
    assert!(stores.recurrences.list_recurrences(false).await.unwrap().contains(&created));
    assert!(stores.recurrences.deactivate_recurrence(created.id).await.unwrap());
    assert!(!stores.recurrences.deactivate_recurrence(created.id).await.unwrap());
    let active = stores.recurrences.list_recurrences(false).await.unwrap();
    assert!(!active.iter().any(|row| row.id == created.id));
    assert_eq!(stores.recurrences.list_recurrences(true).await.unwrap().len(), 1);
}

pub async fn recurrence_mark_generated_only_moves_forward(stores: StorePorts) {
    let created = stores.recurrences.create_recurrence(rent(&stores).await).await.unwrap();
    stores.recurrences.mark_generated(created.id, day(3, 5)).await.unwrap();
    stores.recurrences.mark_generated(created.id, day(2, 5)).await.unwrap();
    let found = stores.recurrences.find_recurrence(created.id).await.unwrap().unwrap();
    assert_eq!(found.last_generated_on, Some(day(3, 5)));
}

fn at(minute: i64) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 3, 10, 12, 0, 0).unwrap() + Duration::minutes(minute)
}

pub async fn job_runs_once_per_day(stores: StorePorts) {
    let runs = &stores.job_runs;
    assert!(runs.claim_job("daily_report", day(3, 10), at(0), 3).await.unwrap());
    assert!(!runs.claim_job("daily_report", day(3, 10), at(1), 3).await.unwrap());
    runs.finish_job("daily_report", day(3, 10), true, at(2)).await.unwrap();
    assert!(!runs.claim_job("daily_report", day(3, 10), at(30), 3).await.unwrap());
    assert!(runs.claim_job("daily_report", day(3, 11), at(30), 3).await.unwrap());
    assert!(runs.claim_job("invoice_events", day(3, 10), at(30), 3).await.unwrap());
    assert_eq!(runs.last_success("daily_report").await.unwrap(), Some(at(2)));
    assert_eq!(runs.last_success("backup").await.unwrap(), None);
}

pub async fn job_failures_retry_until_max_attempts(stores: StorePorts) {
    let runs = &stores.job_runs;
    assert!(runs.claim_job("recurrences", day(3, 10), at(0), 2).await.unwrap());
    runs.finish_job("recurrences", day(3, 10), false, at(1)).await.unwrap();
    assert!(runs.claim_job("recurrences", day(3, 10), at(2), 2).await.unwrap());
    runs.finish_job("recurrences", day(3, 10), false, at(3)).await.unwrap();
    assert!(!runs.claim_job("recurrences", day(3, 10), at(4), 2).await.unwrap());
}

pub async fn job_stale_run_is_reclaimed(stores: StorePorts) {
    let runs = &stores.job_runs;
    assert!(runs.claim_job("cycle_report", day(3, 10), at(0), 3).await.unwrap());
    assert!(!runs.claim_job("cycle_report", day(3, 10), at(5), 3).await.unwrap());
    assert!(runs.claim_job("cycle_report", day(3, 10), at(11), 3).await.unwrap());
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
        accounting_date: day(5, on),
        created_by: None,
    }
}

/// Three expenses in May (one outside the report range) and a deleted one.
async fn may_spending(stores: &StorePorts) -> (crate::model::CategoryId, crate::model::MemberId) {
    let account = open_account(stores, "Conta relatório", AccountKind::Checking, 0).await.id;
    let category = new_category(stores, "relatório", CategoryKind::Expense).await.id;
    let author = member(stores, 9_101).await.id;
    let spent = |cents, on| NewEntry {
        category_id: Some(category),
        created_by: Some(author),
        ..entry(EntryKind::Expense, account, None, cents, on)
    };
    for row in [spent(100, 2), spent(200, 3), spent(999, 20)] {
        stores.entries.record_entry(row, None).await.unwrap();
    }
    let deleted = stores.entries.record_entry(spent(50, 2), None).await.unwrap();
    stores.entries.soft_delete_entry(deleted.id, Utc::now()).await.unwrap();
    (category, author)
}

pub async fn report_flows_group_by_category_author_and_kind(stores: StorePorts) {
    let (category, author) = may_spending(&stores).await;
    let flows = stores.reports.period_flows(day(5, 1), day(5, 10)).await.unwrap();
    let (kind, total) = (EntryKind::Expense, Cents::new(300));
    assert_eq!(
        flows,
        vec![PeriodFlow { category_id: Some(category), created_by: Some(author), kind, total }]
    );
}

pub async fn report_pot_net_inflow_counts_both_directions(stores: StorePorts) {
    let checking = open_account(&stores, "Conta pote", AccountKind::Checking, 0).await.id;
    let pot = open_account(&stores, "Pote relatório", AccountKind::Pot, 0).await.id;
    for row in [
        entry(EntryKind::Transfer, checking, Some(pot), 1_000, 2),
        entry(EntryKind::Transfer, pot, Some(checking), 300, 3),
    ] {
        stores.entries.record_entry(row, None).await.unwrap();
    }
    let outside = entry(EntryKind::Transfer, checking, Some(pot), 5_000, 25);
    stores.entries.record_entry(outside, None).await.unwrap();
    let net = stores.reports.pot_net_inflow(day(5, 1), day(5, 10)).await.unwrap();
    assert_eq!(net, Cents::new(700));
}
