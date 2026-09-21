use async_trait::async_trait;
use chrono::NaiveDate;

use super::StoreResult;
use crate::model::{NewRecurrence, Recurrence, RecurrenceId};

#[async_trait]
pub trait RecurrenceStore: Send + Sync {
    async fn create_recurrence(&self, recurrence: NewRecurrence) -> StoreResult<Recurrence>;
    async fn list_recurrences(&self, include_inactive: bool) -> StoreResult<Vec<Recurrence>>;
    async fn find_recurrence(&self, id: RecurrenceId) -> StoreResult<Option<Recurrence>>;
    /// Returns false when it does not exist or is already inactive.
    async fn deactivate_recurrence(&self, id: RecurrenceId) -> StoreResult<bool>;
    /// Moves `last_generated_on` forward to `date` (never backwards).
    async fn mark_generated(&self, id: RecurrenceId, date: NaiveDate) -> StoreResult<()>;
}
