// Integration-test crate: helpers panic on failure by design.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use app::jobs::JobKind;
use app::model::CategoryKind;
use chrono::NaiveDate;
use common::{ANA, BIA, BotHarness, GROUP, today};

async fn category_named(harness: &BotHarness, name: &str) -> app::model::Category {
    let categories = harness.set.services.categories.list(None).await.unwrap();
    categories.into_iter().find(|category| category.name == name).unwrap()
}

#[tokio::test]
async fn essenciais_toggles_a_category() {
    let mut harness = BotHarness::bound().await.with_basics().await;
    harness.say(ANA, "/essenciais").await;
    harness.tap(BIA, "⬜ mercado").await;
    assert_eq!(harness.last_toast().as_deref(), Some("Marcada como essencial ✅"));
    assert!(category_named(&harness, "mercado").await.essential);
    assert!(harness.gateway.find_button(GROUP, "✅ mercado").is_some());
    assert!(harness.gateway.find_button(GROUP, "salário").is_none(), "income is never essential");
    harness.tap(BIA, "✅ mercado").await;
    assert!(!category_named(&harness, "mercado").await.essential);
}

#[tokio::test]
async fn nova_categoria_asks_essential_only_for_expenses() {
    let mut harness = BotHarness::bound().await.with_basics().await;
    harness.say(ANA, "/novacategoria").await;
    harness.say(ANA, "aluguel").await;
    harness.tap(ANA, "Gasto").await;
    harness.tap(ANA, "Sim, essencial").await;
    harness.tap(ANA, "Pular").await;
    harness.tap(ANA, "Confirmar").await;
    assert!(category_named(&harness, "aluguel").await.essential);
    harness.say(ANA, "/novacategoria").await;
    harness.say(ANA, "bônus").await;
    harness.tap(ANA, "Entrada").await;
    harness.expect_last("Um emoji para ela?");
}

#[tokio::test]
async fn custodevida_sums_essential_spending() {
    let mut harness = BotHarness::bound().await.with_basics().await;
    let market = category_named(&harness, "mercado").await;
    harness.set.services.categories.set_essential(market.id, true).await.unwrap();
    harness.record_entry(CategoryKind::Expense, 10_000, "feira", today(), None).await;
    harness.say(ANA, "/custodevida").await;
    harness.expect_last("<b>🏠 Custo de vida básico</b> · 01/03 → 31/03/2026");
    harness.expect_last("Gasto até agora: R$ 100,00");
    harness.expect_last("Média: aparece depois do primeiro ciclo completo.");
    harness.expect_last("mercado: R$ 100,00");
    harness.say(ANA, "/custodevida março").await;
    harness.expect_last("Use /custodevida para o ciclo atual");
}

#[tokio::test]
async fn cycle_closing_splits_essential_other_and_saved() {
    let harness = BotHarness::bound().await.with_basics().await;
    let market = category_named(&harness, "mercado").await;
    harness.set.services.categories.set_essential(market.id, true).await.unwrap();
    harness.record_entry(CategoryKind::Expense, 30_000, "feira", today(), None).await;
    harness.record_entry(CategoryKind::Income, 100_000, "salário", today(), None).await;
    harness
        .job_runner()
        .run(JobKind::CycleReport, NaiveDate::from_ymd_opt(2026, 4, 1).unwrap())
        .await
        .unwrap();
    harness.expect_last(
        "🏠 Essencial R$ 300,00 (30%) · Outros R$ 0,00 (0%) · Guardado R$ 700,00 (70%)",
    );
}

#[tokio::test]
async fn projecao_shows_how_the_cycle_should_end() {
    let mut harness = BotHarness::bound().await.with_basics().await;
    harness.record_entry(CategoryKind::Expense, 30_000, "feira", today(), None).await;
    harness.record_entry(CategoryKind::Income, 500_000, "salário", today(), None).await;
    harness.say(ANA, "/projecao").await;
    harness.expect_last("<b>🔮 Projeção do ciclo</b> · 01/03 → 31/03/2026 (faltam 22 dias)");
    harness.expect_last("Entradas: R$ 5.000,00");
    harness.expect_last("Gastos: R$ 300,00");
    harness.expect_last("<b>Sobra prevista: R$ 4.700,00</b> (94% das entradas)");
    harness.expect_last("Disponível hoje: R$ 5.700,00");
    harness.expect_last("<b>Maiores gastos previstos</b>\nmercado: R$ 300,00");
    harness.say(BIA, "/projecao 13/2026").await;
    harness.expect_last("Use /projecao para o ciclo atual");
}
