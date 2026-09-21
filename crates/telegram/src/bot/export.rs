//! `/exportar [mm/aaaa]`: the cycle's entries as a CSV document.

use app::AppResult;
use domain::YearMonth;
use domain::cycle::Cycle;

use super::BotContext;
use crate::gateway::{GatewayError, OutgoingDocument};

const USAGE: &str = "Use /exportar para o ciclo atual ou /exportar 03/2026 para o ciclo que começa em março de 2026.";

pub async fn export_entries(
    context: &BotContext,
    chat_id: i64,
    args: &str,
) -> Result<(), GatewayError> {
    let cycle = match export_cycle(context, args).await {
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

/// The cycle named by `args` (`mm/aaaa`), or the current one when empty;
/// `None` when `args` is not a month.
async fn export_cycle(context: &BotContext, args: &str) -> AppResult<Option<Cycle>> {
    let settings = context.services.settings.get().await?;
    let args = args.trim();
    if args.is_empty() {
        return Ok(Some(Cycle::containing(context.clock.today(), settings.cycle_start_day)));
    }
    Ok(parse_month(args).map(|month| Cycle::starting_in(month, settings.cycle_start_day)))
}

fn parse_month(text: &str) -> Option<YearMonth> {
    let (month, year) = text.split_once('/')?;
    YearMonth::new(year.trim().parse().ok()?, month.trim().parse().ok()?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_month_and_year() {
        assert_eq!(parse_month("03/2026"), YearMonth::new(2026, 3).ok());
        assert_eq!(parse_month(" 3 / 2026 "), YearMonth::new(2026, 3).ok());
        assert_eq!(parse_month("13/2026"), None);
        assert_eq!(parse_month("março"), None);
    }
}
