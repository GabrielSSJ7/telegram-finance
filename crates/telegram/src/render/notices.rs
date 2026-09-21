//! One-off messages from scheduled jobs.

use app::model::{CreditCard, InvoiceView, Recurrence};
use chrono::{DateTime, NaiveDate, Utc};
use domain::money_format::format_brl;

use crate::html::escape;

pub fn invoice_closed_text(card: &CreditCard, invoice: &InvoiceView) -> String {
    let due = invoice.invoice.period.due_date.format("%d/%m");
    format!(
        "💳 A fatura do <b>{}</b> fechou: {}, vence {due}.\nPara pagar: /pagarfatura",
        escape(&card.name),
        format_brl(invoice.statement.outstanding)
    )
}

pub fn invoice_due_text(card: &CreditCard, invoice: &InvoiceView, days_left: i64) -> String {
    let (name, owed) = (escape(&card.name), format_brl(invoice.statement.outstanding));
    if days_left == 0 {
        return format!("⚠️ A fatura do <b>{name}</b> ({owed}) vence <b>hoje</b>! /pagarfatura");
    }
    let due = invoice.invoice.period.due_date.format("%d/%m");
    format!("⏰ A fatura do <b>{name}</b> ({owed}) vence em {days_left} dias ({due}). /pagarfatura")
}

pub fn recurrence_recorded_text(recurrence: &Recurrence, date: NaiveDate) -> String {
    format!(
        "🔁 Registrado automaticamente: <b>{}</b> {} ({})",
        escape(&recurrence.description),
        format_brl(recurrence.amount),
        date.format("%d/%m")
    )
}

pub fn recurrence_confirm_text(recurrence: &Recurrence, date: NaiveDate) -> String {
    format!(
        "🔁 <b>{}</b> (dia {}), valor previsto {}.\nRegistrar? Se o valor mudou, toque em Pular e use /gasto.",
        escape(&recurrence.description),
        date.format("%d/%m"),
        format_brl(recurrence.amount)
    )
}

pub fn backup_missing_text(last_success: Option<DateTime<Utc>>) -> String {
    let since = last_success.map_or_else(
        || "nunca rodou com sucesso".to_owned(),
        |at| format!("não roda com sucesso desde {}", at.format("%d/%m %H:%M UTC")),
    );
    format!("⚠️ O backup do finbot {since}. Veja os logs do container backup.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn backup_text_mentions_last_success() {
        assert!(backup_missing_text(None).contains("nunca"));
        let at = Utc.with_ymd_and_hms(2026, 3, 1, 6, 0, 0).unwrap();
        assert!(backup_missing_text(Some(at)).contains("01/03 06:00 UTC"));
    }
}
