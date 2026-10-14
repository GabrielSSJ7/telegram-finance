//! What each card owes: invoice statements, the per-card summary shown in
//! `/fatura` and reports, and the total that lowers "Disponível".

use domain::Cents;
use domain::invoice_settlement::{
    InvoiceStatus, current_open, future_committed, statements, unpaid_closed,
};

use super::CardService;
use crate::AppResult;
use crate::model::{CardId, CardSummary, CreditCard, InvoiceDebt, InvoiceView};

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

    /// What every card takes, archived cards included: the closed invoices
    /// that lower "Disponível" and everything owed on existing invoices.
    pub async fn owed(&self) -> AppResult<InvoiceDebt> {
        let mut debt = InvoiceDebt::default();
        for card in self.cards.list_cards(true).await? {
            let views = self.views_of(&card).await?;
            let statements: Vec<_> = views.iter().map(|view| view.statement).collect();
            let closed = unpaid_closed(&statements);
            let open = current_open(&statements).map_or(Cents::ZERO, |open| open.outstanding);
            debt.closed_unpaid += closed;
            debt.all_invoices += closed + open;
        }
        Ok(debt)
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
