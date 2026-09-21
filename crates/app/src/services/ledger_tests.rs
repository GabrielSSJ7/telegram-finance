use chrono::NaiveDate;
use domain::{AccountKind, Cents, EntryKind};

use super::ledger::{AccountEntry, AdjustmentEntry, EntryOrigin, EntryRequest, TransferEntry};
use super::{AllowedUsers, LedgerService, OpenAccount};
use crate::AppError;
use crate::fakes::FakeServiceSet;
use crate::model::{
    AccountId, CategoryId, CategoryKind, DraftId, EntryFilter, EntryPatch, LedgerEntry, MemberId,
};

struct LedgerFixture {
    set: FakeServiceSet,
    checking: AccountId,
    savings: AccountId,
    groceries: CategoryId,
    salary: CategoryId,
}

impl LedgerFixture {
    fn ledger(&self) -> &LedgerService {
        &self.set.services.ledger
    }

    async fn spend(&self, cents: i64, origin: EntryOrigin) -> Result<LedgerEntry, AppError> {
        let request = account_entry(self.checking, self.groceries, cents);
        self.ledger().record(EntryRequest::Expense(request), origin).await
    }

    async fn balances(&self) -> Vec<i64> {
        let balances = self.set.services.accounts.balances().await.unwrap();
        balances.iter().map(|item| item.balance.value()).collect()
    }
}

fn day(value: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 3, value).unwrap()
}

async fn open(set: &FakeServiceSet, name: &str, kind: AccountKind) -> AccountId {
    let initial_balance = Cents::new(100_000);
    let request = OpenAccount { name: name.into(), kind, initial_balance, opened_on: None };
    set.services.accounts.open(request).await.unwrap().id
}

async fn fixture() -> LedgerFixture {
    let set = FakeServiceSet::new(day(10), AllowedUsers::default());
    let checking = open(&set, "Nubank", AccountKind::Checking).await;
    let savings = open(&set, "Poupança", AccountKind::Savings).await;
    let groceries = set.store.seed_category("mercado", CategoryKind::Expense);
    let salary = set.store.seed_category("salário", CategoryKind::Income);
    LedgerFixture { set, checking, savings, groceries, salary }
}

fn account_entry(account_id: AccountId, category_id: CategoryId, cents: i64) -> AccountEntry {
    let (amount, description) = (Cents::new(cents), " mercado ".to_owned());
    AccountEntry { account_id, category_id, amount, description, date: None }
}

fn by(member: MemberId) -> EntryOrigin {
    EntryOrigin { created_by: Some(member), draft: None }
}

#[tokio::test]
async fn expense_defaults_to_today_and_lowers_balance() {
    let fixture = fixture().await;
    let entry = fixture.spend(1050, EntryOrigin::default()).await.unwrap();
    assert_eq!((entry.kind, entry.accounting_date), (EntryKind::Expense, day(10)));
    assert_eq!(entry.description, "mercado");
    assert_eq!(fixture.balances().await, vec![100_000 - 1050, 100_000]);
}

#[tokio::test]
async fn same_draft_saves_once() {
    let fixture = fixture().await;
    let origin = EntryOrigin { created_by: None, draft: Some(DraftId::generate()) };
    fixture.spend(500, origin).await.unwrap();
    assert_eq!(fixture.spend(500, origin).await, Err(AppError::AlreadyCommitted));
    let listed = fixture.ledger().list(&EntryFilter::default()).await.unwrap();
    assert_eq!(listed.len(), 1);
}

#[tokio::test]
async fn rejects_wrong_category_kind() {
    let fixture = fixture().await;
    let request = account_entry(fixture.checking, fixture.salary, 500);
    let result =
        fixture.ledger().record(EntryRequest::Expense(request), EntryOrigin::default()).await;
    assert!(matches!(result, Err(AppError::Invalid { field: "category", .. })));
}

#[tokio::test]
async fn rejects_bad_amount_and_unknown_account() {
    let fixture = fixture().await;
    let negative = fixture.spend(-5, EntryOrigin::default()).await;
    assert!(matches!(negative, Err(AppError::Invalid { field: "amount", .. })));
    let request = account_entry(AccountId::generate(), fixture.groceries, 5);
    let unknown =
        fixture.ledger().record(EntryRequest::Expense(request), EntryOrigin::default()).await;
    assert!(matches!(unknown, Err(AppError::NotFound { .. })));
}

fn transfer(from: AccountId, to: AccountId, cents: i64) -> EntryRequest {
    let (amount, description) = (Cents::new(cents), String::new());
    let entry = TransferEntry {
        from_account_id: from,
        to_account_id: to,
        amount,
        description,
        date: Some(day(9)),
    };
    EntryRequest::Transfer(entry)
}

#[tokio::test]
async fn transfer_moves_money_between_accounts() {
    let fixture = fixture().await;
    let request = transfer(fixture.checking, fixture.savings, 30_000);
    fixture.ledger().record(request, EntryOrigin::default()).await.unwrap();
    assert_eq!(fixture.balances().await, vec![70_000, 130_000]);
    let same = transfer(fixture.checking, fixture.checking, 1);
    assert!(fixture.ledger().record(same, EntryOrigin::default()).await.is_err());
}

#[tokio::test]
async fn update_changes_amount_and_cleans_description() {
    let fixture = fixture().await;
    let entry = fixture.spend(500, EntryOrigin::default()).await.unwrap();
    let description = Some(" feira ".to_owned());
    let patch = EntryPatch { amount: Some(Cents::new(700)), description, ..EntryPatch::default() };
    let updated = fixture.ledger().update(entry.id, patch).await.unwrap();
    assert_eq!((updated.amount, updated.description.as_str()), (Cents::new(700), "feira"));
}

#[tokio::test]
async fn update_rejects_empty_patch_and_wrong_category() {
    let fixture = fixture().await;
    let entry = fixture.spend(500, EntryOrigin::default()).await.unwrap();
    let wrong = EntryPatch { category_id: Some(fixture.salary), ..EntryPatch::default() };
    assert!(fixture.ledger().update(entry.id, wrong).await.is_err());
    assert!(fixture.ledger().update(entry.id, EntryPatch::default()).await.is_err());
}

#[tokio::test]
async fn adjustment_rejects_category_patch() {
    let fixture = fixture().await;
    let (amount, description) = (Cents::new(3), "centavos".to_owned());
    let adjust = AdjustmentEntry { account_id: fixture.checking, amount, description, date: None };
    let entry = fixture.ledger().record(EntryRequest::AdjustIn(adjust), EntryOrigin::default());
    let entry = entry.await.unwrap();
    let patch = EntryPatch { category_id: Some(fixture.groceries), ..EntryPatch::default() };
    let error = fixture.ledger().update(entry.id, patch).await.unwrap_err().to_string();
    assert!(error.contains("adjust_in"), "{error}");
}

#[tokio::test]
async fn undo_last_removes_only_my_latest_entry() {
    let fixture = fixture().await;
    let (me, spouse) = (MemberId::generate(), MemberId::generate());
    let first = fixture.spend(100, by(me)).await.unwrap();
    let second = fixture.spend(200, by(me)).await.unwrap();
    fixture.spend(300, by(spouse)).await.unwrap();
    assert_eq!(fixture.ledger().undo_last(me).await.unwrap().id, second.id);
    assert_eq!(fixture.ledger().undo_last(me).await.unwrap().id, first.id);
    assert!(matches!(fixture.ledger().undo_last(me).await, Err(AppError::NotFound { .. })));
}

#[tokio::test]
async fn income_is_found_until_deleted() {
    let fixture = fixture().await;
    let income = account_entry(fixture.checking, fixture.salary, 500_000);
    let entry = fixture.ledger().record(EntryRequest::Income(income), EntryOrigin::default());
    let entry = entry.await.unwrap();
    assert_eq!(entry.kind, EntryKind::Income);
    assert!(fixture.ledger().find(entry.id).await.is_ok());
    fixture.ledger().delete(entry.id).await.unwrap();
    assert!(fixture.ledger().find(entry.id).await.is_err());
}
