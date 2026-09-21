// Integration-test crate: helpers panic on failure by design.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::sync::Arc;

use app::fakes::requests::monthly_expense;
use app::jobs::{JobKind, JobRunner};
use app::model::{CategoryKind, EntryFilter, RecurrenceMode, ReportDay};
use app::ports::HouseholdNotifier;
use app::services::CreateRecurrence;
use app::services::EntryOrigin;
use app::services::ledger::{AccountEntry, EntryRequest};
use chrono::NaiveDate;
use common::{ANA, BIA, BotHarness, GROUP};
use domain::Cents;
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
    runner(&harness).run(JobKind::TodayReport, date(3, 10)).await.unwrap();
    let report = harness.last_html();
    assert!(report.starts_with("<b>📊 Resumo de terça, 10/03</b>"), "{report}");
    assert!(report.contains("• Ana: mercado R$ 25,00 — pão"), "{report}");
    assert!(report.contains("Gasto hoje: R$ 25,00"), "{report}");
    assert!(report.contains("Disponível: R$ 975,00"), "{report}");
    runner(&harness).run(JobKind::YesterdayReport, date(3, 11)).await.unwrap();
    let morning = harness.last_html();
    assert!(morning.starts_with("<b>📊 Resumo de ontem (terça, 10/03)</b>"), "{morning}");
    assert!(morning.contains("<b>Ontem</b> (1 lançamentos)"), "{morning}");
    assert!(morning.contains("Gasto ontem: R$ 25,00"), "{morning}");
}

#[tokio::test]
async fn cycle_report_and_resumo_command() {
    let mut harness = BotHarness::bound().await.with_basics().await;
    runner(&harness).run(JobKind::CycleReport, date(3, 1)).await.unwrap();
    let report = harness.last_html();
    assert!(report.starts_with("<b>🗓️ Fechamento do ciclo 01/02 → 28/02</b>"), "{report}");
    assert!(report.contains("Nenhum gasto no ciclo."), "{report}");
    harness.say(BIA, "/resumo").await;
    harness.expect_last("Nenhum lançamento hoje.");
    harness.say(BIA, "/ontem").await;
    harness.expect_last("Nenhum lançamento ontem.");
}

#[tokio::test]
async fn confirm_recurrence_can_be_registered_once() {
    let mut harness = BotHarness::bound().await.with_basics().await;
    light_bill(&harness, RecurrenceMode::Confirm).await;
    runner(&harness).run(JobKind::Recurrences, date(3, 10)).await.unwrap();
    harness.expect_last("Conta de luz</b> (dia 05/03)");
    let (message_id, data) = harness.gateway.find_button(GROUP, "Registrar").unwrap();
    harness.press(BIA, message_id, &data).await;
    harness.press(BIA, message_id, &data).await;
    harness.expect_last("Já estava registrado");
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
    harness.expect_last("Recorrência criada");
    harness.say(BIA, "/recorrentes").await;
    harness.expect_last("💰 Salário: R$ 8.000,00, todo dia 5");
    harness.tap(BIA, "Desativar Salário").await;
    harness.expect_last("Nenhuma recorrência ativa");
}

#[tokio::test]
async fn scheduled_messages_are_dropped_before_binding_and_backups_go_to_dms() {
    let mut harness = BotHarness::new().with_basics().await;
    let notifier = TelegramNotifier::new(harness.gateway.clone(), harness.set.services.clone());
    let report = harness.set.services.reports.daily(date(3, 10)).await.unwrap();
    notifier.daily_report(&report, ReportDay::Today).await.unwrap();
    assert_eq!(harness.gateway.sent_count(), 0);
    harness.say_in(ANA, ChatKind::Private, ANA, "/start").await;
    notifier.backup_missing(None).await.unwrap();
    assert!(harness.gateway.last_message(ANA).unwrap().html.contains("nunca rodou"));
}

#[tokio::test]
async fn resumo_with_a_date_shows_that_day() {
    let mut harness = BotHarness::bound().await.with_basics().await;
    record_bread_on(&harness, date(3, 5)).await;
    harness.say(ANA, "/resumo 05/03").await;
    let report = harness.last_html();
    assert!(report.starts_with("<b>📊 Resumo de quinta, 05/03</b>"), "{report}");
    assert!(report.contains("<b>Dia 05/03</b> (1 lançamentos)"), "{report}");
    assert!(report.contains("Gasto no dia: R$ 25,00"), "{report}");
    harness.say(ANA, "/resumo 09/03").await;
    harness.expect_last("Resumo de ontem (segunda, 09/03)");
    harness.say(ANA, "/resumo 11/03/2026").await;
    harness.expect_last("Esse dia ainda não chegou.");
    harness.say(ANA, "/resumo semana passada").await;
    harness.expect_last("Não entendi a data.");
}

/// A R$ 25,00 "pão" expense at the Nubank account, dated `day`.
async fn record_bread_on(harness: &BotHarness, day: NaiveDate) {
    let account_id = harness.set.services.accounts.list(false).await.unwrap()[0].id;
    let expense = CategoryKind::Expense;
    let category_id = harness.set.services.categories.list(Some(expense)).await.unwrap()[0].id;
    let bread = AccountEntry {
        account_id,
        category_id,
        amount: Cents::new(2_500),
        description: "pão".into(),
        date: Some(day),
    };
    let request = EntryRequest::Expense(bread);
    harness.set.services.ledger.record(request, EntryOrigin::default()).await.unwrap();
}
