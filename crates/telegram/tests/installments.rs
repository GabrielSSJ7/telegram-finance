// Integration-test crate: helpers panic on failure by design.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use app::model::EntryFilter;
use common::{ANA, BIA, BotHarness};
use domain::{Cents, EntryKind};

#[tokio::test]
async fn gasto_with_three_of_ten_creates_only_the_remaining_installments() {
    let mut harness = BotHarness::bound().await.with_basics().await.with_card().await;
    harness.say(ANA, "/gasto").await;
    harness.say(ANA, "150").await;
    harness.say(ANA, "sofá").await;
    harness.tap(ANA, "mercado").await;
    harness.tap(ANA, "Roxinho").await;
    harness.say(ANA, "3/10").await;
    harness.expect_last("Data da compra original");
    harness.tap(ANA, "Outra data").await;
    harness.say(ANA, "10/01/2026").await;
    harness.expect_last("parcela 3 de 10 (valor por parcela)");
    harness.tap(ANA, "Confirmar").await;
    let filter = EntryFilter { kind: Some(EntryKind::CardInstallment), ..EntryFilter::default() };
    let rows = harness.set.services.ledger.list(&filter).await.unwrap();
    let numbers: Vec<u32> = rows.iter().filter_map(|row| row.installment_no).collect();
    assert_eq!(numbers.len(), 8, "installments 3 to 10: {numbers:?}");
    assert!(rows.iter().all(|row| row.amount == Cents::new(15_000)));
    harness.say(BIA, "/parcelas").await;
    harness.expect_last("<b>sofá</b> · 💳 Roxinho");
    harness.expect_last("R$ 450,00 de R$ 1.500,00 (30%) · parcela 3 de 10");
}

#[tokio::test]
async fn recorrente_with_installments_is_a_financing() {
    let mut harness = BotHarness::bound().await.with_basics().await;
    harness.say(ANA, "/recorrente").await;
    harness.tap(ANA, "Gasto").await;
    harness.say(ANA, "Financiamento").await;
    harness.say(ANA, "1200").await;
    harness.tap(ANA, "mercado").await;
    harness.tap(ANA, "Nubank").await;
    harness.say(ANA, "5").await;
    harness.say(ANA, "36").await;
    harness.expect_last("Quantas já foram pagas?");
    harness.say(ANA, "22").await;
    harness.tap(ANA, "Automático").await;
    harness.expect_last("🔢 Parcelas: 36");
    harness.tap(ANA, "Confirmar").await;
    harness.say(BIA, "/parcelas").await;
    harness.expect_last("<b>Financiamento</b> · 🏦 Nubank");
    harness.expect_last("R$ 26.400,00 de R$ 43.200,00 (61%) · parcela 22 de 36");
    // Next due date is 05/04 (installment 23), so the 36th is in May 2027.
    harness.expect_last("R$ 1.200,00/mês até 05/2027");
}

#[tokio::test]
async fn parcelas_without_plans() {
    let mut harness = BotHarness::bound().await.with_basics().await;
    harness.say(ANA, "/parcelas").await;
    harness.expect_last("Nenhum parcelamento em andamento.");
}
