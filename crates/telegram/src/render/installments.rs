//! `/parcelas`: card purchases and financings paid in installments.

use app::model::{Category, InstallmentProgress, PlanSource};
use domain::Cents;
use domain::money_format::format_brl;

use super::catalog::category_label;
use super::reports::progress_bar;
use crate::html::escape;

pub fn installments_text(plans: &[InstallmentProgress], categories: &[Category]) -> String {
    if plans.is_empty() {
        return "<b>💳 Parcelamentos</b>\nNenhum parcelamento em andamento.".into();
    }
    let left: Cents = plans.iter().map(InstallmentProgress::remaining).sum();
    let monthly: Cents = plans.iter().map(|plan| plan.installment).sum();
    let header = format!(
        "<b>💳 Parcelamentos</b>\nFalta pagar: {} · por mês: {}",
        format_brl(left),
        format_brl(monthly)
    );
    let blocks: Vec<String> = plans.iter().map(|plan| plan_block(plan, categories)).collect();
    format!("{header}\n\n{}", blocks.join("\n\n"))
}

/// Four lines per plan, in the style of `/metas`.
fn plan_block(plan: &InstallmentProgress, categories: &[Category]) -> String {
    let progress = format!(
        "{} de {} ({}%) · parcela {} de {}",
        format_brl(plan.paid),
        format_brl(plan.total),
        plan.paid_bp() / 100,
        plan.paid_count,
        plan.count
    );
    let left = format!("{} faltam {}", progress_bar(plan.paid_bp()), format_brl(plan.remaining()));
    let pace =
        format!("{}/mês até {}", format_brl(plan.installment), plan.last_due.format("%m/%Y"));
    format!("{}\n{progress}\n{left}\n{pace}", plan_title(plan, categories))
}

/// `<b>Enoxaparina</b> · 💳 Itau Black · 💊 saúde`.
fn plan_title(plan: &InstallmentProgress, categories: &[Category]) -> String {
    let source = match &plan.source {
        PlanSource::Card(name) => format!("💳 {}", escape(name)),
        PlanSource::Account(name) => format!("🏦 {}", escape(name)),
    };
    let category = categories.iter().find(|category| category.id == plan.category_id);
    let category =
        category.map_or_else(String::new, |category| format!(" · {}", category_label(category)));
    let title = if plan.description.is_empty() {
        "Compra parcelada".to_owned()
    } else {
        escape(&plan.description)
    };
    format!("<b>{title}</b> · {source}{category}")
}

#[cfg(test)]
mod tests {
    use app::model::CategoryId;
    use chrono::NaiveDate;

    use super::*;

    fn plan(description: &str, source: PlanSource, paid: i64, total: i64) -> InstallmentProgress {
        InstallmentProgress {
            description: description.into(),
            source,
            category_id: CategoryId::generate(),
            count: 4,
            paid_count: 1,
            total: Cents::new(total),
            paid: Cents::new(paid),
            installment: Cents::new(total / 4),
            last_due: NaiveDate::from_ymd_opt(2026, 12, 21).unwrap(),
        }
    }

    #[test]
    fn plans_with_totals_like_goals() {
        let plans = [
            plan("Enoxaparina", PlanSource::Card("Itau Black".into()), 102_675, 410_700),
            plan("", PlanSource::Account("BTG".into()), 0, 400_000),
        ];
        let text = installments_text(&plans, &[]);
        assert!(
            text.starts_with(
                "<b>💳 Parcelamentos</b>\nFalta pagar: R$ 7.080,25 · por mês: R$ 2.026,75"
            ),
            "{text}"
        );
        assert!(text.contains("<b>Enoxaparina</b> · 💳 Itau Black\nR$ 1.026,75 de R$ 4.107,00 (25%) · parcela 1 de 4"), "{text}");
        assert!(
            text.contains("▓▓░░░░░░░░ faltam R$ 3.080,25\nR$ 1.026,75/mês até 12/2026"),
            "{text}"
        );
        assert!(text.contains("<b>Compra parcelada</b> · 🏦 BTG"), "{text}");
    }

    #[test]
    fn no_plans() {
        assert!(installments_text(&[], &[]).contains("Nenhum parcelamento em andamento."));
    }
}
