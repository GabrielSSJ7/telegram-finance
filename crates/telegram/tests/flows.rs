// Integration-test crate: helpers panic on failure by design.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use app::model::EntryFilter;
use common::{ANA, BIA, BotHarness};
use domain::{AccountKind, Cents, EntryKind};

async fn expense_until_confirmation(harness: &mut BotHarness, user: i64) {
    harness.say(user, "/gasto").await;
    harness.say(user, "10,50").await;
    harness.tap(user, "Pular").await;
    harness.tap(user, "mercado").await;
    harness.tap(user, "Nubank").await;
    harness.tap(user, "Hoje").await;
}

#[tokio::test]
async fn expense_flow_records_entry_and_shows_card() {
    let mut harness = BotHarness::bound().await.with_basics().await;
    expense_until_confirmation(&mut harness, ANA).await;
    harness.expect_last("Tudo certo?");
    harness.tap(ANA, "Confirmar").await;
    let entries = harness.set.services.ledger.list(&EntryFilter::default()).await.unwrap();
    assert_eq!(
        (entries.len(), entries[0].kind, entries[0].amount),
        (1, EntryKind::Expense, Cents::new(1050))
    );
    let card = harness.last_html();
    assert!(card.contains("✅ <b>Gasto registrado</b> · Ana"), "{card}");
    assert!(card.contains("🛒") || card.contains("mercado"), "{card}");
    assert!(card.contains("📅 Data: hoje"), "{card}");
}

#[tokio::test]
async fn double_confirm_saves_once() {
    let mut harness = BotHarness::bound().await.with_basics().await;
    expense_until_confirmation(&mut harness, ANA).await;
    let (message_id, data) = harness.gateway.find_button(common::GROUP, "Confirmar").unwrap();
    harness.press(ANA, message_id, &data).await;
    harness.press(ANA, message_id, &data).await;
    assert_eq!(harness.set.services.ledger.list(&EntryFilter::default()).await.unwrap().len(), 1);
    assert!(harness.last_toast().unwrap().contains("expirou"));
}

#[tokio::test]
async fn undo_button_only_works_for_author() {
    let mut harness = BotHarness::bound().await.with_basics().await;
    expense_until_confirmation(&mut harness, ANA).await;
    harness.tap(ANA, "Confirmar").await;
    harness.tap(BIA, "Desfazer").await;
    assert_eq!(harness.last_toast().as_deref(), Some("Só quem registrou pode desfazer."));
    harness.tap(ANA, "Desfazer").await;
    harness.expect_last("Desfeito por Ana");
    assert!(harness.set.services.ledger.list(&EntryFilter::default()).await.unwrap().is_empty());
}

#[tokio::test]
async fn desfazer_command_removes_my_last_entry() {
    let mut harness = BotHarness::bound().await.with_basics().await;
    harness.say(ANA, "/desfazer").await;
    harness.expect_last("não tem lançamentos");
    expense_until_confirmation(&mut harness, ANA).await;
    harness.tap(ANA, "Confirmar").await;
    harness.say(ANA, "/desfazer").await;
    assert_eq!(harness.last_html(), "↩️ Desfeito: R$ 10,50");
}

async fn both_spouses_start_forms(harness: &mut BotHarness) {
    harness.say(ANA, "/gasto").await;
    harness.say(BIA, "/entrada").await;
    harness.say(ANA, "10").await;
    harness.say(BIA, "5000").await;
}

#[tokio::test]
async fn spouses_fill_forms_at_the_same_time() {
    let mut harness = BotHarness::bound().await.with_basics().await;
    both_spouses_start_forms(&mut harness).await;
    harness.tap_on_card_of(BIA, "Bia", "Pular").await;
    harness.tap_on_card_of(BIA, "Bia", "salário").await;
    let bia_card = harness.card_of("Bia").html;
    assert!(bia_card.contains("Nova entrada</b> · Bia\n💰 Valor: R$ 5.000,00"), "{bia_card}");
    let ana_card = harness.card_of("Ana").html;
    assert!(ana_card.contains("R$ 10,00") && !ana_card.contains("5.000"), "{ana_card}");
}

#[tokio::test]
async fn spouse_cannot_tap_the_other_card() {
    let mut harness = BotHarness::bound().await.with_basics().await;
    both_spouses_start_forms(&mut harness).await;
    harness.tap_on_card_of(BIA, "Ana", "Pular").await;
    assert!(harness.last_toast().unwrap().contains("não é seu"));
    assert!(harness.card_of("Ana").html.contains("Descrição?"));
}

#[tokio::test]
async fn bad_amount_asks_again_with_warning() {
    let mut harness = BotHarness::bound().await.with_basics().await;
    harness.say(ANA, "/gasto").await;
    harness.say(ANA, "dez reais").await;
    assert!(harness.last_html().starts_with("⚠️ Não entendi o valor"), "{}", harness.last_html());
    harness.say(ANA, "10").await;
    harness.expect_last("Descrição?");
}

#[tokio::test]
async fn other_date_accepts_typed_date() {
    let mut harness = BotHarness::bound().await.with_basics().await;
    harness.say(ANA, "/gasto").await;
    harness.say(ANA, "7").await;
    harness.say(ANA, "padaria").await;
    harness.tap(ANA, "mercado").await;
    harness.tap(ANA, "Nubank").await;
    harness.tap(ANA, "Outra data").await;
    harness.say(ANA, "01/03").await;
    harness.expect_last("📅 Data: 01/03");
    harness.tap(ANA, "Confirmar").await;
    let entries = harness.set.services.ledger.list(&EntryFilter::default()).await.unwrap();
    assert_eq!(entries[0].description, "padaria");
}

#[tokio::test]
async fn cancel_button_and_command_drop_the_flow() {
    let mut harness = BotHarness::bound().await.with_basics().await;
    harness.say(ANA, "/gasto").await;
    harness.tap(ANA, "Cancelar").await;
    assert_eq!(harness.last_html(), "✖️ Cancelado.");
    harness.say(ANA, "/entrada").await;
    harness.say(ANA, "/cancelar").await;
    assert!(
        harness
            .gateway
            .messages(common::GROUP)
            .iter()
            .any(|message| message.html == "✖️ Cancelado." && message.keyboard.is_none())
    );
    let before = harness.gateway.sent_count();
    harness.say(ANA, "10").await;
    assert_eq!(harness.gateway.sent_count(), before, "text after cancel is ignored");
}

#[tokio::test]
async fn flows_stop_when_prerequisites_are_missing() {
    let mut harness = BotHarness::bound().await;
    harness.set.store.seed_category("mercado", app::model::CategoryKind::Expense);
    harness.say(ANA, "/gasto").await;
    harness.say(ANA, "10").await;
    harness.tap(ANA, "Pular").await;
    harness.tap(ANA, "mercado").await;
    harness.expect_last("/novaconta");
    harness.say(ANA, "/guardar").await;
    harness.expect_last("/novameta");
    harness.open_account("Nubank", AccountKind::Checking, 0).await;
    harness.say(ANA, "/transferir").await;
    harness.say(ANA, "10").await;
    harness.tap(ANA, "Nubank").await;
    harness.expect_last("duas contas");
}

#[tokio::test]
async fn new_account_flow_creates_account() {
    let mut harness = BotHarness::bound().await.with_basics().await;
    harness.say(ANA, "/novaconta").await;
    harness.say(ANA, "Carteira").await;
    harness.tap(ANA, "Dinheiro").await;
    harness.say(ANA, "50").await;
    harness.tap(ANA, "Confirmar").await;
    harness.expect_last("Conta criada");
    harness.say(ANA, "/contas").await;
    harness.expect_last("Carteira (Dinheiro): R$ 50,00");
}

#[tokio::test]
async fn new_goal_with_deadline_then_deposit() {
    let mut harness = BotHarness::bound().await.with_basics().await;
    harness.say(BIA, "/novameta").await;
    harness.say(BIA, "Casa própria").await;
    harness.say(BIA, "100.000").await;
    harness.tap(BIA, "Zero").await;
    harness.say(BIA, "31/12/2030").await;
    harness.tap(BIA, "Confirmar").await;
    harness.say(BIA, "/guardar").await;
    harness.tap(BIA, "Casa própria").await;
    harness.say(BIA, "200").await;
    harness.tap(BIA, "Nubank").await;
    harness.tap(BIA, "Confirmar").await;
    harness.say(ANA, "/metas").await;
    harness.expect_last("R$ 200,00 de R$ 100.000,00");
    harness.expect_last("/mês até 12/2030");
}
