//! Slash commands: guided forms start a flow; the rest answer at once.

use app::model::Member;

use super::BotContext;
use super::entry_actions::recent_entries;
use super::export::export_entries;
use super::flow_runner::{cancel_current, start_flow};
use super::undo::undo_last;
use crate::flows::FormKind;
use crate::gateway::GatewayError;
use crate::render::help::help_text;
use crate::render::report_text::{daily_report_text, month_report_text};
use crate::render::reports::{
    accounts_text, balance_text, budgets_text, cards_text, categories_text, goals_text,
    invoices_text, recurrences_view,
};

/// `"/gasto@finbot resto"` → `("gasto", "resto")`. Not a command → `None`.
///
/// ```
/// use telegram::bot::commands::parse_command;
/// assert_eq!(parse_command("/Saldo@finbot"), Some(("saldo".to_owned(), String::new())));
/// ```
pub fn parse_command(text: &str) -> Option<(String, String)> {
    let body = text.trim().strip_prefix('/')?;
    let (head, rest) = body.split_once(char::is_whitespace).unwrap_or((body, ""));
    let name = head.split('@').next().unwrap_or(head).to_lowercase();
    (!name.is_empty()).then(|| (name, rest.trim().to_owned()))
}

/// Commands inside the household's group.
pub async fn household_command(
    context: &BotContext,
    chat_id: i64,
    member: &Member,
    command: &str,
    args: &str,
) -> Result<(), GatewayError> {
    if let Some(form) = FormKind::from_command(command) {
        return start_flow(context, chat_id, member, form).await;
    }
    match command {
        "desfazer" => undo_last(context, chat_id, member).await,
        "exportar" => export_entries(context, chat_id, args).await,
        "ultimos" => recent_entries(context, chat_id).await,
        "cancelar" => cancel_current(context, chat_id, member).await,
        "ajuda" | "start" | "help" => context.reply(chat_id, help_text()).await.map(|_| ()),
        report => report_command(context, chat_id, report).await,
    }
}

/// Commands that only read and reply.
async fn report_command(
    context: &BotContext,
    chat_id: i64,
    command: &str,
) -> Result<(), GatewayError> {
    match command {
        "saldo" => balances(context, chat_id).await,
        "contas" => accounts(context, chat_id).await,
        "metas" => goals(context, chat_id).await,
        "categorias" => categories(context, chat_id).await,
        "fatura" | "faturas" => invoices(context, chat_id).await,
        "cartoes" => cards(context, chat_id).await,
        "recorrentes" => recurrences(context, chat_id).await,
        "resumo" => summary(context, chat_id).await,
        "mes" => month(context, chat_id).await,
        "orcamentos" => budgets(context, chat_id).await,
        other => {
            context.reply(chat_id, format!("Não conheço /{other}. Veja /ajuda.")).await.map(|_| ())
        }
    }
}

async fn balances(context: &BotContext, chat_id: i64) -> Result<(), GatewayError> {
    match context.services.position.balance_sheet().await {
        Ok(sheet) => context.reply(chat_id, balance_text(&sheet)).await.map(|_| ()),
        Err(error) => context.reply_error(chat_id, &error).await,
    }
}

async fn accounts(context: &BotContext, chat_id: i64) -> Result<(), GatewayError> {
    match context.services.position.balance_sheet().await {
        Ok(sheet) => context.reply(chat_id, accounts_text(&sheet)).await.map(|_| ()),
        Err(error) => context.reply_error(chat_id, &error).await,
    }
}

async fn goals(context: &BotContext, chat_id: i64) -> Result<(), GatewayError> {
    match context.services.goals.list_progress().await {
        Ok(progress) => {
            context.reply(chat_id, goals_text(&progress, context.clock.today())).await.map(|_| ())
        }
        Err(error) => context.reply_error(chat_id, &error).await,
    }
}

async fn invoices(context: &BotContext, chat_id: i64) -> Result<(), GatewayError> {
    match context.services.cards.summaries().await {
        Ok(summaries) => context.reply(chat_id, invoices_text(&summaries)).await.map(|_| ()),
        Err(error) => context.reply_error(chat_id, &error).await,
    }
}

async fn cards(context: &BotContext, chat_id: i64) -> Result<(), GatewayError> {
    match context.services.cards.list().await {
        Ok(found) => context.reply(chat_id, cards_text(&found)).await.map(|_| ()),
        Err(error) => context.reply_error(chat_id, &error).await,
    }
}

async fn recurrences(context: &BotContext, chat_id: i64) -> Result<(), GatewayError> {
    match context.services.recurrences.list(false).await {
        Ok(found) => {
            let (html, keyboard) = recurrences_view(&found);
            context.send(chat_id, html, keyboard).await.map(|_| ())
        }
        Err(error) => context.reply_error(chat_id, &error).await,
    }
}

/// `/resumo`: today's report on demand.
async fn summary(context: &BotContext, chat_id: i64) -> Result<(), GatewayError> {
    match context.services.reports.daily(context.clock.today()).await {
        Ok(report) => context.reply(chat_id, daily_report_text(&report)).await.map(|_| ()),
        Err(error) => context.reply_error(chat_id, &error).await,
    }
}

/// `/mes`: the current cycle so far.
async fn month(context: &BotContext, chat_id: i64) -> Result<(), GatewayError> {
    let reports = &context.services.reports;
    let report = match reports.cycle_of(context.clock.today()).await {
        Ok(cycle) => reports.cycle(cycle).await,
        Err(error) => Err(error),
    };
    match report {
        Ok(report) => context
            .reply(chat_id, month_report_text(&report, context.clock.today()))
            .await
            .map(|_| ()),
        Err(error) => context.reply_error(chat_id, &error).await,
    }
}

async fn budgets(context: &BotContext, chat_id: i64) -> Result<(), GatewayError> {
    let budgets = &context.services.budgets;
    let statuses = match budgets.current_cycle().await {
        Ok(cycle) => budgets.statuses(cycle).await,
        Err(error) => Err(error),
    };
    match statuses {
        Ok(statuses) => context.reply(chat_id, budgets_text(&statuses)).await.map(|_| ()),
        Err(error) => context.reply_error(chat_id, &error).await,
    }
}

async fn categories(context: &BotContext, chat_id: i64) -> Result<(), GatewayError> {
    match context.services.categories.list(None).await {
        Ok(found) => context.reply(chat_id, categories_text(&found)).await.map(|_| ()),
        Err(error) => context.reply_error(chat_id, &error).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_commands_with_bot_mentions_and_arguments() {
        assert_eq!(parse_command("/gasto"), Some(("gasto".into(), String::new())));
        assert_eq!(
            parse_command(" /GASTO@FinBot  10 mercado "),
            Some(("gasto".into(), "10 mercado".into()))
        );
        assert_eq!(parse_command("gasto"), None);
        assert_eq!(parse_command("/"), None);
        assert_eq!(parse_command("/@bot"), None);
    }
}
