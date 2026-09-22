//! `/extrato [mm/aaaa]`: spending and income per category for a cycle;
//! each category button lists the entries behind its total.

use app::model::{CategoryId, EntryFilter};
use app::services::category_activity::category_activity;
use app::{AppError, AppResult};
use chrono::NaiveDate;

use super::BotContext;
use super::periods::cycle_from_args;
use crate::gateway::{ButtonPress, GatewayError, Keyboard};
use crate::render::category_statement::{
    StatementPeriod, category_entries_text, statement_overview,
};

const USAGE: &str = "Use /extrato para o ciclo atual ou /extrato 08/2026 para o ciclo que começa em agosto de 2026.";

/// More than a couple ever records in a cycle; the list is cut for chat
/// anyway, and the full list is in `/exportar`.
const MAX_STATEMENT_ENTRIES: u32 = 5_000;

pub async fn category_overview(
    context: &BotContext,
    chat_id: i64,
    args: &str,
) -> Result<(), GatewayError> {
    let cycle = match cycle_from_args(context, args).await {
        Ok(Some(cycle)) => cycle,
        Ok(None) => return context.reply(chat_id, USAGE).await.map(|_| ()),
        Err(error) => return context.reply_error(chat_id, &error).await,
    };
    let period = StatementPeriod { from: cycle.start, to: cycle.last_day() };
    match overview(context, period).await {
        Ok((html, keyboard)) => context.send(chat_id, html, keyboard).await.map(|_| ()),
        Err(error) => context.reply_error(chat_id, &error).await,
    }
}

async fn overview(
    context: &BotContext,
    period: StatementPeriod,
) -> AppResult<(String, Option<Keyboard>)> {
    let entries = context.services.ledger.list(&period_filter(period, None)).await?;
    let categories = context.services.categories.list(None).await?;
    Ok(statement_overview(&category_activity(&entries, &categories), period))
}

/// A category button: sends its entries as a new message, so the overview
/// stays for the next tap.
pub async fn category_entries(
    context: &BotContext,
    press: &ButtonPress,
    category: CategoryId,
    (from, to): (NaiveDate, NaiveDate),
) -> Result<(), GatewayError> {
    context.gateway.answer_button(&press.callback_id, None).await?;
    match entries_text(context, category, StatementPeriod { from, to }).await {
        Ok(html) => context.send(press.chat_id, html, None).await.map(|_| ()),
        Err(error) => context.reply_error(press.chat_id, &error).await,
    }
}

async fn entries_text(
    context: &BotContext,
    id: CategoryId,
    period: StatementPeriod,
) -> AppResult<String> {
    let categories = context.services.categories.list(None).await?;
    let category = categories
        .into_iter()
        .find(|category| category.id == id)
        .ok_or_else(|| AppError::not_found("category", id))?;
    let mut entries = context.services.ledger.list(&period_filter(period, Some(id))).await?;
    // The ledger lists newest first; a statement reads oldest first.
    entries.reverse();
    let members = context.services.members.list().await?;
    Ok(category_entries_text(&category, &entries, &members, period))
}

fn period_filter(period: StatementPeriod, category_id: Option<CategoryId>) -> EntryFilter {
    EntryFilter {
        from: Some(period.from),
        to_inclusive: Some(period.to),
        category_id,
        limit: MAX_STATEMENT_ENTRIES,
        ..EntryFilter::default()
    }
}
