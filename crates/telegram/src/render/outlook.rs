//! How the cycle should end: which categories make up the basic cost of
//! living (`/essenciais`), what it adds up to (`/custodevida`) and where
//! the cycle lands (`/projecao`).

use app::model::{Category, CategoryKind, CycleProjection, Flow, LivingCost, RESERVE_MONTHS};
use domain::money_format::format_brl;

use super::catalog::category_label;
use crate::callback_data::toggle_essential_button;
use crate::gateway::{Button, Keyboard};

/// Guideline for needs in the 50/30/20 budget: half of the income.
const NEEDS_GUIDELINE_PERCENT: i64 = 50;

/// Expense categories as toggles: ✅ essential, ⬜ not.
pub fn essentials_view(categories: &[Category]) -> (String, Option<Keyboard>) {
    let expenses: Vec<&Category> =
        categories.iter().filter(|category| category.kind == CategoryKind::Expense).collect();
    if expenses.is_empty() {
        return ("Nenhuma categoria de gasto ainda. Crie com /novacategoria.".into(), None);
    }
    let buttons: Vec<Button> = expenses
        .iter()
        .map(|category| {
            let mark = if category.essential { "✅" } else { "⬜" };
            let label = format!("{mark} {}", category_label(category));
            Button { label, data: toggle_essential_button(category.id) }
        })
        .collect();
    let html = "<b>🏠 Categorias essenciais</b>\nToque para marcar ou desmarcar. As marcadas com ✅ formam o custo de vida básico (/custodevida).";
    (html.to_owned(), Some(Keyboard { rows: buttons.chunks(2).map(<[Button]>::to_vec).collect() }))
}

/// The cost of living of one cycle.
pub fn living_cost_text(cost: &LivingCost) -> String {
    let title = format!(
        "<b>🏠 Custo de vida básico</b> · {} → {}",
        cost.cycle.start.format("%d/%m"),
        cost.cycle.last_day().format("%d/%m/%Y")
    );
    if cost.by_category.is_empty() && cost.recent_average.is_none() {
        return format!(
            "{title}\nNada essencial nesse ciclo ainda. Marque as categorias em /essenciais."
        );
    }
    let sections = [cycle_lines(cost), category_lines(cost), reserve_lines(cost)];
    format!("{title}\n{}", sections.join("\n\n"))
}

fn cycle_lines(cost: &LivingCost) -> String {
    let mut lines = vec![format!("Gasto até agora: {}", format_brl(cost.spent))];
    if cost.still_coming.is_positive() {
        lines.push(format!(
            "Ainda vem no ciclo: {} (parcelas e contas recorrentes)",
            format_brl(cost.still_coming)
        ));
    }
    lines.push(format!("<b>Previsto no ciclo: {}</b>", format_brl(cost.projected())));
    lines.push(average_line(cost));
    if let Some(share) = cost.income_share_bp() {
        lines.push(format!(
            "Renda do ciclo: {} → essencial = {}% (referência: até {NEEDS_GUIDELINE_PERCENT}%)",
            format_brl(cost.expected_income),
            share / 100
        ));
    }
    lines.join("\n")
}

fn average_line(cost: &LivingCost) -> String {
    match cost.recent_average {
        Some(average) => {
            format!("Média dos últimos {} ciclos: {}", cost.averaged_cycles, format_brl(average))
        }
        None => "Média: aparece depois do primeiro ciclo completo.".into(),
    }
}

fn category_lines(cost: &LivingCost) -> String {
    let lines: Vec<String> = cost
        .by_category
        .iter()
        .map(|(category, total)| format!("{}: {}", category_label(category), format_brl(*total)))
        .collect();
    format!("<b>Por categoria</b>\n{}", lines.join("\n"))
}

fn reserve_lines(cost: &LivingCost) -> String {
    let target = format!(
        "<b>Reserva de emergência</b> ({RESERVE_MONTHS} meses): {}",
        format_brl(cost.reserve_target())
    );
    match cost.reserve_tenths_of_month() {
        Some(tenths) => format!(
            "{target}\nGuardado em metas: {} → {},{} meses",
            format_brl(cost.reserved),
            tenths / 10,
            tenths % 10
        ),
        None => target,
    }
}

/// `/projecao`: the cycle's result and the cash left at its last day.
pub fn projection_text(projected: &CycleProjection) -> String {
    let title = format!(
        "<b>🔮 Projeção do ciclo</b> · {} → {} (faltam {} dias)",
        projected.cycle.start.format("%d/%m"),
        projected.cycle.last_day().format("%d/%m/%Y"),
        projected.days_left
    );
    let mut sections = vec![title, result_lines(projected), cash_lines(projected)];
    if !projected.by_category.is_empty() {
        sections.push(biggest_lines(projected));
    }
    if projected.bills_to_confirm > 0 {
        sections.push(format!(
            "⚠️ {} recorrente(s) esperando confirmação; veja /recorrentes.",
            projected.bills_to_confirm
        ));
    }
    sections.join("\n\n")
}

fn result_lines(projected: &CycleProjection) -> String {
    let result = projected.result();
    let (label, amount) =
        if result.value() < 0 { ("Falta", -result) } else { ("Sobra prevista", result) };
    // A negative result would read as "-3% das entradas", which says nothing.
    let share = projected
        .saved_bp()
        .filter(|_| result.value() > 0)
        .map(|bp| format!(" ({}% das entradas)", bp / 100))
        .unwrap_or_default();
    format!(
        "<b>Resultado</b>\nEntradas: {}\nGastos: {}\n<b>{label}: {}</b>{share}",
        flow_line(projected.income, "recebidas", "a receber"),
        flow_line(projected.spending, "lançados", "a lançar"),
        format_brl(amount)
    )
}

/// `R$ 12.000,00 (R$ 8.000,00 recebidas · R$ 4.000,00 a receber)`.
fn flow_line(flow: Flow, done: &str, coming: &str) -> String {
    if !flow.coming.is_positive() {
        return format_brl(flow.total());
    }
    format!(
        "{} ({} {done} · {} {coming})",
        format_brl(flow.total()),
        format_brl(flow.recorded),
        format_brl(flow.coming)
    )
}

fn cash_lines(projected: &CycleProjection) -> String {
    format!(
        "<b>Caixa até {}</b>\nDisponível hoje: {}\n+ a receber: {}\n− faturas e contas a pagar: {}\n<b>= no fim do ciclo: {}</b>",
        projected.cycle.last_day().format("%d/%m"),
        format_brl(projected.available),
        format_brl(projected.income.coming),
        format_brl(projected.due_from_accounts),
        format_brl(projected.cash_at_end())
    )
}

fn biggest_lines(projected: &CycleProjection) -> String {
    let lines: Vec<String> = projected
        .by_category
        .iter()
        .map(|(category, total)| format!("{}: {}", category_label(category), format_brl(*total)))
        .collect();
    format!("<b>Maiores gastos previstos</b>\n{}", lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use app::model::CategoryId;
    use chrono::NaiveDate;
    use domain::cycle::Cycle;
    use domain::{Cents, DayOfMonth};

    use super::*;

    fn category(name: &str, kind: CategoryKind, essential: bool) -> Category {
        Category {
            id: CategoryId::generate(),
            name: name.into(),
            kind,
            emoji: None,
            archived: false,
            essential,
        }
    }

    fn cost(by_category: Vec<(Category, Cents)>, average: Option<i64>) -> LivingCost {
        let today = NaiveDate::from_ymd_opt(2026, 9, 22).unwrap();
        LivingCost {
            cycle: Cycle::containing(today, DayOfMonth::new(5).unwrap()),
            spent: Cents::new(321_000),
            still_coming: Cents::new(84_000),
            by_category,
            recent_average: average.map(Cents::new),
            averaged_cycles: usize::from(average.is_some()) * 3,
            expected_income: Cents::new(1_200_000),
            reserved: Cents::new(2_000_000),
        }
    }

    #[test]
    fn toggles_list_only_expense_categories() {
        let categories = [
            category("casa", CategoryKind::Expense, true),
            category("lazer", CategoryKind::Expense, false),
            category("salário", CategoryKind::Income, false),
        ];
        let (_, keyboard) = essentials_view(&categories);
        let labels: Vec<String> =
            keyboard.unwrap().rows.concat().into_iter().map(|button| button.label).collect();
        assert_eq!(labels, vec!["✅ casa", "⬜ lazer"]);
        assert!(essentials_view(&categories[2..]).1.is_none());
    }

    #[test]
    fn full_cost_of_living() {
        let home = category("casa", CategoryKind::Expense, true);
        let text = living_cost_text(&cost(vec![(home, Cents::new(405_000))], Some(398_000)));
        assert!(text.starts_with("<b>🏠 Custo de vida básico</b> · 05/09 → 04/10/2026"), "{text}");
        assert!(text.contains("Ainda vem no ciclo: R$ 840,00"), "{text}");
        assert!(text.contains("<b>Previsto no ciclo: R$ 4.050,00</b>"), "{text}");
        assert!(text.contains("Média dos últimos 3 ciclos: R$ 3.980,00"), "{text}");
        assert!(text.contains("essencial = 33% (referência: até 50%)"), "{text}");
        assert!(text.contains("casa: R$ 4.050,00"), "{text}");
        assert!(
            text.contains("(6 meses): R$ 23.880,00\nGuardado em metas: R$ 20.000,00 → 5,0 meses"),
            "{text}"
        );
    }

    #[test]
    fn nothing_essential_points_to_the_setup() {
        let text = living_cost_text(&cost(Vec::new(), None));
        assert!(text.contains("Marque as categorias em /essenciais."), "{text}");
    }

    fn projection(result_positive: bool) -> CycleProjection {
        let today = NaiveDate::from_ymd_opt(2026, 9, 22).unwrap();
        let income = if result_positive { 1_200_000 } else { 300_000 };
        CycleProjection {
            cycle: Cycle::containing(today, DayOfMonth::new(5).unwrap()),
            days_left: 13,
            income: Flow { recorded: Cents::new(income), coming: Cents::new(400_000) },
            spending: Flow { recorded: Cents::new(407_923), coming: Cents::new(313_890) },
            by_category: vec![(category("casa", CategoryKind::Expense, true), Cents::new(328_218))],
            available: Cents::new(540_384),
            due_from_accounts: Cents::new(313_890),
            bills_to_confirm: 1,
        }
    }

    #[test]
    fn projection_shows_result_cash_and_warnings() {
        let text = projection_text(&projection(true));
        assert!(
            text.starts_with("<b>🔮 Projeção do ciclo</b> · 05/09 → 04/10/2026 (faltam 13 dias)"),
            "{text}"
        );
        assert!(
            text.contains(
                "Entradas: R$ 16.000,00 (R$ 12.000,00 recebidas · R$ 4.000,00 a receber)"
            ),
            "{text}"
        );
        assert!(
            text.contains("Gastos: R$ 7.218,13 (R$ 4.079,23 lançados · R$ 3.138,90 a lançar)"),
            "{text}"
        );
        assert!(text.contains("<b>Sobra prevista: R$ 8.781,87</b> (54% das entradas)"), "{text}");
        assert!(text.contains("Disponível hoje: R$ 5.403,84\n+ a receber: R$ 4.000,00"), "{text}");
        assert!(text.contains("<b>= no fim do ciclo: R$ 6.264,94</b>"), "{text}");
        assert!(text.contains("<b>Maiores gastos previstos</b>\ncasa: R$ 3.282,18"), "{text}");
        assert!(text.contains("⚠️ 1 recorrente(s) esperando confirmação"), "{text}");
    }

    #[test]
    fn a_cycle_that_ends_short_says_falta() {
        let text = projection_text(&projection(false));
        assert!(text.contains("<b>Falta: R$ 218,13</b>"), "{text}");
        assert!(!text.contains("das entradas"), "no negative share: {text}");
    }
}
