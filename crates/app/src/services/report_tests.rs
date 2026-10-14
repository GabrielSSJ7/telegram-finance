use chrono::NaiveDate;
use domain::{AccountKind, Cents};

use super::card_spending::{CardPurchaseRequest, InvoicePaymentRequest};
use super::cards::OpenCard;
use super::ledger::{AccountEntry, EntryOrigin, EntryRequest};
use super::{AllowedUsers, OpenAccount, PotMove};
use crate::fakes::FakeServiceSet;
use crate::fakes::requests::goal;
use crate::model::{AccountId, CategoryId, CategoryKind, MemberId, MemberProfile};
use domain::DayOfMonth;

struct Scene {
    set: FakeServiceSet,
    checking: AccountId,
    groceries: CategoryId,
    salary: CategoryId,
    ana: MemberId,
}

fn date(month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, month, day).unwrap()
}

async fn scene() -> Scene {
    let set = FakeServiceSet::new(date(3, 10), AllowedUsers::new([11]));
    let open = OpenAccount {
        name: "Nubank".into(),
        kind: AccountKind::Checking,
        initial_balance: Cents::new(100_000),
        opened_on: None,
    };
    let checking = set.services.accounts.open(open).await.unwrap().id;
    let groceries = set.store.seed_category("mercado", CategoryKind::Expense);
    let salary = set.store.seed_category("salário", CategoryKind::Income);
    let ana = set
        .services
        .members
        .authorize(MemberProfile { telegram_user_id: 11, display_name: "Ana".into() })
        .await
        .unwrap()
        .id;
    Scene { set, checking, groceries, salary, ana }
}

impl Scene {
    async fn record(&self, income: bool, cents: i64, on: NaiveDate) {
        let category_id = if income { self.salary } else { self.groceries };
        let entry = AccountEntry {
            account_id: self.checking,
            category_id,
            amount: Cents::new(cents),
            description: String::new(),
            date: Some(on),
        };
        let request =
            if income { EntryRequest::Income(entry) } else { EntryRequest::Expense(entry) };
        let origin = EntryOrigin { created_by: Some(self.ana), draft: None };
        self.set.services.ledger.record(request, origin).await.unwrap();
    }

    async fn open_card(&self) -> crate::model::CardId {
        let card = OpenCard {
            name: "Roxinho".into(),
            closing_day: DayOfMonth::new(3).unwrap(),
            due_day: DayOfMonth::new(10).unwrap(),
            closing_day_goes_next: true,
            limit: None,
            default_payment_account_id: None,
        };
        self.set.services.cards.open(card).await.unwrap().id
    }

    async fn buy(
        &self,
        card_id: crate::model::CardId,
        total: i64,
        installments: u32,
        on: NaiveDate,
    ) {
        let request = CardPurchaseRequest {
            card_id,
            category_id: self.groceries,
            total: Cents::new(total),
            installments,
            first_installment_no: 1,
            description: "tv".into(),
            purchased_on: Some(on),
        };
        self.set.services.cards.purchase(request, EntryOrigin::default()).await.unwrap();
    }

    async fn pay(&self, invoice_id: &crate::model::InvoiceId, cents: i64, on: NaiveDate) {
        let request = InvoicePaymentRequest {
            invoice_id: *invoice_id,
            account_id: self.checking,
            amount: Cents::new(cents),
            date: Some(on),
        };
        self.set.services.cards.pay_invoice(request, EntryOrigin::default()).await.unwrap();
    }
}

#[tokio::test]
async fn daily_report_separates_today_from_cycle() {
    let scene = scene().await;
    scene.record(true, 500_000, date(3, 5)).await;
    scene.record(false, 10_000, date(3, 9)).await;
    scene.record(false, 2_500, date(3, 10)).await;
    let report = scene.set.services.reports.daily(date(3, 10)).await.unwrap();
    assert_eq!((report.cycle.start, report.entries_today.len()), (date(3, 1), 1));
    assert_eq!(report.today.summary.spending, Cents::new(2_500));
    assert_eq!(report.cycle_to_date.summary.income, Cents::new(500_000));
    assert_eq!(report.cycle_to_date.by_member, vec![(Some(scene.ana), Cents::new(12_500))]);
    assert_eq!(report.balances.position.available, Cents::new(100_000 + 500_000 - 12_500));
}

#[tokio::test]
async fn cycle_report_compares_with_previous_and_counts_pots() {
    let scene = scene().await;
    scene.record(true, 100_000, date(2, 10)).await;
    scene.record(false, 40_000, date(3, 3)).await;
    let goal = scene.set.services.goals.create(goal("Casa", 1_000_000, 0)).await.unwrap();
    let (amount, description) = (Cents::new(30_000), String::new());
    let deposit = PotMove { goal_id: goal.id, account_id: scene.checking, amount, description };
    scene.set.services.goals.deposit(deposit, EntryOrigin::default()).await.unwrap();
    let cycle = scene.set.services.reports.cycle_of(date(3, 10)).await.unwrap();
    let report = scene.set.services.reports.cycle(cycle).await.unwrap();
    let totals = (report.totals.summary.spending, report.previous.income, report.saved_in_pots);
    assert_eq!(totals, (Cents::new(40_000), Cents::new(100_000), Cents::new(30_000)));
    assert_eq!(report.totals.by_category, vec![(Some(scene.groceries), Cents::new(40_000))]);
    assert_eq!(report.members.len(), 1);
}

/// A card bought on, with part of its first invoice paid.
async fn scene_with_card() -> (Scene, Cents) {
    let scene = scene().await;
    let card = scene.open_card().await;
    scene.record(false, 20_000, date(3, 4)).await;
    scene.buy(card, 90_000, 3, date(3, 4)).await;
    let invoice = scene.set.services.cards.invoices(card).await.unwrap().remove(0);
    scene.pay(&invoice.invoice.id, 15_000, date(3, 9)).await;
    (scene, Cents::new(15_000))
}

/// Only the installment that fell inside the cycle is spending; the rest of
/// the purchase belongs to the months its installments land on.
#[tokio::test]
async fn cycle_counts_the_installment_not_the_whole_purchase() {
    let (scene, _) = scene_with_card().await;
    let report = scene.set.services.reports.daily(date(3, 10)).await.unwrap();
    let summary = report.cycle_to_date.summary;
    assert_eq!(summary.spending, Cents::new(50_000));
    assert_eq!(summary.on_cards, Cents::new(30_000));
}

/// What left the account and what the card is still going to take.
#[tokio::test]
async fn cycle_shows_cash_out_and_what_is_still_due() {
    let (scene, paid) = scene_with_card().await;
    let report = scene.set.services.reports.daily(date(3, 10)).await.unwrap();
    // Do caixa saíram a despesa em conta e o pagamento da fatura, nada mais.
    assert_eq!(report.cycle_to_date.cash.out, Cents::new(35_000));
    assert_eq!(report.cycle_to_date.cash.invoice_payments, paid);
    // A fatura vence 10/04, fora deste ciclo, com 15.000 em aberto e 60.000
    // em parcelas já comprometidas com faturas seguintes.
    assert_eq!(report.invoices_due.in_cycle, Cents::ZERO);
    assert_eq!(report.invoices_due.later, Cents::new(75_000));
    let position = report.balances.position;
    assert_eq!(position.invoices_owed, paid);
    assert_eq!(position.free_after_invoices, position.available - paid);
}
