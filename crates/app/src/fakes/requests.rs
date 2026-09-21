//! Short constructors for requests that tests build over and over.

use chrono::NaiveDate;
use domain::{AccountKind, Cents, DayOfMonth};

use crate::model::{
    AccountId, CardId, CategoryId, RecurrenceKind, RecurrenceMode, RecurrenceTarget,
};
use crate::services::{CardPurchaseRequest, CreateGoal, CreateRecurrence, OpenAccount, OpenCard};

pub fn open_checking(name: &str, initial_cents: i64) -> OpenAccount {
    OpenAccount {
        name: name.into(),
        kind: AccountKind::Checking,
        initial_balance: Cents::new(initial_cents),
        opened_on: None,
    }
}

/// A card that closes on `closing_day` and is due on `due_day`.
pub fn open_card(name: &str, closing_day: u8, due_day: u8) -> OpenCard {
    OpenCard {
        name: name.into(),
        closing_day: day(closing_day),
        due_day: day(due_day),
        closing_day_goes_next: true,
        limit: None,
        default_payment_account_id: None,
    }
}

/// Bought today, split into `installments`.
pub fn card_purchase(
    card_id: CardId,
    category_id: CategoryId,
    total_cents: i64,
    installments: u32,
) -> CardPurchaseRequest {
    CardPurchaseRequest {
        card_id,
        category_id,
        total: Cents::new(total_cents),
        installments,
        first_installment_no: 1,
        description: String::new(),
        purchased_on: None,
    }
}

pub fn goal(name: &str, target_cents: i64, already_saved_cents: i64) -> CreateGoal {
    let (target, already_saved) = (Cents::new(target_cents), Cents::new(already_saved_cents));
    CreateGoal { name: name.into(), target, target_date: None, already_saved }
}

/// A monthly expense paid from `account` on `day_of_month`.
pub fn monthly_expense(
    name: &str,
    cents: i64,
    category_id: CategoryId,
    account: AccountId,
    day_of_month: u8,
) -> CreateRecurrence {
    CreateRecurrence {
        kind: RecurrenceKind::Expense,
        amount: Cents::new(cents),
        description: name.into(),
        category_id,
        target: RecurrenceTarget::Account(account),
        day: day(day_of_month),
        mode: RecurrenceMode::Auto,
        starts_on: None,
    }
}

/// Starting `starts_on`, recorded after confirmation.
pub fn confirming(request: CreateRecurrence, starts_on: NaiveDate) -> CreateRecurrence {
    CreateRecurrence { mode: RecurrenceMode::Confirm, starts_on: Some(starts_on), ..request }
}

fn day(value: u8) -> DayOfMonth {
    DayOfMonth::new(value).unwrap_or(DayOfMonth::FIRST)
}
