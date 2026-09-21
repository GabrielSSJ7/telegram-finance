use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use super::interpret::{Interpreted, PICK_A_BUTTON, interpret, interpret_typed_date};
use super::{Answer, Answers, Field, FormKind};
use crate::callback_data::ButtonValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "step", content = "field", rename_all = "snake_case")]
pub enum Awaiting {
    Field(Field),
    /// After [Outra data]: waiting for a typed `dd/mm`.
    TypedDate,
    Confirmation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormInput {
    Text(String),
    Button(ButtonValue),
}

/// A form in progress; serialized into the flow store between messages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormState {
    pub form: FormKind,
    pub answers: Answers,
    pub awaiting: Awaiting,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Advance {
    /// Answer accepted; ask what `state.awaiting` says next.
    Next(FormState),
    /// Input rejected; tell `problem` and ask the same thing again.
    Retry {
        state: FormState,
        problem: String,
    },
    /// The couple confirmed; build and run the command.
    Complete(FormState),
    Cancelled,
}

impl FormState {
    /// A form with some answers already known (the entry being edited).
    pub fn with_answers(form: FormKind, answers: Answers) -> Self {
        let mut state = Self { form, answers, awaiting: Awaiting::Confirmation };
        state.awaiting = state.next_awaiting();
        state
    }

    pub fn start(form: FormKind) -> Self {
        let mut state =
            Self { form, answers: Answers::default(), awaiting: Awaiting::Confirmation };
        state.awaiting = state.next_awaiting();
        state
    }

    /// Applies one input.
    ///
    /// ```
    /// use chrono::NaiveDate;
    /// use telegram::flows::{Advance, FormInput, FormKind, FormState};
    /// let today = NaiveDate::from_ymd_opt(2026, 3, 10).unwrap();
    /// let next = FormState::start(FormKind::Expense).advance(&FormInput::Text("10,50".into()), today);
    /// assert!(matches!(next, Advance::Next(_)));
    /// ```
    pub fn advance(self, input: &FormInput, today: NaiveDate) -> Advance {
        if *input == FormInput::Button(ButtonValue::Cancel) {
            return Advance::Cancelled;
        }
        match self.awaiting {
            Awaiting::Confirmation => self.confirm(input),
            Awaiting::TypedDate => self.typed_date(input, today),
            Awaiting::Field(field) => self.answer_field(field, input, today),
        }
    }

    fn confirm(self, input: &FormInput) -> Advance {
        if *input == FormInput::Button(ButtonValue::Confirm) {
            return Advance::Complete(self);
        }
        Advance::Retry { state: self, problem: PICK_A_BUTTON.into() }
    }

    fn typed_date(self, input: &FormInput, today: NaiveDate) -> Advance {
        match interpret_typed_date(input, today) {
            Ok(answer) => self.record(Field::Date, answer),
            Err(problem) => Advance::Retry { state: self, problem },
        }
    }

    fn answer_field(mut self, field: Field, input: &FormInput, today: NaiveDate) -> Advance {
        match interpret(field, input, today) {
            Ok(Interpreted::Answer(answer)) => self.record(field, answer),
            Ok(Interpreted::AskTypedDate) => {
                self.awaiting = Awaiting::TypedDate;
                Advance::Next(self)
            }
            Err(problem) => Advance::Retry { state: self, problem },
        }
    }

    fn record(mut self, field: Field, answer: Answer) -> Advance {
        self.answers.set(field, answer);
        self.awaiting = self.next_awaiting();
        Advance::Next(self)
    }

    fn next_awaiting(&self) -> Awaiting {
        let pending =
            self.form.fields().iter().find(|field| {
                field.applies(self.form, &self.answers) && !self.answers.has(**field)
            });
        pending.map_or(Awaiting::Confirmation, |field| Awaiting::Field(*field))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use app::model::{AccountId, CategoryId};

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 3, 10).unwrap()
    }

    fn next(state: FormState, input: &FormInput) -> FormState {
        match state.advance(input, today()) {
            Advance::Next(state) => state,
            other => panic!("expected Next, got {other:?}"),
        }
    }

    fn button(value: ButtonValue) -> FormInput {
        FormInput::Button(value)
    }

    #[test]
    fn expense_walks_every_field_then_confirms() {
        let state = FormState::start(FormKind::Expense);
        assert_eq!(state.awaiting, Awaiting::Field(Field::Amount));
        let state = next(state, &FormInput::Text("10,50".into()));
        let state = next(state, &button(ButtonValue::Skip));
        let state = next(state, &button(ButtonValue::Category(CategoryId::generate())));
        let state = next(state, &button(ButtonValue::Account(AccountId::generate())));
        assert_eq!(state.awaiting, Awaiting::Field(Field::Date));
        let state = next(state, &button(ButtonValue::Today));
        assert_eq!(state.awaiting, Awaiting::Confirmation);
        assert!(matches!(
            state.advance(&button(ButtonValue::Confirm), today()),
            Advance::Complete(_)
        ));
    }

    #[test]
    fn other_date_waits_for_typed_date() {
        let mut state = FormState::start(FormKind::Expense);
        state.awaiting = Awaiting::Field(Field::Date);
        let state = next(state, &button(ButtonValue::OtherDate));
        assert_eq!(state.awaiting, Awaiting::TypedDate);
        let retry = state.clone().advance(&FormInput::Text("amanhã".into()), today());
        assert!(matches!(retry, Advance::Retry { .. }));
        let state = next(state, &FormInput::Text("01/03".into()));
        assert_eq!(state.answers.date(), NaiveDate::from_ymd_opt(2026, 3, 1));
    }

    #[test]
    fn bad_input_retries_same_field_and_cancel_always_works() {
        let state = FormState::start(FormKind::Income);
        let Advance::Retry { state, problem } =
            state.advance(&FormInput::Text("dez".into()), today())
        else {
            panic!("expected retry");
        };
        assert_eq!(state.awaiting, Awaiting::Field(Field::Amount));
        assert!(problem.contains("valor"));
        assert_eq!(state.advance(&button(ButtonValue::Cancel), today()), Advance::Cancelled);
    }

    #[test]
    fn confirmation_only_accepts_confirm() {
        let mut state = FormState::start(FormKind::NewAccount);
        state.awaiting = Awaiting::Confirmation;
        assert!(matches!(
            state.advance(&FormInput::Text("sim".into()), today()),
            Advance::Retry { .. }
        ));
    }

    #[test]
    fn state_survives_json_round_trip() {
        let state = next(FormState::start(FormKind::Transfer), &FormInput::Text("5".into()));
        let json = serde_json::to_value(&state).unwrap();
        assert_eq!(serde_json::from_value::<FormState>(json).unwrap(), state);
    }
}
