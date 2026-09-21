// Integration-test crate: helpers panic on failure by design.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use app::model::EntryFilter;
use chrono::NaiveTime;
use common::{ANA, BIA, BotHarness};
use domain::{Cents, EntryKind};

async fn adjust_nubank_to(harness: &mut BotHarness, typed_balance: &str) {
    harness.say(ANA, "/ajuste").await;
    harness.tap(ANA, "Nubank").await;
    harness.say(ANA, typed_balance).await;
    harness.tap(ANA, "Confirmar").await;
}

#[tokio::test]
async fn ajuste_records_the_gap_to_the_bank_balance() {
    let mut harness = BotHarness::bound().await.with_basics().await;
    adjust_nubank_to(&mut harness, "950").await;
    harness.expect_last("Saldo ajustado");
    harness.expect_last("⚖️ Saldo real: R$ 950,00");
    let entries = harness.set.services.ledger.list(&EntryFilter::default()).await.unwrap();
    assert_eq!((entries[0].kind, entries[0].amount), (EntryKind::AdjustOut, Cents::new(5_000)));
    adjust_nubank_to(&mut harness, "950").await;
    let entries = harness.set.services.ledger.list(&EntryFilter::default()).await.unwrap();
    assert_eq!(entries.len(), 1, "a matching balance records nothing");
}

#[tokio::test]
async fn config_changes_only_what_was_answered() {
    let mut harness = BotHarness::bound().await;
    harness.say(BIA, "/config").await;
    harness.say(BIA, "5").await;
    harness.tap(BIA, "Manter").await;
    harness.tap(BIA, "Manter").await;
    harness.tap(BIA, "Confirmar").await;
    harness.expect_last("Configuração salva");
    harness.expect_last("🔄 Ciclo começa: dia 5");
    let settings = harness.set.services.settings.get().await.unwrap();
    assert_eq!(settings.cycle_start_day.get(), 5);
    assert_eq!(settings.yesterday_report_time, at(9, 0));
    assert_eq!(settings.today_report_time, at(21, 0));
}

#[tokio::test]
async fn config_takes_both_report_times_and_keeps_today_in_the_evening() {
    let mut harness = BotHarness::bound().await;
    harness.say(ANA, "/config").await;
    harness.tap(ANA, "Manter").await;
    harness.say(ANA, "tarde").await;
    harness.expect_last("Digite o horário");
    harness.say(ANA, "7h30").await;
    harness.say(ANA, "18:00").await;
    harness.expect_last("O resumo de hoje só pode ser a partir das 19:00.");
    harness.say(ANA, "20h30").await;
    harness.expect_last("🌅 Resumo de ontem: 07:30");
    harness.expect_last("🌙 Resumo de hoje: 20:30");
    harness.tap(ANA, "Confirmar").await;
    let settings = harness.set.services.settings.get().await.unwrap();
    assert_eq!(settings.cycle_start_day.get(), 1);
    assert_eq!(
        (settings.yesterday_report_time, settings.today_report_time),
        (at(7, 30), at(20, 30))
    );
}

fn at(hour: u32, minute: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(hour, minute, 0).unwrap()
}
