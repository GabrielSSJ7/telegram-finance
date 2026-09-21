// Integration-test crate: helpers panic on failure by design.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::sync::Arc;

use app::fakes::requests::monthly_expense;
use app::jobs::{JobKind, JobRunner};
use app::model::{EntryFilter, RecurrenceMode};
use app::ports::HouseholdNotifier;
use app::services::CreateRecurrence;
use chrono::NaiveDate;
use common::{ANA, BIA, BotHarness, GROUP};
use telegram::gateway::ChatKind;
use telegram::notifier::TelegramNotifier;

fn date(month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, month, day).unwrap()
}

fn runner(harness: &BotHarness) -> JobRunner {
    let notifier =
        Arc::new(TelegramNotifier::new(harness.gateway.clone(), harness.set.services.clone()));
    JobRunner::new(
        harness.set.services.clone(),
        notifier,
        harness.set.store.clone(),
        harness.set.clock.clone(),
    )
}

async fn light_bill(harness: &BotHarness, mode: RecurrenceMode) {
    let accounts = harness.set.services.accounts.list(false).await.unwrap();
    let categories = harness
        .set
        .services
        .categories
        .list(Some(app::model::CategoryKind::Expense))
        .await
        .unwrap();
    let request = monthly_expense("Conta de luz", 18_000, categories[0].id, accounts[0].id, 5);
    let request = CreateRecurrence { mode, starts_on: Some(date(3, 1)), ..request };
    harness.set.services.recurrences.create(request).await.unwrap();
}

#[tokio::test]
async fn daily_report_reaches_the_group() {
    let mut harness = BotHarness::bound().await.with_basics().await;
    harness.say(ANA, "/gasto").await;
    harness.say(ANA, "25").await;
    harness.say(ANA, "pão").await;
    harness.tap(ANA, "mercado").await;
    harness.tap(ANA, "Nubank").await;
    harness.tap(ANA, "Hoje").await;
    harness.tap(ANA, "Confirmar").await;
    runner(&harness).run(JobKind::DailyReport, date(3, 10)).await.unwrap();
    let report = harness.last_html();
    assert!(report.starts_with("<b>📊 Resumo de terça, 10/03</b>"), "{report}");
    assert!(report.contains("• Ana: mercado R$ 25,00 — pão"), "{report}");
    assert!(report.contains("Gasto hoje: R$ 25,00"), "{report}");
    assert!(report.contains("Disponível: R$ 975,00"), "{report}");
}

#[tokio::test]
async fn cycle_report_and_resumo_command() {
    let mut harness = BotHarness::bound().await.with_basics().await;
    runner(&harness).run(JobKind::CycleReport, date(3, 1)).await.unwrap();
    let report = harness.last_html();
    assert!(report.starts_with("<b>🗓️ Fechamento do ciclo 01/02 → 28/02</b>"), "{report}");
    assert!(report.contains("Nenhum gasto no ciclo."), "{report}");
    harness.say(BIA, "/resumo").await;
    assert!(harness.last_html().contains("Nenhum lançamento hoje."), "{}", harness.last_html());
}

#[tokio::test]
async fn confirm_recurrence_can_be_registered_once() {
    let mut harness = BotHarness::bound().await.with_basics().await;
    light_bill(&harness, RecurrenceMode::Confirm).await;
    runner(&harness).run(JobKind::Recurrences, date(3, 10)).await.unwrap();
    assert!(
        harness.last_html().contains("Conta de luz</b> (dia 05/03)"),
        "{}",
        harness.last_html()
    );
    let (message_id, data) = harness.gateway.find_button(GROUP, "Registrar").unwrap();
    harness.press(BIA, message_id, &data).await;
    harness.press(BIA, message_id, &data).await;
    assert!(harness.last_html().contains("Já estava registrado"), "{}", harness.last_html());
    assert_eq!(harness.set.services.ledger.list(&EntryFilter::default()).await.unwrap().len(), 1);
}

#[tokio::test]
async fn confirm_recurrence_can_be_skipped() {
    let mut harness = BotHarness::bound().await.with_basics().await;
    light_bill(&harness, RecurrenceMode::Confirm).await;
    runner(&harness).run(JobKind::Recurrences, date(3, 10)).await.unwrap();
    harness.tap(ANA, "Pular").await;
    assert!(harness.last_html().starts_with("⏭️ Pulado por Ana"), "{}", harness.last_html());
    assert!(harness.set.services.ledger.list(&EntryFilter::default()).await.unwrap().is_empty());
}

#[tokio::test]
async fn auto_recurrence_announces_itself() {
    let harness = BotHarness::bound().await.with_basics().await;
    light_bill(&harness, RecurrenceMode::Auto).await;
    runner(&harness).run(JobKind::Recurrences, date(3, 10)).await.unwrap();
    assert!(
        harness
            .last_html()
            .starts_with("🔁 Registrado automaticamente: <b>Conta de luz</b> R$ 180,00"),
        "{}",
        harness.last_html()
    );
}

#[tokio::test]
async fn new_recurrence_flow_and_deactivation() {
    let mut harness = BotHarness::bound().await.with_basics().await;
    harness.say(ANA, "/recorrente").await;
    harness.tap(ANA, "Entrada").await;
    harness.say(ANA, "Salário").await;
    harness.say(ANA, "8.000").await;
    harness.tap(ANA, "salário").await;
    harness.tap(ANA, "Nubank").await;
    harness.say(ANA, "5").await;
    harness.tap(ANA, "Automático").await;
    harness.tap(ANA, "Confirmar").await;
    assert!(harness.last_html().contains("Recorrência criada"), "{}", harness.last_html());
    harness.say(BIA, "/recorrentes").await;
    assert!(
        harness.last_html().contains("💰 Salário: R$ 8.000,00, todo dia 5"),
        "{}",
        harness.last_html()
    );
    harness.tap(BIA, "Desativar Salário").await;
    assert!(harness.last_html().contains("Nenhuma recorrência ativa"), "{}", harness.last_html());
}

#[tokio::test]
async fn scheduled_messages_are_dropped_before_binding_and_backups_go_to_dms() {
    let mut harness = BotHarness::new().with_basics().await;
    let notifier = TelegramNotifier::new(harness.gateway.clone(), harness.set.services.clone());
    let report = harness.set.services.reports.daily(date(3, 10)).await.unwrap();
    notifier.daily_report(&report).await.unwrap();
    assert_eq!(harness.gateway.sent_count(), 0);
    harness.say_in(ANA, ChatKind::Private, ANA, "/start").await;
    notifier.backup_missing(None).await.unwrap();
    assert!(harness.gateway.last_message(ANA).unwrap().html.contains("nunca rodou"));
}
