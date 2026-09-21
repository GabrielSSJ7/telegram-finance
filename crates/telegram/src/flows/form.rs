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
}

use Field::{
    AccountKind as KindField, AccountName, AlreadySaved, Amount, CardChoice, CardName, ClosingDay,
    Date, Description, DueDay, ExpenseCategory, FromAccount, Goal, GoalName, GoalTarget,
    IncomeCategory, InitialBalance, Installments, InvoiceChoice, PaymentAccount, ReceivingAccount,
    RecurrenceDay, RecurrenceKindChoice, RecurrenceModeChoice, RecurrenceName, RefundTarget,
    ToAccount,
};

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
    RecurrenceModeChoice,
];

impl FormKind {
    pub const ALL: [FormKind; 11] = [
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
            FormKind::NewGoal => &[GoalName, GoalTarget, AlreadySaved],
            FormKind::NewCard => &[CardName, ClosingDay, DueDay],
            FormKind::PayInvoice => &[CardChoice, InvoiceChoice, Amount, FromAccount, Date],
            FormKind::Refund => &[Amount, Description, ExpenseCategory, RefundTarget, Date],
            FormKind::NewRecurrence => RECURRENCE_FIELDS,
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
        }
    }

    pub fn from_command(command: &str) -> Option<FormKind> {
        FormKind::ALL.into_iter().find(|form| form.command() == command)
    }
}

impl Field {
    /// Whether this field is asked given the answers so far. Installments
    /// only make sense for an expense paid by card.
    pub fn applies(self, form: FormKind, answers: &super::Answers) -> bool {
        use app::model::RecurrenceKind::{Expense, Income};
        let recurrence_kind = answers.recurrence_kind();
        match (form, self) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_round_trip() {
        for form in FormKind::ALL {
            assert_eq!(FormKind::from_command(form.command()), Some(form));
            assert!(!form.fields().is_empty() && !form.title().is_empty());
        }
        assert_eq!(FormKind::from_command("saldo"), None);
    }

    #[test]
    fn expense_asks_amount_first_and_date_last() {
        let fields = FormKind::Expense.fields();
        assert_eq!((fields.first(), fields.last()), (Some(&Field::Amount), Some(&Field::Date)));
    }
}
