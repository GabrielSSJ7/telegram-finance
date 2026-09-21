//! Inline button payloads. Telegram caps `callback_data` at 64 bytes.
//!
//! Flow buttons carry a nonce derived from the flow's draft id, so a tap
//! on an old keyboard, or on the other spouse's keyboard, is recognised
//! and rejected instead of answering the wrong flow.

use app::model::{AccountId, CategoryId, DraftId, EntryId, GoalId};
use domain::AccountKind;

pub const MAX_CALLBACK_BYTES: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonValue {
    Skip,
    Category(CategoryId),
    Account(AccountId),
    Goal(GoalId),
    Today,
    Yesterday,
    OtherDate,
    Kind(AccountKind),
    Confirm,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallbackPayload {
    Flow {
        nonce: String,
        value: ButtonValue,
    },
    /// [Desfazer] under a confirmation message.
    Undo(EntryId),
}

/// Short, per-flow tag: the random tail of the draft UUID.
///
/// ```
/// use app::model::DraftId;
/// assert_eq!(telegram::callback_data::nonce_of(DraftId::generate()).len(), 8);
/// ```
pub fn nonce_of(draft: DraftId) -> String {
    let simple = draft.0.simple().to_string();
    simple[simple.len() - 8..].to_owned()
}

pub fn flow_button(nonce: &str, value: ButtonValue) -> String {
    format!("{nonce}|{}", encode_value(value))
}

pub fn undo_button(entry: EntryId) -> String {
    format!("u|{entry}")
}

fn encode_value(value: ButtonValue) -> String {
    match value {
        ButtonValue::Skip => "s".into(),
        ButtonValue::Category(id) => format!("c:{id}"),
        ButtonValue::Account(id) => format!("a:{id}"),
        ButtonValue::Goal(id) => format!("g:{id}"),
        ButtonValue::Today => "dt".into(),
        ButtonValue::Yesterday => "dy".into(),
        ButtonValue::OtherDate => "do".into(),
        ButtonValue::Kind(kind) => format!("k:{}", kind.as_str()),
        ButtonValue::Confirm => "ok".into(),
        ButtonValue::Cancel => "x".into(),
    }
}

/// Reads a payload produced by this module; anything else is `None`.
pub fn parse(data: &str) -> Option<CallbackPayload> {
    let (head, tail) = data.split_once('|')?;
    if head == "u" {
        return tail.parse().ok().map(CallbackPayload::Undo);
    }
    let value = decode_value(tail)?;
    Some(CallbackPayload::Flow { nonce: head.to_owned(), value })
}

fn decode_value(code: &str) -> Option<ButtonValue> {
    let fixed = match code {
        "s" => Some(ButtonValue::Skip),
        "dt" => Some(ButtonValue::Today),
        "dy" => Some(ButtonValue::Yesterday),
        "do" => Some(ButtonValue::OtherDate),
        "ok" => Some(ButtonValue::Confirm),
        "x" => Some(ButtonValue::Cancel),
        _ => None,
    };
    fixed.or_else(|| decode_tagged(code))
}

fn decode_tagged(code: &str) -> Option<ButtonValue> {
    let (tag, value) = code.split_once(':')?;
    match tag {
        "c" => value.parse().ok().map(ButtonValue::Category),
        "a" => value.parse().ok().map(ButtonValue::Account),
        "g" => value.parse().ok().map(ButtonValue::Goal),
        "k" => value.parse().ok().map(ButtonValue::Kind),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn every_value() -> Vec<ButtonValue> {
        vec![
            ButtonValue::Skip,
            ButtonValue::Category(CategoryId::generate()),
            ButtonValue::Account(AccountId::generate()),
            ButtonValue::Goal(GoalId::generate()),
            ButtonValue::Today,
            ButtonValue::Yesterday,
            ButtonValue::OtherDate,
            ButtonValue::Kind(AccountKind::Savings),
            ButtonValue::Confirm,
            ButtonValue::Cancel,
        ]
    }

    #[test]
    fn flow_buttons_round_trip_within_limit() {
        let nonce = nonce_of(DraftId::generate());
        for value in every_value() {
            let data = flow_button(&nonce, value);
            assert!(data.len() <= MAX_CALLBACK_BYTES, "{data} is {} bytes", data.len());
            assert_eq!(parse(&data), Some(CallbackPayload::Flow { nonce: nonce.clone(), value }));
        }
    }

    #[test]
    fn undo_round_trips() {
        let entry = EntryId::generate();
        assert_eq!(parse(&undo_button(entry)), Some(CallbackPayload::Undo(entry)));
    }

    #[test]
    fn rejects_foreign_payloads() {
        for data in
            ["", "nonsense", "abc|zz", "abc|c:not-a-uuid", "abc|k:crypto", "u|bad", "abc|q:1"]
        {
            assert_eq!(parse(data), None, "{data}");
        }
    }
}
