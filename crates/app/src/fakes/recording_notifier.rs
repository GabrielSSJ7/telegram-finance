use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};

use crate::model::{
    BudgetAlert, BudgetId, CardId, CreditCard, CycleReport, DailyReport, InvoiceId, InvoiceView,
    Recurrence, RecurrenceId, ReportDay,
};
use crate::ports::{HouseholdNotifier, NotifyError};

/// One message the scheduler asked to send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Notice {
    Daily(Box<DailyReport>, ReportDay),
    Cycle(Box<CycleReport>),
    InvoiceClosed(CardId, InvoiceId),
    InvoiceDue(CardId, InvoiceId, i64),
    RecurrenceRecorded(RecurrenceId, NaiveDate),
    RecurrenceToConfirm(RecurrenceId, NaiveDate),
    BackupMissing(Option<DateTime<Utc>>),
    BudgetAlert(BudgetId, u8),
}

/// Keeps every notice; can be told to fail to test retries.
#[derive(Debug, Default)]
pub struct RecordingNotifier {
    notices: Mutex<Vec<Notice>>,
    failing: AtomicBool,
}

impl RecordingNotifier {
    pub fn notices(&self) -> Vec<Notice> {
        self.notices.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone()
    }

    pub fn set_failing(&self, failing: bool) {
        self.failing.store(failing, Ordering::SeqCst);
    }

    fn push(&self, notice: Notice) -> Result<(), NotifyError> {
        if self.failing.load(Ordering::SeqCst) {
            return Err(NotifyError("telegram is down".into()));
        }
        self.notices.lock().unwrap_or_else(std::sync::PoisonError::into_inner).push(notice);
        Ok(())
    }
}

#[async_trait]
impl HouseholdNotifier for RecordingNotifier {
    async fn daily_report(&self, report: &DailyReport, day: ReportDay) -> Result<(), NotifyError> {
        self.push(Notice::Daily(Box::new(report.clone()), day))
    }

    async fn cycle_report(&self, report: &CycleReport) -> Result<(), NotifyError> {
        self.push(Notice::Cycle(Box::new(report.clone())))
    }

    async fn invoice_closed(
        &self,
        card: &CreditCard,
        invoice: &InvoiceView,
    ) -> Result<(), NotifyError> {
        self.push(Notice::InvoiceClosed(card.id, invoice.invoice.id))
    }

    async fn invoice_due(
        &self,
        card: &CreditCard,
        invoice: &InvoiceView,
        days_left: i64,
    ) -> Result<(), NotifyError> {
        self.push(Notice::InvoiceDue(card.id, invoice.invoice.id, days_left))
    }

    async fn recurrence_recorded(
        &self,
        recurrence: &Recurrence,
        date: NaiveDate,
    ) -> Result<(), NotifyError> {
        self.push(Notice::RecurrenceRecorded(recurrence.id, date))
    }

    async fn recurrence_to_confirm(
        &self,
        recurrence: &Recurrence,
        date: NaiveDate,
    ) -> Result<(), NotifyError> {
        self.push(Notice::RecurrenceToConfirm(recurrence.id, date))
    }

    async fn budget_alerts(&self, alerts: &[BudgetAlert]) -> Result<(), NotifyError> {
        for alert in alerts {
            self.push(Notice::BudgetAlert(alert.status.budget.id, alert.threshold))?;
        }
        Ok(())
    }

    async fn backup_missing(&self, last_success: Option<DateTime<Utc>>) -> Result<(), NotifyError> {
        self.push(Notice::BackupMissing(last_success))
    }
}
