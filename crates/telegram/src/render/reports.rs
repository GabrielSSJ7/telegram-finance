//! Replies of the instant commands: balances, accounts, goals, categories.

use app::model::{BalanceSheet, Category, CategoryKind, GoalProgress};
use domain::money_format::format_brl;
use domain::{AccountKind, Cents};

use super::catalog::{account_label, category_label, kind_name};
use crate::flows::dates::short_date;
use crate::html::escape;

/// `/saldo`: spendable accounts, "Disponível" and pots apart.
pub fn balance_text(sheet: &BalanceSheet) -> String {
    let lines: Vec<String> = sheet
        .accounts
        .iter()
        .filter(|item| item.account.kind != AccountKind::Pot)
        .map(|item| format!("{}: {}", account_label(&item.account), format_brl(item.balance)))
        .collect();
    if lines.is_empty() {
        return "Nenhuma conta ainda. Crie uma com /novaconta.".into();
    }
    let date = short_date(sheet.as_of, sheet.as_of);
    let available = format_brl(sheet.position.available);
    let reserved = reserved_line(sheet.position.reserved_in_pots);
    format!(
        "<b>💰 Saldo</b> · {date}\n{}\n\n<b>Disponível: {available}</b>{reserved}",
        lines.join("\n")
    )
}

fn reserved_line(reserved: Cents) -> String {
    if reserved == Cents::ZERO {
        return String::new();
    }
    format!("\n🎯 Reservado em metas: {}", format_brl(reserved))
}

/// `/contas`: every active account with its kind.
pub fn accounts_text(sheet: &BalanceSheet) -> String {
    if sheet.accounts.is_empty() {
        return "Nenhuma conta ainda. Crie uma com /novaconta.".into();
    }
    let lines: Vec<String> = sheet
        .accounts
        .iter()
        .map(|item| {
            format!(
                "{} ({}): {}",
                account_label(&item.account),
                kind_name(item.account.kind),
                format_brl(item.balance)
            )
        })
        .collect();
    format!("<b>🏦 Contas</b>\n{}\n\nNova conta: /novaconta", lines.join("\n"))
}

/// `/metas`: progress bar per goal.
pub fn goals_text(goals: &[GoalProgress]) -> String {
    if goals.is_empty() {
        return "Nenhuma meta ainda. Crie uma com /novameta.".into();
    }
    let blocks: Vec<String> = goals.iter().map(goal_block).collect();
    format!("<b>🎯 Metas</b>\n\n{}", blocks.join("\n\n"))
}

fn goal_block(progress: &GoalProgress) -> String {
    let percent = progress.progress_bp / 100;
    let saved = format_brl(progress.saved);
    let target = format_brl(progress.goal.target.target);
    let name = escape(&progress.goal.pot.name);
    let remaining = format_brl(progress.remaining);
    format!(
        "<b>{name}</b>: {saved} de {target} ({percent}%)\n{} faltam {remaining}",
        progress_bar(progress.progress_bp)
    )
}

/// Ten-slot bar: `▓▓▓░░░░░░░`.
pub fn progress_bar(progress_bp: i64) -> String {
    let filled = usize::try_from(progress_bp.clamp(0, 10_000) / 1_000).unwrap_or(0);
    format!("{}{}", "▓".repeat(filled), "░".repeat(10 - filled))
}

/// `/categorias`: expense and income categories.
pub fn categories_text(categories: &[Category]) -> String {
    let list = |kind| {
        let names: Vec<String> = categories
            .iter()
            .filter(|category| category.kind == kind)
            .map(category_label)
            .collect();
        if names.is_empty() { "—".to_owned() } else { names.join(", ") }
    };
    format!(
        "<b>🏷️ Categorias</b>\nGastos: {}\nEntradas: {}",
        list(CategoryKind::Expense),
        list(CategoryKind::Income)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use app::model::{Account, AccountBalance, AccountId, CategoryId, Goal, GoalId, GoalTarget};
    use chrono::NaiveDate;
    use domain::balance::MoneyPosition;

    fn account(name: &str, kind: AccountKind) -> Account {
        let opened_on = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        Account {
            id: AccountId::generate(),
            name: name.into(),
            kind,
            initial_balance: Cents::ZERO,
            opened_on,
            archived: false,
        }
    }

    fn sheet(accounts: Vec<(Account, i64)>, available: i64, reserved: i64) -> BalanceSheet {
        BalanceSheet {
            as_of: NaiveDate::from_ymd_opt(2026, 3, 10).unwrap(),
            accounts: accounts
                .into_iter()
                .map(|(account, cents)| AccountBalance { account, balance: Cents::new(cents) })
                .collect(),
            position: MoneyPosition {
                available: Cents::new(available),
                reserved_in_pots: Cents::new(reserved),
            },
        }
    }

    #[test]
    fn balance_lists_spendable_and_reserved() {
        let text = balance_text(&sheet(
            vec![
                (account("Nubank", AccountKind::Checking), 123_456),
                (account("Casa", AccountKind::Pot), 2_000_000),
            ],
            123_456,
            2_000_000,
        ));
        assert!(text.contains("🏦 Nubank: R$ 1.234,56"), "{text}");
        assert!(!text.contains("Casa:"), "{text}");
        assert!(
            text.contains("<b>Disponível: R$ 1.234,56</b>\n🎯 Reservado em metas: R$ 20.000,00"),
            "{text}"
        );
        assert!(balance_text(&sheet(vec![], 0, 0)).contains("/novaconta"));
    }

    #[test]
    fn accounts_list_includes_kinds() {
        let text =
            accounts_text(&sheet(vec![(account("Carteira", AccountKind::Cash), 5_000)], 5_000, 0));
        assert!(text.contains("💵 Carteira (Dinheiro): R$ 50,00"), "{text}");
        assert!(accounts_text(&sheet(vec![], 0, 0)).contains("/novaconta"));
    }

    #[test]
    fn goals_show_bar_and_remaining() {
        let goal = Goal {
            id: GoalId::generate(),
            pot: account("Casa própria", AccountKind::Pot),
            target: GoalTarget { target: Cents::new(10_000_000), target_date: None },
        };
        let progress = GoalProgress {
            goal,
            saved: Cents::new(2_000_000),
            remaining: Cents::new(8_000_000),
            progress_bp: 2000,
        };
        let text = goals_text(&[progress]);
        assert!(text.contains("<b>Casa própria</b>: R$ 20.000,00 de R$ 100.000,00 (20%)\n▓▓░░░░░░░░ faltam R$ 80.000,00"), "{text}");
        assert!(goals_text(&[]).contains("/novameta"));
        assert_eq!(progress_bar(15_000), "▓".repeat(10));
    }

    #[test]
    fn categories_grouped_by_kind() {
        let category = |name: &str, kind| Category {
            id: CategoryId::generate(),
            name: name.into(),
            kind,
            emoji: None,
            archived: false,
        };
        let text = categories_text(&[category("mercado", CategoryKind::Expense)]);
        assert_eq!(text, "<b>🏷️ Categorias</b>\nGastos: mercado\nEntradas: —");
    }
}
