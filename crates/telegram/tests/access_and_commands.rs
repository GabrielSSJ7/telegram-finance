// Integration-test crate: helpers panic on failure by design.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::{ANA, BIA, BotHarness, GROUP, STRANGER};
use telegram::gateway::{ChatKind, Sender, TextMessage, UpdateKind};

#[tokio::test]
async fn start_binds_group_and_welcomes() {
    let harness = BotHarness::bound().await;
    let settings = harness.set.services.settings.get().await.unwrap();
    assert_eq!(settings.telegram_chat_id, Some(GROUP));
    assert!(harness.last_html().starts_with("Olá!"));
}

#[tokio::test]
async fn unbound_group_ignores_everything_but_start() {
    let mut harness = BotHarness::new();
    harness.say(ANA, "/saldo").await;
    assert_eq!(harness.gateway.sent_count(), 0);
}

#[tokio::test]
async fn strangers_are_ignored_in_group_and_refused_in_private() {
    let mut harness = BotHarness::bound().await;
    let before = harness.gateway.sent_count();
    harness.say(STRANGER, "/saldo").await;
    assert_eq!(harness.gateway.sent_count(), before);
    harness.say_in(STRANGER, ChatKind::Private, STRANGER, "/start").await;
    let reply = harness.gateway.last_message(STRANGER).unwrap();
    assert!(reply.html.contains("privado"));
}

#[tokio::test]
async fn bots_and_other_groups_are_ignored() {
    let mut harness = BotHarness::bound().await;
    let before = harness.gateway.sent_count();
    let robot = Sender { user_id: ANA, display_name: "bot".into(), is_bot: true };
    let message = TextMessage {
        chat_id: GROUP,
        chat_kind: ChatKind::Group,
        message_id: 1,
        sender: robot,
        text: "/saldo".into(),
    };
    harness.deliver(UpdateKind::Text(message)).await;
    harness.say_in(-555, ChatKind::Group, ANA, "/saldo").await;
    harness.say_in(-556, ChatKind::Other, ANA, "/saldo").await;
    assert_eq!(harness.gateway.sent_count(), before);
}

#[tokio::test]
async fn private_start_registers_backup_chat() {
    let mut harness = BotHarness::bound().await;
    harness.say_in(ANA, ChatKind::Private, ANA, "/start").await;
    let members = harness.set.services.members.list().await.unwrap();
    let ana = members.iter().find(|member| member.telegram_user_id == ANA).unwrap();
    assert_eq!(ana.dm_chat_id, Some(ANA));
    harness.say_in(ANA, ChatKind::Private, ANA, "/ajuda").await;
    assert!(harness.gateway.last_message(ANA).unwrap().html.contains("/gasto"));
    harness.say_in(ANA, ChatKind::Private, ANA, "/saldo").await;
    assert!(harness.gateway.last_message(ANA).unwrap().html.contains("grupo"));
}

#[tokio::test]
async fn instant_reports_render() {
    let mut harness = BotHarness::bound().await.with_basics().await;
    harness.say(ANA, "/saldo").await;
    harness.expect_last("Disponível: R$ 1.000,00");
    harness.say(BIA, "/contas").await;
    harness.expect_last("Nubank (Conta corrente)");
    harness.say(ANA, "/metas").await;
    harness.expect_last("/novameta");
    harness.say(ANA, "/categorias").await;
    harness.expect_last("mercado");
    harness.say(ANA, "/ajuda").await;
    harness.expect_last("/desfazer");
    harness.say(ANA, "/voar").await;
    harness.expect_last("Não conheço /voar");
}

#[tokio::test]
async fn plain_text_without_flow_is_ignored() {
    let mut harness = BotHarness::bound().await;
    let before = harness.gateway.sent_count();
    harness.say(ANA, "bom dia").await;
    harness.say(ANA, "/cancelar").await;
    assert_eq!(harness.gateway.sent_count(), before + 1);
    assert_eq!(harness.last_html(), "Nada para cancelar.");
}

#[tokio::test]
async fn membership_greets_household_and_leaves_strangers() {
    let mut harness = BotHarness::new();
    harness.membership(GROUP, ANA).await;
    harness.expect_last("/start");
    harness.membership(-777, STRANGER).await;
    assert_eq!(harness.gateway.left_chats(), vec![-777]);
    harness.say(ANA, "/start").await;
    harness.membership(-888, BIA).await;
    assert_eq!(harness.gateway.left_chats(), vec![-777, -888]);
}

#[tokio::test]
async fn foreign_group_stays_silent_after_binding() {
    let mut harness = BotHarness::bound().await;
    harness.say_in(-999, ChatKind::Group, ANA, "/start").await;
    assert!(harness.gateway.messages(-999).is_empty());
    let settings = harness.set.services.settings.get().await.unwrap();
    assert_eq!(settings.telegram_chat_id, Some(GROUP));
}
