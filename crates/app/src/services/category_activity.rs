//! Totals of a period for `/extrato`, by category, by account or by card,
//! computed from the entries. Categories use the same signs as the
//! reports: refunds and card credits reduce spending, and only income
//! counts in income categories.

use domain::Cents;
use domain::entry_kind::AccountRole;

use crate::model::{Account, AccountId, Category, CategoryKind, LedgerEntry};

/// What one category moved in a period.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryActivity {
    pub category: Category,
    /// Net spending for expense categories, income for income categories.
    pub total: Cents,
    pub entries: usize,
}

/// Signed effect of `entry` on a category of `kind`: a refund is negative
/// in an expense category; a transfer counts nowhere.
///
/// ```
/// use app::model::CategoryKind;
/// use app::services::category_activity::category_effect;
/// use domain::EntryKind;
/// let refund = app::fakes::requests::bare_entry(EntryKind::Refund, 500, "", chrono::NaiveDate::MIN);
/// assert_eq!(category_effect(&refund, CategoryKind::Expense).value(), -500);
/// ```
pub fn category_effect(entry: &LedgerEntry, kind: CategoryKind) -> Cents {
    let sign = match kind {
        CategoryKind::Expense => entry.kind.spend_effect(),
        CategoryKind::Income => entry.kind.income_effect(),
    };
    entry.amount.times(sign)
}

/// Categories with entries in `entries`: expenses first, then income, each
/// largest total first. Entries without a category are left out.
pub fn category_activity(
    entries: &[LedgerEntry],
    categories: &[Category],
) -> Vec<CategoryActivity> {
    let mut activity: Vec<CategoryActivity> =
        categories.iter().filter_map(|category| activity_of(category, entries)).collect();
    activity.sort_by_key(|item| (item.category.kind != CategoryKind::Expense, -item.total.value()));
    activity
}

fn activity_of(category: &Category, entries: &[LedgerEntry]) -> Option<CategoryActivity> {
    let own: Vec<&LedgerEntry> =
        entries.iter().filter(|entry| entry.category_id == Some(category.id)).collect();
    if own.is_empty() {
        return None;
    }
    let total = own.iter().map(|entry| category_effect(entry, category.kind)).sum();
    Some(CategoryActivity { category: category.clone(), total, entries: own.len() })
}

/// What one account moved in a period: money in minus money out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountActivity {
    pub account: Account,
    pub total: Cents,
    pub entries: usize,
}

/// Signed effect of `entry` on `account`: a transfer leaves one and lands
/// in the other, so each side is counted with its own role.
pub fn account_effect(entry: &LedgerEntry, account: AccountId) -> Cents {
    let role = if entry.account_id == Some(account) {
        AccountRole::Primary
    } else if entry.counter_account_id == Some(account) {
        AccountRole::Counter
    } else {
        return Cents::ZERO;
    };
    entry.amount.times(entry.kind.account_effect(role))
}

/// Accounts that moved in `entries`, biggest movement first.
pub fn account_activity(entries: &[LedgerEntry], accounts: &[Account]) -> Vec<AccountActivity> {
    let mut activity: Vec<AccountActivity> = accounts
        .iter()
        .filter_map(|account| {
            let touched: Vec<&LedgerEntry> = entries
                .iter()
                .filter(|entry| account_effect(entry, account.id) != Cents::ZERO)
                .collect();
            (!touched.is_empty()).then(|| AccountActivity {
                account: account.clone(),
                total: touched.iter().map(|entry| account_effect(entry, account.id)).sum(),
                entries: touched.len(),
            })
        })
        .collect();
    activity.sort_by_key(|item| item.total.value().abs());
    activity.reverse();
    activity
}

/// What was charged to a card in a period: installments minus credits.
pub fn card_charges(entries: &[LedgerEntry]) -> Cents {
    entries.iter().map(|entry| entry.amount.times(entry.kind.spend_effect())).sum()
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use domain::EntryKind;

    use super::*;
    use crate::fakes::requests::bare_entry;
    use crate::model::CategoryId;

    fn category(name: &str, kind: CategoryKind) -> Category {
        Category {
            id: CategoryId::generate(),
            name: name.into(),
            kind,
            emoji: None,
            archived: false,
            essential: false,
        }
    }

    fn entry(kind: EntryKind, cents: i64, category: &Category) -> LedgerEntry {
        let date = NaiveDate::from_ymd_opt(2026, 9, 10).unwrap();
        LedgerEntry { category_id: Some(category.id), ..bare_entry(kind, cents, "", date) }
    }

    fn summary(activity: &[CategoryActivity]) -> Vec<(String, i64, usize)> {
        activity
            .iter()
            .map(|item| (item.category.name.clone(), item.total.value(), item.entries))
            .collect()
    }

    #[test]
    fn totals_net_refunds_and_order_expenses_before_income() {
        let (market, pets) =
            (category("mercado", CategoryKind::Expense), category("pets", CategoryKind::Expense));
        let (salary, unused) =
            (category("salário", CategoryKind::Income), category("extra", CategoryKind::Income));
        let entries = [
            entry(EntryKind::Expense, 10_000, &market),
            entry(EntryKind::CardInstallment, 5_000, &market),
            entry(EntryKind::Refund, 2_000, &market),
            entry(EntryKind::Expense, 20_000, &pets),
            entry(EntryKind::Income, 800_000, &salary),
            bare_entry(EntryKind::Transfer, 1, "", NaiveDate::MIN),
        ];
        let activity = category_activity(&entries, &[salary, unused, market, pets]);
        let expected = vec![
            ("pets".into(), 20_000, 1),
            ("mercado".into(), 13_000, 3),
            ("salário".into(), 800_000, 1),
        ];
        assert_eq!(summary(&activity), expected);
    }

    fn account(name: &str) -> Account {
        Account {
            id: AccountId::generate(),
            name: name.into(),
            kind: domain::AccountKind::Checking,
            initial_balance: Cents::ZERO,
            opened_on: NaiveDate::MIN,
            archived: false,
        }
    }

    #[test]
    fn accounts_count_both_sides_of_a_transfer() {
        let (checking, savings) = (account("Nubank"), account("Poupança"));
        let date = NaiveDate::from_ymd_opt(2026, 9, 10).unwrap();
        let transfer = LedgerEntry {
            account_id: Some(checking.id),
            counter_account_id: Some(savings.id),
            ..bare_entry(EntryKind::Transfer, 50_000, "", date)
        };
        let salary = LedgerEntry {
            account_id: Some(checking.id),
            ..bare_entry(EntryKind::Income, 800_000, "", date)
        };
        let activity = account_activity(&[transfer, salary], &[checking, savings]);
        let totals: Vec<(String, i64, usize)> = activity
            .iter()
            .map(|item| (item.account.name.clone(), item.total.value(), item.entries))
            .collect();
        assert_eq!(totals, vec![("Nubank".into(), 750_000, 2), ("Poupança".into(), 50_000, 1)]);
    }

    #[test]
    fn card_charges_subtract_credits() {
        let date = NaiveDate::from_ymd_opt(2026, 9, 10).unwrap();
        let entries = [
            bare_entry(EntryKind::CardInstallment, 10_000, "", date),
            bare_entry(EntryKind::CardCredit, 2_500, "", date),
            bare_entry(EntryKind::InvoicePayment, 7_500, "", date),
        ];
        assert_eq!(card_charges(&entries), Cents::new(7_500));
    }
}
