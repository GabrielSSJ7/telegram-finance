use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::EntryKind;
use domain::installments::InstallmentSlot;
use domain::invoice_cycle::InvoicePeriod;
use domain::invoice_settlement::InvoiceTotals;

use super::entries::ledger_entry;
use super::{InMemoryStore, MemoryState, same_name, unique_violation};
use crate::model::{
    CardEdit, CardId, CardPurchase, CreditCard, DraftId, EntryId, Invoice, InvoiceId, LedgerEntry,
    NewCard, NewCardPurchase, NewEntry, PurchaseId,
};
use crate::ports::{CardStore, StoreError, StoreResult};

/// Another active card already uses `name`.
fn card_name_taken(state: &MemoryState, id: CardId, name: Option<&str>) -> bool {
    let Some(name) = name else {
        return false;
    };
    state.cards.iter().any(|row| row.id != id && !row.archived && same_name(&row.name, name))
}

#[async_trait]
impl CardStore for InMemoryStore {
    async fn update_card(&self, id: CardId, edit: CardEdit) -> StoreResult<Option<CreditCard>> {
        let mut state = self.lock();
        if card_name_taken(&state, id, edit.name.as_deref()) {
            return Err(unique_violation("credit_cards_active_name"));
        }
        let Some(row) = state.cards.iter_mut().find(|row| row.id == id && !row.archived) else {
            return Ok(None);
        };
        if let Some(name) = edit.name {
            row.name = name;
        }
        if let Some(day) = edit.closing_day {
            row.schedule.closing_day = day;
        }
        if let Some(day) = edit.due_day {
            row.schedule.due_day = day;
        }
        Ok(Some(row.clone()))
    }

    async fn create_card(&self, card: NewCard) -> StoreResult<CreditCard> {
        let mut state = self.lock();
        if state.cards.iter().any(|row| !row.archived && same_name(&row.name, &card.name)) {
            return Err(unique_violation("credit_cards_active_name"));
        }
        let created = CreditCard {
            id: CardId::generate(),
            name: card.name,
            schedule: card.schedule,
            limit: card.limit,
            default_payment_account_id: card.default_payment_account_id,
            archived: false,
        };
        state.cards.push(created.clone());
        Ok(created)
    }

    async fn list_cards(&self, include_archived: bool) -> StoreResult<Vec<CreditCard>> {
        Ok(self
            .lock()
            .cards
            .iter()
            .filter(|row| include_archived || !row.archived)
            .cloned()
            .collect())
    }

    async fn find_card(&self, id: CardId) -> StoreResult<Option<CreditCard>> {
        Ok(self.lock().cards.iter().find(|row| row.id == id).cloned())
    }

    async fn archive_card(&self, id: CardId, _at: DateTime<Utc>) -> StoreResult<bool> {
        let mut state = self.lock();
        let Some(card) = state.cards.iter_mut().find(|row| row.id == id && !row.archived) else {
            return Ok(false);
        };
        card.archived = true;
        Ok(true)
    }

    async fn ensure_invoice(&self, card: CardId, period: InvoicePeriod) -> StoreResult<Invoice> {
        Ok(ensure_invoice_in(&mut self.lock(), card, period))
    }

    async fn find_invoice(&self, id: InvoiceId) -> StoreResult<Option<Invoice>> {
        Ok(self.lock().invoices.iter().find(|row| row.id == id).copied())
    }

    async fn invoice_totals(&self, card: CardId) -> StoreResult<Vec<(Invoice, InvoiceTotals)>> {
        let state = self.lock();
        let mut invoices: Vec<Invoice> =
            state.invoices.iter().filter(|row| row.card_id == card).copied().collect();
        invoices.sort_by_key(|invoice| invoice.period.closing_date);
        Ok(invoices.into_iter().map(|invoice| (invoice, totals_of(&state, invoice.id))).collect())
    }

    async fn record_purchase(
        &self,
        purchase: NewCardPurchase,
        slots: &[InstallmentSlot],
        draft: Option<DraftId>,
    ) -> StoreResult<CardPurchase> {
        let mut state = self.lock();
        claim_draft(&mut state, draft)?;
        let created = purchase_from(purchase);
        for slot in slots {
            let invoice = ensure_invoice_in(&mut state, created.card_id, slot.invoice);
            state.entries.push(installment_entry(&created, slot, invoice.id));
        }
        state.purchases.push(created.clone());
        Ok(created)
    }

    async fn find_purchase(&self, id: PurchaseId) -> StoreResult<Option<CardPurchase>> {
        Ok(self.lock().purchases.iter().find(|row| row.id == id).cloned())
    }

    async fn delete_purchase(&self, id: PurchaseId, _at: DateTime<Utc>) -> StoreResult<bool> {
        let mut state = self.lock();
        let Some(purchase) = state.purchases.iter_mut().find(|row| row.id == id && !row.deleted)
        else {
            return Ok(false);
        };
        purchase.deleted = true;
        let installments =
            state.entries.iter_mut().filter(|entry| entry.card_purchase_id == Some(id));
        installments.for_each(|entry| entry.deleted = true);
        Ok(true)
    }

    async fn record_invoice_entry(
        &self,
        entry: NewEntry,
        draft: Option<DraftId>,
    ) -> StoreResult<LedgerEntry> {
        let mut state = self.lock();
        claim_draft(&mut state, draft)?;
        let recorded = ledger_entry(entry);
        state.entries.push(recorded.clone());
        Ok(recorded)
    }
}

fn claim_draft(state: &mut MemoryState, draft: Option<DraftId>) -> StoreResult<()> {
    match draft {
        Some(draft_id) if !state.drafts.insert(draft_id) => Err(StoreError::DuplicateDraft),
        _ => Ok(()),
    }
}

fn ensure_invoice_in(state: &mut MemoryState, card: CardId, period: InvoicePeriod) -> Invoice {
    let existing = state
        .invoices
        .iter()
        .find(|row| row.card_id == card && row.period.reference_month == period.reference_month);
    if let Some(invoice) = existing {
        return *invoice;
    }
    let invoice = Invoice { id: InvoiceId::generate(), card_id: card, period };
    state.invoices.push(invoice);
    invoice
}

fn totals_of(state: &MemoryState, invoice: InvoiceId) -> InvoiceTotals {
    let live =
        state.entries.iter().filter(|entry| !entry.deleted && entry.invoice_id == Some(invoice));
    live.fold(InvoiceTotals::default(), |mut totals, entry| {
        match entry.kind {
            EntryKind::CardInstallment => totals.charges += entry.amount,
            EntryKind::CardCredit => totals.credits += entry.amount,
            EntryKind::InvoicePayment => totals.payments += entry.amount,
            _ => {}
        }
        totals
    })
}

fn purchase_from(purchase: NewCardPurchase) -> CardPurchase {
    CardPurchase {
        id: PurchaseId::generate(),
        card_id: purchase.card_id,
        description: purchase.description,
        category_id: purchase.category_id,
        total: purchase.total,
        installment_count: purchase.installment_count,
        first_installment_no: purchase.first_installment_no,
        purchased_on: purchase.purchased_on,
        created_by: purchase.created_by,
        deleted: false,
    }
}

fn installment_entry(
    purchase: &CardPurchase,
    slot: &InstallmentSlot,
    invoice: InvoiceId,
) -> LedgerEntry {
    LedgerEntry {
        id: EntryId::generate(),
        kind: EntryKind::CardInstallment,
        amount: slot.amount,
        description: purchase.description.clone(),
        category_id: Some(purchase.category_id),
        account_id: None,
        counter_account_id: None,
        card_purchase_id: Some(purchase.id),
        installment_no: Some(slot.number),
        invoice_id: Some(invoice),
        accounting_date: slot.accounting_date,
        created_by: purchase.created_by,
        created_at: Utc::now(),
        deleted: false,
    }
}
