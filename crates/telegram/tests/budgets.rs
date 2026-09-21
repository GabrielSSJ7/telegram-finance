// Integration-test crate: helpers panic on failure by design.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::{ANA, BIA, BotHarness};

async fn spend(harness: &mut BotHarness, amount: &str) {
    harness.say(ANA, "/gasto").await;
    harness.say(ANA, amount).await;
    harness.tap(ANA, "Pular").await;
    harness.tap(ANA, "mercado").await;
    harness.tap(ANA, "Nubank").await;
    harness.tap(ANA, "Hoje").await;
    harness.tap(ANA, "Confirmar").await;
}

async fn budget(harness: &mut BotHarness, limit: &str) {
    harness.say(BIA, "/orcamento").await;
    harness.tap(BIA, "mercado").await;
    harness.say(BIA, limit).await;
    harness.tap(BIA, "Confirmar").await;
}

#[tokio::test]
async fn spending_past_80_and_100_percent_alerts_the_group() {
    let mut harness = BotHarness::bound().await.with_basics().await;
    budget(&mut harness, "100").await;
    harness.expect_last("Orçamento salvo");
    spend(&mut harness, "85").await;
    harness.expect_last("⚠️ Orçamento de <b>mercado</b> em 85%");
    spend(&mut harness, "5").await;
    harness.expect_last("Gasto registrado");
    spend(&mut harness, "20").await;
    harness.expect_last("🚨 Orçamento de <b>mercado</b> estourado");
}

#[tokio::test]
async fn budgets_list_and_month() {
    let mut harness = BotHarness::bound().await.with_basics().await;
    budget(&mut harness, "200").await;
    spend(&mut harness, "50").await;
    harness.say(ANA, "/orcamentos").await;
    harness.expect_last("mercado ▓▓░░░░░░░░ 25% (R$ 50,00 de R$ 200,00)");
    harness.say(ANA, "/mes").await;
    harness.expect_last("<b>📅 Ciclo atual 01/03 → 31/03</b>");
}

#[tokio::test]
async fn budget_removal() {
    let mut harness = BotHarness::bound().await.with_basics().await;
    budget(&mut harness, "200").await;
    harness.say(BIA, "/orcamento").await;
    harness.tap(BIA, "mercado").await;
    harness.tap(BIA, "Remover orçamento").await;
    harness.tap(BIA, "Confirmar").await;
    harness.expect_last("Orçamento removido");
    harness.say(ANA, "/orcamentos").await;
    harness.expect_last("Nenhum orçamento");
}
