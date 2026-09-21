use app::model::DraftId;
use app::ports::{StoreError, StoreResult};
use sqlx::PgConnection;

use crate::error_mapping::store_error;

/// Claims `draft` inside the caller's transaction. A concurrent claim of
/// the same draft waits on the primary key and then inserts nothing.
pub(crate) async fn commit_draft(connection: &mut PgConnection, draft: DraftId) -> StoreResult<()> {
    let inserted = sqlx::query!(
        "insert into committed_drafts (draft_id) values ($1) on conflict do nothing",
        draft.0
    )
    .execute(connection)
    .await
    .map_err(store_error)?;
    if inserted.rows_affected() == 0 {
        return Err(StoreError::DuplicateDraft);
    }
    Ok(())
}
