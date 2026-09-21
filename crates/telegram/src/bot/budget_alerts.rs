//! Tells the group right away when an entry pushes a budget past 80% or
//! 100% of its limit.

use super::BotContext;
use crate::gateway::GatewayError;
use crate::render::notices::budget_alert_text;

pub async fn announce_budget_alerts(
    context: &BotContext,
    chat_id: i64,
) -> Result<(), GatewayError> {
    let alerts = match context.services.budgets.new_alerts().await {
        Ok(alerts) => alerts,
        Err(error) => {
            // The daily report retries the check, so a failure here is only logged.
            tracing::warn!(%error, "could not check budget alerts");
            return Ok(());
        }
    };
    if alerts.is_empty() {
        return Ok(());
    }
    let text: Vec<String> = alerts.iter().map(budget_alert_text).collect();
    context.reply(chat_id, text.join("\n")).await.map(|_| ())
}
