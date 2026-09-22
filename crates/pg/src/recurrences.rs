use app::model::{
    AccountId, CardId, CategoryId, NewRecurrence, Recurrence, RecurrenceId, RecurrenceKind,
    RecurrenceMode, RecurrenceTarget,
};
use app::ports::{RecurrenceStore, StoreResult};
use async_trait::async_trait;
use chrono::NaiveDate;
use domain::recurrence::InstallmentPlan;
use domain::{Cents, DayOfMonth};
use uuid::Uuid;

use crate::PgStore;
use crate::error_mapping::{corrupt, store_error};

struct RecurrenceRow {
    id: Uuid,
    kind: String,
    amount_cents: i64,
    description: String,
    category_id: Option<Uuid>,
    account_id: Option<Uuid>,
    card_id: Option<Uuid>,
    day_of_month: i16,
    mode: String,
    active: bool,
    starts_on: NaiveDate,
    last_generated_on: Option<NaiveDate>,
    installment_count: Option<i16>,
    first_installment_no: Option<i16>,
}

struct CheckedColumns {
    kind: RecurrenceKind,
    mode: RecurrenceMode,
    day: DayOfMonth,
    category_id: CategoryId,
}

impl RecurrenceRow {
    fn into_recurrence(self) -> StoreResult<Recurrence> {
        let checked = self.checked()?;
        Ok(Recurrence {
            id: RecurrenceId(self.id),
            kind: checked.kind,
            amount: Cents::new(self.amount_cents),
            description: self.description,
            category_id: checked.category_id,
            target: target(self.account_id, self.card_id)?,
            day: checked.day,
            mode: checked.mode,
            active: self.active,
            starts_on: self.starts_on,
            last_generated_on: self.last_generated_on,
            plan: plan(self.first_installment_no, self.installment_count)?,
        })
    }

    /// Columns the database stores loosely (text, smallint, nullable).
    fn checked(&self) -> StoreResult<CheckedColumns> {
        let kind = self.kind.parse().map_err(|error| corrupt("recurrences.kind", error))?;
        let mode = self.mode.parse().map_err(|error| corrupt("recurrences.mode", error))?;
        let day = DayOfMonth::try_from(self.day_of_month)
            .map_err(|error| corrupt("recurrences.day_of_month", error))?;
        let category =
            self.category_id.ok_or_else(|| corrupt("recurrences.category_id", "null"))?;
        Ok(CheckedColumns { kind, mode, day, category_id: CategoryId(category) })
    }
}

/// Both columns set, or neither (the table checks it too).
fn plan(first: Option<i16>, count: Option<i16>) -> StoreResult<Option<InstallmentPlan>> {
    let number = |value: i16| {
        u32::try_from(value).map_err(|error| corrupt("recurrences.installments", error))
    };
    match (first, count) {
        (Some(first), Some(count)) => {
            Ok(Some(InstallmentPlan { first_number: number(first)?, count: number(count)? }))
        }
        (None, None) => Ok(None),
        _ => Err(corrupt(
            "recurrences.installment_count/first_installment_no",
            "expected both or neither",
        )),
    }
}

/// The nullable columns of a new recurrence: account or card, and the plan.
fn optional_columns(
    recurrence: &NewRecurrence,
) -> (Option<Uuid>, Option<Uuid>, Option<i16>, Option<i16>) {
    let (account_id, card_id) = target_ids(recurrence.target);
    let (count, first) = plan_columns(recurrence.plan);
    (account_id, card_id, count, first)
}

fn plan_columns(plan: Option<InstallmentPlan>) -> (Option<i16>, Option<i16>) {
    let small = |value: u32| i16::try_from(value).unwrap_or(i16::MAX);
    plan.map_or((None, None), |plan| (Some(small(plan.count)), Some(small(plan.first_number))))
}

fn target(account: Option<Uuid>, card: Option<Uuid>) -> StoreResult<RecurrenceTarget> {
    match (account, card) {
        (Some(account), None) => Ok(RecurrenceTarget::Account(AccountId(account))),
        (None, Some(card)) => Ok(RecurrenceTarget::Card(CardId(card))),
        _ => Err(corrupt("recurrences.account_id/card_id", "expected exactly one")),
    }
}

fn target_ids(target: RecurrenceTarget) -> (Option<Uuid>, Option<Uuid>) {
    match target {
        RecurrenceTarget::Account(id) => (Some(id.0), None),
        RecurrenceTarget::Card(id) => (None, Some(id.0)),
    }
}

#[async_trait]
impl RecurrenceStore for PgStore {
    // One insert with eleven bind parameters, which the formatter puts one
    // per line; splitting it would only scatter a single statement.
    #[allow(clippy::too_many_lines)]
    async fn create_recurrence(&self, recurrence: NewRecurrence) -> StoreResult<Recurrence> {
        let (account_id, card_id, installment_count, first_installment_no) =
            optional_columns(&recurrence);
        sqlx::query_file_as!(
            RecurrenceRow,
            "queries/insert_recurrence.sql",
            recurrence.kind.as_str(),
            recurrence.amount.value(),
            recurrence.description,
            recurrence.category_id.0,
            account_id,
            card_id,
            i16::from(recurrence.day),
            recurrence.mode.as_str(),
            recurrence.starts_on,
            installment_count,
            first_installment_no,
        )
        .fetch_one(self.pool())
        .await
        .map_err(store_error)?
        .into_recurrence()
    }

    async fn list_recurrences(&self, include_inactive: bool) -> StoreResult<Vec<Recurrence>> {
        let rows = sqlx::query_as!(
            RecurrenceRow,
            "select id, kind, amount_cents, description, category_id, account_id, card_id, day_of_month, mode,
                    active, starts_on, last_generated_on, installment_count, first_installment_no
             from recurrences where $1 or active order by day_of_month, created_at",
            include_inactive,
        )
        .fetch_all(self.pool())
        .await
        .map_err(store_error)?;
        rows.into_iter().map(RecurrenceRow::into_recurrence).collect()
    }

    async fn find_recurrence(&self, id: RecurrenceId) -> StoreResult<Option<Recurrence>> {
        let row = sqlx::query_as!(
            RecurrenceRow,
            "select id, kind, amount_cents, description, category_id, account_id, card_id, day_of_month, mode,
                    active, starts_on, last_generated_on, installment_count, first_installment_no
             from recurrences where id = $1",
            id.0,
        )
        .fetch_optional(self.pool())
        .await
        .map_err(store_error)?;
        row.map(RecurrenceRow::into_recurrence).transpose()
    }

    async fn deactivate_recurrence(&self, id: RecurrenceId) -> StoreResult<bool> {
        let result =
            sqlx::query!("update recurrences set active = false where id = $1 and active", id.0)
                .execute(self.pool())
                .await
                .map_err(store_error)?;
        Ok(result.rows_affected() == 1)
    }

    async fn mark_generated(&self, id: RecurrenceId, date: NaiveDate) -> StoreResult<()> {
        sqlx::query!(
            "update recurrences set last_generated_on = greatest(coalesce(last_generated_on, $2), $2) where id = $1",
            id.0,
            date,
        )
        .execute(self.pool())
        .await
        .map_err(store_error)?;
        Ok(())
    }
}
