//! Converts the client library's `Update` into [`IncomingUpdate`].

use frankenstein::types::{Chat, ChatMember, ChatType, MaybeInaccessibleMessage, Message, User};
use frankenstein::updates::{Update, UpdateContent};

use super::{
    ButtonPress, ChatKind, IncomingUpdate, MembershipChange, Sender, TextMessage, UpdateKind,
};

pub fn map_update(update: Update) -> IncomingUpdate {
    let kind = match update.content {
        UpdateContent::Message(message) => {
            text_message(&message).map_or(UpdateKind::Ignored, UpdateKind::Text)
        }
        UpdateContent::CallbackQuery(query) => {
            button_press(&query).map_or(UpdateKind::Ignored, UpdateKind::Button)
        }
        UpdateContent::MyChatMember(change) => UpdateKind::Membership(MembershipChange {
            chat_id: change.chat.id,
            chat_kind: chat_kind(&change.chat),
            changed_by: sender(&change.from),
            bot_is_member: is_member(&change.new_chat_member),
        }),
        _ => UpdateKind::Ignored,
    };
    IncomingUpdate { update_id: i64::from(update.update_id), kind }
}

fn text_message(message: &Message) -> Option<TextMessage> {
    let from = message.from.as_deref()?;
    Some(TextMessage {
        chat_id: message.chat.id,
        chat_kind: chat_kind(&message.chat),
        message_id: i64::from(message.message_id),
        sender: sender(from),
        text: message.text.clone()?,
    })
}

fn button_press(query: &frankenstein::types::CallbackQuery) -> Option<ButtonPress> {
    let Some(MaybeInaccessibleMessage::Message(message)) = &query.message else {
        return None;
    };
    Some(ButtonPress {
        callback_id: query.id.clone(),
        chat_id: message.chat.id,
        message_id: i64::from(message.message_id),
        sender: sender(&query.from),
        data: query.data.clone()?,
    })
}

fn sender(user: &User) -> Sender {
    let display_name = match &user.last_name {
        Some(last) => format!("{} {last}", user.first_name),
        None => user.first_name.clone(),
    };
    Sender {
        user_id: i64::try_from(user.id).unwrap_or(i64::MAX),
        display_name,
        is_bot: user.is_bot,
    }
}

fn chat_kind(chat: &Chat) -> ChatKind {
    match chat.type_field {
        ChatType::Private => ChatKind::Private,
        ChatType::Group | ChatType::Supergroup => ChatKind::Group,
        ChatType::Channel => ChatKind::Other,
    }
}

fn is_member(member: &ChatMember) -> bool {
    !matches!(member, ChatMember::Left(_) | ChatMember::Kicked(_))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parse(value: serde_json::Value) -> IncomingUpdate {
        map_update(serde_json::from_value(value).unwrap())
    }

    fn user(id: u64, first: &str) -> serde_json::Value {
        json!({"id": id, "is_bot": false, "first_name": first, "last_name": "Luz"})
    }

    #[test]
    fn maps_group_text_message() {
        let update = parse(json!({"update_id": 7, "message": {"message_id": 3, "date": 0,
            "chat": {"id": -100, "type": "supergroup", "title": "Casa"}, "from": user(11, "Ana"), "text": "/gasto"}}));
        let UpdateKind::Text(message) = update.kind else { panic!("expected text: {update:?}") };
        assert_eq!(
            (update.update_id, message.chat_id, message.chat_kind),
            (7, -100, ChatKind::Group)
        );
        assert_eq!((message.sender.user_id, message.sender.display_name.as_str()), (11, "Ana Luz"));
        assert_eq!(message.text, "/gasto");
    }

    #[test]
    fn maps_button_press() {
        let message = json!({"message_id": 9, "date": 0, "chat": {"id": 5, "type": "private"}, "text": "Valor?"});
        let update =
            parse(json!({"update_id": 8, "callback_query": {"id": "cb1", "from": user(11, "Ana"),
            "message": message, "chat_instance": "x", "data": "abcd1234|ok"}}));
        let UpdateKind::Button(press) = update.kind else { panic!("expected button: {update:?}") };
        assert_eq!((press.callback_id.as_str(), press.chat_id, press.message_id), ("cb1", 5, 9));
        assert_eq!(press.data, "abcd1234|ok");
    }

    #[test]
    fn maps_bot_membership_changes() {
        let member = |status: &str| json!({"status": status, "user": {"id": 1, "is_bot": true, "first_name": "finbot"}});
        let update = parse(
            json!({"update_id": 9, "my_chat_member": {"chat": {"id": -5, "type": "group", "title": "x"},
            "from": user(11, "Ana"), "date": 0, "old_chat_member": member("left"), "new_chat_member": member("member")}}),
        );
        let UpdateKind::Membership(change) = update.kind else {
            panic!("expected membership: {update:?}")
        };
        assert!(change.bot_is_member);
        assert_eq!((change.chat_id, change.changed_by.user_id), (-5, 11));
    }

    #[test]
    fn ignores_messages_without_text_or_sender() {
        let photo = parse(json!({"update_id": 10, "message": {"message_id": 3, "date": 0,
            "chat": {"id": -100, "type": "group", "title": "Casa"}, "from": user(11, "Ana")}}));
        assert_eq!(photo.kind, UpdateKind::Ignored);
        let edited = parse(json!({"update_id": 11, "edited_message": {"message_id": 3, "date": 0,
            "chat": {"id": -100, "type": "group", "title": "Casa"}, "text": "x"}}));
        assert_eq!(edited.kind, UpdateKind::Ignored);
    }
}
