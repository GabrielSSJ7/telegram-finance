//! Credit cards: registration here; purchases and payments in
//! `card_spending`; invoices and summaries in `card_statements`.

use std::sync::Arc;

use domain::invoice_cycle::CardSchedule;
use domain::{Cents, DayOfMonth};

use super::text_rules::clean_name;
use super::{AccountService, CategoryService};
use crate::model::{AccountId, CardEdit, CardId, CreditCard, NewCard};
use crate::ports::{CardStore, Clock};
use crate::{AppError, AppResult};

pub const MAX_CARD_NAME_CHARS: usize = 40;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenCard {
    pub name: String,
    pub closing_day: DayOfMonth,
    pub due_day: DayOfMonth,
    /// Purchases on the closing day go to the next invoice (most banks).
    pub closing_day_goes_next: bool,
    pub limit: Option<Cents>,
    pub default_payment_account_id: Option<AccountId>,
}

pub struct CardService {
    pub(super) cards: Arc<dyn CardStore>,
    pub(super) accounts: Arc<AccountService>,
    pub(super) categories: Arc<CategoryService>,
    pub(super) clock: Arc<dyn Clock>,
}

impl CardService {
    pub fn new(
        cards: Arc<dyn CardStore>,
        accounts: Arc<AccountService>,
        categories: Arc<CategoryService>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self { cards, accounts, categories, clock }
    }

    /// Registers a card.
    ///
    /// ```ignore
    /// cards.open(OpenCard { name: "Nubank".into(), closing_day, due_day, closing_day_goes_next: true,
    ///     limit: None, default_payment_account_id: None }).await?;
    /// ```
    pub async fn open(&self, request: OpenCard) -> AppResult<CreditCard> {
        if let Some(limit) = request.limit.filter(|limit| !limit.is_positive()) {
            return Err(AppError::invalid(
                "card limit",
                limit.value(),
                "a positive number of cents",
            ));
        }
        if let Some(account) = request.default_payment_account_id {
            self.accounts.require_active(account).await?;
        }
        let name = clean_name("card name", &request.name, MAX_CARD_NAME_CHARS)?;
        Ok(self.cards.create_card(new_card(name, &request)).await?)
    }

    pub async fn list(&self) -> AppResult<Vec<CreditCard>> {
        Ok(self.cards.list_cards(false).await?)
    }

    /// Changes a card's name or the days its invoice closes and falls due.
    pub async fn update(&self, id: CardId, edit: CardEdit) -> AppResult<CreditCard> {
        let name = edit.name.map(|name| clean_name("card name", &name, MAX_CARD_NAME_CHARS));
        let edit = CardEdit { name: name.transpose()?, ..edit };
        let updated = self.cards.update_card(id, edit).await?;
        updated.ok_or_else(|| AppError::not_found("active card", id))
    }

    pub async fn archive(&self, id: CardId) -> AppResult<()> {
        if !self.cards.archive_card(id, self.clock.now()).await? {
            return Err(AppError::not_found("active card", id));
        }
        Ok(())
    }

    pub async fn require_active(&self, id: CardId) -> AppResult<CreditCard> {
        match self.cards.find_card(id).await? {
            Some(card) if !card.archived => Ok(card),
            _ => Err(AppError::not_found("active card", id)),
        }
    }
}

fn new_card(name: String, request: &OpenCard) -> NewCard {
    let schedule = CardSchedule {
        closing_day: request.closing_day,
        due_day: request.due_day,
        closing_day_goes_next: request.closing_day_goes_next,
    };
    NewCard {
        name,
        schedule,
        limit: request.limit,
        default_payment_account_id: request.default_payment_account_id,
    }
}
