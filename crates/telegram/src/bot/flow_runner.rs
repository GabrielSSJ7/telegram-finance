//! Runs guided flows: shows the card for the current question, applies
//! each answer, and executes the command on confirmation.
//!
//! A flow lives in the flow store between messages, keyed by (chat, user)
//! and tagged with a draft id. The draft id doubles as the idempotency key,
//! so a double tap on Confirmar, or a redelivered update, saves once.

use app::model::{DraftId, Member};
use app::ports::{ChatUserKey, StoredFlow};
use app::services::EntryOrigin;
use app::{AppError, AppResult};
use serde::{Deserialize, Serialize};

use super::BotContext;
use super::executor::{Committed, execute, headline};
use crate::callback_data::{nonce_of, undo_button, undo_purchase_button};
use crate::flows::{Advance, FormInput, FormKind, FormState, build_command};
use crate::gateway::{Button, GatewayError, Keyboard};
use crate::render::{CardContext, CardView, Catalog, card_view, committed_card};

/// Idle flows expire; a half-typed entry from yesterday should not
/// swallow today's messages.
pub const FLOW_TTL_MINUTES: i64 = 30;

const CANCELLED: &str = "✖️ Cancelado.";
const UNREADABLE_FORM: &str = "Não consegui ler esse formulário. Comece de novo, por favor.";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct FlowEnvelope {
    state: FormState,
}

/// A flow's identity across messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowSession {
    pub key: ChatUserKey,
    pub member: Member,
    pub draft: DraftId,
    /// The message showing the current card.
    pub card_message_id: Option<i64>,
}

/// Where the next card goes: in place of the tapped card, or below the
/// message the person just typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    EditCard(i64),
    NewCard,
}

pub fn member_key(chat_id: i64, member: &Member) -> ChatUserKey {
    ChatUserKey { chat_id, user_id: member.telegram_user_id }
}

pub async fn start_flow(
    context: &BotContext,
    chat_id: i64,
    member: &Member,
    form: FormKind,
) -> Result<(), GatewayError> {
    let key = member_key(chat_id, member);
    let previous = load_session(context, key, member).await.map(|(session, _)| session);
    let card_message_id = previous.and_then(|session| session.card_message_id);
    let session =
        FlowSession { key, member: member.clone(), draft: DraftId::generate(), card_message_id };
    present(context, session, FormState::start(form), Placement::NewCard, None).await
}

/// The stored flow of `member` in `key.chat_id`, if any and readable.
pub async fn load_session(
    context: &BotContext,
    key: ChatUserKey,
    member: &Member,
) -> Option<(FlowSession, FormState)> {
    let stored = match context.flows.load_flow(key, context.clock.now()).await {
        Ok(stored) => stored?,
        Err(error) => {
            tracing::error!(%error, chat_id = key.chat_id, "could not load chat flow");
            return None;
        }
    };
    let envelope: FlowEnvelope = serde_json::from_value(stored.flow).ok()?;
    let session = FlowSession {
        key,
        member: member.clone(),
        draft: stored.draft_id,
        card_message_id: stored.prompt_message_id,
    };
    Some((session, envelope.state))
}

pub async fn continue_flow(
    context: &BotContext,
    session: FlowSession,
    state: FormState,
    input: FormInput,
    placement: Placement,
) -> Result<(), GatewayError> {
    match state.advance(&input, context.clock.today()) {
        Advance::Next(next) => present(context, session, next, placement, None).await,
        Advance::Retry { state, problem } => {
            present(context, session, state, placement, Some(&problem)).await
        }
        Advance::Complete(done) => commit(context, session, done, placement).await,
        Advance::Cancelled => finish_cancelled(context, &session, placement).await,
    }
}

/// `/cancelar`: drops the member's flow, if any.
pub async fn cancel_current(
    context: &BotContext,
    chat_id: i64,
    member: &Member,
) -> Result<(), GatewayError> {
    let Some((session, _)) = load_session(context, member_key(chat_id, member), member).await
    else {
        return context.reply(chat_id, "Nada para cancelar.").await.map(|_| ());
    };
    match session.card_message_id {
        Some(message_id) => {
            finish_cancelled(context, &session, Placement::EditCard(message_id)).await
        }
        None => finish_cancelled(context, &session, Placement::NewCard).await,
    }
}

async fn present(
    context: &BotContext,
    mut session: FlowSession,
    state: FormState,
    placement: Placement,
    problem: Option<&str>,
) -> Result<(), GatewayError> {
    let view = match render_card(context, &session, &state, problem).await {
        Ok(view) => view,
        Err(error) => return context.reply_error(session.key.chat_id, &error).await,
    };
    match view {
        CardView::Blocked { html } => {
            clear(context, &session).await;
            place(context, &mut session, html, None, placement).await
        }
        CardView::Ask { html, keyboard } => {
            place(context, &mut session, html, Some(keyboard), placement).await?;
            save(context, &session, state).await
        }
    }
}

async fn render_card(
    context: &BotContext,
    session: &FlowSession,
    state: &FormState,
    problem: Option<&str>,
) -> AppResult<CardView> {
    let catalog = Catalog::load(&context.services, state.form == FormKind::PayInvoice).await?;
    let nonce = nonce_of(session.draft);
    let card = CardContext {
        owner: &session.member.display_name,
        catalog: &catalog,
        nonce: &nonce,
        today: context.clock.today(),
    };
    Ok(card_view(state, card, problem))
}

async fn place(
    context: &BotContext,
    session: &mut FlowSession,
    html: String,
    keyboard: Option<Keyboard>,
    placement: Placement,
) -> Result<(), GatewayError> {
    let chat_id = session.key.chat_id;
    let message_id = match placement {
        Placement::EditCard(message_id) => {
            context.edit(chat_id, message_id, html, keyboard).await?;
            message_id
        }
        Placement::NewCard => {
            retire_card(context, session).await;
            context.send(chat_id, html, keyboard).await?
        }
    };
    session.card_message_id = Some(message_id);
    Ok(())
}

/// Removes the buttons of the previous card so only the newest is live.
async fn retire_card(context: &BotContext, session: &FlowSession) {
    let Some(message_id) = session.card_message_id else {
        return;
    };
    if let Err(error) = context.gateway.remove_keyboard(session.key.chat_id, message_id).await {
        tracing::warn!(%error, message_id, "could not remove keyboard of previous card");
    }
}

async fn save(
    context: &BotContext,
    session: &FlowSession,
    state: FormState,
) -> Result<(), GatewayError> {
    let flow = serde_json::to_value(FlowEnvelope { state }).unwrap_or_default();
    let expires_at = context.clock.now() + chrono::Duration::minutes(FLOW_TTL_MINUTES);
    let stored = StoredFlow {
        flow,
        prompt_message_id: session.card_message_id,
        draft_id: session.draft,
        expires_at,
    };
    match context.flows.save_flow(session.key, &stored).await {
        Ok(()) => Ok(()),
        Err(error) => context.reply_error(session.key.chat_id, &AppError::from(error)).await,
    }
}

async fn clear(context: &BotContext, session: &FlowSession) {
    if let Err(error) = context.flows.clear_flow(session.key).await {
        tracing::error!(%error, chat_id = session.key.chat_id, "could not clear chat flow");
    }
}

async fn commit(
    context: &BotContext,
    session: FlowSession,
    state: FormState,
    placement: Placement,
) -> Result<(), GatewayError> {
    let Some(command) = build_command(&state) else {
        clear(context, &session).await;
        return context.reply(session.key.chat_id, UNREADABLE_FORM).await.map(|_| ());
    };
    let origin = EntryOrigin { created_by: Some(session.member.id), draft: Some(session.draft) };
    match execute(&context.services, command, origin).await {
        Ok(committed) => {
            clear(context, &session).await;
            show_committed(context, session, &state, &committed, placement).await
        }
        Err(error) => commit_failed(context, &session, &error).await,
    }
}

async fn commit_failed(
    context: &BotContext,
    session: &FlowSession,
    error: &AppError,
) -> Result<(), GatewayError> {
    // Transient: keep the flow so tapping Confirmar again retries.
    if !matches!(error, AppError::Storage(_)) {
        clear(context, session).await;
    }
    context.reply_error(session.key.chat_id, error).await
}

async fn show_committed(
    context: &BotContext,
    mut session: FlowSession,
    state: &FormState,
    committed: &Committed,
    placement: Placement,
) -> Result<(), GatewayError> {
    let catalog = Catalog::load(&context.services, state.form == FormKind::PayInvoice)
        .await
        .unwrap_or_default();
    let card = CardContext {
        owner: &session.member.display_name,
        catalog: &catalog,
        nonce: "",
        today: context.clock.today(),
    };
    let html = committed_card(state, card, headline(state.form));
    let keyboard = undo_keyboard_for(committed);
    place(context, &mut session, html, keyboard, placement).await
}

/// [Desfazer] for what created ledger rows; setup commands have none.
fn undo_keyboard_for(committed: &Committed) -> Option<Keyboard> {
    match committed {
        Committed::Entry(entry) => Some(undo_keyboard(undo_button(entry.id))),
        Committed::Purchase(purchase) => Some(undo_keyboard(undo_purchase_button(purchase.id))),
        Committed::Account(_)
        | Committed::Goal(_)
        | Committed::Card(_)
        | Committed::Recurrence(_) => None,
    }
}

/// A single [Desfazer] button carrying `data`.
pub fn undo_keyboard(data: String) -> Keyboard {
    Keyboard { rows: vec![vec![Button { label: "↩️ Desfazer".into(), data }]] }
}

async fn finish_cancelled(
    context: &BotContext,
    session: &FlowSession,
    placement: Placement,
) -> Result<(), GatewayError> {
    clear(context, session).await;
    match placement {
        Placement::EditCard(message_id) => {
            context.edit(session.key.chat_id, message_id, CANCELLED, None).await
        }
        Placement::NewCard => context.reply(session.key.chat_id, CANCELLED).await.map(|_| ()),
    }
}
