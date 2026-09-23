use std::sync::atomic::{AtomicU32, Ordering};

use chrono::{NaiveDate, Utc};
use domain::installments::{InstallmentPlan, schedule_installments};
use domain::invoice_cycle::CardSchedule;
use domain::{AccountKind, Cents, DayOfMonth, EntryKind};

use super::{member, new_category, open_account};
use crate::model::{
    CardEdit, CardId, CardPurchase, CategoryKind, CreditCard, DraftId, NewCard, NewCardPurchase,
    NewEntry,
};
use crate::ports::StoreError;
use crate::services::StorePorts;

fn schedule() -> CardSchedule {
    let day = |value| DayOfMonth::new(value).unwrap();
    CardSchedule { closing_day: day(3), due_day: day(10), closing_day_goes_next: true }
}

fn on(month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, month, day).unwrap()
}

async fn new_card(stores: &StorePorts, name: &str) -> CreditCard {
    let card = NewCard {
        name: name.into(),
        schedule: schedule(),
        limit: Some(Cents::new(500_000)),
        default_payment_account_id: None,
    };
    stores.cards.create_card(card).await.unwrap()
}

/// Next category name; categories are unique per store, purchases are not.
fn category_name() -> String {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    format!("cartão {}", NEXT.fetch_add(1, Ordering::Relaxed))
}

async fn buy(
    stores: &StorePorts,
    card: CardId,
    total: i64,
    count: u32,
    draft: Option<DraftId>,
) -> Result<CardPurchase, StoreError> {
    let category = new_category(stores, &category_name(), CategoryKind::Expense).await.id;
    let author = member(stores, 7_001).await.id;
    let (total, purchased_on) = (Cents::new(total), on(1, 20));
    let purchase = NewCardPurchase {
        card_id: card,
        description: "tv".into(),
        category_id: category,
        total,
        installment_count: count,
        first_installment_no: 1,
        purchased_on,
        created_by: Some(author),
    };
    let plan = InstallmentPlan { total, count, first_number: 1, purchase_date: purchased_on };
    let slots = schedule_installments(plan, schedule()).unwrap();
    stores.cards.record_purchase(purchase, &slots, draft).await
}

pub async fn card_create_find_list_unique(stores: StorePorts) {
    let card = new_card(&stores, "Nubank Roxinho").await;
    assert_eq!(stores.cards.find_card(card.id).await.unwrap().as_ref(), Some(&card));
    assert!(stores.cards.list_cards(false).await.unwrap().contains(&card));
    let (name, limit) = ("nubank roxinho".to_owned(), None);
    let clash = NewCard { name, schedule: schedule(), limit, default_payment_account_id: None };
    let error = stores.cards.create_card(clash).await.unwrap_err();
    assert!(matches!(error, StoreError::UniqueViolation { .. }), "{error:?}");
}

pub async fn card_archive_hides_card(stores: StorePorts) {
    let card = new_card(&stores, "Arquivado").await;
    assert!(stores.cards.archive_card(card.id, Utc::now()).await.unwrap());
    assert!(!stores.cards.archive_card(card.id, Utc::now()).await.unwrap());
    let active = stores.cards.list_cards(false).await.unwrap();
    assert!(!active.iter().any(|row| row.id == card.id));
    let all = stores.cards.list_cards(true).await.unwrap();
    assert!(all.iter().any(|row| row.id == card.id && row.archived));
}

pub async fn card_invoice_ensure_keeps_first_dates(stores: StorePorts) {
    let card = new_card(&stores, "Itaú Click").await;
    let period = schedule().period_for_purchase(on(1, 1));
    let first = stores.cards.ensure_invoice(card.id, period).await.unwrap();
    let moved = domain::invoice_cycle::InvoicePeriod { closing_date: on(1, 5), ..period };
    let again = stores.cards.ensure_invoice(card.id, moved).await.unwrap();
    assert_eq!((again.id, again.period), (first.id, period));
    assert_eq!(stores.cards.find_invoice(first.id).await.unwrap(), Some(first));
}

pub async fn card_purchase_spreads_installments(stores: StorePorts) {
    let card = new_card(&stores, "Inter").await;
    let draft = Some(DraftId::generate());
    let purchase = buy(&stores, card.id, 30_000, 3, draft).await.unwrap();
    assert_eq!((purchase.total, purchase.installment_count), (Cents::new(30_000), 3));
    let totals = stores.cards.invoice_totals(card.id).await.unwrap();
    let charges: Vec<i64> = totals.iter().map(|(_, totals)| totals.charges.value()).collect();
    assert_eq!(charges, vec![10_000, 10_000, 10_000]);
    assert!(
        totals.windows(2).all(|pair| pair[0].0.period.closing_date < pair[1].0.period.closing_date)
    );
    assert_eq!(
        buy(&stores, card.id, 30_000, 3, draft).await.unwrap_err(),
        StoreError::DuplicateDraft
    );
    assert_eq!(
        stores.cards.find_purchase(purchase.id).await.unwrap().map(|found| found.id),
        Some(purchase.id)
    );
}

pub async fn card_purchase_delete_removes_installments(stores: StorePorts) {
    let card = new_card(&stores, "C6").await;
    let purchase = buy(&stores, card.id, 999, 2, None).await.unwrap();
    assert!(stores.cards.delete_purchase(purchase.id, Utc::now()).await.unwrap());
    assert!(!stores.cards.delete_purchase(purchase.id, Utc::now()).await.unwrap());
    let totals = stores.cards.invoice_totals(card.id).await.unwrap();
    assert!(totals.iter().all(|(_, totals)| totals.charges == Cents::ZERO));
    assert!(stores.cards.find_purchase(purchase.id).await.unwrap().unwrap().deleted);
}

fn invoice_entry(
    kind: EntryKind,
    cents: i64,
    invoice: crate::model::InvoiceId,
    account: Option<crate::model::AccountId>,
) -> NewEntry {
    NewEntry {
        kind,
        amount: Cents::new(cents),
        description: String::new(),
        category_id: None,
        account_id: account,
        counter_account_id: None,
        invoice_id: Some(invoice),
        accounting_date: on(2, 1),
        created_by: None,
    }
}

pub async fn card_invoice_entries_count_as_credits_and_payments(stores: StorePorts) {
    let card = new_card(&stores, "XP").await;
    let checking = open_account(&stores, "Conta XP", AccountKind::Checking, 0).await.id;
    let period = schedule().period_for_purchase(on(1, 1));
    let invoice = stores.cards.ensure_invoice(card.id, period).await.unwrap();
    let credit = invoice_entry(EntryKind::CardCredit, 300, invoice.id, None);
    stores.cards.record_invoice_entry(credit, None).await.unwrap();
    let payment = invoice_entry(EntryKind::InvoicePayment, 700, invoice.id, Some(checking));
    let draft = Some(DraftId::generate());
    let recorded = stores.cards.record_invoice_entry(payment.clone(), draft).await.unwrap();
    assert_eq!((recorded.kind, recorded.invoice_id), (EntryKind::InvoicePayment, Some(invoice.id)));
    let again = stores.cards.record_invoice_entry(payment, draft).await.unwrap_err();
    assert_eq!(again, StoreError::DuplicateDraft);
    let totals = stores.cards.invoice_totals(card.id).await.unwrap()[0].1;
    assert_eq!((totals.credits, totals.payments), (Cents::new(300), Cents::new(700)));
}

pub async fn card_update_name_and_days(stores: StorePorts) {
    let card = new_card(&stores, "contrato-editar").await;
    let edit = CardEdit {
        name: Some("contrato-editado".into()),
        closing_day: DayOfMonth::new(7).ok(),
        due_day: DayOfMonth::new(17).ok(),
    };
    let updated = stores.cards.update_card(card.id, edit).await.unwrap().unwrap();
    assert_eq!(updated.name, "contrato-editado");
    assert_eq!((updated.schedule.closing_day.get(), updated.schedule.due_day.get()), (7, 17));
    assert!(stores.cards.archive_card(card.id, Utc::now()).await.unwrap());
    assert_eq!(stores.cards.update_card(card.id, CardEdit::default()).await.unwrap(), None);
}
