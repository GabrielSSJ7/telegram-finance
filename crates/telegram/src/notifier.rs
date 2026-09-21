//! Sends scheduled messages to the household group (and backups alerts
//! to each member's private chat), rendered in pt-BR.

use std::sync::Arc;

use app::model::{BudgetAlert, CreditCard, CycleReport, DailyReport, InvoiceView, Recurrence};
use app::ports::{HouseholdNotifier, NotifyError};
use app::services::ServiceSet;
use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use domain::money_format::format_brl;

use crate::callback_data::{record_recurrence_button, skip_recurrence_button};
use crate::gateway::{Button, Keyboard, OutgoingMessage, TelegramGateway};
use crate::render::notices::{
    backup_missing_text, budget_alert_text, invoice_closed_text, invoice_due_text,
    recurrence_confirm_text, recurrence_recorded_text,
};
use crate::render::report_text::{cycle_report_text, daily_report_text};

pub struct TelegramNotifier {
    gateway: Arc<dyn TelegramGateway>,
    services: ServiceSet,
}

impl TelegramNotifier {
    pub fn new(gateway: Arc<dyn TelegramGateway>, services: ServiceSet) -> Self {
        Self { gateway, services }
    }

    /// Sends to the household group; before any group is bound there is
    /// nobody to tell, which is not a failure.
    async fn to_household(
        &self,
        html: String,
        keyboard: Option<Keyboard>,
    ) -> Result<(), NotifyError> {
        let settings =
            self.services.settings.get().await.map_err(|error| NotifyError(error.to_string()))?;
        let Some(chat_id) = settings.telegram_chat_id else {
            tracing::warn!("no household group bound yet; scheduled message dropped");
            return Ok(());
        };
        self.send(OutgoingMessage { chat_id, html, keyboard }).await
    }

    async fn send(&self, message: OutgoingMessage) -> Result<(), NotifyError> {
        self.gateway
            .send_message(&message)
            .await
            .map(|_| ())
            .map_err(|error| NotifyError(error.to_string()))
    }
}

#[async_trait]
impl HouseholdNotifier for TelegramNotifier {
    async fn daily_report(&self, report: &DailyReport) -> Result<(), NotifyError> {
        self.to_household(daily_report_text(report), None).await
    }

    async fn cycle_report(&self, report: &CycleReport) -> Result<(), NotifyError> {
        self.to_household(cycle_report_text(report), None).await
    }

    async fn invoice_closed(
        &self,
        card: &CreditCard,
        invoice: &InvoiceView,
    ) -> Result<(), NotifyError> {
        self.to_household(invoice_closed_text(card, invoice), None).await
    }

    async fn invoice_due(
        &self,
        card: &CreditCard,
        invoice: &InvoiceView,
        days_left: i64,
    ) -> Result<(), NotifyError> {
        self.to_household(invoice_due_text(card, invoice, days_left), None).await
    }

    async fn recurrence_recorded(
        &self,
        recurrence: &Recurrence,
        date: NaiveDate,
    ) -> Result<(), NotifyError> {
        self.to_household(recurrence_recorded_text(recurrence, date), None).await
    }

    async fn recurrence_to_confirm(
        &self,
        recurrence: &Recurrence,
        date: NaiveDate,
    ) -> Result<(), NotifyError> {
        let record = Button {
            label: format!("✅ Registrar {}", format_brl(recurrence.amount)),
            data: record_recurrence_button(recurrence.id, date),
        };
        let skip = Button {
            label: "⏭️ Pular".into(),
            data: skip_recurrence_button(recurrence.id, date),
        };
        let keyboard = Keyboard { rows: vec![vec![record, skip]] };
        self.to_household(recurrence_confirm_text(recurrence, date), Some(keyboard)).await
    }

    async fn budget_alerts(&self, alerts: &[BudgetAlert]) -> Result<(), NotifyError> {
        let lines: Vec<String> = alerts.iter().map(budget_alert_text).collect();
        self.to_household(lines.join("\n"), None).await
    }

    async fn backup_missing(&self, last_success: Option<DateTime<Utc>>) -> Result<(), NotifyError> {
        let members =
            self.services.members.list().await.map_err(|error| NotifyError(error.to_string()))?;
        let chats = members
            .iter()
            .filter(|member| member.receives_backups)
            .filter_map(|member| member.dm_chat_id);
        for chat_id in chats {
            self.send(OutgoingMessage {
                chat_id,
                html: backup_missing_text(last_success),
                keyboard: None,
            })
            .await?;
        }
        Ok(())
    }
}
