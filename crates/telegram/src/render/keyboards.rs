//! Inline keyboards for each question. Every flow keyboard ends with a
//! [Cancelar] row so the couple can always back out.

use app::model::{AccountId, CategoryKind, RecurrenceKind, RecurrenceMode};
use domain::AccountKind;
use domain::money_format::format_brl;

use super::Catalog;
use super::card::edit_choice_name;
use super::catalog::{account_label, card_label, category_label, kind_name};
use super::reports::invoice_label;
use crate::callback_data::{ButtonValue, flow_button};
use crate::flows::{Answers, Awaiting, EditChoice, Field, FormKind, FormState};
use crate::gateway::{Button, Keyboard};

type Choices = Vec<(ButtonValue, String)>;

/// Buttons for the current question; `None` when the flow cannot go on
/// (for example, no accounts exist yet).
pub fn question_keyboard(state: &FormState, catalog: &Catalog, nonce: &str) -> Option<Keyboard> {
    let (choices, per_row) = match state.awaiting {
        Awaiting::Confirmation => (vec![(ButtonValue::Confirm, "✅ Confirmar".to_owned())], 2),
        Awaiting::TypedDate => (Vec::new(), 2),
        Awaiting::Field(Field::Installments) => (installment_choices(), 4),
        Awaiting::Field(field) => (field_choices(state.form, field, &state.answers, catalog)?, 2),
    };
    Some(with_cancel(choices, nonce, per_row))
}

fn field_choices(
    form: FormKind,
    field: Field,
    answers: &Answers,
    catalog: &Catalog,
) -> Option<Choices> {
    if let Some(label) = skip_label(field) {
        return Some(vec![(ButtonValue::Skip, label.to_owned())]);
    }
    let choices = match field {
        Field::Amount if form == FormKind::PayInvoice => {
            return Some(full_payment_choice(answers, catalog));
        }
        Field::Date => date_choices(),
        Field::AccountKind => kind_choices(),
        Field::RecurrenceKindChoice => recurrence_kind_choices(),
        Field::RecurrenceModeChoice => recurrence_mode_choices(),
        Field::EditFieldChoice => edit_choices(),
        Field::CategoryKindChoice => category_kind_choices(),
        Field::EssentialChoice => essential_choices(),
        field if typed_only(field) => return Some(Vec::new()),
        other => catalog_choices(form, other, answers, catalog),
    };
    (!choices.is_empty()).then_some(choices)
}

/// Typed fields that can also be answered with one button meaning
/// "nothing" or "no change".
const fn skip_label(field: Field) -> Option<&'static str> {
    match field {
        Field::Description | Field::CategoryEmoji => Some("Pular"),
        Field::RecurrenceInstallments => Some("Sem fim"),
        Field::RecurrencePaid => Some("Nenhuma"),
        Field::InitialBalance | Field::AlreadySaved | Field::ActualBalance => Some("Zero"),
        Field::BudgetLimit => Some("🗑️ Remover orçamento"),
        Field::GoalDeadline => Some("Sem prazo"),
        Field::CycleStartDay | Field::YesterdayReportTime | Field::TodayReportTime => {
            Some("Manter")
        }
        _ => None,
    }
}

/// Fields answered by typing; they only get the cancel button.
const fn typed_only(field: Field) -> bool {
    matches!(
        field,
        Field::Amount
            | Field::GoalTarget
            | Field::AccountName
            | Field::GoalName
            | Field::CardName
            | Field::ClosingDay
            | Field::DueDay
            | Field::Installments
            | Field::RecurrenceName
            | Field::RecurrenceDay
            | Field::EditTarget
            | Field::CategoryName
    )
}

/// Choices that come from the couple's own data.
fn catalog_choices(form: FormKind, field: Field, answers: &Answers, catalog: &Catalog) -> Choices {
    match field {
        Field::ExpenseCategory => category_choices(catalog, CategoryKind::Expense),
        Field::IncomeCategory => category_choices(catalog, CategoryKind::Income),
        Field::Goal => goal_choices(catalog),
        Field::ToAccount => account_choices(catalog, excluded_source(form, answers)),
        Field::CardChoice => card_choices(catalog),
        Field::InvoiceChoice => invoice_choices(answers, catalog),
        Field::RefundTarget => [account_choices(catalog, None), card_choices(catalog)].concat(),
        Field::PaymentAccount if matches!(form, FormKind::Expense | FormKind::NewRecurrence) => {
            [account_choices(catalog, None), card_choices(catalog)].concat()
        }
        _ => account_choices(catalog, None),
    }
}

fn excluded_source(form: FormKind, answers: &Answers) -> Option<AccountId> {
    (form == FormKind::Transfer).then(|| answers.account(Field::FromAccount)).flatten()
}

fn category_choices(catalog: &Catalog, kind: CategoryKind) -> Choices {
    let categories = catalog.categories_of(kind).into_iter();
    categories
        .map(|category| (ButtonValue::Category(category.id), category_label(category)))
        .collect()
}

fn goal_choices(catalog: &Catalog) -> Choices {
    catalog
        .goals
        .iter()
        .map(|goal| (ButtonValue::Goal(goal.id), catalog.goal_label(goal.id)))
        .collect()
}

fn account_choices(catalog: &Catalog, excluded: Option<AccountId>) -> Choices {
    let accounts =
        catalog.spendable_accounts().into_iter().filter(|account| Some(account.id) != excluded);
    accounts.map(|account| (ButtonValue::Account(account.id), account_label(account))).collect()
}

fn card_choices(catalog: &Catalog) -> Choices {
    catalog.cards.iter().map(|card| (ButtonValue::Card(card.id), card_label(card))).collect()
}

fn invoice_choices(answers: &Answers, catalog: &Catalog) -> Choices {
    let Some(card) = answers.card(Field::CardChoice) else {
        return Vec::new();
    };
    let views = catalog.payable_invoices(card).into_iter();
    views.map(|view| (ButtonValue::Invoice(view.invoice.id), invoice_label(&view))).collect()
}

/// [Total R$ X] for the chosen invoice, when it still owes money.
fn full_payment_choice(answers: &Answers, catalog: &Catalog) -> Choices {
    let owed =
        answers.invoice().and_then(|id| catalog.invoice(id)).map(|view| view.statement.outstanding);
    let owed = owed.filter(|amount| amount.is_positive());
    owed.map(|amount| vec![(ButtonValue::Money(amount), format!("Total {}", format_brl(amount)))])
        .unwrap_or_default()
}

fn recurrence_kind_choices() -> Choices {
    vec![
        (ButtonValue::RecurrenceKind(RecurrenceKind::Expense), "💸 Gasto".to_owned()),
        (ButtonValue::RecurrenceKind(RecurrenceKind::Income), "💰 Entrada".to_owned()),
    ]
}

fn category_kind_choices() -> Choices {
    vec![
        (ButtonValue::CategoryKind(CategoryKind::Expense), "💸 Gasto".to_owned()),
        (ButtonValue::CategoryKind(CategoryKind::Income), "💰 Entrada".to_owned()),
    ]
}

fn essential_choices() -> Choices {
    vec![
        (ButtonValue::Essential(true), "✅ Sim, essencial".to_owned()),
        (ButtonValue::Essential(false), "Não".to_owned()),
    ]
}

fn recurrence_mode_choices() -> Choices {
    vec![
        (ButtonValue::RecurrenceMode(RecurrenceMode::Auto), "🤖 Automático".to_owned()),
        (ButtonValue::RecurrenceMode(RecurrenceMode::Confirm), "🙋 Perguntar antes".to_owned()),
    ]
}

fn edit_choices() -> Choices {
    EditChoice::ALL
        .into_iter()
        .map(|choice| (ButtonValue::EditChoice(choice), edit_choice_name(choice).to_owned()))
        .collect()
}

fn installment_choices() -> Choices {
    (1..=12).map(|count| (ButtonValue::Installments(count), format!("{count}x"))).collect()
}

fn date_choices() -> Choices {
    vec![
        (ButtonValue::Today, "Hoje".to_owned()),
        (ButtonValue::Yesterday, "Ontem".to_owned()),
        (ButtonValue::OtherDate, "Outra data".to_owned()),
    ]
}

fn kind_choices() -> Choices {
    [AccountKind::Checking, AccountKind::Savings, AccountKind::Cash]
        .into_iter()
        .map(|kind| (ButtonValue::Kind(kind), kind_name(kind).to_owned()))
        .collect()
}

/// `per_row` buttons per row, then the cancel row.
fn with_cancel(choices: Choices, nonce: &str, per_row: usize) -> Keyboard {
    let buttons: Vec<Button> = choices
        .into_iter()
        .map(|(value, label)| Button { label, data: flow_button(nonce, value) })
        .collect();
    let mut rows: Vec<Vec<Button>> = buttons.chunks(per_row).map(<[Button]>::to_vec).collect();
    rows.push(vec![Button {
        label: "✖️ Cancelar".into(),
        data: flow_button(nonce, ButtonValue::Cancel),
    }]);
    Keyboard { rows }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flows::Answer;
    use app::model::{Account, AccountId};
    use chrono::NaiveDate;
    use domain::Cents;

    fn account(name: &str) -> Account {
        let opened_on = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let (kind, initial_balance) = (AccountKind::Checking, Cents::ZERO);
        Account {
            id: AccountId::generate(),
            name: name.into(),
            kind,
            initial_balance,
            opened_on,
            archived: false,
        }
    }

    fn state(form: FormKind, awaiting: Awaiting) -> FormState {
        FormState { form, answers: Answers::default(), awaiting }
    }

    fn labels(state: &FormState, catalog: &Catalog) -> Vec<String> {
        let keyboard = question_keyboard(state, catalog, "n").unwrap();
        keyboard.rows.iter().flatten().map(|button| button.label.clone()).collect()
    }

    #[test]
    fn transfer_destination_excludes_source() {
        let (nubank, itau) = (account("Nubank"), account("Itaú"));
        let catalog = Catalog { accounts: vec![nubank.clone(), itau], ..Catalog::default() };
        let mut asking = state(FormKind::Transfer, Awaiting::Field(Field::ToAccount));
        asking.answers.set(Field::FromAccount, Answer::Account(nubank.id));
        assert_eq!(labels(&asking, &catalog), vec!["🏦 Itaú", "✖️ Cancelar"]);
    }

    #[test]
    fn empty_choices_block_the_flow() {
        let asking = state(FormKind::Expense, Awaiting::Field(Field::PaymentAccount));
        assert_eq!(question_keyboard(&asking, &Catalog::default(), "n"), None);
    }

    #[test]
    fn text_questions_offer_cancel_and_dates_offer_three() {
        let catalog = Catalog::default();
        let amount = state(FormKind::Expense, Awaiting::Field(Field::Amount));
        assert_eq!(labels(&amount, &catalog), vec!["✖️ Cancelar"]);
        let date = state(FormKind::Expense, Awaiting::Field(Field::Date));
        assert_eq!(labels(&date, &catalog), vec!["Hoje", "Ontem", "Outra data", "✖️ Cancelar"]);
    }

    #[test]
    fn confirmation_button_carries_nonce() {
        let confirm = question_keyboard(
            &state(FormKind::NewGoal, Awaiting::Confirmation),
            &Catalog::default(),
            "n",
        );
        assert_eq!(confirm.unwrap().rows[0][0].data, "n|ok");
    }
}
