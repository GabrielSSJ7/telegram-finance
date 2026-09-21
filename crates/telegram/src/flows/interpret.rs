//! Reads one input (typed text or a tapped button) as the answer to one
//! field. Problems come back as pt-BR text for the chat.

use app::services::text_rules::{clean_description, clean_name};
use chrono::{Days, NaiveDate};
use domain::money_parse::parse_brl;
use domain::{AccountKind, Cents};

use super::dates::parse_typed_date;
use super::engine::FormInput;
use super::{Answer, Field};
use crate::callback_data::ButtonValue;

pub const PICK_A_BUTTON: &str = "Escolha uma das opções nos botões acima.";
pub const BAD_AMOUNT: &str = "Não entendi o valor. Exemplos: 10, 10,50 ou 1.234,56.";
pub const BAD_DATE: &str = "Não entendi a data. Use dd/mm ou dd/mm/aaaa, por exemplo 05/03.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Interpreted {
    Answer(Answer),
    /// The user tapped [Outra data]; the next text is a typed date.
    AskTypedDate,
}

pub fn interpret(field: Field, input: &FormInput, today: NaiveDate) -> Result<Interpreted, String> {
    match field {
        Field::Amount | Field::GoalTarget => positive_amount(input).map(Interpreted::Answer),
        Field::InitialBalance | Field::AlreadySaved => {
            amount_or_zero(input).map(Interpreted::Answer)
        }
        Field::Description => description(input).map(Interpreted::Answer),
        Field::AccountName | Field::GoalName => name(input).map(Interpreted::Answer),
        Field::Date => date(input, today),
        _ => button_choice(field, input).map(Interpreted::Answer),
    }
}

pub fn interpret_typed_date(input: &FormInput, today: NaiveDate) -> Result<Answer, String> {
    let FormInput::Text(text) = input else {
        return Err(BAD_DATE.into());
    };
    parse_typed_date(text, today).map(Answer::Date).ok_or_else(|| BAD_DATE.into())
}

fn positive_amount(input: &FormInput) -> Result<Answer, String> {
    let FormInput::Text(text) = input else {
        return Err("Digite o valor, por exemplo 10,50.".into());
    };
    parse_brl(text).map(Answer::Money).map_err(|_| BAD_AMOUNT.into())
}

fn amount_or_zero(input: &FormInput) -> Result<Answer, String> {
    match input {
        FormInput::Button(ButtonValue::Skip) => Ok(Answer::Money(Cents::ZERO)),
        FormInput::Text(text) if is_zero(text) => Ok(Answer::Money(Cents::ZERO)),
        other => positive_amount(other),
    }
}

fn is_zero(text: &str) -> bool {
    let digits = text.trim().trim_start_matches("R$").trim();
    !digits.is_empty() && digits.chars().all(|character| matches!(character, '0' | ',' | '.'))
}

fn description(input: &FormInput) -> Result<Answer, String> {
    match input {
        FormInput::Button(ButtonValue::Skip) => Ok(Answer::Skipped),
        FormInput::Text(text) => clean_description(text)
            .map(Answer::Text)
            .map_err(|_| "Descrição longa demais (máximo 120 letras).".into()),
        FormInput::Button(_) => Err("Digite a descrição ou toque em Pular.".into()),
    }
}

fn name(input: &FormInput) -> Result<Answer, String> {
    let FormInput::Text(text) = input else {
        return Err("Digite o nome.".into());
    };
    clean_name("name", text, 40)
        .map(Answer::Text)
        .map_err(|_| "Use um nome de 1 a 40 letras.".into())
}

fn date(input: &FormInput, today: NaiveDate) -> Result<Interpreted, String> {
    match input {
        FormInput::Button(ButtonValue::Today) => Ok(Interpreted::Answer(Answer::Date(today))),
        FormInput::Button(ButtonValue::Yesterday) => {
            let yesterday = today.checked_sub_days(Days::new(1)).unwrap_or(today);
            Ok(Interpreted::Answer(Answer::Date(yesterday)))
        }
        FormInput::Button(ButtonValue::OtherDate) => Ok(Interpreted::AskTypedDate),
        FormInput::Text(_) => interpret_typed_date(input, today).map(Interpreted::Answer),
        FormInput::Button(_) => Err(PICK_A_BUTTON.into()),
    }
}

fn button_choice(field: Field, input: &FormInput) -> Result<Answer, String> {
    let FormInput::Button(value) = input else {
        return Err(PICK_A_BUTTON.into());
    };
    let answer = match (field, *value) {
        (Field::ExpenseCategory | Field::IncomeCategory, ButtonValue::Category(id)) => {
            Answer::Category(id)
        }
        (Field::Goal, ButtonValue::Goal(id)) => Answer::Goal(id),
        (Field::AccountKind, ButtonValue::Kind(kind)) if kind != AccountKind::Pot => {
            Answer::AccountKind(kind)
        }
        (
            Field::PaymentAccount | Field::ReceivingAccount | Field::FromAccount | Field::ToAccount,
            ButtonValue::Account(id),
        ) => Answer::Account(id),
        _ => return Err(PICK_A_BUTTON.into()),
    };
    Ok(answer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use app::model::{AccountId, CategoryId, GoalId};

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 3, 10).unwrap()
    }

    fn typed(field: Field, text: &str) -> Result<Interpreted, String> {
        interpret(field, &FormInput::Text(text.into()), today())
    }

    fn tap(field: Field, value: ButtonValue) -> Result<Interpreted, String> {
        interpret(field, &FormInput::Button(value), today())
    }

    // Returns `Result` so assertions compare against `interpret` directly.
    #[allow(clippy::unnecessary_wraps)]
    fn got(answer: Answer) -> Result<Interpreted, String> {
        Ok(Interpreted::Answer(answer))
    }

    #[test]
    fn positive_amounts() {
        assert_eq!(typed(Field::Amount, "10,50"), got(Answer::Money(Cents::new(1050))));
        assert_eq!(typed(Field::Amount, "abc"), Err(BAD_AMOUNT.into()));
        assert!(typed(Field::Amount, "0").is_err());
        assert!(tap(Field::GoalTarget, ButtonValue::Skip).is_err());
    }

    #[test]
    fn zero_allowed_amounts() {
        let zero = got(Answer::Money(Cents::ZERO));
        assert_eq!(typed(Field::InitialBalance, "R$ 0,00"), zero);
        assert_eq!(tap(Field::AlreadySaved, ButtonValue::Skip), zero);
        assert_eq!(typed(Field::AlreadySaved, "25"), got(Answer::Money(Cents::new(2500))));
    }

    #[test]
    fn descriptions_and_names() {
        assert_eq!(typed(Field::Description, " feira "), got(Answer::Text("feira".into())));
        assert_eq!(tap(Field::Description, ButtonValue::Skip), got(Answer::Skipped));
        assert!(typed(Field::Description, &"a".repeat(121)).is_err());
        assert!(tap(Field::Description, ButtonValue::Confirm).is_err());
        assert!(typed(Field::AccountName, "   ").is_err());
        assert!(tap(Field::GoalName, ButtonValue::Skip).is_err());
    }

    #[test]
    fn dates() {
        let yesterday = NaiveDate::from_ymd_opt(2026, 3, 9).unwrap();
        assert_eq!(tap(Field::Date, ButtonValue::Today), got(Answer::Date(today())));
        assert_eq!(tap(Field::Date, ButtonValue::Yesterday), got(Answer::Date(yesterday)));
        assert_eq!(tap(Field::Date, ButtonValue::OtherDate), Ok(Interpreted::AskTypedDate));
        assert_eq!(typed(Field::Date, "09/03"), got(Answer::Date(yesterday)));
        assert_eq!(tap(Field::Date, ButtonValue::Skip), Err(PICK_A_BUTTON.into()));
        let tapped = interpret_typed_date(&FormInput::Button(ButtonValue::Today), today());
        assert_eq!(tapped, Err(BAD_DATE.into()));
    }

    #[test]
    fn button_fields_need_matching_buttons() {
        let (category, account, goal) =
            (CategoryId::generate(), AccountId::generate(), GoalId::generate());
        assert_eq!(
            tap(Field::ExpenseCategory, ButtonValue::Category(category)),
            got(Answer::Category(category))
        );
        assert_eq!(
            tap(Field::ToAccount, ButtonValue::Account(account)),
            got(Answer::Account(account))
        );
        assert_eq!(tap(Field::Goal, ButtonValue::Goal(goal)), got(Answer::Goal(goal)));
        assert!(tap(Field::AccountKind, ButtonValue::Kind(AccountKind::Pot)).is_err());
        assert!(tap(Field::PaymentAccount, ButtonValue::Category(category)).is_err());
        assert_eq!(typed(Field::IncomeCategory, "salário"), Err(PICK_A_BUTTON.into()));
    }
}
