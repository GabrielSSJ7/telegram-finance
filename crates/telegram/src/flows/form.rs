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
}

use Field::{
    AccountKind as KindField, AccountName, AlreadySaved, Amount, Date, Description,
    ExpenseCategory, FromAccount, Goal, GoalName, GoalTarget, IncomeCategory, InitialBalance,
    PaymentAccount, ReceivingAccount, ToAccount,
};

impl FormKind {
    pub const ALL: [FormKind; 7] = [
        FormKind::Expense,
        FormKind::Income,
        FormKind::Transfer,
        FormKind::PotDeposit,
        FormKind::PotWithdraw,
        FormKind::NewAccount,
        FormKind::NewGoal,
    ];

    pub const fn fields(self) -> &'static [Field] {
        match self {
            FormKind::Expense => &[Amount, Description, ExpenseCategory, PaymentAccount, Date],
            FormKind::Income => &[Amount, Description, IncomeCategory, ReceivingAccount, Date],
            FormKind::Transfer => &[Amount, FromAccount, ToAccount, Description, Date],
            FormKind::PotDeposit => &[Goal, Amount, FromAccount],
            FormKind::PotWithdraw => &[Goal, Amount, ToAccount],
            FormKind::NewAccount => &[AccountName, KindField, InitialBalance],
            FormKind::NewGoal => &[GoalName, GoalTarget, AlreadySaved],
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
        }
    }

    pub fn from_command(command: &str) -> Option<FormKind> {
        FormKind::ALL.into_iter().find(|form| form.command() == command)
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
