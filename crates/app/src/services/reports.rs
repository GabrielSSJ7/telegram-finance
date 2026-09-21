//! Builds the daily report and the cycle ("mês financeiro") report.

use std::sync::Arc;

use chrono::{Days, NaiveDate};
use domain::cycle::Cycle;
use domain::spend::{CategoryFlow, spending_by_category, summarize_period};

use super::{
    CardService, CategoryService, GoalService, LedgerService, MemberService, PositionService,
};
use super::{RecurrenceService, SettingsService};
use crate::AppResult;
use crate::model::{
    BalanceSheet, CardSummary, Category, CycleReport, DailyReport, EntryFilter, GoalProgress,
    Member, PeriodFlow, PeriodTotals,
};
use crate::ports::ReportStore;

/// Days of recurrences listed as "coming up" in the daily report.
pub const UPCOMING_DAYS: u64 = 3;

/// The services a report reads from.
pub struct ReportSources {
    pub position: Arc<PositionService>,
    pub cards: Arc<CardService>,
    pub goals: Arc<GoalService>,
    pub recurrences: Arc<RecurrenceService>,
    pub categories: Arc<CategoryService>,
    pub members: Arc<MemberService>,
    pub settings: Arc<SettingsService>,
    pub ledger: Arc<LedgerService>,
}

pub struct ReportService {
    store: Arc<dyn ReportStore>,
    sources: ReportSources,
}

/// Parts shared by both reports.
struct Snapshot {
    balances: BalanceSheet,
    cards: Vec<CardSummary>,
    goals: Vec<GoalProgress>,
    categories: Vec<Category>,
    members: Vec<Member>,
}

impl ReportService {
    pub fn new(store: Arc<dyn ReportStore>, sources: ReportSources) -> Self {
        Self { store, sources }
    }

    /// The cycle that contains `date`, per the household settings.
    pub async fn cycle_of(&self, date: NaiveDate) -> AppResult<Cycle> {
        let settings = self.sources.settings.get().await?;
        Ok(Cycle::containing(date, settings.cycle_start_day))
    }

    pub async fn daily(&self, date: NaiveDate) -> AppResult<DailyReport> {
        let cycle = self.cycle_of(date).await?;
        let tomorrow = date.checked_add_days(Days::new(1)).unwrap_or(date);
        let filter = EntryFilter {
            from: Some(date),
            to_inclusive: Some(date),
            limit: 100,
            ..EntryFilter::default()
        };
        let today = DailyParts {
            date,
            cycle,
            entries_today: self.sources.ledger.list(&filter).await?,
            today: self.totals(date, tomorrow).await?,
            cycle_to_date: self.totals(cycle.start, tomorrow).await?,
            upcoming: self.sources.recurrences.upcoming(date, UPCOMING_DAYS).await?,
        };
        Ok(today.into_report(self.snapshot().await?))
    }

    pub async fn cycle(&self, cycle: Cycle) -> AppResult<CycleReport> {
        let settings = self.sources.settings.get().await?;
        let previous = cycle.previous(settings.cycle_start_day);
        let snapshot = self.snapshot().await?;
        Ok(CycleReport {
            cycle,
            totals: self.totals(cycle.start, cycle.end_exclusive).await?,
            previous: self.totals(previous.start, previous.end_exclusive).await?.summary,
            saved_in_pots: self.store.pot_net_inflow(cycle.start, cycle.end_exclusive).await?,
            balances: snapshot.balances,
            cards: snapshot.cards,
            goals: snapshot.goals,
            categories: snapshot.categories,
            members: snapshot.members,
        })
    }

    async fn totals(&self, from: NaiveDate, to_exclusive: NaiveDate) -> AppResult<PeriodTotals> {
        let flows = self.store.period_flows(from, to_exclusive).await?;
        Ok(period_totals(&flows))
    }

    async fn snapshot(&self) -> AppResult<Snapshot> {
        Ok(Snapshot {
            balances: self.sources.position.balance_sheet().await?,
            cards: self.sources.cards.summaries().await?,
            goals: self.sources.goals.list_progress().await?,
            categories: self.sources.categories.list(None).await?,
            members: self.sources.members.list().await?,
        })
    }
}

/// The day-specific half of a daily report.
struct DailyParts {
    date: NaiveDate,
    cycle: Cycle,
    entries_today: Vec<crate::model::LedgerEntry>,
    today: PeriodTotals,
    cycle_to_date: PeriodTotals,
    upcoming: Vec<(crate::model::Recurrence, NaiveDate)>,
}

impl DailyParts {
    fn into_report(self, snapshot: Snapshot) -> DailyReport {
        DailyReport {
            date: self.date,
            cycle: self.cycle,
            entries_today: self.entries_today,
            today: self.today,
            cycle_to_date: self.cycle_to_date,
            upcoming: self.upcoming,
            balances: snapshot.balances,
            cards: snapshot.cards,
            goals: snapshot.goals,
            categories: snapshot.categories,
            members: snapshot.members,
        }
    }
}

/// Income, spending and savings, plus spending per category and person.
pub fn period_totals(flows: &[PeriodFlow]) -> PeriodTotals {
    let by_category: Vec<_> = flows
        .iter()
        .map(|flow| CategoryFlow { category: flow.category_id, kind: flow.kind, total: flow.total })
        .collect();
    let by_member: Vec<_> = flows
        .iter()
        .map(|flow| CategoryFlow { category: flow.created_by, kind: flow.kind, total: flow.total })
        .collect();
    PeriodTotals {
        summary: summarize_period(&by_category),
        by_category: spending_by_category(&by_category),
        by_member: spending_by_category(&by_member),
    }
}
