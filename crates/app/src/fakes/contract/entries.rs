use chrono::{Datelike, Utc};
use domain::{AccountKind, Cents, EntryKind};

use super::{day, member, new_category, open_account};
use crate::model::{
    AccountId, CategoryId, CategoryKind, DraftId, EntryFilter, EntryId, EntryPatch, LedgerEntry,
    MemberId, NewEntry,
};
use crate::ports::StoreError;
use crate::services::StorePorts;

struct EntryScene {
    stores: StorePorts,
    checking: AccountId,
    savings: AccountId,
    groceries: CategoryId,
}

async fn scene(stores: StorePorts, suffix: &str) -> EntryScene {
    let checking =
        open_account(&stores, &format!("Corrente {suffix}"), AccountKind::Checking, 0).await.id;
    let savings =
        open_account(&stores, &format!("Poupança {suffix}"), AccountKind::Savings, 0).await.id;
    let groceries =
        new_category(&stores, &format!("mercado {suffix}"), CategoryKind::Expense).await.id;
    EntryScene { stores, checking, savings, groceries }
}

impl EntryScene {
    fn expense(&self, cents: i64, on: u32, created_by: Option<MemberId>) -> NewEntry {
        NewEntry {
            kind: EntryKind::Expense,
            amount: Cents::new(cents),
            description: format!("gasto {cents}"),
            category_id: Some(self.groceries),
            account_id: Some(self.checking),
            counter_account_id: None,
            invoice_id: None,
            accounting_date: day(4, on),
            created_by,
        }
    }

    fn transfer(&self, cents: i64, on: u32) -> NewEntry {
        let base = self.expense(cents, on, None);
        NewEntry {
            kind: EntryKind::Transfer,
            category_id: None,
            counter_account_id: Some(self.savings),
            ..base
        }
    }

    async fn record(&self, entry: NewEntry) -> LedgerEntry {
        self.stores.entries.record_entry(entry, None).await.unwrap()
    }

    async fn list(&self, filter: EntryFilter) -> Vec<EntryId> {
        let scoped = EntryFilter { from: filter.from.or(Some(day(4, 1))), ..filter };
        let found = self.stores.entries.list_entries(&scoped).await.unwrap();
        found
            .into_iter()
            .filter(|entry| entry.accounting_date.month0() == 3)
            .map(|entry| entry.id)
            .collect()
    }
}

pub async fn entry_record_and_find(stores: StorePorts) {
    let scene = scene(stores, "registro").await;
    let author = member(&scene.stores, 9001).await;
    let recorded = scene.record(scene.expense(1050, 5, Some(author.id))).await;
    assert_eq!(
        (recorded.kind, recorded.amount, recorded.deleted),
        (EntryKind::Expense, Cents::new(1050), false)
    );
    assert_eq!((recorded.created_by, recorded.accounting_date), (Some(author.id), day(4, 5)));
    let found = scene.stores.entries.find_entry(recorded.id).await.unwrap();
    assert_eq!(found, Some(recorded));
    assert_eq!(scene.stores.entries.find_entry(EntryId::generate()).await.unwrap(), None);
}

pub async fn entry_duplicate_draft_is_rejected(stores: StorePorts) {
    let scene = scene(stores, "rascunho").await;
    let draft = Some(DraftId::generate());
    scene.stores.entries.record_entry(scene.expense(10, 1, None), draft).await.unwrap();
    let again = scene.stores.entries.record_entry(scene.expense(10, 1, None), draft).await;
    assert_eq!(again.unwrap_err(), StoreError::DuplicateDraft);
    let only_one = scene
        .list(EntryFilter { account_id: Some(scene.checking), ..EntryFilter::default() })
        .await;
    assert_eq!(only_one.len(), 1);
}

pub async fn entry_list_filters_by_range_and_kind(stores: StorePorts) {
    let scene = scene(stores, "filtros-a").await;
    let early = scene.record(scene.expense(100, 2, None)).await;
    let moved = scene.record(scene.transfer(200, 3)).await;
    let late = scene.record(scene.expense(300, 9, None)).await;
    let range = EntryFilter {
        from: Some(day(4, 3)),
        to_inclusive: Some(day(4, 8)),
        ..EntryFilter::default()
    };
    assert_eq!(scene.list(range).await, vec![moved.id]);
    let kind = EntryFilter { kind: Some(EntryKind::Expense), ..EntryFilter::default() };
    assert_eq!(scene.list(kind).await, vec![late.id, early.id]);
}

pub async fn entry_list_filters_by_account_category_author(stores: StorePorts) {
    let scene = scene(stores, "filtros-b").await;
    let author = member(&scene.stores, 9002).await;
    let early = scene.record(scene.expense(100, 2, Some(author.id))).await;
    let moved = scene.record(scene.transfer(200, 3)).await;
    let late = scene.record(scene.expense(300, 9, None)).await;
    let counter = EntryFilter { account_id: Some(scene.savings), ..EntryFilter::default() };
    assert_eq!(scene.list(counter).await, vec![moved.id]);
    let by_category = EntryFilter { category_id: Some(scene.groceries), ..EntryFilter::default() };
    assert_eq!(scene.list(by_category).await, vec![late.id, early.id]);
    let by_author = EntryFilter { created_by: Some(author.id), ..EntryFilter::default() };
    assert_eq!(scene.list(by_author).await, vec![early.id]);
}

pub async fn entry_list_orders_newest_first_with_limit(stores: StorePorts) {
    let scene = scene(stores, "ordem").await;
    let first = scene.record(scene.expense(1, 10, None)).await;
    let second = scene.record(scene.expense(2, 12, None)).await;
    let third = scene.record(scene.expense(3, 11, None)).await;
    let mine = EntryFilter { account_id: Some(scene.checking), ..EntryFilter::default() };
    assert_eq!(scene.list(mine.clone()).await, vec![second.id, third.id, first.id]);
    assert_eq!(scene.list(EntryFilter { limit: 2, ..mine }).await, vec![second.id, third.id]);
}

pub async fn entry_update_and_soft_delete(stores: StorePorts) {
    let scene = scene(stores, "edição").await;
    let entry = scene.record(scene.expense(500, 5, None)).await;
    let description = Some("feira".to_owned());
    let patch = EntryPatch {
        amount: Some(Cents::new(700)),
        description,
        accounting_date: Some(day(4, 6)),
        ..EntryPatch::default()
    };
    let updated = scene.stores.entries.update_entry(entry.id, &patch).await.unwrap().unwrap();
    assert_eq!((updated.amount, updated.description.as_str()), (Cents::new(700), "feira"));
    assert_eq!((updated.accounting_date, updated.category_id), (day(4, 6), entry.category_id));
    assert!(scene.stores.entries.soft_delete_entry(entry.id, Utc::now()).await.unwrap());
    assert!(!scene.stores.entries.soft_delete_entry(entry.id, Utc::now()).await.unwrap());
    assert_eq!(scene.stores.entries.update_entry(entry.id, &patch).await.unwrap(), None);
    let deleted = scene.stores.entries.find_entry(entry.id).await.unwrap().unwrap();
    assert!(deleted.deleted);
}

pub async fn entry_latest_by_member(stores: StorePorts) {
    let scene = scene(stores, "último").await;
    let (me, spouse) = (member(&scene.stores, 9003).await.id, member(&scene.stores, 9004).await.id);
    assert_eq!(scene.stores.entries.latest_entry_by(me).await.unwrap(), None);
    let older = scene.record(scene.expense(1, 20, Some(me))).await;
    let newer = scene.record(scene.expense(2, 1, Some(me))).await;
    scene.record(scene.expense(3, 2, Some(spouse))).await;
    let latest = scene.stores.entries.latest_entry_by(me).await.unwrap().map(|entry| entry.id);
    assert_eq!(latest, Some(newer.id));
    scene.stores.entries.soft_delete_entry(newer.id, Utc::now()).await.unwrap();
    let latest = scene.stores.entries.latest_entry_by(me).await.unwrap().map(|entry| entry.id);
    assert_eq!(latest, Some(older.id));
}
