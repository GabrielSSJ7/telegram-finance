//! Inline button payloads. Telegram caps `callback_data` at 64 bytes.
//!
//! Flow buttons carry a nonce derived from the flow's draft id, so a tap
//! on an old keyboard, or on the other spouse's keyboard, is recognised
//! and rejected instead of answering the wrong flow.

use app::model::{
    AccountId, CardId, CategoryId, CategoryKind, DraftId, EntryId, GoalId, InvoiceId, PurchaseId,
    RecurrenceId, RecurrenceKind, RecurrenceMode,
};
use chrono::NaiveDate;

use crate::flows::EditChoice;
use domain::{AccountKind, Cents};

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
    Card(CardId),
    Invoice(InvoiceId),
    Installments(u32),
    /// A suggested amount, such as the full invoice total.
    Money(Cents),
    RecurrenceKind(RecurrenceKind),
    RecurrenceMode(RecurrenceMode),
    EditChoice(EditChoice),
    CategoryKind(CategoryKind),
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
    /// [Desfazer] under a card purchase: removes every installment.
    UndoPurchase(PurchaseId),
    /// [Registrar] on a bill that asks before recording.
    RecordRecurrence(RecurrenceId, NaiveDate),
    /// [Pular] on the same message.
    SkipRecurrence(RecurrenceId, NaiveDate),
    /// [Desativar] in `/recorrentes`.
    DeactivateRecurrence(RecurrenceId),
    /// [✏️] in `/ultimos`.
    EditEntry(EntryId),
    /// [🗑️] in `/ultimos`.
    DeleteEntry(EntryId),
    /// A category button in `/extrato`: its entries between two dates.
    CategoryStatement {
        category: CategoryId,
        from: NaiveDate,
        to: NaiveDate,
    },
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

pub fn undo_purchase_button(purchase: PurchaseId) -> String {
    format!("up|{purchase}")
}

pub fn record_recurrence_button(recurrence: RecurrenceId, date: NaiveDate) -> String {
    format!("rr|{recurrence}|{date}")
}

pub fn skip_recurrence_button(recurrence: RecurrenceId, date: NaiveDate) -> String {
    format!("rs|{recurrence}|{date}")
}

pub fn deactivate_recurrence_button(recurrence: RecurrenceId) -> String {
    format!("rd|{recurrence}")
}

pub fn edit_entry_button(entry: EntryId) -> String {
    format!("ee|{entry}")
}

/// Dates as `yyyymmdd` keep the payload at 57 bytes.
pub fn category_statement_button(category: CategoryId, from: NaiveDate, to: NaiveDate) -> String {
    format!("cs|{category}|{}|{}", from.format("%Y%m%d"), to.format("%Y%m%d"))
}

pub fn delete_entry_button(entry: EntryId) -> String {
    format!("ed|{entry}")
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
        ButtonValue::Card(id) => format!("cc:{id}"),
        ButtonValue::Invoice(id) => format!("i:{id}"),
        ButtonValue::Installments(count) => format!("n:{count}"),
        ButtonValue::Money(amount) => format!("m:{}", amount.value()),
        ButtonValue::RecurrenceKind(kind) => format!("rk:{}", kind.as_str()),
        ButtonValue::RecurrenceMode(mode) => format!("rm:{}", mode.as_str()),
        ButtonValue::EditChoice(choice) => format!("ec:{}", choice.code()),
        ButtonValue::CategoryKind(kind) => format!("ck:{}", kind.as_str()),
        ButtonValue::Confirm => "ok".into(),
        ButtonValue::Cancel => "x".into(),
    }
}

/// Reads a payload produced by this module; anything else is `None`.
pub fn parse(data: &str) -> Option<CallbackPayload> {
    let (head, tail) = data.split_once('|')?;
    match head {
        "u" => return tail.parse().ok().map(CallbackPayload::Undo),
        "up" => return tail.parse().ok().map(CallbackPayload::UndoPurchase),
        "rd" => return tail.parse().ok().map(CallbackPayload::DeactivateRecurrence),
        "ee" => return tail.parse().ok().map(CallbackPayload::EditEntry),
        "ed" => return tail.parse().ok().map(CallbackPayload::DeleteEntry),
        "cs" => return category_statement_payload(tail),
        "rr" | "rs" => return recurrence_payload(head, tail),
        _ => {}
    }
    let value = decode_value(tail)?;
    Some(CallbackPayload::Flow { nonce: head.to_owned(), value })
}

fn category_statement_payload(tail: &str) -> Option<CallbackPayload> {
    let mut parts = tail.split('|');
    let category = parts.next()?.parse().ok()?;
    let date = |text: &str| NaiveDate::parse_from_str(text, "%Y%m%d").ok();
    let (from, to) = (date(parts.next()?)?, date(parts.next()?)?);
    Some(CallbackPayload::CategoryStatement { category, from, to })
}

fn recurrence_payload(head: &str, tail: &str) -> Option<CallbackPayload> {
    let (id, date) = tail.split_once('|')?;
    let (id, date) = (id.parse().ok()?, date.parse().ok()?);
    Some(if head == "rr" {
        CallbackPayload::RecordRecurrence(id, date)
    } else {
        CallbackPayload::SkipRecurrence(id, date)
    })
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
        "cc" => value.parse().ok().map(ButtonValue::Card),
        "i" => value.parse().ok().map(ButtonValue::Invoice),
        "n" => value.parse().ok().map(ButtonValue::Installments),
        "ck" => value.parse().ok().map(ButtonValue::CategoryKind),
        "m" => value.parse().ok().map(|cents| ButtonValue::Money(Cents::new(cents))),
        "rk" => value.parse().ok().map(ButtonValue::RecurrenceKind),
        "rm" => value.parse().ok().map(ButtonValue::RecurrenceMode),
        "ec" => EditChoice::from_code(value).map(ButtonValue::EditChoice),
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
            ButtonValue::Card(CardId::generate()),
            ButtonValue::Invoice(InvoiceId::generate()),
            ButtonValue::Installments(12),
            ButtonValue::Money(Cents::new(123_456)),
            ButtonValue::RecurrenceKind(RecurrenceKind::Income),
            ButtonValue::RecurrenceMode(RecurrenceMode::Confirm),
            ButtonValue::EditChoice(EditChoice::Category),
            ButtonValue::CategoryKind(CategoryKind::Income),
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
    fn category_statement_round_trips_within_limit() {
        let category = CategoryId::generate();
        let (from, to) = (
            NaiveDate::from_ymd_opt(2026, 9, 5).unwrap(),
            NaiveDate::from_ymd_opt(2026, 10, 4).unwrap(),
        );
        let data = category_statement_button(category, from, to);
        assert!(data.len() <= MAX_CALLBACK_BYTES, "{data} is {} bytes", data.len());
        assert_eq!(parse(&data), Some(CallbackPayload::CategoryStatement { category, from, to }));
        assert_eq!(parse(&format!("cs|{category}|20261304|20261004")), None);
        assert_eq!(parse(&format!("cs|{category}|20260905")), None);
    }

    #[test]
    fn undo_round_trips() {
        let entry = EntryId::generate();
        assert_eq!(parse(&undo_button(entry)), Some(CallbackPayload::Undo(entry)));
        let purchase = PurchaseId::generate();
        assert_eq!(
            parse(&undo_purchase_button(purchase)),
            Some(CallbackPayload::UndoPurchase(purchase))
        );
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
