use app::model::{
    AccountId, CardId, CategoryId, CategoryKind, EntryId, GoalId, InvoiceId, RecurrenceId,
    RecurrenceKind, RecurrenceMode,
};
use chrono::{NaiveDate, NaiveTime};
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
    /// `3/10` typed for a plan already under way: this installment of that
    /// many, and the amount typed is the value of one installment.
    InstallmentsFrom {
        current: u32,
        total: u32,
    },
    /// A plain number, such as how many installments a financing has.
    Count(u32),
    Day(u8),
    RecurrenceKind(RecurrenceKind),
    RecurrenceMode(RecurrenceMode),
    /// The entry being edited, with a short label for the card.
    Entry {
        id: EntryId,
        income: bool,
        label: String,
    },
    EditChoice(EditChoice),
    Time(NaiveTime),
    CategoryKind(CategoryKind),
    Essential(bool),
    RecordKind(RecordKind),
    RecordField(RecordField),
    Recurrence(RecurrenceId),
}

/// Which kind of record `/editar` changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordKind {
    Account,
    Card,
    Category,
    Goal,
    Recurrence,
}

impl RecordKind {
    pub const ALL: [RecordKind; 5] = [
        RecordKind::Account,
        RecordKind::Card,
        RecordKind::Category,
        RecordKind::Goal,
        RecordKind::Recurrence,
    ];

    pub const fn code(self) -> &'static str {
        match self {
            RecordKind::Account => "account",
            RecordKind::Card => "card",
            RecordKind::Category => "category",
            RecordKind::Goal => "goal",
            RecordKind::Recurrence => "recurrence",
        }
    }

    pub fn from_code(code: &str) -> Option<RecordKind> {
        RecordKind::ALL.into_iter().find(|kind| kind.code() == code)
    }

    /// The fields that kind of record can change.
    pub const fn fields(self) -> &'static [RecordField] {
        match self {
            RecordKind::Account => &[RecordField::Name],
            RecordKind::Card => &[RecordField::Name, RecordField::Closing, RecordField::Due],
            RecordKind::Category => &[RecordField::Name, RecordField::Emoji],
            RecordKind::Goal => &[RecordField::Name, RecordField::Target, RecordField::Deadline],
            RecordKind::Recurrence => &[RecordField::Amount, RecordField::Day, RecordField::Mode],
        }
    }
}

/// Which field of that record `/editar` changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordField {
    Name,
    Emoji,
    Closing,
    Due,
    Target,
    Deadline,
    Amount,
    Day,
    Mode,
}

impl RecordField {
    pub const ALL: [RecordField; 9] = [
        RecordField::Name,
        RecordField::Emoji,
        RecordField::Closing,
        RecordField::Due,
        RecordField::Target,
        RecordField::Deadline,
        RecordField::Amount,
        RecordField::Day,
        RecordField::Mode,
    ];

    pub const fn code(self) -> &'static str {
        match self {
            RecordField::Name => "name",
            RecordField::Emoji => "emoji",
            RecordField::Closing => "closing",
            RecordField::Due => "due",
            RecordField::Target => "target",
            RecordField::Deadline => "deadline",
            RecordField::Amount => "amount",
            RecordField::Day => "day",
            RecordField::Mode => "mode",
        }
    }

    pub fn from_code(code: &str) -> Option<RecordField> {
        RecordField::ALL.into_iter().find(|field| field.code() == code)
    }
}

/// Which part of an entry `/ultimos` → ✏️ changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EditChoice {
    Amount,
    Description,
    Category,
    Date,
}

impl EditChoice {
    pub const ALL: [EditChoice; 4] =
        [EditChoice::Amount, EditChoice::Description, EditChoice::Category, EditChoice::Date];

    pub const fn code(self) -> &'static str {
        match self {
            EditChoice::Amount => "amount",
            EditChoice::Description => "description",
            EditChoice::Category => "category",
            EditChoice::Date => "date",
        }
    }

    pub fn from_code(code: &str) -> Option<EditChoice> {
        EditChoice::ALL.into_iter().find(|choice| choice.code() == code)
    }
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
        self.goal_at(Field::Goal)
    }

    pub fn goal_at(&self, field: Field) -> Option<GoalId> {
        match self.get(field) {
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
            Some(Answer::InstallmentsFrom { total, .. }) => *total,
            _ => 1,
        }
    }

    /// The installment the purchase starts at: above 1 when it was typed
    /// as `3/10`, in which case the amount is per installment.
    pub fn first_installment(&self) -> u32 {
        match self.get(Field::Installments) {
            Some(Answer::InstallmentsFrom { current, .. }) => *current,
            _ => 1,
        }
    }

    pub fn amount_is_per_installment(&self) -> bool {
        matches!(self.get(Field::Installments), Some(Answer::InstallmentsFrom { .. }))
    }

    pub fn count(&self, field: Field) -> Option<u32> {
        match self.get(field) {
            Some(Answer::Count(count)) => Some(*count),
            _ => None,
        }
    }

    pub fn day(&self, field: Field) -> Option<u8> {
        match self.get(field) {
            Some(Answer::Day(day)) => Some(*day),
            _ => None,
        }
    }

    pub fn time(&self, field: Field) -> Option<NaiveTime> {
        match self.get(field) {
            Some(Answer::Time(time)) => Some(*time),
            _ => None,
        }
    }

    pub fn category_kind(&self) -> Option<CategoryKind> {
        match self.get(Field::CategoryKindChoice) {
            Some(Answer::CategoryKind(kind)) => Some(*kind),
            _ => None,
        }
    }

    pub fn record_kind(&self) -> Option<RecordKind> {
        match self.get(Field::RecordKindChoice) {
            Some(Answer::RecordKind(kind)) => Some(*kind),
            _ => None,
        }
    }

    pub fn record_field(&self) -> Option<RecordField> {
        match self.get(Field::RecordFieldChoice) {
            Some(Answer::RecordField(field)) => Some(*field),
            _ => None,
        }
    }

    pub fn recurrence(&self) -> Option<RecurrenceId> {
        match self.get(Field::RecordTarget) {
            Some(Answer::Recurrence(id)) => Some(*id),
            _ => None,
        }
    }

    pub fn essential(&self) -> Option<bool> {
        match self.get(Field::EssentialChoice) {
            Some(Answer::Essential(essential)) => Some(*essential),
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

    /// The edited entry and whether it is income.
    pub fn edited_entry(&self) -> Option<(EntryId, bool)> {
        match self.get(Field::EditTarget) {
            Some(Answer::Entry { id, income, .. }) => Some((*id, *income)),
            _ => None,
        }
    }

    pub fn edit_choice(&self) -> Option<EditChoice> {
        match self.get(Field::EditFieldChoice) {
            Some(Answer::EditChoice(choice)) => Some(*choice),
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
        let nine = NaiveTime::from_hms_opt(21, 0, 0).unwrap();
        answers.set(Field::TodayReportTime, Answer::Time(nine));
        assert_eq!(
            (answers.time(Field::TodayReportTime), answers.time(Field::Amount)),
            (Some(nine), None)
        );
        assert!(answers.has(Field::Amount) && !answers.has(Field::Date));
    }

    #[test]
    fn serializes_tagged() {
        let json = serde_json::to_value(Answer::Date(NaiveDate::from_ymd_opt(2026, 3, 1).unwrap()))
            .unwrap();
        assert_eq!(json, serde_json::json!({"type": "date", "value": "2026-03-01"}));
    }
}
