// Integration-test crate: helpers panic on failure by design.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use app::model::EntryFilter;
use chrono::NaiveDate;
use common::{ANA, BIA, BotHarness};
use domain::Cents;

async fn card_harness() -> BotHarness {
    BotHarness::bound().await.with_basics().await.with_card().await
}

async fn buy_on_card(harness: &mut BotHarness, amount: &str, installments: &str) {
    harness.say(ANA, "/gasto").await;
    harness.say(ANA, amount).await;
    harness.say(ANA, "geladeira").await;
    harness.tap(ANA, "mercado").await;
    harness.tap(ANA, "Roxinho").await;
    harness.tap(ANA, installments).await;
    harness.tap(ANA, "Hoje").await;
    harness.tap(ANA, "Confirmar").await;
}

#[tokio::test]
async fn new_card_flow_registers_card() {
    let mut harness = BotHarness::bound().await;
    harness.say(ANA, "/novocartao").await;
    harness.say(ANA, "Itaú Click").await;
    harness.say(ANA, "40").await;
    assert!(
        harness.last_html().starts_with("⚠️ Digite um dia de 1 a 31"),
        "{}",
        harness.last_html()
    );
    harness.say(ANA, "25").await;
    harness.say(ANA, "5").await;
    harness.tap(ANA, "Confirmar").await;
    harness.expect_last("Cartão cadastrado");
    harness.say(BIA, "/cartoes").await;
    harness.expect_last("Itaú Click: fecha dia 25, vence dia 5");
}

#[tokio::test]
async fn card_expense_in_installments_lands_on_invoices() {
    let mut harness = card_harness().await;
    buy_on_card(&mut harness, "3.000", "3x").await;
    let card = harness.last_html();
    assert!(card.contains("🔢 Parcelas: 3x") && card.contains("💳 Roxinho"), "{card}");
    let entries = harness.set.services.ledger.list(&EntryFilter::default()).await.unwrap();
    assert_eq!(entries.len(), 3);
    harness.say(BIA, "/fatura").await;
    let invoices = harness.last_html();
    assert!(invoices.contains("Aberta (fecha 03/04): R$ 1.000,00"), "{invoices}");
    assert!(invoices.contains("Parcelas nas próximas faturas: R$ 2.000,00"), "{invoices}");
}

#[tokio::test]
async fn account_expense_skips_installments() {
    let mut harness = card_harness().await;
    harness.say(ANA, "/gasto").await;
    harness.say(ANA, "10").await;
    harness.tap(ANA, "Pular").await;
    harness.tap(ANA, "mercado").await;
    harness.tap(ANA, "Nubank").await;
    harness.expect_last("Quando foi?");
}

#[tokio::test]
async fn typed_installments_and_purchase_undo() {
    let mut harness = card_harness().await;
    harness.say(ANA, "/gasto").await;
    harness.say(ANA, "1800").await;
    harness.tap(ANA, "Pular").await;
    harness.tap(ANA, "mercado").await;
    harness.tap(ANA, "Roxinho").await;
    harness.say(ANA, "18").await;
    harness.tap(ANA, "Hoje").await;
    harness.tap(ANA, "Confirmar").await;
    assert_eq!(harness.set.services.ledger.list(&EntryFilter::default()).await.unwrap().len(), 18);
    harness.tap(BIA, "Desfazer").await;
    assert_eq!(harness.last_toast().as_deref(), Some("Só quem registrou pode desfazer."));
    harness.tap(ANA, "Desfazer").await;
    harness.expect_last("R$ 1.800,00 em 18x no cartão");
    assert!(harness.set.services.ledger.list(&EntryFilter::default()).await.unwrap().is_empty());
}

#[tokio::test]
async fn pay_closed_invoice_in_full() {
    let mut harness = card_harness().await;
    buy_on_card(&mut harness, "500", "1x").await;
    harness.set.clock.set_local_noon(NaiveDate::from_ymd_opt(2026, 4, 5).unwrap());
    harness.say(BIA, "/pagarfatura").await;
    harness.tap(BIA, "Roxinho").await;
    harness.tap(BIA, "Fatura 04/2026").await;
    harness.tap(BIA, "Total R$ 500,00").await;
    harness.tap(BIA, "Nubank").await;
    harness.tap(BIA, "Hoje").await;
    harness.tap(BIA, "Confirmar").await;
    harness.expect_last("Pagamento registrado");
    harness.say(ANA, "/saldo").await;
    harness.expect_last("Disponível: R$ 500,00");
}

#[tokio::test]
async fn pay_invoice_blocks_without_cards_or_debt() {
    let mut harness = BotHarness::bound().await.with_basics().await;
    harness.say(ANA, "/pagarfatura").await;
    harness.expect_last("/novocartao");
    let mut harness = card_harness().await;
    harness.say(ANA, "/pagarfatura").await;
    harness.tap(ANA, "Roxinho").await;
    harness.expect_last("não tem fatura");
}

#[tokio::test]
async fn refund_to_card_lowers_the_invoice() {
    let mut harness = card_harness().await;
    buy_on_card(&mut harness, "300", "1x").await;
    harness.say(ANA, "/estorno").await;
    harness.say(ANA, "100").await;
    harness.say(ANA, "devolução").await;
    harness.tap(ANA, "mercado").await;
    harness.tap(ANA, "Roxinho").await;
    harness.tap(ANA, "Hoje").await;
    harness.tap(ANA, "Confirmar").await;
    let summary = &harness.set.services.cards.summaries().await.unwrap()[0];
    assert_eq!(summary.current.unwrap().statement.outstanding, Cents::new(20_000));
}

#[tokio::test]
async fn refund_to_account_is_an_account_refund() {
    let mut harness = card_harness().await;
    harness.say(ANA, "/estorno").await;
    harness.say(ANA, "50").await;
    harness.tap(ANA, "Pular").await;
    harness.tap(ANA, "mercado").await;
    harness.tap(ANA, "Nubank").await;
    harness.tap(ANA, "Hoje").await;
    harness.tap(ANA, "Confirmar").await;
    harness.say(ANA, "/saldo").await;
    harness.expect_last("Disponível: R$ 1.050,00");
}
