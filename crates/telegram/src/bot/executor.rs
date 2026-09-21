//! Runs a confirmed form against the services.

use app::AppResult;
use app::model::{
    Account, Budget, CardPurchase, CategoryId, CreditCard, Goal, HouseholdSettings, LedgerEntry,
    Recurrence,
};
use app::services::{EntryOrigin, ServiceSet};
use domain::Cents;

use crate::flows::{FormCommand, FormKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Committed {
    Entry(LedgerEntry),
    Purchase(CardPurchase),
    Account(Account),
    Goal(Goal),
    Card(CreditCard),
    Recurrence(Recurrence),
    Budget(Budget),
    BudgetRemoved,
    Settings(HouseholdSettings),
}

pub async fn execute(
    services: &ServiceSet,
    command: FormCommand,
    origin: EntryOrigin,
) -> AppResult<Committed> {
    match command {
        FormCommand::CardPurchase(request) => {
            services.cards.purchase(request, origin).await.map(Committed::Purchase)
        }
        FormCommand::EditEntry { entry, patch } => {
            services.ledger.update(entry, patch).await.map(Committed::Entry)
        }
        FormCommand::Record(_)
        | FormCommand::PotDeposit(_)
        | FormCommand::PotWithdraw(_)
        | FormCommand::CardCredit(_)
        | FormCommand::PayInvoice(_)
        | FormCommand::Reconcile(_) => {
            record_money(services, command, origin).await.map(Committed::Entry)
        }
        setup => configure(services, setup).await,
    }
}

/// Commands that create or change accounts, goals, cards, recurrences
/// and budgets.
async fn configure(services: &ServiceSet, command: FormCommand) -> AppResult<Committed> {
    match command {
        FormCommand::OpenAccount(request) => {
            services.accounts.open(request).await.map(Committed::Account)
        }
        FormCommand::CreateGoal(request) => {
            services.goals.create(request).await.map(Committed::Goal)
        }
        FormCommand::OpenCard(request) => services.cards.open(request).await.map(Committed::Card),
        FormCommand::CreateRecurrence(request) => {
            services.recurrences.create(request).await.map(Committed::Recurrence)
        }
        limits => household_rules(services, limits).await,
    }
}

/// Budgets and household settings.
async fn household_rules(services: &ServiceSet, command: FormCommand) -> AppResult<Committed> {
    match command {
        FormCommand::SetBudget { category, limit } => set_budget(services, category, limit).await,
        FormCommand::UpdateSettings(patch) => {
            services.settings.update(patch).await.map(Committed::Settings)
        }
        other => {
            Err(app::AppError::invalid("form command", format!("{other:?}"), "a setup command"))
        }
    }
}

/// A zero limit removes the budget.
async fn set_budget(
    services: &ServiceSet,
    category: CategoryId,
    limit: Cents,
) -> AppResult<Committed> {
    if limit.is_positive() {
        return services.budgets.set(category, limit).await.map(Committed::Budget);
    }
    services.budgets.remove(category).await.map(|()| Committed::BudgetRemoved)
}

/// Commands that become one ledger row.
async fn record_money(
    services: &ServiceSet,
    command: FormCommand,
    origin: EntryOrigin,
) -> AppResult<LedgerEntry> {
    match command {
        FormCommand::Record(request) => services.ledger.record(request, origin).await,
        FormCommand::PotDeposit(request) => services.goals.deposit(request, origin).await,
        FormCommand::PotWithdraw(request) => services.goals.withdraw(request, origin).await,
        FormCommand::CardCredit(request) => services.cards.credit(request, origin).await,
        FormCommand::PayInvoice(request) => services.cards.pay_invoice(request, origin).await,
        FormCommand::Reconcile(request) => services.adjustments.reconcile(request, origin).await,
        other => Err(app::AppError::invalid(
            "form command",
            format!("{other:?}"),
            "a command that records money",
        )),
    }
}

/// Headline of the confirmation card; removals read as such.
pub const fn headline_for(form: FormKind, committed: &Committed) -> &'static str {
    if matches!(committed, Committed::BudgetRemoved) {
        return "Orçamento removido";
    }
    headline(form)
}

pub const fn headline(form: FormKind) -> &'static str {
    match form {
        FormKind::Expense => "Gasto registrado",
        FormKind::Income => "Entrada registrada",
        FormKind::Transfer => "Transferência registrada",
        FormKind::PotDeposit => "Guardado na meta",
        FormKind::PotWithdraw => "Resgatado da meta",
        FormKind::NewAccount => "Conta criada",
        FormKind::NewGoal => "Meta criada",
        FormKind::NewCard => "Cartão cadastrado",
        FormKind::PayInvoice => "Pagamento registrado",
        FormKind::Refund => "Estorno registrado",
        FormKind::NewRecurrence => "Recorrência criada",
        FormKind::SetBudget => "Orçamento salvo",
        FormKind::EditEntry => "Lançamento alterado",
        FormKind::Adjust => "Saldo ajustado",
        FormKind::Settings => "Configuração salva",
    }
}
