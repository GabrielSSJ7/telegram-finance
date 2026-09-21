use app::model::{Member, MemberId, MemberProfile};
use app::ports::{MemberStore, StoreResult};
use async_trait::async_trait;
use uuid::Uuid;

use crate::PgStore;
use crate::error_mapping::store_error;

struct MemberRow {
    id: Uuid,
    telegram_user_id: i64,
    display_name: String,
    dm_chat_id: Option<i64>,
    receives_backups: bool,
}

impl From<MemberRow> for Member {
    fn from(row: MemberRow) -> Self {
        Member {
            id: MemberId(row.id),
            telegram_user_id: row.telegram_user_id,
            display_name: row.display_name,
            dm_chat_id: row.dm_chat_id,
            receives_backups: row.receives_backups,
        }
    }
}

#[async_trait]
impl MemberStore for PgStore {
    async fn upsert_member(&self, profile: MemberProfile) -> StoreResult<Member> {
        let row = sqlx::query_as!(
            MemberRow,
            "insert into members (telegram_user_id, display_name) values ($1, $2)
             on conflict (telegram_user_id) do update set display_name = excluded.display_name
             returning id, telegram_user_id, display_name, dm_chat_id, receives_backups",
            profile.telegram_user_id,
            profile.display_name,
        )
        .fetch_one(self.pool())
        .await
        .map_err(store_error)?;
        Ok(row.into())
    }

    async fn find_member_by_telegram(&self, telegram_user_id: i64) -> StoreResult<Option<Member>> {
        let row = sqlx::query_as!(
            MemberRow,
            "select id, telegram_user_id, display_name, dm_chat_id, receives_backups
             from members where telegram_user_id = $1",
            telegram_user_id,
        )
        .fetch_optional(self.pool())
        .await
        .map_err(store_error)?;
        Ok(row.map(Member::from))
    }

    async fn list_members(&self) -> StoreResult<Vec<Member>> {
        let rows = sqlx::query_as!(
            MemberRow,
            "select id, telegram_user_id, display_name, dm_chat_id, receives_backups from members order by created_at",
        )
        .fetch_all(self.pool())
        .await
        .map_err(store_error)?;
        Ok(rows.into_iter().map(Member::from).collect())
    }

    async fn set_dm_chat(&self, id: MemberId, dm_chat_id: i64) -> StoreResult<bool> {
        let result =
            sqlx::query!("update members set dm_chat_id = $2 where id = $1", id.0, dm_chat_id)
                .execute(self.pool())
                .await
                .map_err(store_error)?;
        Ok(result.rows_affected() == 1)
    }
}
