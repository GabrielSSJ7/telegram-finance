use app::model::{
    AccountId, CardId, CategoryId, GoalId, InvoiceId, RecurrenceKind, RecurrenceMode,
};
use chrono::NaiveDate;
use domain::{AccountKind, Cents};
use serde::{Deserialize, Serialize};

use super::Field;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum Answer {
    Money(Cents),
    Text(String),
    Skipped,
    Category(CategoryId),
    Account(AccountId),
    Goal(GoalId),
    Date(NaiveDate),
    AccountKind(AccountKind),
    Card(CardId),
    Invoice(InvoiceId),
    Installments(u32),
    Day(u8),
    RecurrenceKind(RecurrenceKind),
    RecurrenceMode(RecurrenceMode),
}

/// Answers given so far, in the order the fields were asked.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Answers(pub Vec<(Field, Answer)>);

impl Answers {
    pub fn get(&self, field: Field) -> Option<&Answer> {
        self.0.iter().find(|(asked, _)| *asked == field).map(|(_, answer)| answer)
    }

    pub fn has(&self, field: Field) -> bool {
        self.get(field).is_some()
    }

    pub fn set(&mut self, field: Field, answer: Answer) {
        self.0.retain(|(asked, _)| *asked != field);
        self.0.push((field, answer));
    }

    pub fn money(&self, field: Field) -> Option<Cents> {
        match self.get(field) {
            Some(Answer::Money(amount)) => Some(*amount),
            _ => None,
        }
    }

    /// Text answer; a skipped field reads as empty.
    pub fn text(&self, field: Field) -> Option<String> {
        match self.get(field) {
            Some(Answer::Text(text)) => Some(text.clone()),
            Some(Answer::Skipped) => Some(String::new()),
            _ => None,
        }
    }

    pub fn category(&self, field: Field) -> Option<CategoryId> {
        match self.get(field) {
            Some(Answer::Category(id)) => Some(*id),
            _ => None,
        }
    }

    pub fn account(&self, field: Field) -> Option<AccountId> {
        match self.get(field) {
            Some(Answer::Account(id)) => Some(*id),
            _ => None,
        }
    }

    pub fn goal(&self) -> Option<GoalId> {
        match self.get(Field::Goal) {
            Some(Answer::Goal(id)) => Some(*id),
            _ => None,
        }
    }

    /// The optional goal deadline; `Some(None)` when skipped.
    pub fn deadline(&self) -> Option<Option<NaiveDate>> {
        match self.get(Field::GoalDeadline) {
            Some(Answer::Date(date)) => Some(Some(*date)),
            Some(Answer::Skipped) => Some(None),
            _ => None,
        }
    }

    pub fn date(&self) -> Option<NaiveDate> {
        match self.get(Field::Date) {
            Some(Answer::Date(date)) => Some(*date),
            _ => None,
        }
    }

    pub fn card(&self, field: Field) -> Option<CardId> {
        match self.get(field) {
            Some(Answer::Card(id)) => Some(*id),
            _ => None,
        }
    }

    pub fn invoice(&self) -> Option<InvoiceId> {
        match self.get(Field::InvoiceChoice) {
            Some(Answer::Invoice(id)) => Some(*id),
            _ => None,
        }
    }

    /// Installments chosen; a card purchase without the step is 1x.
    pub fn installments(&self) -> u32 {
        match self.get(Field::Installments) {
            Some(Answer::Installments(count)) => *count,
            _ => 1,
        }
    }

    pub fn day(&self, field: Field) -> Option<u8> {
        match self.get(field) {
            Some(Answer::Day(day)) => Some(*day),
            _ => None,
        }
    }

    pub fn recurrence_kind(&self) -> Option<RecurrenceKind> {
        match self.get(Field::RecurrenceKindChoice) {
            Some(Answer::RecurrenceKind(kind)) => Some(*kind),
            _ => None,
        }
    }

    pub fn recurrence_mode(&self) -> Option<RecurrenceMode> {
        match self.get(Field::RecurrenceModeChoice) {
            Some(Answer::RecurrenceMode(mode)) => Some(*mode),
            _ => None,
        }
    }

    pub fn account_kind(&self) -> Option<AccountKind> {
        match self.get(Field::AccountKind) {
            Some(Answer::AccountKind(kind)) => Some(*kind),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_replaces_and_typed_getters_match_variant() {
        let mut answers = Answers::default();
        answers.set(Field::Amount, Answer::Money(Cents::new(1)));
        answers.set(Field::Amount, Answer::Money(Cents::new(2)));
        answers.set(Field::Description, Answer::Skipped);
        assert_eq!(answers.0.len(), 2);
        assert_eq!(answers.money(Field::Amount), Some(Cents::new(2)));
        assert_eq!(answers.text(Field::Description), Some(String::new()));
        assert_eq!(answers.category(Field::Amount), None);
        assert!(answers.has(Field::Amount) && !answers.has(Field::Date));
    }

    #[test]
    fn serializes_tagged() {
        let json = serde_json::to_value(Answer::Date(NaiveDate::from_ymd_opt(2026, 3, 1).unwrap()))
            .unwrap();
        assert_eq!(json, serde_json::json!({"type": "date", "value": "2026-03-01"}));
    }
}
