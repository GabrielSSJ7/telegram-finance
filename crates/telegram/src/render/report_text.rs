//! pt-BR text of the daily and cycle reports.

use app::model::{
    BudgetStatus, Category, CategoryId, CycleReport, DailyReport, LedgerEntry, Member, MemberId,
    PeriodTotals,
};
use chrono::{Datelike, NaiveDate, Weekday};
use domain::Cents;
use domain::cycle::Cycle;
use domain::money_format::format_brl;
use domain::spend::PeriodSummary;

use super::catalog::category_label;
use super::notices::budget_line;
use super::reports::{balance_text, goals_text, invoices_text};
use crate::html::escape;

/// The daily report lists budgets from 80% on.
const WARNING_BP: i64 = 8_000;

/// Categories listed in the cycle report before "outros".
const TOP_CATEGORIES: usize = 8;

pub fn daily_report_text(report: &DailyReport) -> String {
    let names = Names { categories: &report.categories, members: &report.members };
    let mut sections = vec![format!("<b>📊 Resumo de {}</b>", weekday_date(report.date))];
    sections.push(today_section(report, &names));
    sections.push(cycle_section(report.cycle, report.date, &report.cycle_to_date, &names));
    sections.push(balance_text(&report.balances));
    if !report.cards.is_empty() {
        sections.push(invoices_text(&report.cards));
    }
    if let Some(budgets) = budgets_section(&report.budgets, WARNING_BP) {
        sections.push(budgets);
    }
    if !report.goals.is_empty() {
        sections.push(goals_text(&report.goals, report.date));
    }
    if !report.upcoming.is_empty() {
        sections.push(upcoming_section(report));
    }
    sections.join("\n\n")
}

pub fn cycle_report_text(report: &CycleReport) -> String {
    let header = format!("<b>🗓️ Fechamento do ciclo {}</b>", cycle_range(report.cycle));
    cycle_body(report, header)
}

/// `/mes`: the same sections for the cycle still running.
pub fn month_report_text(report: &CycleReport, today: NaiveDate) -> String {
    let days_left = (report.cycle.end_exclusive - today).num_days() - 1;
    let header =
        format!("<b>📅 Ciclo atual {}</b> (faltam {days_left} dias)", cycle_range(report.cycle));
    cycle_body(report, header)
}

fn cycle_body(report: &CycleReport, header: String) -> String {
    let names = Names { categories: &report.categories, members: &report.members };
    let summary = &report.totals.summary;
    let mut sections = vec![header];
    sections.push(cycle_totals(summary, &report.previous, report.saved_in_pots));
    sections.push(category_breakdown(&report.totals, &names));
    sections.push(member_line(&report.totals, &names));
    sections.push(balance_text(&report.balances));
    if !report.cards.is_empty() {
        sections.push(invoices_text(&report.cards));
    }
    if let Some(budgets) = budgets_section(&report.budgets, 0) {
        sections.push(budgets);
    }
    if !report.goals.is_empty() {
        sections.push(goals_text(&report.goals, report.cycle.last_day()));
    }
    sections.join("\n\n")
}

/// Budgets at or above `min_used_bp`; `None` when there are none.
fn budgets_section(budgets: &[BudgetStatus], min_used_bp: i64) -> Option<String> {
    let lines: Vec<String> =
        budgets.iter().filter(|status| status.used_bp >= min_used_bp).map(budget_line).collect();
    (!lines.is_empty()).then(|| format!("<b>Orçamentos</b>\n{}", lines.join("\n")))
}

/// Category and member names carried by a report.
struct Names<'a> {
    categories: &'a [Category],
    members: &'a [Member],
}

impl Names<'_> {
    fn category(&self, id: Option<CategoryId>) -> String {
        let found = id.and_then(|id| self.categories.iter().find(|category| category.id == id));
        found.map_or_else(|| "sem categoria".to_owned(), category_label)
    }

    fn member(&self, id: Option<MemberId>) -> String {
        let found = id.and_then(|id| self.members.iter().find(|member| member.id == id));
        found.map_or_else(|| "automático".to_owned(), |member| escape(&member.display_name))
    }
}

fn today_section(report: &DailyReport, names: &Names<'_>) -> String {
    if report.entries_today.is_empty() {
        return "<b>Hoje</b>\nNenhum lançamento hoje.".into();
    }
    let lines: Vec<String> =
        report.entries_today.iter().map(|entry| entry_line(entry, names)).collect();
    let spent = format_brl(report.today.summary.spending);
    format!("<b>Hoje</b> ({} lançamentos)\n{}\nGasto hoje: {spent}", lines.len(), lines.join("\n"))
}

fn entry_line(entry: &LedgerEntry, names: &Names<'_>) -> String {
    let description = if entry.description.is_empty() {
        String::new()
    } else {
        format!(" — {}", escape(&entry.description))
    };
    let what = if entry.category_id.is_some() {
        names.category(entry.category_id)
    } else {
        kind_label(entry.kind).into()
    };
    format!(
        "• {}: {what} {}{description}",
        names.member(entry.created_by),
        format_brl(entry.amount)
    )
}

const fn kind_label(kind: domain::EntryKind) -> &'static str {
    match kind {
        domain::EntryKind::Transfer => "↔️ transferência",
        domain::EntryKind::InvoicePayment => "💳 pagamento de fatura",
        domain::EntryKind::AdjustIn | domain::EntryKind::AdjustOut => "⚖️ ajuste",
        _ => "lançamento",
    }
}

fn cycle_section(
    cycle: Cycle,
    date: NaiveDate,
    totals: &PeriodTotals,
    names: &Names<'_>,
) -> String {
    let days_left = (cycle.end_exclusive - date).num_days() - 1;
    let summary = &totals.summary;
    let header = format!("<b>Ciclo {}</b> (faltam {days_left} dias)", cycle_range(cycle));
    let body = format!(
        "Entradas: {}\nGastos: {}\nSobra: {}{}",
        format_brl(summary.income),
        format_brl(summary.spending),
        format_brl(summary.saved),
        rate_suffix(summary)
    );
    format!("{header}\n{body}\n{}", member_line(totals, names))
}

fn cycle_totals(summary: &PeriodSummary, previous: &PeriodSummary, saved_in_pots: Cents) -> String {
    let lines = [
        format!("Entradas: {}", format_brl(summary.income)),
        format!(
            "Gastos: {}{}",
            format_brl(summary.spending),
            change_suffix(summary.spending, previous.spending)
        ),
        format!("Economizado: {}{}", format_brl(summary.saved), rate_suffix(summary)),
        format!("Guardado nas metas: {}", format_brl(saved_in_pots)),
    ];
    lines.join("\n")
}

fn category_breakdown(totals: &PeriodTotals, names: &Names<'_>) -> String {
    if totals.by_category.is_empty() {
        return "<b>Onde foi o dinheiro</b>\nNenhum gasto no ciclo.".into();
    }
    let spending = totals.summary.spending.value().max(1);
    let lines: Vec<String> = totals
        .by_category
        .iter()
        .take(TOP_CATEGORIES)
        .map(|(category, amount)| {
            format!(
                "{} {} ({}%)",
                names.category(*category),
                format_brl(*amount),
                amount.value() * 100 / spending
            )
        })
        .collect();
    format!("<b>Onde foi o dinheiro</b>\n{}", lines.join("\n"))
}

fn member_line(totals: &PeriodTotals, names: &Names<'_>) -> String {
    let parts: Vec<String> = totals
        .by_member
        .iter()
        .map(|(member, amount)| format!("{}: {}", names.member(*member), format_brl(*amount)))
        .collect();
    if parts.is_empty() {
        return "Gastos por pessoa: —".into();
    }
    format!("Gastos por pessoa: {}", parts.join(" · "))
}

fn upcoming_section(report: &DailyReport) -> String {
    let lines: Vec<String> = report
        .upcoming
        .iter()
        .map(|(recurrence, date)| {
            format!(
                "🔁 {} {} {}",
                date.format("%d/%m"),
                escape(&recurrence.description),
                format_brl(recurrence.amount)
            )
        })
        .collect();
    format!("<b>Próximos dias</b>\n{}", lines.join("\n"))
}

fn rate_suffix(summary: &PeriodSummary) -> String {
    summary
        .savings_rate_bp
        .map(|rate| format!(" ({}% das entradas)", rate / 100))
        .unwrap_or_default()
}

/// ` (▲12% vs ciclo anterior)`; empty without a previous cycle to compare.
fn change_suffix(current: Cents, previous: Cents) -> String {
    if !previous.is_positive() {
        return String::new();
    }
    let change = (current.value() - previous.value()) * 100 / previous.value();
    let arrow = if change >= 0 { "▲" } else { "▼" };
    format!(" ({arrow}{}% vs ciclo anterior)", change.abs())
}

fn cycle_range(cycle: Cycle) -> String {
    format!("{} → {}", cycle.start.format("%d/%m"), cycle.last_day().format("%d/%m"))
}

fn weekday_date(date: NaiveDate) -> String {
    let weekday = match date.weekday() {
        Weekday::Mon => "segunda",
        Weekday::Tue => "terça",
        Weekday::Wed => "quarta",
        Weekday::Thu => "quinta",
        Weekday::Fri => "sexta",
        Weekday::Sat => "sábado",
        Weekday::Sun => "domingo",
    };
    format!("{weekday}, {}", date.format("%d/%m"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn change_and_rate_suffixes() {
        assert_eq!(change_suffix(Cents::new(112), Cents::new(100)), " (▲12% vs ciclo anterior)");
        assert_eq!(change_suffix(Cents::new(50), Cents::new(100)), " (▼50% vs ciclo anterior)");
        assert_eq!(change_suffix(Cents::new(50), Cents::ZERO), "");
        let summary = PeriodSummary {
            income: Cents::new(100),
            spending: Cents::new(66),
            saved: Cents::new(34),
            savings_rate_bp: Some(3400),
        };
        assert_eq!(rate_suffix(&summary), " (34% das entradas)");
    }

    #[test]
    fn weekday_names_in_portuguese() {
        assert_eq!(weekday_date(NaiveDate::from_ymd_opt(2026, 3, 10).unwrap()), "terça, 10/03");
        assert_eq!(weekday_date(NaiveDate::from_ymd_opt(2026, 3, 15).unwrap()), "domingo, 15/03");
    }
}
