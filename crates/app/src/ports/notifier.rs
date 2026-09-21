//! Messages the scheduler sends to the couple. The Telegram adapter
//! renders them in pt-BR; the jobs only decide what to say and when.

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use thiserror::Error;

use crate::model::{CreditCard, CycleReport, DailyReport, InvoiceView, Recurrence};

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("could not notify the household: {0}")]
pub struct NotifyError(pub String);

#[async_trait]
pub trait HouseholdNotifier: Send + Sync {
    async fn daily_report(&self, report: &DailyReport) -> Result<(), NotifyError>;
    async fn cycle_report(&self, report: &CycleReport) -> Result<(), NotifyError>;
    async fn invoice_closed(
        &self,
        card: &CreditCard,
        invoice: &InvoiceView,
    ) -> Result<(), NotifyError>;
    async fn invoice_due(
        &self,
        card: &CreditCard,
        invoice: &InvoiceView,
        days_left: i64,
    ) -> Result<(), NotifyError>;
    async fn recurrence_recorded(
        &self,
        recurrence: &Recurrence,
        date: NaiveDate,
    ) -> Result<(), NotifyError>;
    /// A bill that varies: ask before recording it.
    async fn recurrence_to_confirm(
        &self,
        recurrence: &Recurrence,
        date: NaiveDate,
    ) -> Result<(), NotifyError>;
    /// Sent to each member's private chat.
    async fn backup_missing(&self, last_success: Option<DateTime<Utc>>) -> Result<(), NotifyError>;
}
