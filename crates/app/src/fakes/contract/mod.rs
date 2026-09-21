// Contract cases are test code shipped behind `test-support`: panicking on
// a failed expectation is how they report.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::missing_panics_doc)]

//! Store contract: behaviour every adapter of the store ports must show.
//! The in-memory fake and the Postgres store both run these cases, so the
//! fake the service tests rely on cannot drift from the real database.
//!
//! Adapters generate one test per case with [`store_contract_cases!`].
//! Cases never assume an empty store (Postgres seeds categories).

mod accounts;
mod api_keys;
mod budgets;
mod cards;
mod categories;
mod chat_flows;
mod entries;
mod goals;
mod members;
mod scheduling;
mod settings;

#[cfg(test)]
mod memory_cases;

pub use accounts::*;
pub use api_keys::*;
pub use budgets::*;
pub use cards::*;
pub use categories::*;
pub use chat_flows::*;
pub use entries::*;
pub use goals::*;
pub use members::*;
pub use scheduling::*;
pub use settings::*;

use chrono::NaiveDate;
use domain::{AccountKind, Cents};

use crate::model::{
    Account, Category, CategoryKind, Member, MemberProfile, NewAccount, NewCategory,
};
use crate::services::StorePorts;

/// Invokes `$case!(name)` for every contract case.
#[macro_export]
macro_rules! store_contract_cases {
    ($case:ident) => {
        $case!(account_create_find_list_archive);
        $case!(account_active_names_are_unique);
        $case!(account_flows_follow_entries);
        $case!(category_create_find_archive);
        $case!(category_names_unique_per_kind);
        $case!(entry_record_and_find);
        $case!(entry_duplicate_draft_is_rejected);
        $case!(entry_list_filters_by_range_and_kind);
        $case!(entry_list_filters_by_account_category_author);
        $case!(entry_list_orders_newest_first_with_limit);
        $case!(entry_update_and_soft_delete);
        $case!(entry_latest_by_member);
        $case!(goal_create_and_find);
        $case!(goal_update_and_hide_when_archived);
        $case!(member_upsert_find_list_dm);
        $case!(settings_update_and_bind_chat);
        $case!(api_key_create_find_revoke);
        $case!(chat_flow_save_load_replace_clear);
        $case!(chat_flow_expired_is_absent);
        $case!(bot_offset_round_trip);
        $case!(card_create_find_list_unique);
        $case!(card_archive_hides_card);
        $case!(card_invoice_ensure_keeps_first_dates);
        $case!(card_purchase_spreads_installments);
        $case!(card_purchase_delete_removes_installments);
        $case!(card_invoice_entries_count_as_credits_and_payments);
        $case!(recurrence_create_find_list_deactivate);
        $case!(recurrence_mark_generated_only_moves_forward);
        $case!(job_runs_once_per_day);
        $case!(job_failures_retry_until_max_attempts);
        $case!(job_stale_run_is_reclaimed);
        $case!(report_flows_group_by_category_author_and_kind);
        $case!(report_pot_net_inflow_counts_both_directions);
        $case!(budget_set_replace_list_remove);
        $case!(budget_alert_claimed_once_per_cycle_and_threshold);
    };
}

pub(crate) fn day(month: u32, value: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, month, value).unwrap_or_default()
}

pub(crate) async fn open_account(
    stores: &StorePorts,
    name: &str,
    kind: AccountKind,
    initial: i64,
) -> Account {
    let account = NewAccount {
        name: name.into(),
        kind,
        initial_balance: Cents::new(initial),
        opened_on: day(1, 1),
    };
    stores.accounts.create_account(account).await.expect("create account")
}

pub(crate) async fn new_category(stores: &StorePorts, name: &str, kind: CategoryKind) -> Category {
    let category = NewCategory { name: name.into(), kind, emoji: None };
    stores.categories.create_category(category).await.expect("create category")
}

pub(crate) async fn member(stores: &StorePorts, telegram_user_id: i64) -> Member {
    let profile =
        MemberProfile { telegram_user_id, display_name: format!("user {telegram_user_id}") };
    stores.members.upsert_member(profile).await.expect("upsert member")
}
