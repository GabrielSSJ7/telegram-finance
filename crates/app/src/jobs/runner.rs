//! What each job does. Failures are returned so the scheduler records
//! them and retries on a later tick.

use std::sync::Arc;

use chrono::{Duration, NaiveDate};
use domain::invoice_settlement::InvoiceStatus;
use thiserror::Error;

use super::JobKind;
use crate::AppError;
use crate::model::{CreditCard, InvoiceView, Recurrence, RecurrenceMode, ReportDay};
use crate::ports::{Clock, HouseholdNotifier, JobRunStore, NotifyError};
use crate::services::ServiceSet;

/// The backup runs at 03:00; a day plus slack without success is a problem.
pub const MAX_BACKUP_SILENCE_HOURS: i64 = 26;

#[derive(Debug, Error)]
pub enum JobError {
    #[error(transparent)]
    App(#[from] AppError),
    #[error(transparent)]
    Notify(#[from] NotifyError),
}

impl From<crate::ports::StoreError> for JobError {
    fn from(error: crate::ports::StoreError) -> Self {
        JobError::App(error.into())
    }
}

pub struct JobRunner {
    services: ServiceSet,
    notifier: Arc<dyn HouseholdNotifier>,
    job_runs: Arc<dyn JobRunStore>,
    clock: Arc<dyn Clock>,
}

impl JobRunner {
    pub fn new(
        services: ServiceSet,
        notifier: Arc<dyn HouseholdNotifier>,
        job_runs: Arc<dyn JobRunStore>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self { services, notifier, job_runs, clock }
    }

    /// Runs `kind` for `date` now, without scheduling checks.
    pub async fn run(&self, kind: JobKind, date: NaiveDate) -> Result<(), JobError> {
        match kind {
            JobKind::Recurrences => self.recurrences(date).await,
            JobKind::InvoiceEvents => self.invoice_events(date).await,
            JobKind::TodayReport => self.today_report(date).await,
            JobKind::YesterdayReport => self.yesterday_report(date).await,
            JobKind::CycleReport => self.cycle_report(date).await,
            JobKind::BackupWatch => self.backup_watch().await,
        }
    }

    async fn recurrences(&self, date: NaiveDate) -> Result<(), JobError> {
        for (recurrence, dates) in self.services.recurrences.due(date).await? {
            for due in dates {
                self.occurrence(&recurrence, due).await?;
                self.services.recurrences.mark_generated(recurrence.id, due).await?;
            }
        }
        self.budget_alerts().await
    }

    /// Entries from the API or recurrences may have crossed a budget.
    async fn budget_alerts(&self) -> Result<(), JobError> {
        let alerts = self.services.budgets.new_alerts().await?;
        if alerts.is_empty() {
            return Ok(());
        }
        Ok(self.notifier.budget_alerts(&alerts).await?)
    }

    async fn today_report(&self, date: NaiveDate) -> Result<(), JobError> {
        self.budget_alerts().await?;
        let report = self.services.reports.daily(date).await?;
        Ok(self.notifier.daily_report(&report, ReportDay::Today).await?)
    }

    async fn yesterday_report(&self, date: NaiveDate) -> Result<(), JobError> {
        let yesterday = date.pred_opt().unwrap_or(date);
        let report = self.services.reports.daily(yesterday).await?;
        Ok(self.notifier.daily_report(&report, ReportDay::Yesterday).await?)
    }

    async fn occurrence(&self, recurrence: &Recurrence, date: NaiveDate) -> Result<(), JobError> {
        if recurrence.mode == RecurrenceMode::Confirm {
            return Ok(self.notifier.recurrence_to_confirm(recurrence, date).await?);
        }
        if self.services.recurrences.record(recurrence, date, recurrence.amount, None).await? {
            self.notifier.recurrence_recorded(recurrence, date).await?;
        }
        Ok(())
    }

    async fn invoice_events(&self, date: NaiveDate) -> Result<(), JobError> {
        for card in self.services.cards.list().await? {
            for view in self.services.cards.invoices(card.id).await? {
                self.invoice_event(&card, &view, date).await?;
            }
        }
        Ok(())
    }

    async fn invoice_event(
        &self,
        card: &CreditCard,
        view: &InvoiceView,
        date: NaiveDate,
    ) -> Result<(), JobError> {
        let period = view.invoice.period;
        if period.closing_date == date && view.statement.outstanding.is_positive() {
            self.notifier.invoice_closed(card, view).await?;
        }
        let days_left = (period.due_date - date).num_days();
        if view.statement.status == InvoiceStatus::Closed && matches!(days_left, 0 | 3) {
            self.notifier.invoice_due(card, view, days_left).await?;
        }
        Ok(())
    }

    /// On the first day of a cycle, reports the cycle that just ended.
    async fn cycle_report(&self, date: NaiveDate) -> Result<(), JobError> {
        let settings = self.services.settings.get().await?;
        let finished =
            self.services.reports.cycle_of(date).await?.previous(settings.cycle_start_day);
        Ok(self.notifier.cycle_report(&self.services.reports.cycle(finished).await?).await?)
    }

    async fn backup_watch(&self) -> Result<(), JobError> {
        let last = self.job_runs.last_success("backup").await?;
        let limit = Duration::hours(MAX_BACKUP_SILENCE_HOURS);
        if last.is_some_and(|at| self.clock.now() - at <= limit) {
            return Ok(());
        }
        Ok(self.notifier.backup_missing(last).await?)
    }
}
