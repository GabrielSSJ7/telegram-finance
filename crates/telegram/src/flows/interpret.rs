//! Reads one input (typed text or a tapped button) as the answer to one
//! field. Problems come back as pt-BR text for the chat.

use app::services::text_rules::{clean_description, clean_name};
use chrono::{Days, NaiveDate};
use domain::installments::MAX_INSTALLMENTS;
use domain::money_parse::parse_brl;
use domain::{AccountKind, Cents};

use super::dates::{parse_typed_date, parse_typed_time};
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
    let answer = match field {
        Field::Date => return date(input, today),
        Field::Amount | Field::GoalTarget => positive_amount(input),
        Field::InitialBalance | Field::AlreadySaved | Field::BudgetLimit => amount_or_zero(input),
        Field::GoalDeadline => deadline(input, today),
        Field::ActualBalance => signed_amount(input),
        Field::CycleStartDay => keep_or(input, day_of_month),
        Field::ReportTime => keep_or(input, report_time),
        Field::Description => description(input),
        Field::AccountName | Field::GoalName | Field::CardName | Field::RecurrenceName => {
            name(input)
        }
        Field::ClosingDay | Field::DueDay | Field::RecurrenceDay => day_of_month(input),
        Field::Installments => installments(input),
        _ => button_choice(field, input),
    };
    answer.map(Interpreted::Answer)
}

pub fn interpret_typed_date(input: &FormInput, today: NaiveDate) -> Result<Answer, String> {
    let FormInput::Text(text) = input else {
        return Err(BAD_DATE.into());
    };
    parse_typed_date(text, today).map(Answer::Date).ok_or_else(|| BAD_DATE.into())
}

fn positive_amount(input: &FormInput) -> Result<Answer, String> {
    match input {
        FormInput::Text(text) => parse_brl(text).map(Answer::Money).map_err(|_| BAD_AMOUNT.into()),
        FormInput::Button(ButtonValue::Money(amount)) if amount.is_positive() => {
            Ok(Answer::Money(*amount))
        }
        FormInput::Button(_) => Err("Digite o valor, por exemplo 10,50.".into()),
    }
}

fn day_of_month(input: &FormInput) -> Result<Answer, String> {
    let problem = || "Digite um dia de 1 a 31.".to_owned();
    let FormInput::Text(text) = input else {
        return Err(problem());
    };
    let day: u8 = text.trim().parse().map_err(|_| problem())?;
    (1..=31).contains(&day).then_some(Answer::Day(day)).ok_or_else(problem)
}

fn installments(input: &FormInput) -> Result<Answer, String> {
    let problem = || format!("Escolha as parcelas ou digite um número de 1 a {MAX_INSTALLMENTS}.");
    let count = match input {
        FormInput::Button(ButtonValue::Installments(count)) => *count,
        FormInput::Text(text) => {
            text.trim().trim_end_matches(['x', 'X']).parse().map_err(|_| problem())?
        }
        FormInput::Button(_) => return Err(problem()),
    };
    (1..=MAX_INSTALLMENTS)
        .contains(&count)
        .then_some(Answer::Installments(count))
        .ok_or_else(problem)
}

fn amount_or_zero(input: &FormInput) -> Result<Answer, String> {
    match input {
        FormInput::Button(ButtonValue::Skip) => Ok(Answer::Money(Cents::ZERO)),
        FormInput::Text(text) if is_zero(text) => Ok(Answer::Money(Cents::ZERO)),
        other => positive_amount(other),
    }
}

/// A balance: `-` in front means overdrawn.
fn signed_amount(input: &FormInput) -> Result<Answer, String> {
    let FormInput::Text(text) = input else {
        return amount_or_zero(input);
    };
    let Some(owed) = text.trim().strip_prefix('-') else {
        return amount_or_zero(input);
    };
    parse_brl(owed).map(|amount| Answer::Money(-amount)).map_err(|_| BAD_AMOUNT.into())
}

/// Settings fields: the [Manter] button keeps the current value.
fn keep_or(
    input: &FormInput,
    typed: fn(&FormInput) -> Result<Answer, String>,
) -> Result<Answer, String> {
    match input {
        FormInput::Button(ButtonValue::Skip) => Ok(Answer::Skipped),
        other => typed(other),
    }
}

fn report_time(input: &FormInput) -> Result<Answer, String> {
    let problem = || "Digite o horário, por exemplo 21:00.".to_owned();
    let FormInput::Text(text) = input else {
        return Err(problem());
    };
    parse_typed_time(text).map(Answer::Time).ok_or_else(problem)
}

fn is_zero(text: &str) -> bool {
    let digits = text.trim().trim_start_matches("R$").trim();
    !digits.is_empty() && digits.chars().all(|character| matches!(character, '0' | ',' | '.'))
}

fn deadline(input: &FormInput, today: NaiveDate) -> Result<Answer, String> {
    match input {
        FormInput::Button(ButtonValue::Skip) => Ok(Answer::Skipped),
        FormInput::Text(text) => match parse_typed_date(text, today) {
            Some(date) if date > today => Ok(Answer::Date(date)),
            Some(_) => Err("O prazo precisa ser uma data futura.".into()),
            None => Err(BAD_DATE.into()),
        },
        FormInput::Button(_) => Err("Digite a data (dd/mm/aaaa) ou toque em Sem prazo.".into()),
    }
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
            Field::PaymentAccount | Field::RefundTarget | Field::CardChoice,
            ButtonValue::Card(id),
        ) => Answer::Card(id),
        (Field::InvoiceChoice, ButtonValue::Invoice(id)) => Answer::Invoice(id),
        (field, ButtonValue::Account(id)) if takes_account(field) => Answer::Account(id),
        (field, value) => return option_choice(field, value),
    };
    Ok(answer)
}

/// Fields answered by picking one of a fixed set of options.
fn option_choice(field: Field, value: ButtonValue) -> Result<Answer, String> {
    match (field, value) {
        (Field::EditFieldChoice, ButtonValue::EditChoice(choice)) => Ok(Answer::EditChoice(choice)),
        (Field::RecurrenceKindChoice, ButtonValue::RecurrenceKind(kind)) => {
            Ok(Answer::RecurrenceKind(kind))
        }
        (Field::RecurrenceModeChoice, ButtonValue::RecurrenceMode(mode)) => {
            Ok(Answer::RecurrenceMode(mode))
        }
        _ => Err(PICK_A_BUTTON.into()),
    }
}

const fn takes_account(field: Field) -> bool {
    matches!(
        field,
        Field::PaymentAccount
            | Field::ReceivingAccount
            | Field::FromAccount
            | Field::ToAccount
            | Field::RefundTarget
    )
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
    fn card_fields() {
        use app::model::{CardId, InvoiceId};
        let (card, invoice) = (CardId::generate(), InvoiceId::generate());
        assert_eq!(tap(Field::PaymentAccount, ButtonValue::Card(card)), got(Answer::Card(card)));
        assert_eq!(
            tap(Field::InvoiceChoice, ButtonValue::Invoice(invoice)),
            got(Answer::Invoice(invoice))
        );
        assert_eq!(
            tap(Field::Installments, ButtonValue::Installments(3)),
            got(Answer::Installments(3))
        );
        assert_eq!(typed(Field::Installments, "18x"), got(Answer::Installments(18)));
        assert!(
            typed(Field::Installments, "49").is_err()
                && tap(Field::Installments, ButtonValue::Skip).is_err()
        );
        assert_eq!(
            tap(Field::Amount, ButtonValue::Money(Cents::new(500))),
            got(Answer::Money(Cents::new(500)))
        );
    }

    #[test]
    fn goal_deadline() {
        let later = NaiveDate::from_ymd_opt(2030, 12, 31).unwrap();
        assert_eq!(typed(Field::GoalDeadline, "31/12/2030"), got(Answer::Date(later)));
        assert_eq!(tap(Field::GoalDeadline, ButtonValue::Skip), got(Answer::Skipped));
        assert!(
            typed(Field::GoalDeadline, "01/01/2020").is_err()
                && typed(Field::GoalDeadline, "logo").is_err()
        );
        assert!(tap(Field::GoalDeadline, ButtonValue::Today).is_err());
        assert_eq!(tap(Field::BudgetLimit, ButtonValue::Skip), got(Answer::Money(Cents::ZERO)));
    }

    #[test]
    fn days_of_month() {
        assert_eq!(typed(Field::ClosingDay, " 5 "), got(Answer::Day(5)));
        assert!(typed(Field::DueDay, "32").is_err() && typed(Field::DueDay, "dez").is_err());
        assert!(tap(Field::ClosingDay, ButtonValue::Skip).is_err());
    }

    #[test]
    fn balances_may_be_negative_or_zero() {
        assert_eq!(typed(Field::ActualBalance, "-50,10"), got(Answer::Money(Cents::new(-5010))));
        assert_eq!(typed(Field::ActualBalance, "1.234"), got(Answer::Money(Cents::new(123_400))));
        assert_eq!(tap(Field::ActualBalance, ButtonValue::Skip), got(Answer::Money(Cents::ZERO)));
        assert_eq!(typed(Field::ActualBalance, "-abc"), Err(BAD_AMOUNT.into()));
    }

    #[test]
    fn settings_fields_keep_or_take_typed_values() {
        assert_eq!(tap(Field::CycleStartDay, ButtonValue::Skip), got(Answer::Skipped));
        assert_eq!(typed(Field::CycleStartDay, "5"), got(Answer::Day(5)));
        let nine = chrono::NaiveTime::from_hms_opt(21, 30, 0).unwrap();
        assert_eq!(typed(Field::ReportTime, "21h30"), got(Answer::Time(nine)));
        assert!(typed(Field::ReportTime, "tarde").is_err());
        assert!(tap(Field::ReportTime, ButtonValue::Today).is_err());
    }

    #[test]
    fn edit_choice_needs_an_edit_button() {
        let choice = crate::flows::EditChoice::Date;
        assert_eq!(
            tap(Field::EditFieldChoice, ButtonValue::EditChoice(choice)),
            got(Answer::EditChoice(choice))
        );
        assert_eq!(tap(Field::EditFieldChoice, ButtonValue::Skip), Err(PICK_A_BUTTON.into()));
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
