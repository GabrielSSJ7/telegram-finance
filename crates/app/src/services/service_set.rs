//! Builds every service from the ports. Production (Postgres) and tests
//! (in-memory fake) share this wiring, so they cannot drift apart.

use std::sync::Arc;

use super::{
    AccountService, AllowedUsers, ApiKeyService, CardService, CategoryService, GoalService,
    LedgerService, MemberService, PositionService, SettingsService,
};
use crate::ports::{
    AccountStore, ApiKeyStore, BotStateStore, CardStore, CategoryStore, ChatFlowStore, Clock,
    EntryStore, GoalStore, MemberStore, SettingsStore, TokenSource,
};

/// One handle per store port.
#[derive(Clone)]
pub struct StorePorts {
    pub accounts: Arc<dyn AccountStore>,
    pub categories: Arc<dyn CategoryStore>,
    pub entries: Arc<dyn EntryStore>,
    pub goals: Arc<dyn GoalStore>,
    pub members: Arc<dyn MemberStore>,
    pub settings: Arc<dyn SettingsStore>,
    pub api_keys: Arc<dyn ApiKeyStore>,
    pub flows: Arc<dyn ChatFlowStore>,
    pub bot_state: Arc<dyn BotStateStore>,
    pub cards: Arc<dyn CardStore>,
}

impl StorePorts {
    /// All ports served by one adapter that implements every store trait.
    pub fn from_single<S>(store: &Arc<S>) -> Self
    where
        S: AccountStore
            + CategoryStore
            + EntryStore
            + GoalStore
            + MemberStore
            + SettingsStore
            + ApiKeyStore
            + ChatFlowStore
            + BotStateStore
            + CardStore
            + 'static,
    {
        Self {
            accounts: store.clone(),
            categories: store.clone(),
            entries: store.clone(),
            goals: store.clone(),
            members: store.clone(),
            settings: store.clone(),
            api_keys: store.clone(),
            flows: store.clone(),
            bot_state: store.clone(),
            cards: store.clone(),
        }
    }
}

/// Everything outside the stores that services need.
#[derive(Clone)]
pub struct ServiceEnvironment {
    pub clock: Arc<dyn Clock>,
    pub tokens: Arc<dyn TokenSource>,
    pub allowed_users: AllowedUsers,
}

#[derive(Clone)]
pub struct ServiceSet {
    pub accounts: Arc<AccountService>,
    pub categories: Arc<CategoryService>,
    pub ledger: Arc<LedgerService>,
    pub goals: Arc<GoalService>,
    pub members: Arc<MemberService>,
    pub settings: Arc<SettingsService>,
    pub api_keys: Arc<ApiKeyService>,
    pub cards: Arc<CardService>,
    pub position: Arc<PositionService>,
}

impl ServiceSet {
    /// Wires services in dependency order.
    ///
    /// ```ignore
    /// let services = ServiceSet::wire(StorePorts::from_single(&pg_store), environment);
    /// ```
    pub fn wire(stores: StorePorts, environment: ServiceEnvironment) -> Self {
        let money = MoneyServices::wire(&stores, &environment.clock);
        let clock = environment.clock;
        Self {
            api_keys: Arc::new(ApiKeyService::new(stores.api_keys, environment.tokens, clock)),
            members: Arc::new(MemberService::new(stores.members, environment.allowed_users)),
            settings: Arc::new(SettingsService::new(stores.settings)),
            accounts: money.accounts,
            categories: money.categories,
            ledger: money.ledger,
            goals: money.goals,
            cards: money.cards,
            position: money.position,
        }
    }
}

/// The services that move or report money, which depend on each other.
struct MoneyServices {
    accounts: Arc<AccountService>,
    categories: Arc<CategoryService>,
    ledger: Arc<LedgerService>,
    goals: Arc<GoalService>,
    cards: Arc<CardService>,
    position: Arc<PositionService>,
}

impl MoneyServices {
    fn wire(stores: &StorePorts, clock: &Arc<dyn Clock>) -> Self {
        let accounts = Arc::new(AccountService::new(stores.accounts.clone(), clock.clone()));
        let categories = Arc::new(CategoryService::new(stores.categories.clone(), clock.clone()));
        let base = BaseServices { accounts, categories, clock: clock.clone() };
        let cards = Arc::new(base.cards(stores));
        let ledger = Arc::new(base.ledger(stores));
        let goals = Arc::new(base.goals(stores, &ledger));
        let position = Arc::new(PositionService::new(
            base.accounts.clone(),
            cards.clone(),
            base.clock.clone(),
        ));
        Self {
            accounts: base.accounts,
            categories: base.categories,
            ledger,
            goals,
            cards,
            position,
        }
    }
}

/// Services the others are built on.
struct BaseServices {
    accounts: Arc<AccountService>,
    categories: Arc<CategoryService>,
    clock: Arc<dyn Clock>,
}

impl BaseServices {
    fn cards(&self, stores: &StorePorts) -> CardService {
        CardService::new(
            stores.cards.clone(),
            self.accounts.clone(),
            self.categories.clone(),
            self.clock.clone(),
        )
    }

    fn ledger(&self, stores: &StorePorts) -> LedgerService {
        let (accounts, categories) = (self.accounts.clone(), self.categories.clone());
        LedgerService::new(
            stores.entries.clone(),
            stores.cards.clone(),
            accounts,
            categories,
            self.clock.clone(),
        )
    }

    fn goals(&self, stores: &StorePorts, ledger: &Arc<LedgerService>) -> GoalService {
        GoalService::new(
            stores.goals.clone(),
            self.accounts.clone(),
            ledger.clone(),
            self.clock.clone(),
        )
    }
}
