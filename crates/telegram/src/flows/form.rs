use serde::{Deserialize, Serialize};

/// The guided commands. Each is a fixed sequence of [`Field`]s.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormKind {
    Expense,
    Income,
    Transfer,
    PotDeposit,
    PotWithdraw,
    NewAccount,
    NewGoal,
    NewCard,
    PayInvoice,
    Refund,
    NewRecurrence,
    SetBudget,
    /// Started from `/ultimos`, never by typing a command.
    EditEntry,
    /// Makes an account's balance match the bank.
    Adjust,
    Settings,
    NewCategory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Field {
    Amount,
    Description,
    ExpenseCategory,
    IncomeCategory,
    /// Where an expense is paid from.
    PaymentAccount,
    /// Where income lands.
    ReceivingAccount,
    FromAccount,
    ToAccount,
    Goal,
    Date,
    AccountName,
    AccountKind,
    /// Zero allowed.
    InitialBalance,
    GoalName,
    GoalTarget,
    /// Zero allowed.
    AlreadySaved,
    /// Only asked when the expense is paid with a card.
    Installments,
    CardName,
    ClosingDay,
    DueDay,
    /// Which card's invoice to pay.
    CardChoice,
    InvoiceChoice,
    /// Where a refund goes back to: an account or a card.
    RefundTarget,
    RecurrenceKindChoice,
    RecurrenceName,
    RecurrenceDay,
    RecurrenceModeChoice,
    /// Zero (or the remove button) removes the budget.
    BudgetLimit,
    /// Optional; skipped means no deadline.
    GoalDeadline,
    /// Filled in when the edit starts.
    EditTarget,
    EditFieldChoice,
    /// The balance the bank shows; may be zero or negative.
    ActualBalance,
    /// Skippable, like the report times: skipped keeps the current value.
    CycleStartDay,
    /// Any hour.
    YesterdayReportTime,
    /// 19:00 or later.
    TodayReportTime,
    CategoryName,
    CategoryKindChoice,
    /// Only asked for expense categories.
    EssentialChoice,
    /// Total installments of a financing; skipped means it never ends.
    RecurrenceInstallments,
    /// Installments of that financing already paid; only asked with a total.
    RecurrencePaid,
    /// Optional; skipped means no emoji.
    CategoryEmoji,
}

use Field::{
    AccountKind as KindField, AccountName, AlreadySaved, Amount, BudgetLimit, CardChoice, CardName,
    ClosingDay, Date, Description, DueDay, EditFieldChoice, EditTarget, ExpenseCategory,
    FromAccount, Goal, GoalDeadline, GoalName, GoalTarget, IncomeCategory, InitialBalance,
    Installments, InvoiceChoice, PaymentAccount, ReceivingAccount, RecurrenceDay,
    RecurrenceKindChoice, RecurrenceModeChoice, RecurrenceName, RefundTarget, ToAccount,
};

/// Only the field picked in `EditFieldChoice` applies.
const EDIT_FIELDS: &[Field] =
    &[EditTarget, EditFieldChoice, Amount, Description, ExpenseCategory, IncomeCategory, Date];

const CATEGORY_FIELDS: &[Field] =
    &[Field::CategoryName, Field::CategoryKindChoice, Field::EssentialChoice, Field::CategoryEmoji];

/// Each one can be kept as it is with the [Manter] button.
const SETTINGS_FIELDS: &[Field] =
    &[Field::CycleStartDay, Field::YesterdayReportTime, Field::TodayReportTime];

/// Expense-only and income-only fields are skipped by `Field::applies`.
const RECURRENCE_FIELDS: &[Field] = &[
    RecurrenceKindChoice,
    RecurrenceName,
    Amount,
    ExpenseCategory,
    IncomeCategory,
    PaymentAccount,
    ReceivingAccount,
    RecurrenceDay,
    Field::RecurrenceInstallments,
    Field::RecurrencePaid,
    RecurrenceModeChoice,
];

impl FormKind {
    pub const ALL: [FormKind; 16] = [
        FormKind::Expense,
        FormKind::Income,
        FormKind::Transfer,
        FormKind::PotDeposit,
        FormKind::PotWithdraw,
        FormKind::NewAccount,
        FormKind::NewGoal,
        FormKind::NewCard,
        FormKind::PayInvoice,
        FormKind::Refund,
        FormKind::NewRecurrence,
        FormKind::SetBudget,
        FormKind::EditEntry,
        FormKind::Adjust,
        FormKind::Settings,
        FormKind::NewCategory,
    ];

    pub const fn fields(self) -> &'static [Field] {
        match self {
            FormKind::Expense => {
                &[Amount, Description, ExpenseCategory, PaymentAccount, Installments, Date]
            }
            FormKind::Income => &[Amount, Description, IncomeCategory, ReceivingAccount, Date],
            FormKind::Transfer => &[Amount, FromAccount, ToAccount, Description, Date],
            FormKind::PotDeposit => &[Goal, Amount, FromAccount],
            FormKind::PotWithdraw => &[Goal, Amount, ToAccount],
            FormKind::NewAccount => &[AccountName, KindField, InitialBalance],
            FormKind::NewGoal => &[GoalName, GoalTarget, AlreadySaved, GoalDeadline],
            FormKind::NewCard => &[CardName, ClosingDay, DueDay],
            FormKind::PayInvoice => &[CardChoice, InvoiceChoice, Amount, FromAccount, Date],
            FormKind::Refund => &[Amount, Description, ExpenseCategory, RefundTarget, Date],
            FormKind::NewRecurrence => RECURRENCE_FIELDS,
            FormKind::SetBudget => &[ExpenseCategory, BudgetLimit],
            FormKind::EditEntry => EDIT_FIELDS,
            FormKind::Adjust => &[ReceivingAccount, Field::ActualBalance],
            FormKind::Settings => SETTINGS_FIELDS,
            FormKind::NewCategory => CATEGORY_FIELDS,
        }
    }

    /// The slash command (without `/`) that starts this form.
    pub const fn command(self) -> &'static str {
        match self {
            FormKind::Expense => "gasto",
            FormKind::Income => "entrada",
            FormKind::Transfer => "transferir",
            FormKind::PotDeposit => "guardar",
            FormKind::PotWithdraw => "resgatar",
            FormKind::NewAccount => "novaconta",
            FormKind::NewGoal => "novameta",
            FormKind::NewCard => "novocartao",
            FormKind::PayInvoice => "pagarfatura",
            FormKind::Refund => "estorno",
            FormKind::NewRecurrence => "recorrente",
            FormKind::SetBudget => "orcamento",
            FormKind::EditEntry => "editar",
            FormKind::Adjust => "ajuste",
            FormKind::Settings => "config",
            FormKind::NewCategory => "novacategoria",
        }
    }

    pub const fn title(self) -> &'static str {
        match self {
            FormKind::Expense => "Novo gasto",
            FormKind::Income => "Nova entrada",
            FormKind::Transfer => "Transferência",
            FormKind::PotDeposit => "Guardar na meta",
            FormKind::PotWithdraw => "Resgatar da meta",
            FormKind::NewAccount => "Nova conta",
            FormKind::NewGoal => "Nova meta",
            FormKind::NewCard => "Novo cartão",
            FormKind::PayInvoice => "Pagar fatura",
            FormKind::Refund => "Estorno",
            FormKind::NewRecurrence => "Nova recorrência",
            FormKind::SetBudget => "Orçamento mensal",
            FormKind::EditEntry => "Editar lançamento",
            FormKind::Adjust => "Ajustar saldo",
            FormKind::Settings => "Configurações",
            FormKind::NewCategory => "Nova categoria",
        }
    }

    /// The form a typed command starts; the edit form needs an entry
    /// and only starts from `/ultimos`.
    pub fn from_command(command: &str) -> Option<FormKind> {
        let typed = FormKind::ALL.into_iter().filter(|form| *form != FormKind::EditEntry);
        typed.into_iter().find(|form| form.command() == command)
    }
}

impl Field {
    /// Whether this field is asked given the answers so far. Installments
    /// only make sense for an expense paid by card.
    pub fn applies(self, form: FormKind, answers: &super::Answers) -> bool {
        use app::model::RecurrenceKind::{Expense, Income};
        let recurrence_kind = answers.recurrence_kind();
        match (form, self) {
            (FormKind::EditEntry, field) => edit_applies(field, answers),
            (_, Field::RecurrencePaid) => answers.count(Field::RecurrenceInstallments).is_some(),
            (_, Field::EssentialChoice) => {
                answers.category_kind() == Some(app::model::CategoryKind::Expense)
            }
            (_, Field::Installments) => answers.card(Field::PaymentAccount).is_some(),
            (FormKind::NewRecurrence, Field::ExpenseCategory | Field::PaymentAccount) => {
                recurrence_kind == Some(Expense)
            }
            (FormKind::NewRecurrence, Field::IncomeCategory | Field::ReceivingAccount) => {
                recurrence_kind == Some(Income)
            }
            _ => true,
        }
    }
}

/// In an edit, only the picked field is asked; income picks income
/// categories.
fn edit_applies(field: Field, answers: &super::Answers) -> bool {
    use super::EditChoice;
    let income = answers.edited_entry().is_some_and(|(_, income)| income);
    match (field, answers.edit_choice()) {
        (Field::EditTarget | Field::EditFieldChoice, _)
        | (Field::Amount, Some(EditChoice::Amount))
        | (Field::Description, Some(EditChoice::Description))
        | (Field::Date, Some(EditChoice::Date)) => true,
        (Field::ExpenseCategory, Some(EditChoice::Category)) => !income,
        (Field::IncomeCategory, Some(EditChoice::Category)) => income,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_round_trip() {
        for form in FormKind::ALL.into_iter().filter(|form| *form != FormKind::EditEntry) {
            assert_eq!(FormKind::from_command(form.command()), Some(form));
            assert!(!form.fields().is_empty() && !form.title().is_empty());
        }
        assert_eq!(FormKind::from_command("saldo"), None);
        assert_eq!(FormKind::from_command("editar"), None);
    }

    #[test]
    fn expense_asks_amount_first_and_date_last() {
        let fields = FormKind::Expense.fields();
        assert_eq!((fields.first(), fields.last()), (Some(&Field::Amount), Some(&Field::Date)));
    }
}
