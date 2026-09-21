//! Account balances and "Disponível". The database aggregates entry totals
//! per (kind, role); the sign rules live in [`EntryKind::account_effect`].

use serde::{Deserialize, Serialize};

use crate::Cents;
use crate::account_kind::AccountKind;
use crate::entry_kind::{AccountRole, EntryKind};

/// Sum of non-deleted entries of one kind touching an account in one role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountFlow {
    pub kind: EntryKind,
    pub role: AccountRole,
    pub total: Cents,
}

/// Balance = initial balance plus every flow with its sign.
///
/// ```
/// use domain::{Cents, EntryKind, balance::{AccountFlow, account_balance}, entry_kind::AccountRole};
/// let flows = [AccountFlow { kind: EntryKind::Expense, role: AccountRole::Primary, total: Cents::new(300) }];
/// assert_eq!(account_balance(Cents::new(1000), &flows), Cents::new(700));
/// ```
pub fn account_balance(initial: Cents, flows: &[AccountFlow]) -> Cents {
    let movement: Cents =
        flows.iter().map(|flow| flow.total.times(flow.kind.account_effect(flow.role))).sum();
    initial + movement
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct KindBalance {
    pub kind: AccountKind,
    pub balance: Cents,
}

/// Money position shown in `/saldo` and the daily report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoneyPosition {
    /// Spendable accounts minus closed invoices not yet paid.
    pub available: Cents,
    /// Sum of pot balances ("Reservado em metas").
    pub reserved_in_pots: Cents,
}

/// Computes "Disponível" and "Reservado em metas".
pub fn money_position(balances: &[KindBalance], unpaid_closed_invoices: Cents) -> MoneyPosition {
    let spendable: Cents =
        balances.iter().filter(|item| item.kind.is_spendable()).map(|item| item.balance).sum();
    let reserved: Cents =
        balances.iter().filter(|item| !item.kind.is_spendable()).map(|item| item.balance).sum();
    MoneyPosition { available: spendable - unpaid_closed_invoices, reserved_in_pots: reserved }
}

#[cfg(test)]
mod tests {
    use super::*;
    use AccountRole::{Counter, Primary};

    fn flow(kind: EntryKind, role: AccountRole, total: i64) -> AccountFlow {
        AccountFlow { kind, role, total: Cents::new(total) }
    }

    #[test]
    fn balance_applies_every_sign() {
        let flows = [
            flow(EntryKind::Income, Primary, 500_000),
            flow(EntryKind::Expense, Primary, 10_000),
            flow(EntryKind::Transfer, Primary, 50_000),
            flow(EntryKind::Transfer, Counter, 20_000),
            flow(EntryKind::InvoicePayment, Primary, 30_000),
            flow(EntryKind::Refund, Primary, 1_000),
            flow(EntryKind::AdjustIn, Primary, 5),
            flow(EntryKind::AdjustOut, Primary, 10),
            flow(EntryKind::CardInstallment, Primary, 99_999),
        ];
        let expected = 100_000 + 500_000 - 10_000 - 50_000 + 20_000 - 30_000 + 1_000 + 5 - 10;
        assert_eq!(account_balance(Cents::new(100_000), &flows), Cents::new(expected));
    }

    #[test]
    fn position_excludes_pots_and_subtracts_closed_invoices() {
        let balances = [
            KindBalance { kind: AccountKind::Checking, balance: Cents::new(300_000) },
            KindBalance { kind: AccountKind::Cash, balance: Cents::new(5_000) },
            KindBalance { kind: AccountKind::Pot, balance: Cents::new(2_000_000) },
        ];
        let position = money_position(&balances, Cents::new(120_000));
        assert_eq!(position.available, Cents::new(185_000));
        assert_eq!(position.reserved_in_pots, Cents::new(2_000_000));
    }
}
