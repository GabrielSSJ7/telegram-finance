//! What each card owes: invoice statements, the per-card summary shown in
//! `/fatura` and reports, and the total that lowers "Disponível".

use domain::Cents;
use domain::invoice_settlement::{
    InvoiceStatus, current_open, future_committed, statements, unpaid_closed,
};

use super::CardService;
use crate::AppResult;
use crate::model::{CardId, CardSummary, CreditCard, InvoiceView};

impl CardService {
    /// Every invoice of the card, oldest first, with derived status.
    pub async fn invoices(&self, card_id: CardId) -> AppResult<Vec<InvoiceView>> {
        let card = self.cards.find_card(card_id).await?;
        let card = card.ok_or_else(|| crate::AppError::not_found("card", card_id))?;
        self.views_of(&card).await
    }

    pub async fn summaries(&self) -> AppResult<Vec<CardSummary>> {
        let mut summaries = Vec::new();
        for card in self.cards.list_cards(false).await? {
            let views = self.views_of(&card).await?;
            summaries.push(summary_of(card, &views));
        }
        Ok(summaries)
    }

    /// Owed on closed, unpaid invoices of every card, archived included.
    pub async fn unpaid_closed_total(&self) -> AppResult<Cents> {
        let mut total = Cents::ZERO;
        for card in self.cards.list_cards(true).await? {
            let views = self.views_of(&card).await?;
            let statements: Vec<_> = views.iter().map(|view| view.statement).collect();
            total += unpaid_closed(&statements);
        }
        Ok(total)
    }

    async fn views_of(&self, card: &CreditCard) -> AppResult<Vec<InvoiceView>> {
        let rows = self.cards.invoice_totals(card.id).await?;
        let periods: Vec<_> =
            rows.iter().map(|(invoice, totals)| (invoice.period, *totals)).collect();
        let computed = statements(card.schedule, &periods, self.clock.today());
        let views = rows
            .into_iter()
            .zip(computed)
            .map(|((invoice, _), statement)| InvoiceView { invoice, statement });
        Ok(views.collect())
    }
}

fn summary_of(card: CreditCard, views: &[InvoiceView]) -> CardSummary {
    let statements: Vec<_> = views.iter().map(|view| view.statement).collect();
    let current = current_open(&statements)
        .and_then(|open| views.iter().find(|view| view.statement == *open))
        .copied();
    let unpaid = views.iter().rev().find(|view| view.statement.status != InvoiceStatus::Open);
    let unpaid = unpaid.filter(|view| view.statement.status == InvoiceStatus::Closed).copied();
    CardSummary { card, current, unpaid, future_committed: future_committed(&statements) }
}
