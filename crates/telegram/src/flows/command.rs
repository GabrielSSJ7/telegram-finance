//! Turns a confirmed form into the use-case request it stands for.

use app::model::{CategoryId, RecurrenceKind, RecurrenceTarget};
use app::services::ledger::{AccountEntry, EntryRequest, TransferEntry};
use app::services::{
    CardCreditRequest, CardPurchaseRequest, CreateGoal, CreateRecurrence, InvoicePaymentRequest,
    OpenAccount, OpenCard, PotMove,
};
use domain::Cents;
use domain::DayOfMonth;

use super::{Answers, Field, FormKind, FormState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormCommand {
    Record(EntryRequest),
    PotDeposit(PotMove),
    PotWithdraw(PotMove),
    OpenAccount(OpenAccount),
    CreateGoal(CreateGoal),
    CardPurchase(CardPurchaseRequest),
    CardCredit(CardCreditRequest),
    PayInvoice(InvoicePaymentRequest),
    OpenCard(OpenCard),
    CreateRecurrence(CreateRecurrence),
    /// A zero limit removes the budget.
    SetBudget {
        category: CategoryId,
        limit: Cents,
    },
}

/// `None` when a required answer is missing (the engine never confirms
/// such a form, so this signals a corrupted stored flow).
pub fn build_command(state: &FormState) -> Option<FormCommand> {
    let answers = &state.answers;
    match state.form {
        FormKind::Expense => expense(answers),
        FormKind::Refund => refund(answers),
        FormKind::Income => account_entry(answers, Field::IncomeCategory, Field::ReceivingAccount)
            .map(|entry| FormCommand::Record(EntryRequest::Income(entry))),
        FormKind::Transfer => {
            transfer(answers).map(|entry| FormCommand::Record(EntryRequest::Transfer(entry)))
        }
        FormKind::PotDeposit => pot_move(answers, Field::FromAccount).map(FormCommand::PotDeposit),
        FormKind::PotWithdraw => pot_move(answers, Field::ToAccount).map(FormCommand::PotWithdraw),
        FormKind::PayInvoice => pay_invoice(answers).map(FormCommand::PayInvoice),
        setup => setup_command(setup, answers),
    }
}

/// Forms that configure things rather than move money.
fn setup_command(form: FormKind, answers: &Answers) -> Option<FormCommand> {
    match form {
        FormKind::NewAccount => open_account(answers).map(FormCommand::OpenAccount),
        FormKind::NewGoal => create_goal(answers).map(FormCommand::CreateGoal),
        FormKind::NewCard => open_card(answers).map(FormCommand::OpenCard),
        FormKind::NewRecurrence => create_recurrence(answers).map(FormCommand::CreateRecurrence),
        FormKind::SetBudget => {
            let (category, limit) =
                (answers.category(Field::ExpenseCategory)?, answers.money(Field::BudgetLimit)?);
            Some(FormCommand::SetBudget { category, limit })
        }
        _ => None,
    }
}

/// Card payments become card purchases; the rest are account expenses.
fn expense(answers: &Answers) -> Option<FormCommand> {
    let Some(card_id) = answers.card(Field::PaymentAccount) else {
        let entry = account_entry(answers, Field::ExpenseCategory, Field::PaymentAccount)?;
        return Some(FormCommand::Record(EntryRequest::Expense(entry)));
    };
    Some(FormCommand::CardPurchase(CardPurchaseRequest {
        card_id,
        category_id: answers.category(Field::ExpenseCategory)?,
        total: answers.money(Field::Amount)?,
        installments: answers.installments(),
        first_installment_no: 1,
        description: answers.text(Field::Description)?,
        purchased_on: Some(answers.date()?),
    }))
}

/// A refund goes back to a card invoice or to an account.
fn refund(answers: &Answers) -> Option<FormCommand> {
    let Some(card_id) = answers.card(Field::RefundTarget) else {
        let entry = account_entry(answers, Field::ExpenseCategory, Field::RefundTarget)?;
        return Some(FormCommand::Record(EntryRequest::Refund(entry)));
    };
    Some(FormCommand::CardCredit(CardCreditRequest {
        card_id,
        category_id: answers.category(Field::ExpenseCategory)?,
        amount: answers.money(Field::Amount)?,
        description: answers.text(Field::Description)?,
        date: Some(answers.date()?),
    }))
}

fn create_recurrence(answers: &Answers) -> Option<CreateRecurrence> {
    let kind = answers.recurrence_kind()?;
    let (category, target) = match kind {
        RecurrenceKind::Income => (
            Field::IncomeCategory,
            RecurrenceTarget::Account(answers.account(Field::ReceivingAccount)?),
        ),
        RecurrenceKind::Expense => (Field::ExpenseCategory, recurrence_target(answers)?),
    };
    Some(CreateRecurrence {
        kind,
        amount: answers.money(Field::Amount)?,
        description: answers.text(Field::RecurrenceName)?,
        category_id: answers.category(category)?,
        target,
        day: DayOfMonth::new(answers.day(Field::RecurrenceDay)?).ok()?,
        mode: answers.recurrence_mode()?,
        starts_on: None,
    })
}

fn recurrence_target(answers: &Answers) -> Option<RecurrenceTarget> {
    let card = answers.card(Field::PaymentAccount).map(RecurrenceTarget::Card);
    card.or_else(|| answers.account(Field::PaymentAccount).map(RecurrenceTarget::Account))
}

fn open_card(answers: &Answers) -> Option<OpenCard> {
    Some(OpenCard {
        name: answers.text(Field::CardName)?,
        closing_day: DayOfMonth::new(answers.day(Field::ClosingDay)?).ok()?,
        due_day: DayOfMonth::new(answers.day(Field::DueDay)?).ok()?,
        closing_day_goes_next: true,
        limit: None,
        default_payment_account_id: None,
    })
}

fn pay_invoice(answers: &Answers) -> Option<InvoicePaymentRequest> {
    Some(InvoicePaymentRequest {
        invoice_id: answers.invoice()?,
        account_id: answers.account(Field::FromAccount)?,
        amount: answers.money(Field::Amount)?,
        date: Some(answers.date()?),
    })
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
        target_date: answers.deadline().flatten(),
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
