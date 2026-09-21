use app::model::{RecurrenceKind, RecurrenceMode};
use chrono::{Days, NaiveDate};
use domain::money_format::format_brl;

use super::Catalog;
use super::catalog::kind_name;
use super::keyboards::question_keyboard;
use super::reports::invoice_label;
use crate::flows::dates::short_date;
use crate::flows::{Answer, Awaiting, Field, FormKind, FormState};
use crate::gateway::Keyboard;
use crate::html::escape;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CardView {
    /// The card with the current question and its buttons.
    Ask { html: String, keyboard: Keyboard },
    /// The flow cannot continue; `html` says what to set up first.
    Blocked { html: String },
}

/// Everything a card needs besides the form itself.
#[derive(Debug, Clone, Copy)]
pub struct CardContext<'a> {
    pub owner: &'a str,
    pub catalog: &'a Catalog,
    pub nonce: &'a str,
    pub today: NaiveDate,
}

pub fn card_view(state: &FormState, context: CardContext<'_>, problem: Option<&str>) -> CardView {
    let keyboard = question_keyboard(state, context.catalog, context.nonce);
    let Some(keyboard) = keyboard else {
        return CardView::Blocked { html: blocked_text(state.awaiting).to_owned() };
    };
    let warning = problem.map(|text| format!("⚠️ {}\n\n", escape(text))).unwrap_or_default();
    let body = answered_card(state, context, &format!("<b>{}</b>", state.form.title()));
    let html = format!("{warning}{body}\n\n{}", question(state.form, state.awaiting));
    CardView::Ask { html, keyboard }
}

/// The card after the command ran, headed by `headline`.
pub fn committed_card(state: &FormState, context: CardContext<'_>, headline: &str) -> String {
    answered_card(state, context, &format!("✅ <b>{headline}</b>"))
}

fn answered_card(state: &FormState, context: CardContext<'_>, header: &str) -> String {
    let lines: Vec<String> = state
        .form
        .fields()
        .iter()
        .filter_map(|field| {
            state.answers.get(*field).map(|answer| answer_line(*field, answer, context))
        })
        .collect();
    let owner = escape(context.owner);
    if lines.is_empty() {
        return format!("{header} · {owner}");
    }
    format!("{header} · {owner}\n{}", lines.join("\n"))
}

fn answer_line(field: Field, answer: &Answer, context: CardContext<'_>) -> String {
    let (icon, label) = field_text(field);
    format!("{icon} {label}: {}", answer_value(answer, context))
}

pub fn answer_value(answer: &Answer, context: CardContext<'_>) -> String {
    match answer {
        Answer::Money(amount) => format_brl(*amount),
        Answer::Text(text) if !text.is_empty() => escape(text),
        Answer::Text(_) | Answer::Skipped => "—".to_owned(),
        Answer::Date(date) => relative_date(*date, context.today),
        Answer::AccountKind(kind) => kind_name(*kind).to_owned(),
        Answer::Installments(count) => format!("{count}x"),
        Answer::Day(day) => format!("dia {day}"),
        Answer::RecurrenceKind(kind) => recurrence_kind_name(*kind).to_owned(),
        Answer::RecurrenceMode(mode) => recurrence_mode_name(*mode).to_owned(),
        reference => referenced_name(reference, context.catalog),
    }
}

/// Answers that point at the couple's own records.
fn referenced_name(answer: &Answer, catalog: &Catalog) -> String {
    match answer {
        Answer::Category(id) => catalog.category_label(*id),
        Answer::Account(id) => catalog.account_label(*id),
        Answer::Goal(id) => catalog.goal_label(*id),
        Answer::Card(id) => catalog.card_label(*id),
        Answer::Invoice(id) => {
            catalog.invoice(*id).map_or_else(|| "fatura".to_owned(), |view| invoice_label(&view))
        }
        _ => String::new(),
    }
}

const fn recurrence_kind_name(kind: RecurrenceKind) -> &'static str {
    match kind {
        RecurrenceKind::Income => "Entrada",
        RecurrenceKind::Expense => "Gasto",
    }
}

const fn recurrence_mode_name(mode: RecurrenceMode) -> &'static str {
    match mode {
        RecurrenceMode::Auto => "Automático",
        RecurrenceMode::Confirm => "Perguntar antes",
    }
}

pub fn relative_date(date: NaiveDate, today: NaiveDate) -> String {
    if date == today {
        return "hoje".to_owned();
    }
    if today.checked_sub_days(Days::new(1)) == Some(date) {
        return "ontem".to_owned();
    }
    short_date(date, today)
}

/// Icon and label shown before each answer on a card.
const FIELD_TEXT: &[(Field, &str, &str)] = &[
    (Field::Amount, "💰", "Valor"),
    (Field::Description, "📝", "Descrição"),
    (Field::ExpenseCategory, "🏷️", "Categoria"),
    (Field::IncomeCategory, "🏷️", "Categoria"),
    (Field::PaymentAccount, "🏦", "Pago com"),
    (Field::ReceivingAccount, "🏦", "Conta"),
    (Field::FromAccount, "↗️", "De"),
    (Field::ToAccount, "↘️", "Para"),
    (Field::Goal, "🎯", "Meta"),
    (Field::Date, "📅", "Data"),
    (Field::AccountName, "✏️", "Nome"),
    (Field::AccountKind, "🗂️", "Tipo"),
    (Field::InitialBalance, "💰", "Saldo atual"),
    (Field::GoalName, "✏️", "Nome"),
    (Field::GoalTarget, "🏁", "Objetivo"),
    (Field::AlreadySaved, "💰", "Já guardado"),
    (Field::Installments, "🔢", "Parcelas"),
    (Field::CardName, "✏️", "Nome"),
    (Field::ClosingDay, "📆", "Fecha"),
    (Field::DueDay, "📆", "Vence"),
    (Field::CardChoice, "💳", "Cartão"),
    (Field::InvoiceChoice, "🧾", "Fatura"),
    (Field::RefundTarget, "↩️", "Volta para"),
    (Field::RecurrenceKindChoice, "🔁", "Tipo"),
    (Field::RecurrenceName, "✏️", "Nome"),
    (Field::RecurrenceDay, "📆", "Todo dia"),
    (Field::RecurrenceModeChoice, "⚙️", "Registro"),
];

fn field_text(field: Field) -> (&'static str, &'static str) {
    let found = FIELD_TEXT.iter().find(|(known, _, _)| *known == field);
    found.map_or(("•", "Campo"), |(_, icon, label)| (*icon, *label))
}

fn question(form: FormKind, awaiting: Awaiting) -> &'static str {
    match awaiting {
        Awaiting::Confirmation => "Tudo certo? Toque em ✅ Confirmar.",
        Awaiting::TypedDate => "Digite a data (dd/mm):",
        Awaiting::Field(field) => field_question(form, field),
    }
}

/// Question per field. Rows naming a form win over the generic row.
const QUESTIONS: &[(Option<FormKind>, Field, &str)] = &[
    (Some(FormKind::Income), Field::Amount, "Quanto entrou?"),
    (Some(FormKind::Transfer), Field::Amount, "Quanto vai transferir?"),
    (Some(FormKind::PotDeposit), Field::Amount, "Quanto vai guardar?"),
    (Some(FormKind::PotWithdraw), Field::Amount, "Quanto vai resgatar?"),
    (Some(FormKind::PayInvoice), Field::Amount, "Quanto vai pagar? Digite ou toque em Total."),
    (Some(FormKind::Refund), Field::Amount, "Quanto voltou?"),
    (Some(FormKind::PayInvoice), Field::FromAccount, "Pagar com qual conta?"),
    (Some(FormKind::PotDeposit), Field::FromAccount, "De qual conta sai o dinheiro?"),
    (Some(FormKind::PotWithdraw), Field::ToAccount, "Para qual conta volta o dinheiro?"),
    (None, Field::Amount, "Quanto foi?"),
    (None, Field::Description, "Descrição? (ex.: mercado, farmácia)"),
    (None, Field::ExpenseCategory, "Qual categoria?"),
    (None, Field::IncomeCategory, "Qual categoria?"),
    (None, Field::PaymentAccount, "Pago com qual conta ou cartão?"),
    (None, Field::Installments, "Em quantas parcelas? Toque ou digite (ex.: 18)."),
    (None, Field::CardName, "Nome do cartão? (ex.: Nubank, Itaú)"),
    (None, Field::ClosingDay, "Em que dia a fatura fecha? (1 a 31)"),
    (None, Field::DueDay, "Em que dia ela vence? (1 a 31)"),
    (None, Field::CardChoice, "Qual cartão?"),
    (None, Field::InvoiceChoice, "Qual fatura?"),
    (None, Field::RefundTarget, "Para onde o dinheiro volta?"),
    (None, Field::ReceivingAccount, "Entrou em qual conta?"),
    (None, Field::FromAccount, "De qual conta sai?"),
    (None, Field::ToAccount, "Para qual conta vai?"),
    (None, Field::Goal, "Qual meta?"),
    (None, Field::Date, "Quando foi?"),
    (None, Field::AccountName, "Nome da conta? (ex.: Nubank, Carteira)"),
    (None, Field::AccountKind, "Que tipo de conta?"),
    (None, Field::InitialBalance, "Saldo atual dela? Digite o valor ou toque em Zero."),
    (None, Field::GoalName, "Nome da meta? (ex.: Casa própria)"),
    (None, Field::GoalTarget, "Quanto quer juntar?"),
    (None, Field::AlreadySaved, "Quanto já tem guardado? Digite o valor ou toque em Zero."),
    (Some(FormKind::NewRecurrence), Field::Amount, "Qual o valor todo mês?"),
    (Some(FormKind::NewRecurrence), Field::ReceivingAccount, "Cai em qual conta?"),
    (None, Field::RecurrenceKindChoice, "É um gasto ou uma entrada?"),
    (None, Field::RecurrenceName, "Nome? (ex.: Aluguel, Salário, Netflix)"),
    (None, Field::RecurrenceDay, "Que dia do mês? (1 a 31)"),
    (
        None,
        Field::RecurrenceModeChoice,
        "Registro automático ou pergunto antes? (Pergunte para contas que variam, como luz.)",
    ),
];

fn field_question(form: FormKind, field: Field) -> &'static str {
    let row = |owner: Option<FormKind>| {
        QUESTIONS.iter().find(|(known_form, known, _)| *known == field && *known_form == owner)
    };
    row(Some(form)).or_else(|| row(None)).map_or("?", |(_, _, question)| *question)
}

fn blocked_text(awaiting: Awaiting) -> &'static str {
    match awaiting {
        Awaiting::Field(Field::ToAccount) => {
            "Para isso você precisa de pelo menos duas contas. Crie outra com /novaconta."
        }
        Awaiting::Field(Field::Goal) => "Nenhuma meta ainda. Crie uma com /novameta.",
        Awaiting::Field(Field::ExpenseCategory | Field::IncomeCategory) => {
            "Nenhuma categoria ativa desse tipo."
        }
        Awaiting::Field(Field::CardChoice) => "Nenhum cartão ainda. Cadastre um com /novocartao.",
        Awaiting::Field(Field::InvoiceChoice) => "Esse cartão não tem fatura com valor a pagar. 🎉",
        _ => "Você ainda não tem contas. Crie uma com /novaconta.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flows::Answers;
    use domain::Cents;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 3, 10).unwrap()
    }

    fn context(catalog: &Catalog) -> CardContext<'_> {
        CardContext { owner: "Ana", catalog, nonce: "n0nce", today: today() }
    }

    fn with_amount(form: FormKind) -> FormState {
        let mut answers = Answers::default();
        answers.set(Field::Amount, Answer::Money(Cents::new(1050)));
        FormState { form, answers, awaiting: Awaiting::Field(Field::Description) }
    }

    #[test]
    fn card_shows_title_owner_answers_question_and_problem() {
        let catalog = Catalog::default();
        let CardView::Ask { html, keyboard } =
            card_view(&with_amount(FormKind::Expense), context(&catalog), Some("<oops>"))
        else {
            panic!("expected a question");
        };
        assert!(html.starts_with("⚠️ &lt;oops&gt;"), "{html}");
        assert!(html.contains("<b>Novo gasto</b> · Ana\n💰 Valor: R$ 10,50"), "{html}");
        assert!(html.ends_with("Descrição? (ex.: mercado, farmácia)"), "{html}");
        assert_eq!(keyboard.rows[0][0].label, "Pular");
    }

    #[test]
    fn blocked_when_no_accounts() {
        let state = FormState {
            awaiting: Awaiting::Field(Field::PaymentAccount),
            ..with_amount(FormKind::Expense)
        };
        let view = card_view(&state, context(&Catalog::default()), None);
        assert!(matches!(view, CardView::Blocked { html } if html.contains("/novaconta")));
    }

    #[test]
    fn committed_card_and_relative_dates() {
        let catalog = Catalog::default();
        let card =
            committed_card(&with_amount(FormKind::Income), context(&catalog), "Entrada registrada");
        assert_eq!(card, "✅ <b>Entrada registrada</b> · Ana\n💰 Valor: R$ 10,50");
        assert_eq!(relative_date(today(), today()), "hoje");
        assert_eq!(relative_date(NaiveDate::from_ymd_opt(2026, 3, 9).unwrap(), today()), "ontem");
        assert_eq!(relative_date(NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(), today()), "01/03");
    }

    #[test]
    fn every_question_has_text() {
        for form in FormKind::ALL {
            for field in form.fields() {
                assert_ne!(
                    question(form, Awaiting::Field(*field)),
                    "?",
                    "{form:?} {field:?} has no question"
                );
                assert_ne!(field_text(*field), ("•", "Campo"), "{field:?} has no icon and label");
            }
        }
        assert_eq!(question(FormKind::Expense, Awaiting::TypedDate), "Digite a data (dd/mm):");
    }
}
