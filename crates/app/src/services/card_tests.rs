use chrono::NaiveDate;
use domain::invoice_settlement::InvoiceStatus;
use domain::{AccountKind, Cents, DayOfMonth};

use super::card_spending::{CardCreditRequest, CardPurchaseRequest, InvoicePaymentRequest};
use super::cards::OpenCard;
use super::goals::CreateGoal;
use super::ledger::EntryOrigin;
use super::{AllowedUsers, CardService, OpenAccount};
use crate::AppError;
use crate::fakes::FakeServiceSet;
use crate::model::{
    AccountId, CardId, CategoryId, CategoryKind, EntryFilter, InvoiceView, MemberId,
};

struct CardFixture {
    set: FakeServiceSet,
    card: CardId,
    checking: AccountId,
    groceries: CategoryId,
}

fn day(month: u32, value: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, month, value).unwrap()
}

fn open_card(name: &str) -> OpenCard {
    let (closing_day, due_day) = (DayOfMonth::new(3).unwrap(), DayOfMonth::new(10).unwrap());
    OpenCard {
        name: name.into(),
        closing_day,
        due_day,
        closing_day_goes_next: true,
        limit: None,
        default_payment_account_id: None,
    }
}

async fn fixture() -> CardFixture {
    let set = FakeServiceSet::new(day(1, 20), AllowedUsers::default());
    let request = OpenAccount {
        name: "Nubank".into(),
        kind: AccountKind::Checking,
        initial_balance: Cents::new(500_000),
        opened_on: None,
    };
    let checking = set.services.accounts.open(request).await.unwrap().id;
    let card = set.services.cards.open(open_card("Roxinho")).await.unwrap().id;
    let groceries = set.store.seed_category("mercado", CategoryKind::Expense);
    CardFixture { set, card, checking, groceries }
}

impl CardFixture {
    fn cards(&self) -> &CardService {
        &self.set.services.cards
    }

    fn purchase(&self, total: i64, installments: u32) -> CardPurchaseRequest {
        let (card_id, category_id, total) = (self.card, self.groceries, Cents::new(total));
        CardPurchaseRequest {
            card_id,
            category_id,
            total,
            installments,
            first_installment_no: 1,
            description: "tv".into(),
            purchased_on: None,
        }
    }

    async fn invoices(&self) -> Vec<InvoiceView> {
        self.cards().invoices(self.card).await.unwrap()
    }

    async fn pay(&self, invoice: &InvoiceView, cents: i64) {
        let request = InvoicePaymentRequest {
            invoice_id: invoice.invoice.id,
            account_id: self.checking,
            amount: Cents::new(cents),
            date: None,
        };
        self.cards().pay_invoice(request, EntryOrigin::default()).await.unwrap();
    }
}

fn charges(invoices: &[InvoiceView]) -> Vec<i64> {
    invoices.iter().map(|view| view.statement.totals.charges.value()).collect()
}

#[tokio::test]
async fn purchase_spreads_over_consecutive_invoices() {
    let fixture = fixture().await;
    fixture.cards().purchase(fixture.purchase(10_000, 3), EntryOrigin::default()).await.unwrap();
    let invoices = fixture.invoices().await;
    assert_eq!(charges(&invoices), vec![3_334, 3_333, 3_333]);
    assert!(invoices.iter().all(|view| view.statement.status == InvoiceStatus::Open));
    let months: Vec<u32> =
        invoices.iter().map(|view| view.invoice.period.reference_month.month()).collect();
    assert_eq!(months, vec![2, 3, 4]);
}

#[tokio::test]
async fn purchase_rejects_bad_installments_category_and_card() {
    let fixture = fixture().await;
    let zero = fixture.cards().purchase(fixture.purchase(100, 0), EntryOrigin::default()).await;
    assert!(matches!(zero, Err(AppError::Invalid { field: "installments", .. })));
    let salary = fixture.set.store.seed_category("salário", CategoryKind::Income);
    let wrong = CardPurchaseRequest { category_id: salary, ..fixture.purchase(100, 1) };
    assert!(fixture.cards().purchase(wrong, EntryOrigin::default()).await.is_err());
    fixture.cards().archive(fixture.card).await.unwrap();
    let archived = fixture.cards().purchase(fixture.purchase(100, 1), EntryOrigin::default()).await;
    assert!(matches!(archived, Err(AppError::NotFound { .. })));
}

#[tokio::test]
async fn partial_payment_leaves_rest_owed_and_lowers_available() {
    let fixture = fixture().await;
    fixture.cards().purchase(fixture.purchase(100_000, 1), EntryOrigin::default()).await.unwrap();
    fixture.set.clock.set_local_noon(day(2, 5));
    let closed = fixture.invoices().await[0];
    assert_eq!(closed.statement.status, InvoiceStatus::Closed);
    fixture.pay(&closed, 60_000).await;
    let sheet = fixture.set.services.position.balance_sheet().await.unwrap();
    assert_eq!(sheet.position.available, Cents::new(500_000 - 60_000 - 40_000));
    let summary = &fixture.cards().summaries().await.unwrap()[0];
    assert_eq!(summary.unpaid.map(|view| view.statement.outstanding), Some(Cents::new(40_000)));
}

#[tokio::test]
async fn credit_lowers_the_open_invoice() {
    let fixture = fixture().await;
    fixture.cards().purchase(fixture.purchase(5_000, 1), EntryOrigin::default()).await.unwrap();
    let request = CardCreditRequest {
        card_id: fixture.card,
        category_id: fixture.groceries,
        amount: Cents::new(1_000),
        description: "estorno".into(),
        date: None,
    };
    fixture.cards().credit(request, EntryOrigin::default()).await.unwrap();
    let invoice = fixture.invoices().await[0];
    assert_eq!(
        (invoice.statement.totals.credits, invoice.statement.outstanding),
        (Cents::new(1_000), Cents::new(4_000))
    );
}

fn payment(invoice_id: crate::model::InvoiceId, account_id: AccountId) -> InvoicePaymentRequest {
    InvoicePaymentRequest { invoice_id, account_id, amount: Cents::new(1), date: None }
}

#[tokio::test]
async fn payment_needs_known_invoice() {
    let fixture = fixture().await;
    let unknown = payment(crate::model::InvoiceId::generate(), fixture.checking);
    let result = fixture.cards().pay_invoice(unknown, EntryOrigin::default()).await;
    assert!(matches!(result, Err(AppError::NotFound { .. })));
}

#[tokio::test]
async fn payment_cannot_come_from_a_pot() {
    let fixture = fixture().await;
    fixture.cards().purchase(fixture.purchase(5_000, 1), EntryOrigin::default()).await.unwrap();
    let invoice = fixture.invoices().await[0].invoice.id;
    let (target, already_saved) = (Cents::new(100), Cents::ZERO);
    let goal = CreateGoal { name: "Casa".into(), target, target_date: None, already_saved };
    let pot = fixture.set.services.goals.create(goal).await.unwrap().pot.id;
    let result = fixture.cards().pay_invoice(payment(invoice, pot), EntryOrigin::default()).await;
    assert!(matches!(result, Err(AppError::Invalid { .. })));
}

#[tokio::test]
async fn summaries_show_current_and_future_commitments() {
    let fixture = fixture().await;
    fixture.cards().purchase(fixture.purchase(30_000, 3), EntryOrigin::default()).await.unwrap();
    let summary = &fixture.cards().summaries().await.unwrap()[0];
    assert_eq!(summary.current.map(|view| view.statement.totals.charges), Some(Cents::new(10_000)));
    assert_eq!((summary.unpaid, summary.future_committed), (None, Cents::new(20_000)));
}

#[tokio::test]
async fn undo_of_an_installment_removes_the_whole_purchase() {
    let fixture = fixture().await;
    let member = MemberId::generate();
    let origin = EntryOrigin { created_by: Some(member), draft: None };
    let purchase = fixture.cards().purchase(fixture.purchase(9_000, 3), origin).await.unwrap();
    fixture.set.services.ledger.undo_last(member).await.unwrap();
    let live = fixture.set.services.ledger.list(&EntryFilter::default()).await.unwrap();
    assert!(live.is_empty(), "{live:?}");
    assert!(matches!(
        fixture.cards().delete_purchase(purchase.id).await,
        Err(AppError::NotFound { .. })
    ));
}

#[tokio::test]
async fn open_validates_limit_and_payment_account() {
    let fixture = fixture().await;
    let negative = OpenCard { limit: Some(Cents::new(-1)), ..open_card("Inter") };
    assert!(matches!(fixture.cards().open(negative).await, Err(AppError::Invalid { .. })));
    let unknown =
        OpenCard { default_payment_account_id: Some(AccountId::generate()), ..open_card("Inter") };
    assert!(matches!(fixture.cards().open(unknown).await, Err(AppError::NotFound { .. })));
    assert_eq!(fixture.cards().list().await.unwrap().len(), 1);
    assert!(fixture.cards().require_active(CardId::generate()).await.is_err());
    assert!(fixture.cards().archive(CardId::generate()).await.is_err());
}

#[tokio::test]
async fn update_changes_the_name_and_the_invoice_days() {
    let fixture = fixture().await;
    let cards = &fixture.set.services.cards;
    let edit = crate::model::CardEdit {
        name: Some(" Itaú Black ".into()),
        closing_day: DayOfMonth::new(5).ok(),
        due_day: DayOfMonth::new(15).ok(),
    };
    let updated = cards.update(fixture.card, edit).await.unwrap();
    assert_eq!(updated.name, "Itaú Black");
    assert_eq!((updated.schedule.closing_day.get(), updated.schedule.due_day.get()), (5, 15));
    cards.open(open_card("Outro")).await.unwrap();
    let clash = crate::model::CardEdit { name: Some("outro".into()), ..Default::default() };
    assert!(matches!(cards.update(fixture.card, clash).await, Err(AppError::Conflict(_))));
    cards.archive(fixture.card).await.unwrap();
    let gone = cards.update(fixture.card, crate::model::CardEdit::default()).await;
    assert!(matches!(gone, Err(AppError::NotFound { .. })), "{gone:?}");
}
