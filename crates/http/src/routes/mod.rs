//! Handlers, one module per resource. Each handler converts the wire type,
//! calls one service method and converts the result back.

pub mod accounts;
pub mod budgets;
pub mod cards;
pub mod categories;
pub mod entries;
pub mod exports;
pub mod goals;
pub mod installments;
pub mod members;
pub mod recurrences;
pub mod reports;
pub mod settings;

use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::state::ApiState;

/// Every `/api/v1` route with its API documentation.
pub fn v1_routes() -> OpenApiRouter<ApiState> {
    ledger_routes().merge(card_routes()).merge(schedule_routes())
}

fn ledger_routes() -> OpenApiRouter<ApiState> {
    OpenApiRouter::new()
        .routes(routes!(accounts::list_accounts, accounts::open_account))
        .routes(routes!(accounts::archive_account))
        .routes(routes!(accounts::balance_sheet))
        .routes(routes!(accounts::reconcile_account))
        .routes(routes!(categories::list_categories, categories::create_category))
        .routes(routes!(categories::archive_category, categories::update_category))
        .routes(routes!(entries::list_entries, entries::create_entry))
        .routes(routes!(entries::get_entry, entries::update_entry, entries::delete_entry))
        .routes(routes!(goals::list_goals, goals::create_goal))
        .routes(routes!(goals::update_goal_target))
        .routes(routes!(goals::deposit_to_goal))
        .routes(routes!(goals::withdraw_from_goal))
        .routes(routes!(settings::get_settings, settings::update_settings))
        .routes(routes!(members::list_members))
}

fn card_routes() -> OpenApiRouter<ApiState> {
    OpenApiRouter::new()
        .routes(routes!(cards::list_cards, cards::open_card))
        .routes(routes!(cards::archive_card))
        .routes(routes!(cards::card_summaries))
        .routes(routes!(cards::card_invoices))
        .routes(routes!(cards::create_purchase))
        .routes(routes!(cards::delete_purchase))
        .routes(routes!(cards::create_credit))
        .routes(routes!(cards::pay_invoice))
}

fn schedule_routes() -> OpenApiRouter<ApiState> {
    OpenApiRouter::new()
        .routes(routes!(recurrences::list_recurrences, recurrences::create_recurrence))
        .routes(routes!(recurrences::deactivate_recurrence))
        .routes(routes!(reports::daily_report))
        .routes(routes!(reports::cycle_report))
        .routes(routes!(reports::living_cost))
        .routes(routes!(reports::projection))
        .routes(routes!(installments::list_installment_plans))
        .routes(routes!(budgets::list_budgets))
        .routes(routes!(budgets::set_budget, budgets::remove_budget))
        .routes(routes!(exports::export_entries))
}
