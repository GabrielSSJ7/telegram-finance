//! Bot harness: real routing over the in-memory services and a fake
//! Telegram gateway. Ana and Bia are the couple; 99 is a stranger.

#![allow(dead_code)]

use std::sync::Arc;

use app::fakes::FakeServiceSet;
use app::model::{AccountId, CategoryKind};
use app::services::{AllowedUsers, OpenAccount};
use chrono::NaiveDate;
use domain::{AccountKind, Cents};
use telegram::bot::{BotContext, handle_update};
use telegram::fakes::FakeTelegramGateway;
use telegram::gateway::{
    ButtonPress, ChatKind, IncomingUpdate, MembershipChange, Sender, TextMessage, UpdateKind,
};

pub const GROUP: i64 = -1001;
pub const ANA: i64 = 11;
pub const BIA: i64 = 22;
pub const STRANGER: i64 = 99;

pub struct BotHarness {
    pub set: FakeServiceSet,
    pub gateway: Arc<FakeTelegramGateway>,
    pub context: BotContext,
    next_update: i64,
}

pub fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 3, 10).unwrap()
}

pub fn sender(user_id: i64) -> Sender {
    let display_name = match user_id {
        ANA => "Ana",
        BIA => "Bia",
        _ => "Estranho",
    };
    Sender { user_id, display_name: display_name.into(), is_bot: false }
}

impl BotHarness {
    pub fn new() -> Self {
        let set = FakeServiceSet::new(today(), AllowedUsers::new([ANA, BIA]));
        let gateway = Arc::new(FakeTelegramGateway::default());
        let context = BotContext {
            services: set.services.clone(),
            flows: set.store.clone(),
            gateway: gateway.clone(),
            clock: set.clock.clone(),
        };
        Self { set, gateway, context, next_update: 1 }
    }

    /// Harness whose household is already bound to `GROUP`.
    pub async fn bound() -> Self {
        let mut harness = Self::new();
        harness.say(ANA, "/start").await;
        harness
    }

    pub async fn deliver(&mut self, kind: UpdateKind) {
        let update = IncomingUpdate { update_id: self.next_update, kind };
        self.next_update += 1;
        handle_update(&self.context, update).await.unwrap();
    }

    pub async fn say_in(&mut self, chat_id: i64, chat_kind: ChatKind, user_id: i64, text: &str) {
        let message = TextMessage {
            chat_id,
            chat_kind,
            message_id: 0,
            sender: sender(user_id),
            text: text.into(),
        };
        self.deliver(UpdateKind::Text(message)).await;
    }

    pub async fn say(&mut self, user_id: i64, text: &str) {
        self.say_in(GROUP, ChatKind::Group, user_id, text).await;
    }

    pub async fn press(&mut self, user_id: i64, message_id: i64, data: &str) {
        let press = ButtonPress {
            callback_id: format!("cb{}", self.next_update),
            chat_id: GROUP,
            message_id,
            sender: sender(user_id),
            data: data.into(),
        };
        self.deliver(UpdateKind::Button(press)).await;
    }

    /// Taps the newest button in the group whose label contains `label`.
    pub async fn tap(&mut self, user_id: i64, label: &str) {
        let (message_id, data) =
            self.gateway.find_button(GROUP, label).unwrap_or_else(|| panic!("no button {label:?}"));
        self.press(user_id, message_id, &data).await;
    }

    /// Newest card (flow message) that belongs to `owner`.
    pub fn card_of(&self, owner: &str) -> telegram::fakes::FakeMessage {
        let marker = format!("· {owner}");
        let newest = self
            .gateway
            .messages(GROUP)
            .into_iter()
            .rev()
            .find(|message| message.html.contains(&marker));
        newest.unwrap_or_else(|| panic!("no card of {owner}"))
    }

    /// Taps `label` on the newest card that belongs to `owner`.
    pub async fn tap_on_card_of(&mut self, user_id: i64, owner: &str, label: &str) {
        let card = self
            .gateway
            .messages(GROUP)
            .into_iter()
            .rev()
            .find(|message| message.html.contains(&format!("· {owner}")));
        let card = card.unwrap_or_else(|| panic!("no card of {owner}"));
        let buttons = card.keyboard.as_ref().unwrap().rows.iter().flatten();
        let button = buttons.clone().find(|button| button.label.contains(label)).unwrap();
        let data = button.data.clone();
        self.press(user_id, card.message_id, &data).await;
    }

    pub async fn membership(&mut self, chat_id: i64, user_id: i64) {
        let change = MembershipChange {
            chat_id,
            chat_kind: ChatKind::Group,
            changed_by: sender(user_id),
            bot_is_member: true,
        };
        self.deliver(UpdateKind::Membership(change)).await;
    }

    pub fn last_html(&self) -> String {
        self.gateway.last_message(GROUP).map(|message| message.html).unwrap_or_default()
    }

    /// Asserts that the newest group message contains `fragment`.
    #[track_caller]
    pub fn expect_last(&self, fragment: &str) {
        let html = self.last_html();
        assert!(html.contains(fragment), "expected {fragment:?} in the last message:\n{html}");
    }

    pub fn last_toast(&self) -> Option<String> {
        self.gateway.answers().last().and_then(|(_, toast)| toast.clone())
    }

    pub async fn open_account(
        &self,
        name: &str,
        kind: AccountKind,
        initial_cents: i64,
    ) -> AccountId {
        let request = OpenAccount {
            name: name.into(),
            kind,
            initial_balance: Cents::new(initial_cents),
            opened_on: None,
        };
        self.set.services.accounts.open(request).await.unwrap().id
    }

    /// Registers the "Roxinho" card closing on the 3rd, due on the 10th.
    pub async fn with_card(self) -> Self {
        let (closing_day, due_day) =
            (domain::DayOfMonth::new(3).unwrap(), domain::DayOfMonth::new(10).unwrap());
        let request = app::services::OpenCard {
            name: "Roxinho".into(),
            closing_day,
            due_day,
            closing_day_goes_next: true,
            limit: None,
            default_payment_account_id: None,
        };
        self.set.services.cards.open(request).await.unwrap();
        self
    }

    /// Nubank account plus mercado/salário categories.
    pub async fn with_basics(self) -> Self {
        self.open_account("Nubank", AccountKind::Checking, 100_000).await;
        self.set.store.seed_category("mercado", CategoryKind::Expense);
        self.set.store.seed_category("salário", CategoryKind::Income);
        self
    }
}
