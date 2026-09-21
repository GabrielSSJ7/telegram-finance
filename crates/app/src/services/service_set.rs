//! Builds every service from the ports. Production (Postgres) and tests
//! (in-memory fake) share this wiring, so they cannot drift apart.

use std::sync::Arc;

use super::{
    AccountService, AllowedUsers, ApiKeyService, CategoryService, GoalService, LedgerService,
    MemberService, SettingsService,
};
use crate::ports::{
    AccountStore, ApiKeyStore, BotStateStore, CategoryStore, ChatFlowStore, Clock, EntryStore,
    GoalStore, MemberStore, SettingsStore, TokenSource,
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
}

impl ServiceSet {
    /// Wires services in dependency order.
    ///
    /// ```ignore
    /// let services = ServiceSet::wire(StorePorts::from_single(&pg_store), environment);
    /// ```
    pub fn wire(stores: StorePorts, environment: ServiceEnvironment) -> Self {
        let clock = environment.clock;
        let accounts = Arc::new(AccountService::new(stores.accounts, clock.clone()));
        let categories = Arc::new(CategoryService::new(stores.categories, clock.clone()));
        let ledger = Arc::new(LedgerService::new(
            stores.entries,
            accounts.clone(),
            categories.clone(),
            clock.clone(),
        ));
        let goals = Arc::new(GoalService::new(
            stores.goals,
            accounts.clone(),
            ledger.clone(),
            clock.clone(),
        ));
        let api_keys = Arc::new(ApiKeyService::new(stores.api_keys, environment.tokens, clock));
        let members = Arc::new(MemberService::new(stores.members, environment.allowed_users));
        let settings = Arc::new(SettingsService::new(stores.settings));
        Self { accounts, categories, ledger, goals, members, settings, api_keys }
    }
}
