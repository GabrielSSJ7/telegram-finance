//! `/desfazer` and the [Desfazer] button: only the author may undo.

use app::AppError;
use app::model::{CardPurchase, EntryId, LedgerEntry, Member, PurchaseId};
use domain::money_format::format_brl;

use super::BotContext;
use crate::gateway::{ButtonPress, GatewayError};
use crate::html::escape;

pub async fn undo_last(
    context: &BotContext,
    chat_id: i64,
    member: &Member,
) -> Result<(), GatewayError> {
    match context.services.ledger.undo_last(member.id).await {
        Ok(entry) => context
            .reply(chat_id, format!("↩️ Desfeito: {}", entry_summary(&entry)))
            .await
            .map(|_| ()),
        Err(AppError::NotFound { .. }) => {
            context.reply(chat_id, "Você não tem lançamentos para desfazer.").await.map(|_| ())
        }
        Err(error) => context.reply_error(chat_id, &error).await,
    }
}

pub async fn undo_button(
    context: &BotContext,
    press: &ButtonPress,
    member: &Member,
    entry_id: EntryId,
) -> Result<(), GatewayError> {
    let Ok(entry) = context.services.ledger.find(entry_id).await else {
        return toast(context, press, "Esse lançamento já foi desfeito.").await;
    };
    if entry.created_by != Some(member.id) {
        return toast(context, press, "Só quem registrou pode desfazer.").await;
    }
    if let Err(error) = context.services.ledger.delete(entry_id).await {
        context.gateway.answer_button(&press.callback_id, None).await?;
        return context.reply_error(press.chat_id, &error).await;
    }
    toast(context, press, "Desfeito").await?;
    context.edit(press.chat_id, press.message_id, undone_html(&entry, member), None).await
}

/// [Desfazer] under a card purchase: removes all its installments.
pub async fn undo_purchase_button(
    context: &BotContext,
    press: &ButtonPress,
    member: &Member,
    purchase_id: PurchaseId,
) -> Result<(), GatewayError> {
    let Ok(purchase) = context.services.cards.find_purchase(purchase_id).await else {
        return toast(context, press, "Essa compra já foi desfeita.").await;
    };
    if purchase.created_by != Some(member.id) {
        return toast(context, press, "Só quem registrou pode desfazer.").await;
    }
    if let Err(error) = context.services.cards.delete_purchase(purchase_id).await {
        context.gateway.answer_button(&press.callback_id, None).await?;
        return context.reply_error(press.chat_id, &error).await;
    }
    toast(context, press, "Desfeito").await?;
    context
        .edit(press.chat_id, press.message_id, undone_purchase_html(&purchase, member), None)
        .await
}

fn undone_purchase_html(purchase: &CardPurchase, member: &Member) -> String {
    let parts = if purchase.installment_count > 1 {
        format!(" em {}x", purchase.installment_count)
    } else {
        String::new()
    };
    let summary = format!("{}{parts} no cartão", format_brl(purchase.total));
    format!("↩️ <s>{summary}</s>\nDesfeito por {}.", escape(&member.display_name))
}

async fn toast(context: &BotContext, press: &ButtonPress, text: &str) -> Result<(), GatewayError> {
    context.gateway.answer_button(&press.callback_id, Some(text)).await
}

fn undone_html(entry: &LedgerEntry, member: &Member) -> String {
    format!("↩️ <s>{}</s>\nDesfeito por {}.", entry_summary(entry), escape(&member.display_name))
}

fn entry_summary(entry: &LedgerEntry) -> String {
    let description = if entry.description.is_empty() {
        String::new()
    } else {
        format!(" · {}", escape(&entry.description))
    };
    format!("{}{description}", format_brl(entry.amount))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, Utc};
    use domain::{Cents, EntryKind};

    #[test]
    fn summary_escapes_description() {
        let entry = LedgerEntry {
            id: EntryId::generate(),
            kind: EntryKind::Expense,
            amount: Cents::new(1050),
            description: "<pão>".into(),
            category_id: None,
            account_id: None,
            counter_account_id: None,
            card_purchase_id: None,
            installment_no: None,
            invoice_id: None,
            accounting_date: NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            created_by: None,
            created_at: Utc::now(),
            deleted: false,
        };
        assert_eq!(entry_summary(&entry), "R$ 10,50 · &lt;pão&gt;");
        assert_eq!(entry_summary(&LedgerEntry { description: String::new(), ..entry }), "R$ 10,50");
    }
}
