//! `/extrato`: what a cycle moved, grouped by category, by account or by
//! card, and the entries behind one of those totals.

use app::model::{LedgerEntry, Member};
use chrono::NaiveDate;
use domain::Cents;
use domain::money_format::format_brl;

use crate::callback_data::{StatementItem, statement_item_button, statement_view_button};
use crate::gateway::{Button, Keyboard};
use crate::html::escape;

/// Telegram refuses messages over 4096 characters; stop well before.
const MAX_MESSAGE_CHARS: usize = 3_600;

/// How `/extrato` groups a cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatementGroup {
    Category,
    Account,
    Card,
}

impl StatementGroup {
    pub const ALL: [StatementGroup; 3] =
        [StatementGroup::Category, StatementGroup::Account, StatementGroup::Card];

    pub const fn code(self) -> &'static str {
        match self {
            StatementGroup::Category => "c",
            StatementGroup::Account => "a",
            StatementGroup::Card => "k",
        }
    }

    pub fn from_code(code: &str) -> Option<StatementGroup> {
        StatementGroup::ALL.into_iter().find(|group| group.code() == code)
    }

    /// Title of the overview, and label of the button that switches to it.
    pub const fn title(self) -> &'static str {
        match self {
            StatementGroup::Category => "🏷️ Por categoria",
            StatementGroup::Account => "🏦 Por conta",
            StatementGroup::Card => "💳 Por cartão",
        }
    }

    const fn empty_text(self) -> &'static str {
        match self {
            StatementGroup::Category => "Nenhum lançamento com categoria nesse ciclo.",
            StatementGroup::Account => "Nenhuma conta movimentou nesse ciclo.",
            StatementGroup::Card => "Nenhum gasto em cartão nesse ciclo.",
        }
    }
}

/// The period of a statement, both days included.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatementPeriod {
    pub from: NaiveDate,
    pub to: NaiveDate,
}

impl StatementPeriod {
    pub fn label(self) -> String {
        format!("{} → {}", self.from.format("%d/%m"), self.to.format("%d/%m/%Y"))
    }
}

/// One total of the overview, with the record it belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatementRow {
    pub item: StatementItem,
    pub label: String,
    pub total: Cents,
    pub entries: usize,
}

/// Rows under a heading; categories come in two blocks, the rest in one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatementSection {
    pub title: Option<&'static str>,
    pub rows: Vec<StatementRow>,
}

/// Totals of the period plus one button per row and the other groupings.
pub fn statement_overview(
    group: StatementGroup,
    period: StatementPeriod,
    sections: &[StatementSection],
) -> (String, Option<Keyboard>) {
    let title = format!("<b>📂 Extrato {}</b> · {}", lower_title(group), period.label());
    let rows: Vec<&StatementRow> =
        sections.iter().flat_map(|section| section.rows.iter()).collect();
    if rows.is_empty() {
        return (format!("{title}\n{}", group.empty_text()), Some(switch_keyboard(group, period)));
    }
    let blocks: Vec<String> =
        sections.iter().filter(|section| !section.rows.is_empty()).map(section_text).collect();
    let html = format!("{title}\n\n{}\n\nToque para ver os lançamentos.", blocks.join("\n\n"));
    (html, Some(overview_keyboard(group, period, &rows)))
}

/// `por categoria` out of the button label `🏷️ Por categoria`.
fn lower_title(group: StatementGroup) -> String {
    let words: Vec<&str> = group.title().split_whitespace().skip(1).collect();
    words.join(" ").to_lowercase()
}

fn section_text(section: &StatementSection) -> String {
    let lines: Vec<String> = section
        .rows
        .iter()
        .map(|row| format!("{}: {} ({})", row.label, format_brl(row.total), row.entries))
        .collect();
    let total: Cents = section.rows.iter().map(|row| row.total).sum();
    let body = format!("{}\nTotal: {}", lines.join("\n"), format_brl(total));
    match section.title {
        Some(title) => format!("<b>{title}</b>\n{body}"),
        None => body,
    }
}

fn overview_keyboard(
    group: StatementGroup,
    period: StatementPeriod,
    rows: &[&StatementRow],
) -> Keyboard {
    let buttons: Vec<Button> = rows
        .iter()
        .map(|row| Button {
            label: row.label.clone(),
            data: statement_item_button(row.item, period.from, period.to),
        })
        .collect();
    let mut keyboard = Keyboard { rows: buttons.chunks(2).map(<[Button]>::to_vec).collect() };
    keyboard.rows.extend(switch_keyboard(group, period).rows);
    keyboard
}

/// One button per other grouping of the same period.
fn switch_keyboard(group: StatementGroup, period: StatementPeriod) -> Keyboard {
    let others: Vec<Button> = StatementGroup::ALL
        .into_iter()
        .filter(|other| *other != group)
        .map(|other| Button {
            label: other.title().to_owned(),
            data: statement_view_button(other, period.from, period.to),
        })
        .collect();
    Keyboard { rows: vec![others] }
}

/// Every entry behind one total, oldest first; `effect` gives each entry
/// the sign of the grouping being read.
pub fn statement_entries_text(
    heading: &str,
    entries: &[LedgerEntry],
    members: &[Member],
    period: StatementPeriod,
    effect: impl Fn(&LedgerEntry) -> Cents,
) -> String {
    let title = format!("<b>{heading}</b> · {}", period.label());
    if entries.is_empty() {
        return format!("{title}\nNenhum lançamento nesse ciclo.");
    }
    let total: Cents = entries.iter().map(&effect).sum();
    let footer = format!("Total: {} ({} lançamentos)", format_brl(total), entries.len());
    let lines: Vec<String> =
        entries.iter().map(|entry| statement_line(entry, members, effect(entry))).collect();
    format!(
        "{title}\n{}\n{footer}",
        fit_lines(&lines, MAX_MESSAGE_CHARS - title.len() - footer.len())
    )
}

/// `16/09 💳 R$ 89,66 — rapé (GL)`; what reduces the total shows negative.
fn statement_line(entry: &LedgerEntry, members: &[Member], amount: Cents) -> String {
    let marker = match entry.kind {
        domain::EntryKind::CardInstallment => "💳 ",
        domain::EntryKind::Refund | domain::EntryKind::CardCredit => "↩️ ",
        domain::EntryKind::Transfer => "↔️ ",
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
    format!(
        "{} {marker}{}{description} ({author})",
        entry.accounting_date.format("%d/%m"),
        format_brl(amount)
    )
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
