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

#[tokio::test]
async fn antecipar_moves_the_last_installments_to_this_invoice() {
    let mut harness = BotHarness::bound().await.with_basics().await.with_card().await;
    buy_in_installments(&mut harness, "300", "3").await;
    harness.say(BIA, "/antecipar").await;
    harness.tap(BIA, "geladeira").await;
    harness.say(BIA, "2").await;
    harness.say(BIA, "190").await;
    harness.tap(BIA, "Confirmar").await;
    harness.expect_last("Antecipação registrada");
    let filter = EntryFilter { kind: Some(EntryKind::CardInstallment), ..EntryFilter::default() };
    let rows = harness.set.services.ledger.list(&filter).await.unwrap();
    let amounts: Vec<Cents> = rows.iter().map(|row| row.amount).collect();
    assert_eq!(amounts.len(), 2, "one installment left plus the anticipation: {amounts:?}");
    assert!(amounts.contains(&Cents::new(19_000)), "{amounts:?}");
    harness.say(ANA, "/parcelas").await;
    harness.expect_last("Nenhum parcelamento em andamento.");
}

#[tokio::test]
async fn antecipar_without_plans_says_so() {
    let mut harness = BotHarness::bound().await.with_basics().await.with_card().await;
    harness.say(ANA, "/antecipar").await;
    harness.expect_last("Nenhum parcelamento");
}

/// A card purchase of `amount` split into `installments`.
async fn buy_in_installments(harness: &mut BotHarness, amount: &str, installments: &str) {
    harness.say(ANA, "/gasto").await;
    harness.say(ANA, amount).await;
    harness.say(ANA, "geladeira").await;
    harness.tap(ANA, "mercado").await;
    harness.tap(ANA, "Roxinho").await;
    harness.say(ANA, installments).await;
    harness.tap(ANA, "Hoje").await;
    harness.tap(ANA, "Confirmar").await;
}
