// Integration-test crate: helpers panic on failure by design.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use app::model::CategoryKind;
use common::{ANA, BIA, BotHarness};
use domain::Cents;

async fn edit(harness: &mut BotHarness, what: &str, which: &str, field: &str) {
    harness.say(ANA, "/editar").await;
    harness.tap(ANA, what).await;
    harness.tap(ANA, which).await;
    harness.tap(ANA, field).await;
}

#[tokio::test]
async fn renames_an_account() {
    let mut harness = BotHarness::bound().await.with_basics().await;
    edit(&mut harness, "Conta", "Nubank", "Nome").await;
    harness.say(ANA, "Nubank da Bia").await;
    harness.tap(ANA, "Confirmar").await;
    harness.expect_last("Cadastro alterado");
    let accounts = harness.set.services.accounts.list(false).await.unwrap();
    assert_eq!(accounts[0].name, "Nubank da Bia");
}

#[tokio::test]
async fn changes_a_card_closing_day_and_a_category_emoji() {
    let mut harness = BotHarness::bound().await.with_basics().await.with_card().await;
    edit(&mut harness, "Cartão", "Roxinho", "Fechamento").await;
    harness.say(ANA, "7").await;
    harness.tap(ANA, "Confirmar").await;
    let cards = harness.set.services.cards.list().await.unwrap();
    assert_eq!(cards[0].schedule.closing_day.get(), 7);
    edit(&mut harness, "Categoria", "mercado", "Emoji").await;
    harness.say(ANA, "🧺").await;
    harness.tap(ANA, "Confirmar").await;
    let expense = CategoryKind::Expense;
    let categories = harness.set.services.categories.list(Some(expense)).await.unwrap();
    assert_eq!(categories[0].emoji.as_deref(), Some("🧺"));
}

#[tokio::test]
async fn changes_a_goal_target_keeping_its_deadline() {
    let mut harness = BotHarness::bound().await.with_basics().await;
    harness.say(ANA, "/novameta").await;
    harness.say(ANA, "Casa própria").await;
    harness.say(ANA, "100000").await;
    harness.tap(ANA, "Zero").await;
    harness.say(ANA, "31/12/2030").await;
    harness.tap(ANA, "Confirmar").await;
    edit(&mut harness, "Meta", "Casa própria", "Objetivo").await;
    harness.say(ANA, "120000").await;
    harness.tap(ANA, "Confirmar").await;
    let goals = harness.set.services.goals.list_progress().await.unwrap();
    assert_eq!(goals[0].goal.target.target, Cents::new(12_000_000));
    assert!(goals[0].goal.target.target_date.is_some(), "the deadline is kept");
}

#[tokio::test]
async fn changes_a_recurring_entry_amount() {
    let mut harness = BotHarness::bound().await.with_basics().await;
    harness.say(ANA, "/recorrente").await;
    harness.tap(ANA, "Gasto").await;
    harness.say(ANA, "Aluguel").await;
    harness.say(ANA, "2000").await;
    harness.tap(ANA, "mercado").await;
    harness.tap(ANA, "Nubank").await;
    harness.say(ANA, "5").await;
    harness.tap(ANA, "Sem fim").await;
    harness.tap(ANA, "Automático").await;
    harness.tap(ANA, "Confirmar").await;
    edit(&mut harness, "Recorrente", "Aluguel", "Valor").await;
    harness.say(ANA, "2500").await;
    harness.tap(ANA, "Confirmar").await;
    harness.say(BIA, "/recorrentes").await;
    harness.expect_last("R$ 2.500,00");
}
