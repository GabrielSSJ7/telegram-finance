//! Buttons on recurrence messages: register or skip a bill that asks
//! first, and deactivate a recurrence from `/recorrentes`.

use app::AppResult;
use app::model::{Member, Recurrence, RecurrenceId};
use chrono::NaiveDate;
use domain::money_format::format_brl;

use super::BotContext;
use crate::gateway::{ButtonPress, GatewayError};
use crate::html::escape;

pub async fn record_recurrence(
    context: &BotContext,
    press: &ButtonPress,
    member: &Member,
    id: RecurrenceId,
    date: NaiveDate,
) -> Result<(), GatewayError> {
    let recorded = record_for(context, member, id, date).await;
    context.gateway.answer_button(&press.callback_id, None).await?;
    let (recurrence, new) = match recorded {
        Ok(result) => result,
        Err(error) => return context.reply_error(press.chat_id, &error).await,
    };
    let status = if new {
        format!("✅ Registrado por {}", escape(&member.display_name))
    } else {
        "✅ Já estava registrado".into()
    };
    let (name, amount) = (escape(&recurrence.description), format_brl(recurrence.amount));
    let html = format!("{status}: <b>{name}</b> {amount} ({})", date.format("%d/%m"));
    context.edit(press.chat_id, press.message_id, html, None).await
}

async fn record_for(
    context: &BotContext,
    member: &Member,
    id: RecurrenceId,
    date: NaiveDate,
) -> AppResult<(Recurrence, bool)> {
    let recurrences = &context.services.recurrences;
    let recurrence = recurrences.find(id).await?;
    let new = recurrences.record(&recurrence, date, recurrence.amount, Some(member.id)).await?;
    Ok((recurrence, new))
}

pub async fn skip_recurrence(
    context: &BotContext,
    press: &ButtonPress,
    member: &Member,
    id: RecurrenceId,
    date: NaiveDate,
) -> Result<(), GatewayError> {
    context.gateway.answer_button(&press.callback_id, None).await?;
    let name = context
        .services
        .recurrences
        .find(id)
        .await
        .map_or_else(|_| "recorrência".into(), |found| escape(&found.description));
    let html = format!(
        "⏭️ Pulado por {}: <b>{name}</b> ({})",
        escape(&member.display_name),
        date.format("%d/%m")
    );
    context.edit(press.chat_id, press.message_id, html, None).await
}

pub async fn deactivate_recurrence(
    context: &BotContext,
    press: &ButtonPress,
    id: RecurrenceId,
) -> Result<(), GatewayError> {
    let recurrences = &context.services.recurrences;
    let name = recurrences
        .find(id)
        .await
        .map_or_else(|_| "recorrência".into(), |found| escape(&found.description));
    let toast = match recurrences.deactivate(id).await {
        Ok(()) => format!("{name} desativada"),
        Err(_) => "Já estava desativada".into(),
    };
    context.gateway.answer_button(&press.callback_id, Some(&toast)).await?;
    let remaining = recurrences.list(false).await.unwrap_or_default();
    let (html, keyboard) = crate::render::reports::recurrences_view(&remaining);
    context.edit(press.chat_id, press.message_id, html, keyboard).await
}
