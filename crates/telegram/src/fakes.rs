//! `FakeTelegramGateway`: records what the bot sends and edits, keeps the
//! latest text and buttons of every message, and serves scripted updates.

use std::collections::{BTreeMap, VecDeque};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::Notify;

use crate::gateway::{
    GatewayError, IncomingUpdate, Keyboard, MessageEdit, OutgoingMessage, TelegramGateway,
};

/// A message as the chat would show it now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FakeMessage {
    pub message_id: i64,
    pub chat_id: i64,
    pub html: String,
    pub keyboard: Option<Keyboard>,
}

#[derive(Debug, Default)]
struct FakeTelegramState {
    next_message_id: i64,
    messages: BTreeMap<i64, FakeMessage>,
    sent_count: usize,
    edit_count: usize,
    answers: Vec<(String, Option<String>)>,
    left_chats: Vec<i64>,
    polls: VecDeque<Result<Vec<IncomingUpdate>, GatewayError>>,
    send_failures: VecDeque<GatewayError>,
    offsets_requested: Vec<Option<i64>>,
    webhook_deleted: bool,
    commands: Vec<String>,
}

#[derive(Debug, Default)]
pub struct FakeTelegramGateway {
    state: Mutex<FakeTelegramState>,
    drained: Notify,
}

impl FakeTelegramGateway {
    fn lock(&self) -> MutexGuard<'_, FakeTelegramState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Queues the result of one future `getUpdates` call.
    pub fn push_poll(&self, result: Result<Vec<IncomingUpdate>, GatewayError>) {
        self.lock().polls.push_back(result);
    }

    /// Makes the next `send_message` fail with `error`.
    pub fn fail_next_send(&self, error: GatewayError) {
        self.lock().send_failures.push_back(error);
    }

    /// Resolves once the poller asked for updates with nothing queued.
    pub async fn wait_until_drained(&self) {
        self.drained.notified().await;
    }

    pub fn messages(&self, chat_id: i64) -> Vec<FakeMessage> {
        self.lock()
            .messages
            .values()
            .filter(|message| message.chat_id == chat_id)
            .cloned()
            .collect()
    }

    pub fn last_message(&self, chat_id: i64) -> Option<FakeMessage> {
        self.messages(chat_id).pop()
    }

    /// Newest message in `chat_id` with a button labelled `label`, and that
    /// button's callback data.
    pub fn find_button(&self, chat_id: i64, label: &str) -> Option<(i64, String)> {
        self.messages(chat_id).into_iter().rev().find_map(|message| {
            let buttons = message.keyboard.as_ref()?.rows.iter().flatten();
            let found = buttons.clone().find(|button| button.label.contains(label))?;
            Some((message.message_id, found.data.clone()))
        })
    }

    pub fn sent_count(&self) -> usize {
        self.lock().sent_count
    }

    pub fn edit_count(&self) -> usize {
        self.lock().edit_count
    }

    pub fn answers(&self) -> Vec<(String, Option<String>)> {
        self.lock().answers.clone()
    }

    pub fn left_chats(&self) -> Vec<i64> {
        self.lock().left_chats.clone()
    }

    pub fn offsets_requested(&self) -> Vec<Option<i64>> {
        self.lock().offsets_requested.clone()
    }

    pub fn webhook_deleted(&self) -> bool {
        self.lock().webhook_deleted
    }

    pub fn commands(&self) -> Vec<String> {
        self.lock().commands.clone()
    }
}

#[async_trait]
impl TelegramGateway for FakeTelegramGateway {
    async fn delete_webhook(&self) -> Result<(), GatewayError> {
        self.lock().webhook_deleted = true;
        Ok(())
    }

    async fn set_commands(&self, commands: &[(&str, &str)]) -> Result<(), GatewayError> {
        self.lock().commands = commands.iter().map(|(command, _)| (*command).to_owned()).collect();
        Ok(())
    }

    async fn get_updates(
        &self,
        offset: Option<i64>,
        _timeout: Duration,
    ) -> Result<Vec<IncomingUpdate>, GatewayError> {
        let next = {
            let mut state = self.lock();
            state.offsets_requested.push(offset);
            state.polls.pop_front()
        };
        if let Some(result) = next {
            return result;
        }
        self.drained.notify_one();
        std::future::pending().await
    }

    async fn send_message(&self, message: &OutgoingMessage) -> Result<i64, GatewayError> {
        let mut state = self.lock();
        if let Some(error) = state.send_failures.pop_front() {
            return Err(error);
        }
        state.next_message_id += 1;
        let message_id = state.next_message_id;
        state.sent_count += 1;
        let stored = FakeMessage {
            message_id,
            chat_id: message.chat_id,
            html: message.html.clone(),
            keyboard: message.keyboard.clone(),
        };
        state.messages.insert(message_id, stored);
        Ok(message_id)
    }

    async fn edit_message(&self, edit: &MessageEdit) -> Result<(), GatewayError> {
        let mut state = self.lock();
        state.edit_count += 1;
        let Some(message) = state.messages.get_mut(&edit.message_id) else {
            return Err(GatewayError::Api {
                code: 400,
                description: format!("message {} not found", edit.message_id),
            });
        };
        message.html.clone_from(&edit.html);
        message.keyboard.clone_from(&edit.keyboard);
        Ok(())
    }

    async fn remove_keyboard(&self, _chat_id: i64, message_id: i64) -> Result<(), GatewayError> {
        if let Some(message) = self.lock().messages.get_mut(&message_id) {
            message.keyboard = None;
        }
        Ok(())
    }

    async fn answer_button(
        &self,
        callback_id: &str,
        toast: Option<&str>,
    ) -> Result<(), GatewayError> {
        self.lock().answers.push((callback_id.to_owned(), toast.map(str::to_owned)));
        Ok(())
    }

    async fn leave_chat(&self, chat_id: i64) -> Result<(), GatewayError> {
        self.lock().left_chats.push(chat_id);
        Ok(())
    }
}
