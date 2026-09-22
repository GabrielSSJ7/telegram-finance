// Integration-test crate: helpers panic on failure by design.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use app::model::CategoryKind;
use chrono::NaiveDate;
use common::{ANA, BIA, BotHarness};

fn march(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 3, day).unwrap()
}

/// Two mercado expenses and a salary, all in March.
async fn harness_with_march_entries() -> BotHarness {
    let harness = BotHarness::bound().await.with_basics().await;
    harness.record_entry(CategoryKind::Expense, 2_500, "pão", march(5), None).await;
    harness.record_entry(CategoryKind::Expense, 7_500, "feira", march(2), None).await;
    harness.record_entry(CategoryKind::Income, 500_000, "salário", march(6), None).await;
    harness
}

#[tokio::test]
async fn extrato_lists_totals_per_category() {
    let mut harness = harness_with_march_entries().await;
    harness.say(ANA, "/extrato").await;
    harness.expect_last("<b>📂 Extrato por categoria</b> · 01/03 → 31/03/2026");
    harness.expect_last("<b>💸 Gastos</b>\nmercado: R$ 100,00 (2)\nTotal: R$ 100,00");
    harness.expect_last("<b>💰 Entradas</b>\nsalário: R$ 5.000,00 (1)");
    assert!(harness.gateway.find_button(common::GROUP, "salário").is_some());
}

#[tokio::test]
async fn tapping_a_category_lists_its_entries_oldest_first() {
    let mut harness = harness_with_march_entries().await;
    harness.say(ANA, "/extrato").await;
    harness.tap(BIA, "mercado").await;
    let html = harness.last_html();
    assert!(html.starts_with("<b>mercado</b> · 01/03 → 31/03/2026"), "{html}");
    let feira = html.find("02/03 R$ 75,00 — feira (automático)").expect(&html);
    let bread = html.find("05/03 R$ 25,00 — pão (automático)").expect(&html);
    assert!(feira < bread, "oldest first: {html}");
    assert!(html.ends_with("Total: R$ 100,00 (2 lançamentos)"), "{html}");
}

#[tokio::test]
async fn extrato_for_another_cycle_or_a_bad_month() {
    let mut harness = harness_with_march_entries().await;
    harness.say(ANA, "/extrato 02/2026").await;
    harness.expect_last("01/02 → 28/02/2026\nNenhum lançamento com categoria nesse ciclo.");
    harness.say(ANA, "/extrato fevereiro").await;
    harness.expect_last("Use /extrato para o ciclo atual");
}
