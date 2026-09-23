use async_trait::async_trait;
use chrono::NaiveDate;

use super::InMemoryStore;
use crate::model::{NewRecurrence, Recurrence, RecurrenceEdit, RecurrenceId};
use crate::ports::{RecurrenceStore, StoreResult};

#[async_trait]
impl RecurrenceStore for InMemoryStore {
    async fn create_recurrence(&self, recurrence: NewRecurrence) -> StoreResult<Recurrence> {
        let created = Recurrence {
            id: RecurrenceId::generate(),
            kind: recurrence.kind,
            amount: recurrence.amount,
            description: recurrence.description,
            category_id: recurrence.category_id,
            target: recurrence.target,
            day: recurrence.day,
            mode: recurrence.mode,
            active: true,
            starts_on: recurrence.starts_on,
            last_generated_on: None,
            plan: recurrence.plan,
        };
        self.lock().recurrences.push(created.clone());
        Ok(created)
    }

    async fn list_recurrences(&self, include_inactive: bool) -> StoreResult<Vec<Recurrence>> {
        let state = self.lock();
        Ok(state.recurrences.iter().filter(|row| include_inactive || row.active).cloned().collect())
    }

    async fn find_recurrence(&self, id: RecurrenceId) -> StoreResult<Option<Recurrence>> {
        Ok(self.lock().recurrences.iter().find(|row| row.id == id).cloned())
    }

    async fn update_recurrence(
        &self,
        id: RecurrenceId,
        edit: RecurrenceEdit,
    ) -> StoreResult<Option<Recurrence>> {
        let mut state = self.lock();
        let Some(row) = state.recurrences.iter_mut().find(|row| row.id == id && row.active) else {
            return Ok(None);
        };
        if let Some(amount) = edit.amount {
            row.amount = amount;
        }
        if let Some(day) = edit.day {
            row.day = day;
        }
        if let Some(mode) = edit.mode {
            row.mode = mode;
        }
        Ok(Some(row.clone()))
    }

    async fn deactivate_recurrence(&self, id: RecurrenceId) -> StoreResult<bool> {
        let mut state = self.lock();
        let Some(row) = state.recurrences.iter_mut().find(|row| row.id == id && row.active) else {
            return Ok(false);
        };
        row.active = false;
        Ok(true)
    }

    async fn mark_generated(&self, id: RecurrenceId, date: NaiveDate) -> StoreResult<()> {
        let mut state = self.lock();
        if let Some(row) = state.recurrences.iter_mut().find(|row| row.id == id) {
            row.last_generated_on = Some(row.last_generated_on.map_or(date, |last| last.max(date)));
        }
        Ok(())
    }
}
