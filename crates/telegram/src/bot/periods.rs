//! Periods typed after a command: nothing for the current cycle, `mm/aaaa`
//! for the cycle that starts in that month (`/exportar 03/2026`).

use app::AppResult;
use domain::YearMonth;
use domain::cycle::Cycle;

use super::BotContext;

/// The cycle named by `args`, or the current one when empty; `None` when
/// `args` is not a month.
pub async fn cycle_from_args(context: &BotContext, args: &str) -> AppResult<Option<Cycle>> {
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
