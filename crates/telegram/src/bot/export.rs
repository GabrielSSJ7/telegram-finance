//! `/exportar [mm/aaaa]`: the cycle's entries as a CSV document.

use super::BotContext;
use super::periods::cycle_from_args;
use crate::gateway::{GatewayError, OutgoingDocument};

const USAGE: &str = "Use /exportar para o ciclo atual ou /exportar 03/2026 para o ciclo que começa em março de 2026.";

pub async fn export_entries(
    context: &BotContext,
    chat_id: i64,
    args: &str,
) -> Result<(), GatewayError> {
    let cycle = match cycle_from_args(context, args).await {
        Ok(Some(cycle)) => cycle,
        Ok(None) => return context.reply(chat_id, USAGE).await.map(|_| ()),
        Err(error) => return context.reply_error(chat_id, &error).await,
    };
    let csv = match context.services.exports.entries_csv(cycle.start, cycle.last_day()).await {
        Ok(csv) => csv,
        Err(error) => return context.reply_error(chat_id, &error).await,
    };
    let (start, end) = (cycle.start.format("%d/%m/%Y"), cycle.last_day().format("%d/%m/%Y"));
    let document = OutgoingDocument {
        chat_id,
        file_name: format!("finbot-{}.csv", cycle.start.format("%Y-%m")),
        contents: csv.into_bytes(),
        caption_html: format!("📄 Lançamentos de {start} a {end}"),
    };
    context.gateway.send_document(&document).await
}
