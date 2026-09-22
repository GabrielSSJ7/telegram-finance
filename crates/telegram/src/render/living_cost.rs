//! `/essenciais` and `/custodevida`: which categories make up the basic
//! cost of living, and what it adds up to.

use app::model::{Category, CategoryKind, LivingCost, RESERVE_MONTHS};
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
}
