//! `/ultimos` and its ✏️/🗑️ buttons. Only the author changes an entry;
//! automatic entries (from recurrences) can be changed by either spouse.

use app::AppResult;
use app::model::{EntryFilter, EntryId, LedgerEntry, Member};
use app::services::ledger_validation::is_editable_kind;
use domain::EntryKind;

use super::BotContext;
use super::flow_runner::start_prepared_flow;
use crate::flows::{Answer, Answers, Field, FormKind, FormState};
use crate::gateway::{ButtonPress, GatewayError, Keyboard};
use crate::render::Catalog;
use crate::render::catalog::CatalogNeeds;
use crate::render::entries::{entry_label, recent_entries_view};

pub const RECENT_ENTRIES: u32 = 10;

const NOT_EDITABLE: &str = "Parcelas, estornos e pagamentos de fatura: apague e registre de novo.";

pub async fn recent_entries(context: &BotContext, chat_id: i64) -> Result<(), GatewayError> {
    match recent_view(context).await {
        Ok((html, keyboard)) => context.send(chat_id, html, keyboard).await.map(|_| ()),
        Err(error) => context.reply_error(chat_id, &error).await,
    }
}

async fn recent_view(context: &BotContext) -> AppResult<(String, Option<Keyboard>)> {
    let filter = EntryFilter { limit: RECENT_ENTRIES, ..EntryFilter::default() };
    let entries = context.services.ledger.list(&filter).await?;
    let catalog = Catalog::load(&context.services, CatalogNeeds::default()).await?;
    let members = context.services.members.list().await?;
    Ok(recent_entries_view(&entries, &catalog, &members, context.clock.today()))
}

pub async fn delete_entry(
    context: &BotContext,
    press: &ButtonPress,
    member: &Member,
    id: EntryId,
) -> Result<(), GatewayError> {
    let Some(entry) = own_entry(context, press, member, id).await? else {
        return Ok(());
    };
    if let Err(error) = context.services.ledger.delete(entry.id).await {
        context.gateway.answer_button(&press.callback_id, None).await?;
        return context.reply_error(press.chat_id, &error).await;
    }
    context.gateway.answer_button(&press.callback_id, Some("Apagado")).await?;
    match recent_view(context).await {
        Ok((html, keyboard)) => context.edit(press.chat_id, press.message_id, html, keyboard).await,
        Err(error) => context.reply_error(press.chat_id, &error).await,
    }
}

pub async fn edit_entry(
    context: &BotContext,
    press: &ButtonPress,
    member: &Member,
    id: EntryId,
) -> Result<(), GatewayError> {
    let Some(entry) = own_entry(context, press, member, id).await? else {
        return Ok(());
    };
    if !is_editable_kind(entry.kind) {
        return context.gateway.answer_button(&press.callback_id, Some(NOT_EDITABLE)).await;
    }
    context.gateway.answer_button(&press.callback_id, None).await?;
    let target = Answer::Entry {
        id,
        income: entry.kind == EntryKind::Income,
        label: entry_label(&entry, context.clock.today()),
    };
    let state =
        FormState::with_answers(FormKind::EditEntry, Answers(vec![(Field::EditTarget, target)]));
    start_prepared_flow(context, press.chat_id, member, state).await
}

/// The entry when it exists and `member` may change it; otherwise the
/// button gets a toast and `None` comes back.
async fn own_entry(
    context: &BotContext,
    press: &ButtonPress,
    member: &Member,
    id: EntryId,
) -> Result<Option<LedgerEntry>, GatewayError> {
    let Ok(entry) = context.services.ledger.find(id).await else {
        context
            .gateway
            .answer_button(&press.callback_id, Some("Esse lançamento já foi apagado."))
            .await?;
        return Ok(None);
    };
    if entry.created_by.is_some_and(|author| author != member.id) {
        context
            .gateway
            .answer_button(&press.callback_id, Some("Só quem registrou pode mudar."))
            .await?;
        return Ok(None);
    }
    Ok(Some(entry))
}
