//! Records the services read and write. Rules live in the `domain` crate;
//! these are plain data with typed ids.

pub mod account;
pub mod api_key;
pub mod budget;
pub mod card;
pub mod category;
pub mod entry;
pub mod goal;
pub mod ids;
pub mod installment_plan;
pub mod living_cost;
pub mod member;
pub mod projection;
pub mod recurrence;
pub mod report;
pub mod settings;

pub use account::{Account, AccountBalance, BalanceSheet, NewAccount};
pub use api_key::{ApiKey, IssuedApiKey};
pub use budget::{Budget, BudgetAlert, BudgetStatus};
pub use card::{
    CardEdit, CardPurchase, CardSummary, CreditCard, Invoice, InvoiceView, NewCard, NewCardPurchase,
};
pub use category::{Category, CategoryEdit, CategoryKind, EmojiChange, NewCategory};
pub use entry::{EntryFilter, EntryPatch, LedgerEntry, NewEntry};
pub use goal::{Goal, GoalProgress, GoalTarget};
pub use ids::{
    AccountId, ApiKeyId, BudgetId, CardId, CategoryId, DraftId, EntryId, GoalId, InvoiceId,
    MemberId, PurchaseId, RecurrenceId,
};
pub use installment_plan::{InstallmentProgress, PlanSource};
pub use living_cost::{LivingCost, RESERVE_MONTHS};
pub use member::{Member, MemberProfile};
pub use projection::{CycleProjection, Flow};
pub use recurrence::{
    NewRecurrence, Recurrence, RecurrenceEdit, RecurrenceKind, RecurrenceMode, RecurrenceTarget,
};
pub use report::{CycleReport, DailyReport, PeriodFlow, PeriodTotals, ReportDay};
pub use settings::{HouseholdSettings, SettingsPatch};
