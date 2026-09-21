//! Kinds of ledger rows and the sign each one applies to balances,
//! spending and card invoices. Every report derives from these signs, so
//! adding a kind means updating exactly this file.

use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryKind {
    Income,
    Expense,
    Transfer,
    CardInstallment,
    /// Card refund ("estorno"): lowers the invoice and the category spend.
    CardCredit,
    InvoicePayment,
    /// Account refund: money back into an account, lowers category spend.
    Refund,
    AdjustIn,
    AdjustOut,
}

/// Which account column of the entry is being evaluated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountRole {
    /// `account_id`: the account the money leaves or enters.
    Primary,
    /// `counter_account_id`: the destination of a transfer.
    Counter,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("unknown entry kind {0:?}: expected one of {ALL_NAMES}")]
pub struct UnknownEntryKind(pub String);

const ALL_NAMES: &str = "income, expense, transfer, card_installment, card_credit, invoice_payment, refund, adjust_in, adjust_out";

impl EntryKind {
    pub const ALL: [EntryKind; 9] = [
        EntryKind::Income,
        EntryKind::Expense,
        EntryKind::Transfer,
        EntryKind::CardInstallment,
        EntryKind::CardCredit,
        EntryKind::InvoicePayment,
        EntryKind::Refund,
        EntryKind::AdjustIn,
        EntryKind::AdjustOut,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            EntryKind::Income => "income",
            EntryKind::Expense => "expense",
            EntryKind::Transfer => "transfer",
            EntryKind::CardInstallment => "card_installment",
            EntryKind::CardCredit => "card_credit",
            EntryKind::InvoicePayment => "invoice_payment",
            EntryKind::Refund => "refund",
            EntryKind::AdjustIn => "adjust_in",
            EntryKind::AdjustOut => "adjust_out",
        }
    }

    /// Sign applied to an account balance: +1 money in, -1 out, 0 none.
    ///
    /// ```
    /// use domain::{EntryKind, entry_kind::AccountRole};
    /// assert_eq!(EntryKind::Transfer.account_effect(AccountRole::Counter), 1);
    /// ```
    pub const fn account_effect(self, role: AccountRole) -> i64 {
        use EntryKind::{AdjustIn, AdjustOut, Expense, Income, InvoicePayment, Refund, Transfer};
        match (self, role) {
            (Income | Refund | AdjustIn, AccountRole::Primary)
            | (Transfer, AccountRole::Counter) => 1,
            (Expense | InvoicePayment | AdjustOut | Transfer, AccountRole::Primary) => -1,
            _ => 0,
        }
    }

    /// Sign applied to spending. Refunds reduce spending, never add income.
    pub const fn spend_effect(self) -> i64 {
        match self {
            EntryKind::Expense | EntryKind::CardInstallment => 1,
            EntryKind::Refund | EntryKind::CardCredit => -1,
            _ => 0,
        }
    }

    pub const fn income_effect(self) -> i64 {
        match self {
            EntryKind::Income => 1,
            _ => 0,
        }
    }

    /// Sign applied to what the couple owes on a card invoice.
    pub const fn invoice_effect(self) -> i64 {
        match self {
            EntryKind::CardInstallment => 1,
            EntryKind::CardCredit | EntryKind::InvoicePayment => -1,
            _ => 0,
        }
    }
}

impl FromStr for EntryKind {
    type Err = UnknownEntryKind;
    fn from_str(name: &str) -> Result<Self, Self::Err> {
        EntryKind::ALL
            .into_iter()
            .find(|kind| kind.as_str() == name)
            .ok_or_else(|| UnknownEntryKind(name.to_owned()))
    }
}

impl std::fmt::Display for EntryKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use AccountRole::{Counter, Primary};

    /// (kind, primary, counter, spend, income, invoice)
    const SIGNS: &[(EntryKind, i64, i64, i64, i64, i64)] = &[
        (EntryKind::Income, 1, 0, 0, 1, 0),
        (EntryKind::Expense, -1, 0, 1, 0, 0),
        (EntryKind::Transfer, -1, 1, 0, 0, 0),
        (EntryKind::CardInstallment, 0, 0, 1, 0, 1),
        (EntryKind::CardCredit, 0, 0, -1, 0, -1),
        (EntryKind::InvoicePayment, -1, 0, 0, 0, -1),
        (EntryKind::Refund, 1, 0, -1, 0, 0),
        (EntryKind::AdjustIn, 1, 0, 0, 0, 0),
        (EntryKind::AdjustOut, -1, 0, 0, 0, 0),
    ];

    #[test]
    fn every_kind_has_expected_signs() {
        assert_eq!(SIGNS.len(), EntryKind::ALL.len());
        for &(kind, primary, counter, spend, income, invoice) in SIGNS {
            assert_eq!(kind.account_effect(Primary), primary, "{kind} primary");
            assert_eq!(kind.account_effect(Counter), counter, "{kind} counter");
            assert_eq!(kind.spend_effect(), spend, "{kind} spend");
            assert_eq!(kind.income_effect(), income, "{kind} income");
            assert_eq!(kind.invoice_effect(), invoice, "{kind} invoice");
        }
    }

    #[test]
    fn round_trips_through_database_names() {
        for kind in EntryKind::ALL {
            assert_eq!(kind.as_str().parse::<EntryKind>(), Ok(kind));
        }
    }

    #[test]
    fn unknown_name_error_lists_expected_values() {
        let message = "salary".parse::<EntryKind>().unwrap_err().to_string();
        assert!(
            message.contains("\"salary\"") && message.contains("card_installment"),
            "{message}"
        );
    }
}
