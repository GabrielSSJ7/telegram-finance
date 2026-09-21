//! Names for the ids a flow stores, loaded fresh for each card.

use app::AppResult;
use app::model::{Account, AccountId, Category, CategoryId, CategoryKind, Goal, GoalId};
use app::services::ServiceSet;
use domain::AccountKind;

use crate::html::escape;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Catalog {
    pub categories: Vec<Category>,
    pub accounts: Vec<Account>,
    pub goals: Vec<Goal>,
}

impl Catalog {
    pub async fn load(services: &ServiceSet) -> AppResult<Self> {
        let categories = services.categories.list(None).await?;
        let accounts = services.accounts.list(false).await?;
        let goals = services
            .goals
            .list_progress()
            .await?
            .into_iter()
            .map(|progress| progress.goal)
            .collect();
        Ok(Self { categories, accounts, goals })
    }

    pub fn categories_of(&self, kind: CategoryKind) -> Vec<&Category> {
        self.categories.iter().filter(|category| category.kind == kind).collect()
    }

    /// Accounts money can be spent from or land in (pots excluded).
    pub fn spendable_accounts(&self) -> Vec<&Account> {
        self.accounts.iter().filter(|account| account.kind.is_spendable()).collect()
    }

    pub fn category_label(&self, id: CategoryId) -> String {
        let found = self.categories.iter().find(|category| category.id == id);
        found.map_or_else(|| "categoria removida".into(), category_label)
    }

    pub fn account_label(&self, id: AccountId) -> String {
        let found = self.accounts.iter().find(|account| account.id == id);
        found.map_or_else(|| "conta removida".into(), account_label)
    }

    pub fn goal_label(&self, id: GoalId) -> String {
        let found = self.goals.iter().find(|goal| goal.id == id);
        found
            .map_or_else(|| "meta removida".into(), |goal| format!("🎯 {}", escape(&goal.pot.name)))
    }
}

pub fn category_label(category: &Category) -> String {
    let name = escape(&category.name);
    match &category.emoji {
        Some(emoji) => format!("{} {name}", escape(emoji)),
        None => name,
    }
}

pub fn account_label(account: &Account) -> String {
    format!("{} {}", kind_emoji(account.kind), escape(&account.name))
}

pub const fn kind_emoji(kind: AccountKind) -> &'static str {
    match kind {
        AccountKind::Checking => "🏦",
        AccountKind::Savings => "🐷",
        AccountKind::Cash => "💵",
        AccountKind::Pot => "🎯",
    }
}

pub const fn kind_name(kind: AccountKind) -> &'static str {
    match kind {
        AccountKind::Checking => "Conta corrente",
        AccountKind::Savings => "Poupança",
        AccountKind::Cash => "Dinheiro",
        AccountKind::Pot => "Caixinha",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use domain::Cents;

    fn account(name: &str, kind: AccountKind) -> Account {
        Account {
            id: AccountId::generate(),
            name: name.into(),
            kind,
            initial_balance: Cents::ZERO,
            opened_on: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            archived: false,
        }
    }

    #[test]
    fn labels_escape_names_and_fall_back_when_missing() {
        let checking = account("R&D <bank>", AccountKind::Checking);
        let catalog = Catalog { accounts: vec![checking.clone()], ..Catalog::default() };
        assert_eq!(catalog.account_label(checking.id), "🏦 R&amp;D &lt;bank&gt;");
        assert_eq!(catalog.account_label(AccountId::generate()), "conta removida");
        assert_eq!(catalog.category_label(CategoryId::generate()), "categoria removida");
        assert_eq!(catalog.goal_label(GoalId::generate()), "meta removida");
    }

    #[test]
    fn spendable_excludes_pots() {
        let catalog = Catalog {
            accounts: vec![
                account("Nubank", AccountKind::Checking),
                account("Casa", AccountKind::Pot),
            ],
            ..Catalog::default()
        };
        assert_eq!(catalog.spendable_accounts().len(), 1);
        assert_eq!(kind_name(AccountKind::Cash), "Dinheiro");
    }
}
