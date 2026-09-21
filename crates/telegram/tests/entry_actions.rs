// Integration-test crate: helpers panic on failure by design.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use app::model::{CategoryKind, EntryFilter, LedgerEntry, MemberId};
use app::services::EntryOrigin;
use app::services::ledger::{AccountEntry, EntryRequest};
use common::{ANA, BIA, BotHarness, GROUP, today};
use domain::Cents;

async fn member_id(harness: &BotHarness, telegram_user_id: i64) -> MemberId {
    let members = harness.set.services.members.list().await.unwrap();
    members.into_iter().find(|member| member.telegram_user_id == telegram_user_id).unwrap().id
}

/// Records "feira" at the Nubank account with the first category of `kind`.
async fn record(
    harness: &BotHarness,
    author: Option<MemberId>,
    cents: i64,
    kind: CategoryKind,
) -> LedgerEntry {
    let account_id = harness.set.services.accounts.list(false).await.unwrap()[0].id;
    let categories = harness.set.services.categories.list(Some(kind)).await.unwrap();
    let entry = AccountEntry {
        account_id,
        category_id: categories[0].id,
        amount: Cents::new(cents),
        description: "feira".into(),
        date: Some(today()),
    };
    let request = match kind {
        CategoryKind::Expense => EntryRequest::Expense(entry),
        CategoryKind::Income => EntryRequest::Income(entry),
    };
    let origin = EntryOrigin { created_by: author, draft: None };
    harness.set.services.ledger.record(request, origin).await.unwrap()
}

async fn entries(harness: &BotHarness) -> Vec<LedgerEntry> {
    harness.set.services.ledger.list(&EntryFilter::default()).await.unwrap()
}

#[tokio::test]
async fn ultimos_lists_entries_with_edit_and_delete_buttons() {
    let mut harness = BotHarness::bound().await.with_basics().await;
    harness.say(ANA, "/ultimos").await;
    harness.expect_last("Nenhum lançamento ainda.");
    record(&harness, Some(member_id(&harness, ANA).await), 1050, CategoryKind::Expense).await;
    record(&harness, None, 500_000, CategoryKind::Income).await;
    harness.say(ANA, "/ultimos").await;
    harness.expect_last("1. hoje 💰 R$ 5.000,00 · salário — feira (automático)");
    harness.expect_last("2. hoje 💸 R$ 10,50");
    harness.expect_last("(Ana)");
    assert!(harness.gateway.find_button(GROUP, "🗑️ 2").is_some());
}

#[tokio::test]
async fn delete_is_limited_to_the_author() {
    let mut harness = BotHarness::bound().await.with_basics().await;
    record(&harness, Some(member_id(&harness, ANA).await), 1050, CategoryKind::Expense).await;
    harness.say(ANA, "/ultimos").await;
    let (message_id, delete) = harness.gateway.find_button(GROUP, "🗑️ 1").unwrap();
    harness.press(BIA, message_id, &delete).await;
    assert_eq!(harness.last_toast().as_deref(), Some("Só quem registrou pode mudar."));
    harness.press(ANA, message_id, &delete).await;
    assert_eq!(harness.last_toast().as_deref(), Some("Apagado"));
    harness.expect_last("Nenhum lançamento ainda.");
    assert!(entries(&harness).await.is_empty());
    harness.press(ANA, message_id, &delete).await;
    assert_eq!(harness.last_toast().as_deref(), Some("Esse lançamento já foi apagado."));
}

#[tokio::test]
async fn edit_changes_the_amount() {
    let mut harness = BotHarness::bound().await.with_basics().await;
    record(&harness, Some(member_id(&harness, ANA).await), 1050, CategoryKind::Expense).await;
    harness.say(ANA, "/ultimos").await;
    harness.tap(ANA, "✏️ 1").await;
    harness.expect_last("Editar lançamento");
    harness.expect_last("R$ 10,50 · feira (hoje)");
    harness.tap(ANA, "Valor").await;
    harness.say(ANA, "20").await;
    harness.tap(ANA, "Confirmar").await;
    harness.expect_last("Lançamento alterado");
    let card = harness.gateway.last_message(GROUP).unwrap();
    assert!(card.keyboard.is_none(), "undo after an edit would delete the entry: {card:?}");
    assert_eq!(entries(&harness).await[0].amount, Cents::new(2000));
}

#[tokio::test]
async fn editing_income_category_offers_income_categories() {
    let mut harness = BotHarness::bound().await.with_basics().await;
    harness.set.store.seed_category("extra", CategoryKind::Income);
    record(&harness, None, 500_000, CategoryKind::Income).await;
    harness.say(BIA, "/ultimos").await;
    harness.tap(BIA, "✏️ 1").await;
    harness.tap(BIA, "Categoria").await;
    assert!(harness.gateway.find_button(GROUP, "mercado").is_none());
    harness.tap(BIA, "extra").await;
    harness.tap(BIA, "Confirmar").await;
    let categories =
        harness.set.services.categories.list(Some(CategoryKind::Income)).await.unwrap();
    let extra = categories.iter().find(|category| category.name == "extra").unwrap().id;
    assert_eq!(entries(&harness).await[0].category_id, Some(extra));
}

#[tokio::test]
async fn card_installments_are_not_editable() {
    let mut harness = BotHarness::bound().await.with_basics().await.with_card().await;
    let card = harness.set.services.cards.list().await.unwrap()[0].id;
    let category =
        harness.set.services.categories.list(Some(CategoryKind::Expense)).await.unwrap()[0].id;
    let purchase = app::fakes::requests::card_purchase(card, category, 900, 3);
    harness.set.services.cards.purchase(purchase, EntryOrigin::default()).await.unwrap();
    harness.say(ANA, "/ultimos").await;
    harness.tap(ANA, "✏️ 1").await;
    assert!(harness.last_toast().unwrap().contains("apague e registre de novo"));
    harness.tap(ANA, "🗑️ 1").await;
    assert!(entries(&harness).await.is_empty(), "deleting one installment removes the purchase");
}

#[tokio::test]
async fn exportar_sends_the_cycle_as_csv() {
    let mut harness = BotHarness::bound().await.with_basics().await;
    record(&harness, Some(member_id(&harness, ANA).await), 1050, CategoryKind::Expense).await;
    harness.say(ANA, "/exportar").await;
    harness.say(ANA, "/exportar 02/2026").await;
    harness.say(ANA, "/exportar fevereiro").await;
    harness.expect_last("/exportar 03/2026");
    let documents = harness.gateway.documents();
    let names: Vec<&str> = documents.iter().map(|document| document.file_name.as_str()).collect();
    assert_eq!(names, ["finbot-2026-03.csv", "finbot-2026-02.csv"]);
    let march = String::from_utf8(documents[0].contents.clone()).unwrap();
    assert!(march.contains("10/03/2026;Gasto;feira;mercado;Nubank;;;;10,50;Ana"), "{march}");
    assert_eq!(documents[1].caption_html, "📄 Lançamentos de 01/02/2026 a 28/02/2026");
}
