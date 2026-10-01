//! `/extrato [mm/aaaa]`: what the cycle moved, grouped by category, by
//! account or by card. Each total opens the entries behind it.

use app::model::{Category, CategoryKind, CreditCard, EntryFilter, LedgerEntry};
use app::services::category_activity::{
    account_activity, account_effect, card_charges, category_activity, category_effect,
};
use app::{AppError, AppResult};
use chrono::NaiveDate;
use domain::Cents;

use super::BotContext;
use super::periods::cycle_from_args;
use crate::callback_data::StatementItem;
use crate::gateway::{ButtonPress, GatewayError, Keyboard};
use crate::render::catalog::{account_label, card_label, category_label};
use crate::render::statement::{
    StatementGroup, StatementPeriod, StatementRow, StatementSection, statement_entries_text,
    statement_overview,
};

const USAGE: &str = "Use /extrato para o ciclo atual ou /extrato 08/2026 para o ciclo que começa em agosto de 2026.";

/// More than a couple ever records in a cycle; the list is cut for chat
/// anyway, and the full list is in `/exportar`.
const MAX_STATEMENT_ENTRIES: u32 = 5_000;

pub async fn statement_command(
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
    match overview(context, StatementGroup::Category, period).await {
        Ok((html, keyboard)) => context.send(chat_id, html, keyboard).await.map(|_| ()),
        Err(error) => context.reply_error(chat_id, &error).await,
    }
}

/// A grouping button: redraws the same message the other way round.
pub async fn switch_grouping(
    context: &BotContext,
    press: &ButtonPress,
    group: StatementGroup,
    period: StatementPeriod,
) -> Result<(), GatewayError> {
    context.gateway.answer_button(&press.callback_id, None).await?;
    match overview(context, group, period).await {
        Ok((html, keyboard)) => context.edit(press.chat_id, press.message_id, html, keyboard).await,
        Err(error) => context.reply_error(press.chat_id, &error).await,
    }
}

/// A total button: sends its entries as a new message, so the overview
/// stays for the next tap.
pub async fn statement_entries(
    context: &BotContext,
    press: &ButtonPress,
    item: StatementItem,
    period: StatementPeriod,
) -> Result<(), GatewayError> {
    context.gateway.answer_button(&press.callback_id, None).await?;
    match entries_text(context, item, period).await {
        Ok(html) => context.send(press.chat_id, html, None).await.map(|_| ()),
        Err(error) => context.reply_error(press.chat_id, &error).await,
    }
}

async fn overview(
    context: &BotContext,
    group: StatementGroup,
    period: StatementPeriod,
) -> AppResult<(String, Option<Keyboard>)> {
    let sections = match group {
        StatementGroup::Category => category_sections(context, period).await?,
        StatementGroup::Account => account_sections(context, period).await?,
        StatementGroup::Card => card_sections(context, period).await?,
    };
    Ok(statement_overview(group, period, &sections))
}

/// Spending and income in two blocks, as the reports read them.
async fn category_sections(
    context: &BotContext,
    period: StatementPeriod,
) -> AppResult<Vec<StatementSection>> {
    let entries = context.services.ledger.list(&period_filter(period, None)).await?;
    let categories = context.services.categories.list(None).await?;
    let activity = category_activity(&entries, &categories);
    let block = |title, kind| StatementSection {
        title: Some(title),
        rows: activity
            .iter()
            .filter(|item| item.category.kind == kind)
            .map(|item| StatementRow {
                item: StatementItem::Category(item.category.id),
                label: category_label(&item.category),
                total: item.total,
                entries: item.entries,
            })
            .collect(),
    };
    Ok(vec![block("💸 Gastos", CategoryKind::Expense), block("💰 Entradas", CategoryKind::Income)])
}

/// What each account took in minus what left it.
async fn account_sections(
    context: &BotContext,
    period: StatementPeriod,
) -> AppResult<Vec<StatementSection>> {
    let entries = context.services.ledger.list(&period_filter(period, None)).await?;
    let accounts = context.services.accounts.list(false).await?;
    let rows = account_activity(&entries, &accounts)
        .into_iter()
        .map(|item| StatementRow {
            item: StatementItem::Account(item.account.id),
            label: account_label(&item.account),
            total: item.total,
            entries: item.entries,
        })
        .collect();
    Ok(vec![StatementSection { title: None, rows }])
}

/// What was charged to each card, installments minus credits.
async fn card_sections(
    context: &BotContext,
    period: StatementPeriod,
) -> AppResult<Vec<StatementSection>> {
    let mut rows = Vec::new();
    for card in context.services.cards.list().await? {
        let entries = context.services.ledger.list(&card_filter(period, &card)).await?;
        if entries.is_empty() {
            continue;
        }
        rows.push(StatementRow {
            item: StatementItem::Card(card.id),
            label: card_label(&card),
            total: card_charges(&entries),
            entries: entries.len(),
        });
    }
    rows.sort_by_key(|row| -row.total.value());
    Ok(vec![StatementSection { title: None, rows }])
}

async fn entries_text(
    context: &BotContext,
    item: StatementItem,
    period: StatementPeriod,
) -> AppResult<String> {
    let members = context.services.members.list().await?;
    let (heading, entries, effect) = item_entries(context, item, period).await?;
    Ok(statement_entries_text(&heading, &entries, &members, period, effect))
}

type SignedEntries = (String, Vec<LedgerEntry>, Box<dyn Fn(&LedgerEntry) -> Cents>);

/// The entries behind one total, oldest first, with the sign its grouping
/// reads them by.
async fn item_entries(
    context: &BotContext,
    item: StatementItem,
    period: StatementPeriod,
) -> AppResult<SignedEntries> {
    match item {
        StatementItem::Category(id) => category_entries(context, id, period).await,
        StatementItem::Account(id) => account_entries(context, id, period).await,
        StatementItem::Card(id) => card_entries(context, id, period).await,
    }
}

async fn category_entries(
    context: &BotContext,
    id: app::model::CategoryId,
    period: StatementPeriod,
) -> AppResult<SignedEntries> {
    let category = active_category(context, id).await?;
    let kind = category.kind;
    let entries = oldest_first(context, period_filter(period, Some(id))).await?;
    Ok((category_label(&category), entries, Box::new(move |entry| category_effect(entry, kind))))
}

async fn account_entries(
    context: &BotContext,
    id: app::model::AccountId,
    period: StatementPeriod,
) -> AppResult<SignedEntries> {
    let account = context.services.accounts.require_active(id).await?;
    let filter = EntryFilter { account_id: Some(id), ..period_filter(period, None) };
    let entries = oldest_first(context, filter).await?;
    Ok((account_label(&account), entries, Box::new(move |entry| account_effect(entry, id))))
}

async fn card_entries(
    context: &BotContext,
    id: app::model::CardId,
    period: StatementPeriod,
) -> AppResult<SignedEntries> {
    let card = context.services.cards.require_active(id).await?;
    let entries = oldest_first(context, card_filter(period, &card)).await?;
    let effect = |entry: &LedgerEntry| entry.amount.times(entry.kind.spend_effect());
    Ok((card_label(&card), entries, Box::new(effect)))
}

async fn active_category(context: &BotContext, id: app::model::CategoryId) -> AppResult<Category> {
    let categories = context.services.categories.list(None).await?;
    categories
        .into_iter()
        .find(|category| category.id == id)
        .ok_or_else(|| AppError::not_found("category", id))
}

/// A statement reads oldest first; the ledger lists newest first.
async fn oldest_first(context: &BotContext, filter: EntryFilter) -> AppResult<Vec<LedgerEntry>> {
    let mut entries = context.services.ledger.list(&filter).await?;
    entries.reverse();
    Ok(entries)
}

fn period_filter(
    period: StatementPeriod,
    category_id: Option<app::model::CategoryId>,
) -> EntryFilter {
    EntryFilter {
        from: Some(period.from),
        to_inclusive: Some(period.to),
        category_id,
        limit: MAX_STATEMENT_ENTRIES,
        ..EntryFilter::default()
    }
}

fn card_filter(period: StatementPeriod, card: &CreditCard) -> EntryFilter {
    EntryFilter { card_id: Some(card.id), ..period_filter(period, None) }
}

/// The period a statement button carries.
pub const fn period_of(from: NaiveDate, to: NaiveDate) -> StatementPeriod {
    StatementPeriod { from, to }
}
