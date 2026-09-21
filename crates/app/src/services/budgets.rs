//! Monthly limits per expense category, measured over the household cycle,
//! with an alert the first time 80% and 100% are reached in a cycle.

use std::collections::HashMap;
use std::sync::Arc;

use domain::Cents;
use domain::budget::{thresholds_reached, used_bp};
use domain::cycle::Cycle;
use domain::spend::{CategoryFlow, spending_by_category};

use super::{CategoryService, SettingsService};
use crate::model::{Budget, BudgetAlert, BudgetStatus, Category, CategoryId, CategoryKind};
use crate::ports::{BudgetStore, Clock, ReportStore};
use crate::{AppError, AppResult};

pub struct BudgetService {
    store: Arc<dyn BudgetStore>,
    reports: Arc<dyn ReportStore>,
    categories: Arc<CategoryService>,
    settings: Arc<SettingsService>,
    clock: Arc<dyn Clock>,
}

/// What `BudgetService::new` reads from besides its store and clock.
pub struct BudgetSources {
    pub reports: Arc<dyn ReportStore>,
    pub categories: Arc<CategoryService>,
    pub settings: Arc<SettingsService>,
}

impl BudgetService {
    pub fn new(store: Arc<dyn BudgetStore>, sources: BudgetSources, clock: Arc<dyn Clock>) -> Self {
        let BudgetSources { reports, categories, settings } = sources;
        Self { store, reports, categories, settings, clock }
    }

    /// Sets the monthly limit of an expense category.
    ///
    /// ```ignore
    /// budgets.set(groceries, Cents::new(150_000)).await?;
    /// ```
    pub async fn set(&self, category: CategoryId, limit: Cents) -> AppResult<Budget> {
        if !limit.is_positive() {
            return Err(AppError::invalid(
                "budget limit",
                limit.value(),
                "a positive number of cents",
            ));
        }
        self.categories.require_kind(category, CategoryKind::Expense).await?;
        Ok(self.store.set_budget(category, limit).await?)
    }

    pub async fn remove(&self, category: CategoryId) -> AppResult<()> {
        if !self.store.remove_budget(category).await? {
            return Err(AppError::not_found("budget for category", category));
        }
        Ok(())
    }

    /// Every budget against spending in `cycle`, most used first.
    pub async fn statuses(&self, cycle: Cycle) -> AppResult<Vec<BudgetStatus>> {
        let budgets = self.store.list_budgets().await?;
        let spent = self.spent_by_category(cycle).await?;
        let categories = self.categories.list(Some(CategoryKind::Expense)).await?;
        let mut statuses: Vec<BudgetStatus> = budgets
            .into_iter()
            .filter_map(|budget| status_of(budget, &spent, &categories))
            .collect();
        statuses.sort_by_key(|status| std::cmp::Reverse(status.used_bp));
        Ok(statuses)
    }

    pub async fn current_cycle(&self) -> AppResult<Cycle> {
        let settings = self.settings.get().await?;
        Ok(Cycle::containing(self.clock.today(), settings.cycle_start_day))
    }

    /// Thresholds crossed for the first time in the current cycle; each
    /// is returned once ever. A jump past both reports only 100%.
    pub async fn new_alerts(&self) -> AppResult<Vec<BudgetAlert>> {
        let cycle = self.current_cycle().await?;
        let mut alerts = Vec::new();
        for status in self.statuses(cycle).await? {
            let mut newest = None;
            for threshold in thresholds_reached(status.spent, status.budget.limit) {
                if self.store.claim_budget_alert(status.budget.id, cycle.start, threshold).await? {
                    newest = Some(threshold);
                }
            }
            alerts
                .extend(newest.map(|threshold| BudgetAlert { status: status.clone(), threshold }));
        }
        Ok(alerts)
    }

    async fn spent_by_category(&self, cycle: Cycle) -> AppResult<HashMap<CategoryId, Cents>> {
        let flows = self.reports.period_flows(cycle.start, cycle.end_exclusive).await?;
        let flows: Vec<_> = flows
            .iter()
            .map(|flow| CategoryFlow {
                category: flow.category_id,
                kind: flow.kind,
                total: flow.total,
            })
            .collect();
        let spent = spending_by_category(&flows).into_iter();
        Ok(spent.filter_map(|(category, total)| category.map(|id| (id, total))).collect())
    }
}

fn status_of(
    budget: Budget,
    spent: &HashMap<CategoryId, Cents>,
    categories: &[Category],
) -> Option<BudgetStatus> {
    let category = categories.iter().find(|category| category.id == budget.category_id)?.clone();
    let spent = spent.get(&budget.category_id).copied().unwrap_or(Cents::ZERO);
    Some(BudgetStatus { used_bp: used_bp(spent, budget.limit), budget, category, spent })
}
