//! Turns a confirmed form into the use-case request it stands for.

use app::services::ledger::{AccountEntry, EntryRequest, TransferEntry};
use app::services::{CreateGoal, OpenAccount, PotMove};

use super::{Answers, Field, FormKind, FormState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormCommand {
    Record(EntryRequest),
    PotDeposit(PotMove),
    PotWithdraw(PotMove),
    OpenAccount(OpenAccount),
    CreateGoal(CreateGoal),
}

/// `None` when a required answer is missing (the engine never confirms
/// such a form, so this signals a corrupted stored flow).
pub fn build_command(state: &FormState) -> Option<FormCommand> {
    let answers = &state.answers;
    match state.form {
        FormKind::Expense => account_entry(answers, Field::ExpenseCategory, Field::PaymentAccount)
            .map(|entry| FormCommand::Record(EntryRequest::Expense(entry))),
        FormKind::Income => account_entry(answers, Field::IncomeCategory, Field::ReceivingAccount)
            .map(|entry| FormCommand::Record(EntryRequest::Income(entry))),
        FormKind::Transfer => {
            transfer(answers).map(|entry| FormCommand::Record(EntryRequest::Transfer(entry)))
        }
        FormKind::PotDeposit => pot_move(answers, Field::FromAccount).map(FormCommand::PotDeposit),
        FormKind::PotWithdraw => pot_move(answers, Field::ToAccount).map(FormCommand::PotWithdraw),
        FormKind::NewAccount => open_account(answers).map(FormCommand::OpenAccount),
        FormKind::NewGoal => create_goal(answers).map(FormCommand::CreateGoal),
    }
}

fn account_entry(answers: &Answers, category: Field, account: Field) -> Option<AccountEntry> {
    Some(AccountEntry {
        account_id: answers.account(account)?,
        category_id: answers.category(category)?,
        amount: answers.money(Field::Amount)?,
        description: answers.text(Field::Description)?,
        date: Some(answers.date()?),
    })
}

fn transfer(answers: &Answers) -> Option<TransferEntry> {
    Some(TransferEntry {
        from_account_id: answers.account(Field::FromAccount)?,
        to_account_id: answers.account(Field::ToAccount)?,
        amount: answers.money(Field::Amount)?,
        description: answers.text(Field::Description)?,
        date: Some(answers.date()?),
    })
}

fn pot_move(answers: &Answers, account: Field) -> Option<PotMove> {
    Some(PotMove {
        goal_id: answers.goal()?,
        account_id: answers.account(account)?,
        amount: answers.money(Field::Amount)?,
        description: String::new(),
    })
}

fn open_account(answers: &Answers) -> Option<OpenAccount> {
    Some(OpenAccount {
        name: answers.text(Field::AccountName)?,
        kind: answers.account_kind()?,
        initial_balance: answers.money(Field::InitialBalance)?,
        opened_on: None,
    })
}

fn create_goal(answers: &Answers) -> Option<CreateGoal> {
    Some(CreateGoal {
        name: answers.text(Field::GoalName)?,
        target: answers.money(Field::GoalTarget)?,
        target_date: None,
        already_saved: answers.money(Field::AlreadySaved)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flows::{Answer, Awaiting};
    use app::model::{AccountId, CategoryId, GoalId};
    use chrono::NaiveDate;
    use domain::{AccountKind, Cents};

    fn confirmed(form: FormKind, answers: Vec<(Field, Answer)>) -> FormState {
        FormState { form, answers: Answers(answers), awaiting: Awaiting::Confirmation }
    }

    fn money(cents: i64) -> Answer {
        Answer::Money(Cents::new(cents))
    }

    fn expense_answers(
        account: AccountId,
        category: CategoryId,
        date: NaiveDate,
    ) -> Vec<(Field, Answer)> {
        vec![
            (Field::Amount, money(1050)),
            (Field::Description, Answer::Skipped),
            (Field::ExpenseCategory, Answer::Category(category)),
            (Field::PaymentAccount, Answer::Account(account)),
            (Field::Date, Answer::Date(date)),
        ]
    }

    #[test]
    fn expense_builds_record_request() {
        let (account, category) = (AccountId::generate(), CategoryId::generate());
        let date = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
        let state = confirmed(FormKind::Expense, expense_answers(account, category, date));
        let Some(FormCommand::Record(EntryRequest::Expense(entry))) = build_command(&state) else {
            panic!("expected an expense request");
        };
        assert_eq!(
            (entry.account_id, entry.category_id, entry.amount),
            (account, category, Cents::new(1050))
        );
        assert_eq!((entry.description.as_str(), entry.date), ("", Some(date)));
    }

    #[test]
    fn pot_deposit_builds_move_into_goal() {
        let (goal, account) = (GoalId::generate(), AccountId::generate());
        let answers = vec![
            (Field::Goal, Answer::Goal(goal)),
            (Field::Amount, money(5)),
            (Field::FromAccount, Answer::Account(account)),
        ];
        let Some(FormCommand::PotDeposit(request)) =
            build_command(&confirmed(FormKind::PotDeposit, answers))
        else {
            panic!("expected a pot deposit");
        };
        assert_eq!(
            (request.goal_id, request.account_id, request.amount),
            (goal, account, Cents::new(5))
        );
    }

    #[test]
    fn setup_forms_build_their_requests() {
        let account_answers = vec![
            (Field::AccountName, Answer::Text("Nubank".into())),
            (Field::AccountKind, Answer::AccountKind(AccountKind::Checking)),
            (Field::InitialBalance, money(0)),
        ];
        assert!(matches!(
            build_command(&confirmed(FormKind::NewAccount, account_answers)),
            Some(FormCommand::OpenAccount(_))
        ));
        let goal_answers = vec![
            (Field::GoalName, Answer::Text("Casa".into())),
            (Field::GoalTarget, money(100)),
            (Field::AlreadySaved, money(0)),
        ];
        assert!(matches!(
            build_command(&confirmed(FormKind::NewGoal, goal_answers)),
            Some(FormCommand::CreateGoal(_))
        ));
    }

    #[test]
    fn missing_answers_yield_none() {
        for form in FormKind::ALL {
            assert_eq!(build_command(&confirmed(form, vec![])), None, "{form:?}");
        }
    }
}
