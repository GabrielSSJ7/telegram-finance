//! Turns a ledger request into the uniform shape the store persists and
//! checks the rules that need no lookups.

use chrono::NaiveDate;
use domain::{Cents, EntryKind};

use crate::model::{AccountId, CategoryId, MemberId, NewEntry};
use crate::services::text_rules::clean_description;
use crate::{AppError, AppResult};

/// Money in or out of one account, with a category.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountEntry {
    pub account_id: AccountId,
    pub category_id: CategoryId,
    pub amount: Cents,
    pub description: String,
    /// `None` means today in the household timezone.
    pub date: Option<NaiveDate>,
}

/// Money moved between two accounts (including pots).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferEntry {
    pub from_account_id: AccountId,
    pub to_account_id: AccountId,
    pub amount: Cents,
    pub description: String,
    pub date: Option<NaiveDate>,
}

/// Correction of an account balance without a category.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdjustmentEntry {
    pub account_id: AccountId,
    pub amount: Cents,
    pub description: String,
    pub date: Option<NaiveDate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryRequest {
    Income(AccountEntry),
    Expense(AccountEntry),
    Refund(AccountEntry),
    Transfer(TransferEntry),
    AdjustIn(AdjustmentEntry),
    AdjustOut(AdjustmentEntry),
}

/// Every request flattened to the columns of a ledger row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryShape {
    pub kind: EntryKind,
    pub amount: Cents,
    pub description: String,
    pub category_id: Option<CategoryId>,
    pub account_id: AccountId,
    pub counter_account_id: Option<AccountId>,
    pub date: Option<NaiveDate>,
}

impl EntryRequest {
    pub fn into_shape(self) -> EntryShape {
        match self {
            EntryRequest::Income(entry) => account_shape(EntryKind::Income, entry),
            EntryRequest::Expense(entry) => account_shape(EntryKind::Expense, entry),
            EntryRequest::Refund(entry) => account_shape(EntryKind::Refund, entry),
            EntryRequest::Transfer(entry) => transfer_shape(entry),
            EntryRequest::AdjustIn(entry) => adjustment_shape(EntryKind::AdjustIn, entry),
            EntryRequest::AdjustOut(entry) => adjustment_shape(EntryKind::AdjustOut, entry),
        }
    }
}

fn account_shape(kind: EntryKind, entry: AccountEntry) -> EntryShape {
    EntryShape {
        kind,
        amount: entry.amount,
        description: entry.description,
        category_id: Some(entry.category_id),
        account_id: entry.account_id,
        counter_account_id: None,
        date: entry.date,
    }
}

fn transfer_shape(entry: TransferEntry) -> EntryShape {
    EntryShape {
        kind: EntryKind::Transfer,
        amount: entry.amount,
        description: entry.description,
        category_id: None,
        account_id: entry.from_account_id,
        counter_account_id: Some(entry.to_account_id),
        date: entry.date,
    }
}

fn adjustment_shape(kind: EntryKind, entry: AdjustmentEntry) -> EntryShape {
    EntryShape {
        kind,
        amount: entry.amount,
        description: entry.description,
        category_id: None,
        account_id: entry.account_id,
        counter_account_id: None,
        date: entry.date,
    }
}

pub fn ensure_positive_amount(amount: Cents) -> AppResult<()> {
    if amount.is_positive() {
        return Ok(());
    }
    Err(AppError::invalid("amount", amount.value(), "a positive number of cents"))
}

pub fn ensure_distinct_accounts(shape: &EntryShape) -> AppResult<()> {
    if shape.counter_account_id != Some(shape.account_id) {
        return Ok(());
    }
    Err(AppError::invalid(
        "destination account",
        shape.account_id,
        "a different account than the source",
    ))
}

/// Final row once lookups passed; `today` fills a missing date.
pub fn shape_to_new_entry(
    shape: &EntryShape,
    today: NaiveDate,
    created_by: Option<MemberId>,
) -> AppResult<NewEntry> {
    Ok(NewEntry {
        kind: shape.kind,
        amount: shape.amount,
        description: clean_description(&shape.description)?,
        category_id: shape.category_id,
        account_id: Some(shape.account_id),
        counter_account_id: shape.counter_account_id,
        accounting_date: shape.date.unwrap_or(today),
        created_by,
    })
}

/// Kinds this service may edit; card rows are edited through their purchase.
pub fn ensure_editable_kind(kind: EntryKind) -> AppResult<()> {
    let editable = !matches!(
        kind,
        EntryKind::CardInstallment | EntryKind::CardCredit | EntryKind::InvoicePayment
    );
    if editable {
        return Ok(());
    }
    Err(AppError::invalid(
        "entry kind",
        kind,
        "an account entry (card rows change through their purchase)",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(value: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 3, value).unwrap()
    }

    #[test]
    fn transfer_shape_uses_counter_account() {
        let (from, to) = (AccountId::generate(), AccountId::generate());
        let request = EntryRequest::Transfer(TransferEntry {
            from_account_id: from,
            to_account_id: to,
            amount: Cents::new(500),
            description: "caixinha".into(),
            date: None,
        });
        let shape = request.into_shape();
        assert_eq!(
            (shape.kind, shape.account_id, shape.counter_account_id),
            (EntryKind::Transfer, from, Some(to))
        );
        assert!(ensure_distinct_accounts(&shape).is_ok());
        let same = EntryShape { counter_account_id: Some(from), ..shape };
        assert!(ensure_distinct_accounts(&same).is_err());
    }

    #[test]
    fn new_entry_defaults_date_and_cleans_description() {
        let request = EntryRequest::AdjustOut(AdjustmentEntry {
            account_id: AccountId::generate(),
            amount: Cents::new(1),
            description: "  tarifa ".into(),
            date: None,
        });
        let entry = shape_to_new_entry(&request.into_shape(), day(10), None).unwrap();
        assert_eq!((entry.kind, entry.accounting_date), (EntryKind::AdjustOut, day(10)));
        assert_eq!(entry.description, "tarifa");
        assert_eq!(entry.category_id, None);
    }

    #[test]
    fn amount_must_be_positive() {
        assert!(ensure_positive_amount(Cents::new(1)).is_ok());
        let error = ensure_positive_amount(Cents::new(-50)).unwrap_err().to_string();
        assert!(error.contains("-50") && error.contains("positive"), "{error}");
    }

    #[test]
    fn card_kinds_are_not_editable_here() {
        assert!(ensure_editable_kind(EntryKind::Expense).is_ok());
        assert!(ensure_editable_kind(EntryKind::CardInstallment).is_err());
        assert!(ensure_editable_kind(EntryKind::InvoicePayment).is_err());
    }
}
