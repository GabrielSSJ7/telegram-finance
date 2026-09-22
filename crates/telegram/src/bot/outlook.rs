//! How the cycle should end: `/essenciais` marks the categories of the
//! basic cost of living, `/custodevida [mm/aaaa]` adds them up and
//! `/projecao [mm/aaaa]` projects income, spending and cash.

use app::AppResult;
use app::model::CategoryId;

use super::BotContext;
use super::periods::cycle_from_args;
use crate::gateway::{ButtonPress, GatewayError, Keyboard};
use crate::render::outlook::{essentials_view, living_cost_text, projection_text};

const USAGE: &str = "Use /custodevida para o ciclo atual ou /custodevida 08/2026 para o ciclo que começa em agosto de 2026.";
const PROJECTION_USAGE: &str = "Use /projecao para o ciclo atual ou /projecao 08/2026 para o ciclo que começa em agosto de 2026.";

pub async fn essential_categories(context: &BotContext, chat_id: i64) -> Result<(), GatewayError> {
    match essentials(context).await {
        Ok((html, keyboard)) => context.send(chat_id, html, keyboard).await.map(|_| ()),
        Err(error) => context.reply_error(chat_id, &error).await,
    }
}

async fn essentials(context: &BotContext) -> AppResult<(String, Option<Keyboard>)> {
    Ok(essentials_view(&context.services.categories.list(None).await?))
}

/// A toggle in `/essenciais`: flips the mark and redraws the buttons.
pub async fn toggle_essential(
    context: &BotContext,
    press: &ButtonPress,
    id: CategoryId,
) -> Result<(), GatewayError> {
    let toast = match flip(context, id).await {
        Ok(true) => "Marcada como essencial ✅",
        Ok(false) => "Não é mais essencial",
        Err(error) => return context.reply_error(press.chat_id, &error).await,
    };
    context.gateway.answer_button(&press.callback_id, Some(toast)).await?;
    match essentials(context).await {
        Ok((html, keyboard)) => context.edit(press.chat_id, press.message_id, html, keyboard).await,
        Err(error) => context.reply_error(press.chat_id, &error).await,
    }
}

/// The new mark of category `id`.
async fn flip(context: &BotContext, id: CategoryId) -> AppResult<bool> {
    let categories = &context.services.categories;
    let current = categories.require_kind(id, app::model::CategoryKind::Expense).await?;
    Ok(categories.set_essential(id, !current.essential).await?.essential)
}

pub async fn living_cost(
    context: &BotContext,
    chat_id: i64,
    args: &str,
) -> Result<(), GatewayError> {
    let cycle = match cycle_from_args(context, args).await {
        Ok(Some(cycle)) => cycle,
        Ok(None) => return context.reply(chat_id, USAGE).await.map(|_| ()),
        Err(error) => return context.reply_error(chat_id, &error).await,
    };
    match context.services.outlook.living_cost(cycle).await {
        Ok(cost) => context.reply(chat_id, living_cost_text(&cost)).await.map(|_| ()),
        Err(error) => context.reply_error(chat_id, &error).await,
    }
}

/// `/projecao [mm/aaaa]`: how the cycle should end.
pub async fn projection(
    context: &BotContext,
    chat_id: i64,
    args: &str,
) -> Result<(), GatewayError> {
    let cycle = match cycle_from_args(context, args).await {
        Ok(Some(cycle)) => cycle,
        Ok(None) => return context.reply(chat_id, PROJECTION_USAGE).await.map(|_| ()),
        Err(error) => return context.reply_error(chat_id, &error).await,
    };
    match context.services.outlook.projection(cycle).await {
        Ok(projected) => context.reply(chat_id, projection_text(&projected)).await.map(|_| ()),
        Err(error) => context.reply_error(chat_id, &error).await,
    }
}
