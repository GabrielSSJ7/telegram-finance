use app::model::{
    AccountId, CardId, CategoryId, NewRecurrence, Recurrence, RecurrenceId, RecurrenceKind,
    RecurrenceMode, RecurrenceTarget,
};
use app::ports::{RecurrenceStore, StoreResult};
use async_trait::async_trait;
use chrono::NaiveDate;
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
    async fn create_recurrence(&self, recurrence: NewRecurrence) -> StoreResult<Recurrence> {
        let (account_id, card_id) = target_ids(recurrence.target);
        let row = sqlx::query_file_as!(
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
        )
        .fetch_one(self.pool())
        .await
        .map_err(store_error)?;
        row.into_recurrence()
    }

    async fn list_recurrences(&self, include_inactive: bool) -> StoreResult<Vec<Recurrence>> {
        let rows = sqlx::query_as!(
            RecurrenceRow,
            "select id, kind, amount_cents, description, category_id, account_id, card_id, day_of_month, mode,
                    active, starts_on, last_generated_on
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
                    active, starts_on, last_generated_on
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
