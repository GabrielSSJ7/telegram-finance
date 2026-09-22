//! `/extrato`: spending and income per category for a cycle, and the
//! entries behind one category.

use app::model::{Category, CategoryKind, LedgerEntry, Member};
use app::services::category_activity::{CategoryActivity, category_effect};
use chrono::NaiveDate;
use domain::money_format::format_brl;
use domain::{Cents, EntryKind};

use super::catalog::category_label;
use crate::callback_data::category_statement_button;
use crate::gateway::{Button, Keyboard};
use crate::html::escape;

/// Telegram refuses messages over 4096 characters; stop well before.
const MAX_MESSAGE_CHARS: usize = 3_600;

/// The period of a statement, both days included.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatementPeriod {
    pub from: NaiveDate,
    pub to: NaiveDate,
}

impl StatementPeriod {
    fn label(self) -> String {
        format!("{} → {}", self.from.format("%d/%m"), self.to.format("%d/%m/%Y"))
    }
}

/// Totals per category plus one button per category.
pub fn statement_overview(
    activity: &[CategoryActivity],
    period: StatementPeriod,
) -> (String, Option<Keyboard>) {
    let title = format!("<b>📂 Extrato por categoria</b> · {}", period.label());
    if activity.is_empty() {
        return (format!("{title}\nNenhum lançamento com categoria nesse ciclo."), None);
    }
    let sections = [
        kind_section("💸 Gastos", CategoryKind::Expense, activity),
        kind_section("💰 Entradas", CategoryKind::Income, activity),
    ];
    let body: Vec<String> = sections.into_iter().flatten().collect();
    let html = format!(
        "{title}\n\n{}\n\nToque numa categoria para ver os lançamentos.",
        body.join("\n\n")
    );
    (html, Some(category_keyboard(activity, period)))
}

fn kind_section(title: &str, kind: CategoryKind, activity: &[CategoryActivity]) -> Option<String> {
    let rows: Vec<&CategoryActivity> =
        activity.iter().filter(|item| item.category.kind == kind).collect();
    if rows.is_empty() {
        return None;
    }
    let lines: Vec<String> = rows
        .iter()
        .map(|item| {
            format!(
                "{}: {} ({})",
                category_label(&item.category),
                format_brl(item.total),
                item.entries
            )
        })
        .collect();
    let total: Cents = rows.iter().map(|item| item.total).sum();
    Some(format!("<b>{title}</b>\n{}\nTotal: {}", lines.join("\n"), format_brl(total)))
}

fn category_keyboard(activity: &[CategoryActivity], period: StatementPeriod) -> Keyboard {
    let buttons: Vec<Button> = activity
        .iter()
        .map(|item| Button {
            label: category_label(&item.category),
            data: category_statement_button(item.category.id, period.from, period.to),
        })
        .collect();
    Keyboard { rows: buttons.chunks(2).map(<[Button]>::to_vec).collect() }
}

/// Every entry of `category` in the period, oldest first.
pub fn category_entries_text(
    category: &Category,
    entries: &[LedgerEntry],
    members: &[Member],
    period: StatementPeriod,
) -> String {
    let title = format!("<b>{}</b> · {}", category_label(category), period.label());
    if entries.is_empty() {
        return format!("{title}\nNenhum lançamento nesse ciclo.");
    }
    let total: Cents = entries.iter().map(|entry| category_effect(entry, category.kind)).sum();
    let footer = format!("Total: {} ({} lançamentos)", format_brl(total), entries.len());
    let lines: Vec<String> =
        entries.iter().map(|entry| statement_line(entry, category.kind, members)).collect();
    format!(
        "{title}\n{}\n{footer}",
        fit_lines(&lines, MAX_MESSAGE_CHARS - title.len() - footer.len())
    )
}

/// `16/09 💳 R$ 89,66 — rapé (GL)`; refunds show as negative.
fn statement_line(entry: &LedgerEntry, kind: CategoryKind, members: &[Member]) -> String {
    let marker = match entry.kind {
        EntryKind::CardInstallment => "💳 ",
        EntryKind::Refund | EntryKind::CardCredit => "↩️ ",
        _ => "",
    };
    let description = if entry.description.is_empty() {
        String::new()
    } else {
        format!(" — {}", escape(&entry.description))
    };
    let author = entry.created_by.and_then(|id| members.iter().find(|member| member.id == id));
    let author =
        author.map_or_else(|| "automático".to_owned(), |member| escape(&member.display_name));
    let amount = format_brl(category_effect(entry, kind));
    format!("{} {marker}{amount}{description} ({author})", entry.accounting_date.format("%d/%m"))
}

/// Joins `lines` within `budget` characters; the rest becomes a note that
/// points to the CSV export.
fn fit_lines(lines: &[String], budget: usize) -> String {
    let mut used = 0;
    let mut kept = Vec::new();
    for line in lines {
        used += line.chars().count() + 1;
        if used > budget {
            break;
        }
        kept.push(line.as_str());
    }
    let hidden = lines.len() - kept.len();
    if hidden == 0 {
        return kept.join("\n");
    }
    format!("{}\n… e mais {hidden}. Veja todos com /exportar.", kept.join("\n"))
}

#[cfg(test)]
mod tests {
    use app::fakes::requests::bare_entry;
    use app::model::CategoryId;

    use super::*;

    fn period() -> StatementPeriod {
        let day = |month, day| NaiveDate::from_ymd_opt(2026, month, day).unwrap();
        StatementPeriod { from: day(9, 5), to: day(10, 4) }
    }

    fn market() -> Category {
        Category {
            id: CategoryId::generate(),
            name: "mercado".into(),
            kind: CategoryKind::Expense,
            emoji: Some("🛒".into()),
            archived: false,
        }
    }

    #[test]
    fn empty_cycle_has_no_buttons() {
        let (html, keyboard) = statement_overview(&[], period());
        assert!(html.contains("Nenhum lançamento com categoria"), "{html}");
        assert!(keyboard.is_none());
    }

    #[test]
    fn refunds_are_negative_lines() {
        let day = NaiveDate::from_ymd_opt(2026, 9, 16).unwrap();
        let entries = [
            bare_entry(EntryKind::CardInstallment, 8_966, "rapé", day),
            bare_entry(EntryKind::Refund, 1_000, "", day),
        ];
        let text = category_entries_text(&market(), &entries, &[], period());
        assert!(text.contains("16/09 💳 R$ 89,66 — rapé (automático)"), "{text}");
        assert!(text.contains("16/09 ↩️ -R$ 10,00 (automático)"), "{text}");
        assert!(text.contains("Total: R$ 79,66 (2 lançamentos)"), "{text}");
    }

    #[test]
    fn long_lists_are_cut_with_a_pointer_to_the_export() {
        let lines: Vec<String> = (0..10).map(|number| format!("linha {number}")).collect();
        let fitted = fit_lines(&lines, 30);
        assert!(fitted.starts_with("linha 0\nlinha 1\nlinha 2\n… e mais 7."), "{fitted}");
        assert_eq!(fit_lines(&lines[..2], 30), "linha 0\nlinha 1");
    }
}
