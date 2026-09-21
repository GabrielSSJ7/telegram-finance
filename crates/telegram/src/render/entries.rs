//! `/ultimos`: the latest entries, each with edit and delete buttons.

use app::model::{LedgerEntry, Member};
use chrono::NaiveDate;
use domain::EntryKind;
use domain::money_format::format_brl;

use super::Catalog;
use super::card::relative_date;
use crate::callback_data::{delete_entry_button, edit_entry_button};
use crate::gateway::{Button, Keyboard};
use crate::html::escape;

pub fn recent_entries_view(
    entries: &[LedgerEntry],
    catalog: &Catalog,
    members: &[Member],
    today: NaiveDate,
) -> (String, Option<Keyboard>) {
    if entries.is_empty() {
        return ("Nenhum lançamento ainda.".into(), None);
    }
    let lines: Vec<String> = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            format!("{}. {}", index + 1, entry_line(entry, catalog, members, today))
        })
        .collect();
    let html =
        format!("<b>🧾 Últimos lançamentos</b>\n{}\n\n✏️ edita · 🗑️ apaga", lines.join("\n"));
    (html, Some(entry_keyboard(entries)))
}

/// `05/03 💸 R$ 10,50 · 🛒 mercado — feira (Ana)`.
pub fn entry_line(
    entry: &LedgerEntry,
    catalog: &Catalog,
    members: &[Member],
    today: NaiveDate,
) -> String {
    let what = entry
        .category_id
        .map_or_else(|| kind_label(entry).to_owned(), |id| catalog.category_label(id));
    let description = if entry.description.is_empty() {
        String::new()
    } else {
        format!(" — {}", escape(&entry.description))
    };
    let author = entry.created_by.and_then(|id| members.iter().find(|member| member.id == id));
    let author =
        author.map_or_else(|| "automático".to_owned(), |member| escape(&member.display_name));
    let date = relative_date(entry.accounting_date, today);
    format!(
        "{date} {} {} · {what}{description} ({author})",
        kind_icon(entry.kind),
        format_brl(entry.amount)
    )
}

/// Short label for the edit card: `R$ 10,50 · feira (05/03)`.
pub fn entry_label(entry: &LedgerEntry, today: NaiveDate) -> String {
    format!("{} ({})", entry_summary(entry), relative_date(entry.accounting_date, today))
}

/// `R$ 10,50 · feira`, or just the amount when there is no description.
pub fn entry_summary(entry: &LedgerEntry) -> String {
    if entry.description.is_empty() {
        return format_brl(entry.amount);
    }
    format!("{} · {}", format_brl(entry.amount), escape(&entry.description))
}

const fn kind_icon(kind: EntryKind) -> &'static str {
    match kind {
        EntryKind::Income => "💰",
        EntryKind::Expense => "💸",
        EntryKind::Transfer => "↔️",
        EntryKind::CardInstallment => "💳",
        EntryKind::CardCredit | EntryKind::Refund => "↩️",
        EntryKind::InvoicePayment => "🧾",
        EntryKind::AdjustIn | EntryKind::AdjustOut => "⚖️",
    }
}

fn kind_label(entry: &LedgerEntry) -> &'static str {
    match entry.kind {
        EntryKind::Transfer => "transferência",
        EntryKind::InvoicePayment => "pagamento de fatura",
        EntryKind::AdjustIn | EntryKind::AdjustOut => "ajuste",
        _ => "sem categoria",
    }
}

/// Two entries per row: [✏️ 1] [🗑️ 1] [✏️ 2] [🗑️ 2].
fn entry_keyboard(entries: &[LedgerEntry]) -> Keyboard {
    let buttons: Vec<Button> = entries
        .iter()
        .enumerate()
        .flat_map(|(index, entry)| {
            let number = index + 1;
            [
                Button { label: format!("✏️ {number}"), data: edit_entry_button(entry.id) },
                Button { label: format!("🗑️ {number}"), data: delete_entry_button(entry.id) },
            ]
        })
        .collect();
    Keyboard { rows: buttons.chunks(4).map(<[Button]>::to_vec).collect() }
}

#[cfg(test)]
mod tests {
    use app::fakes::requests::bare_entry;

    use super::*;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 3, 10).unwrap()
    }

    #[test]
    fn uncategorized_kinds_are_named() {
        let catalog = Catalog::default();
        let line = |kind| entry_line(&bare_entry(kind, 500, "", today()), &catalog, &[], today());
        assert_eq!(line(EntryKind::Transfer), "hoje ↔️ R$ 5,00 · transferência (automático)");
        assert!(line(EntryKind::InvoicePayment).contains("🧾 R$ 5,00 · pagamento de fatura"));
        assert!(line(EntryKind::AdjustOut).contains("⚖️ R$ 5,00 · ajuste"));
        assert!(line(EntryKind::Refund).contains("↩️ R$ 5,00 · sem categoria"));
        assert!(line(EntryKind::CardInstallment).contains("💳"));
    }

    #[test]
    fn label_escapes_and_skips_empty_description() {
        let entry = bare_entry(EntryKind::Expense, 1050, "<pão>", today());
        assert_eq!(entry_label(&entry, today()), "R$ 10,50 · &lt;pão&gt; (hoje)");
        let plain = LedgerEntry { description: String::new(), ..entry };
        assert_eq!(entry_label(&plain, today()), "R$ 10,50 (hoje)");
        assert_eq!(recent_entries_view(&[], &Catalog::default(), &[], today()).1, None);
    }
}
