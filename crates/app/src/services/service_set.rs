//! Builds every service from the ports. Production (Postgres) and tests
//! (in-memory fake) share this wiring, so they cannot drift apart.

use std::sync::Arc;

use super::budgets::BudgetSources;
use super::recurrences::RecurrenceDependencies;
use super::reports::ReportSources;
use super::{
    AccountService, AllowedUsers, ApiKeyService, BudgetService, CardService, CategoryService,
    GoalService, LedgerService, MemberService, PositionService, RecurrenceService, ReportService,
    SettingsService,
};
use crate::ports::{
    AccountStore, ApiKeyStore, BotStateStore, BudgetStore, CardStore, CategoryStore, ChatFlowStore,
    Clock, EntryStore, GoalStore, JobRunStore, MemberStore, RecurrenceStore, ReportStore,
    SettingsStore, TokenSource,
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
    pub recurrences: Arc<dyn RecurrenceStore>,
    pub job_runs: Arc<dyn JobRunStore>,
    pub reports: Arc<dyn ReportStore>,
    pub budgets: Arc<dyn BudgetStore>,
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
            + RecurrenceStore
            + JobRunStore
            + ReportStore
            + BudgetStore
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
            recurrences: store.clone(),
            job_runs: store.clone(),
            reports: store.clone(),
            budgets: store.clone(),
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
    pub recurrences: Arc<RecurrenceService>,
    pub reports: Arc<ReportService>,
    pub budgets: Arc<BudgetService>,
    /// The household clock, for callers that need "today".
    pub clock: Arc<dyn Clock>,
}

impl ServiceSet {
    /// Wires services in dependency order.
    ///
    /// ```ignore
    /// let services = ServiceSet::wire(&StorePorts::from_single(&pg_store), environment);
    /// ```
    pub fn wire(stores: &StorePorts, environment: ServiceEnvironment) -> Self {
        let clock = environment.clock.clone();
        let money = MoneyServices::wire(stores, &environment.clock);
        let people = PeopleServices::wire(stores, environment);
        let planning = money.planning(stores, &people);
        Self {
            accounts: money.accounts,
            categories: money.categories,
            ledger: money.ledger,
            goals: money.goals,
            cards: money.cards,
            position: money.position,
            members: people.members,
            settings: people.settings,
            api_keys: people.api_keys,
            recurrences: planning.recurrences,
            reports: planning.reports,
            budgets: planning.budgets,
            clock,
        }
    }
}

/// Who may use finbot and how it is set up.
struct PeopleServices {
    members: Arc<MemberService>,
    settings: Arc<SettingsService>,
    api_keys: Arc<ApiKeyService>,
}

impl PeopleServices {
    fn wire(stores: &StorePorts, environment: ServiceEnvironment) -> Self {
        let members =
            Arc::new(MemberService::new(stores.members.clone(), environment.allowed_users));
        let settings = Arc::new(SettingsService::new(stores.settings.clone()));
        let api_keys = Arc::new(ApiKeyService::new(
            stores.api_keys.clone(),
            environment.tokens,
            environment.clock,
        ));
        Self { members, settings, api_keys }
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
    clock: Arc<dyn Clock>,
}

impl MoneyServices {
    fn wire(stores: &StorePorts, clock: &Arc<dyn Clock>) -> Self {
        let accounts = Arc::new(AccountService::new(stores.accounts.clone(), clock.clone()));
        let categories = Arc::new(CategoryService::new(stores.categories.clone(), clock.clone()));
        let base = BaseServices { accounts, categories, clock: clock.clone() };
        let cards = Arc::new(base.cards(stores));
        let ledger = Arc::new(base.ledger(stores));
        let goals = Arc::new(base.goals(stores, &ledger));
        let position = Arc::new(base.position(&cards));
        let (accounts, categories, clock) = (base.accounts, base.categories, base.clock);
        Self { accounts, categories, ledger, goals, cards, position, clock }
    }

    fn recurrences(&self, stores: &StorePorts) -> RecurrenceService {
        let dependencies = RecurrenceDependencies {
            ledger: self.ledger.clone(),
            cards: self.cards.clone(),
            accounts: self.accounts.clone(),
            categories: self.categories.clone(),
        };
        RecurrenceService::new(stores.recurrences.clone(), dependencies, self.clock.clone())
    }

    fn planning(&self, stores: &StorePorts, people: &PeopleServices) -> PlanningServices {
        let recurrences = Arc::new(self.recurrences(stores));
        let budgets = Arc::new(self.budgets(stores, people));
        let reports = Arc::new(self.reports(stores, people, &recurrences, &budgets));
        PlanningServices { recurrences, budgets, reports }
    }

    fn budgets(&self, stores: &StorePorts, people: &PeopleServices) -> BudgetService {
        let sources = BudgetSources {
            reports: stores.reports.clone(),
            categories: self.categories.clone(),
            settings: people.settings.clone(),
        };
        BudgetService::new(stores.budgets.clone(), sources, self.clock.clone())
    }

    fn reports(
        &self,
        stores: &StorePorts,
        people: &PeopleServices,
        recurrences: &Arc<RecurrenceService>,
        budgets: &Arc<BudgetService>,
    ) -> ReportService {
        let sources = ReportSources {
            position: self.position.clone(),
            cards: self.cards.clone(),
            goals: self.goals.clone(),
            recurrences: recurrences.clone(),
            categories: self.categories.clone(),
            members: people.members.clone(),
            settings: people.settings.clone(),
            ledger: self.ledger.clone(),
            budgets: budgets.clone(),
        };
        ReportService::new(stores.reports.clone(), sources)
    }
}

/// Recurrences, budgets and the reports that read them.
struct PlanningServices {
    recurrences: Arc<RecurrenceService>,
    budgets: Arc<BudgetService>,
    reports: Arc<ReportService>,
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

    fn position(&self, cards: &Arc<CardService>) -> PositionService {
        PositionService::new(self.accounts.clone(), cards.clone(), self.clock.clone())
    }
}
