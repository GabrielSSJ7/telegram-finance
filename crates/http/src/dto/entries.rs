use app::model::{AccountId, CategoryId, EntryFilter, EntryPatch, LedgerEntry};
use app::services::ledger::{AccountEntry, AdjustmentEntry, EntryRequest, TransferEntry};
use chrono::{DateTime, NaiveDate, Utc};
use domain::{Cents, EntryKind};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

pub const MAX_LIST_LIMIT: u32 = 500;

#[derive(Debug, Serialize, ToSchema)]
pub struct EntryResponse {
    pub id: Uuid,
    #[schema(value_type = String, example = "expense")]
    pub kind: EntryKind,
    #[schema(example = 1050)]
    pub amount_cents: i64,
    #[schema(example = "mercado")]
    pub description: String,
    pub category_id: Option<Uuid>,
    pub account_id: Option<Uuid>,
    pub counter_account_id: Option<Uuid>,
    pub card_purchase_id: Option<Uuid>,
    pub installment_no: Option<u32>,
    pub invoice_id: Option<Uuid>,
    pub accounting_date: NaiveDate,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

impl From<LedgerEntry> for EntryResponse {
    fn from(entry: LedgerEntry) -> Self {
        Self {
            id: entry.id.0,
            kind: entry.kind,
            amount_cents: entry.amount.value(),
            description: entry.description,
            category_id: entry.category_id.map(|id| id.0),
            account_id: entry.account_id.map(|id| id.0),
            counter_account_id: entry.counter_account_id.map(|id| id.0),
            card_purchase_id: entry.card_purchase_id.map(|id| id.0),
            installment_no: entry.installment_no,
            invoice_id: entry.invoice_id.map(|id| id.0),
            accounting_date: entry.accounting_date,
            created_by: entry.created_by.map(|id| id.0),
            created_at: entry.created_at,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AccountEntryBody {
    pub account_id: Uuid,
    pub category_id: Uuid,
    #[schema(example = 1050)]
    pub amount_cents: i64,
    #[serde(default)]
    pub description: String,
    /// Defaults to today in the household timezone.
    pub date: Option<NaiveDate>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct TransferBody {
    pub from_account_id: Uuid,
    pub to_account_id: Uuid,
    pub amount_cents: i64,
    #[serde(default)]
    pub description: String,
    pub date: Option<NaiveDate>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AdjustmentBody {
    pub account_id: Uuid,
    pub amount_cents: i64,
    #[serde(default)]
    pub description: String,
    pub date: Option<NaiveDate>,
}

/// New ledger entry; `kind` selects the shape.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CreateEntryBody {
    Income(AccountEntryBody),
    Expense(AccountEntryBody),
    Refund(AccountEntryBody),
    Transfer(TransferBody),
    AdjustIn(AdjustmentBody),
    AdjustOut(AdjustmentBody),
}

impl From<AccountEntryBody> for AccountEntry {
    fn from(body: AccountEntryBody) -> Self {
        AccountEntry {
            account_id: AccountId(body.account_id),
            category_id: CategoryId(body.category_id),
            amount: Cents::new(body.amount_cents),
            description: body.description,
            date: body.date,
        }
    }
}

impl From<TransferBody> for TransferEntry {
    fn from(body: TransferBody) -> Self {
        TransferEntry {
            from_account_id: AccountId(body.from_account_id),
            to_account_id: AccountId(body.to_account_id),
            amount: Cents::new(body.amount_cents),
            description: body.description,
            date: body.date,
        }
    }
}

impl From<AdjustmentBody> for AdjustmentEntry {
    fn from(body: AdjustmentBody) -> Self {
        let amount = Cents::new(body.amount_cents);
        AdjustmentEntry {
            account_id: AccountId(body.account_id),
            amount,
            description: body.description,
            date: body.date,
        }
    }
}

impl From<CreateEntryBody> for EntryRequest {
    fn from(body: CreateEntryBody) -> Self {
        match body {
            CreateEntryBody::Income(entry) => EntryRequest::Income(entry.into()),
            CreateEntryBody::Expense(entry) => EntryRequest::Expense(entry.into()),
            CreateEntryBody::Refund(entry) => EntryRequest::Refund(entry.into()),
            CreateEntryBody::Transfer(entry) => EntryRequest::Transfer(entry.into()),
            CreateEntryBody::AdjustIn(entry) => EntryRequest::AdjustIn(entry.into()),
            CreateEntryBody::AdjustOut(entry) => EntryRequest::AdjustOut(entry.into()),
        }
    }
}

/// Fields to change; omitted fields stay as they are.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateEntryBody {
    pub amount_cents: Option<i64>,
    pub description: Option<String>,
    pub category_id: Option<Uuid>,
    pub accounting_date: Option<NaiveDate>,
}

impl From<UpdateEntryBody> for EntryPatch {
    fn from(body: UpdateEntryBody) -> Self {
        EntryPatch {
            amount: body.amount_cents.map(Cents::new),
            description: body.description,
            category_id: body.category_id.map(CategoryId),
            accounting_date: body.accounting_date,
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ListEntriesQuery {
    pub from: Option<NaiveDate>,
    /// Inclusive.
    pub to: Option<NaiveDate>,
    #[param(value_type = Option<String>)]
    pub kind: Option<EntryKind>,
    /// Matches source or destination account.
    pub account_id: Option<Uuid>,
    pub category_id: Option<Uuid>,
    /// 1 to 500, default 100.
    pub limit: Option<u32>,
}

impl From<ListEntriesQuery> for EntryFilter {
    fn from(query: ListEntriesQuery) -> Self {
        EntryFilter {
            from: query.from,
            to_inclusive: query.to,
            kind: query.kind,
            account_id: query.account_id.map(AccountId),
            category_id: query.category_id.map(CategoryId),
            created_by: None,
            limit: query.limit.unwrap_or(100).clamp(1, MAX_LIST_LIMIT),
        }
    }
}
