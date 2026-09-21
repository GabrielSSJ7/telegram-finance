//! Runs a confirmed form against the services.

use app::AppResult;
use app::model::{Account, CardPurchase, CreditCard, Goal, LedgerEntry, Recurrence};
use app::services::{EntryOrigin, ServiceSet};

use crate::flows::{FormCommand, FormKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Committed {
    Entry(LedgerEntry),
    Purchase(CardPurchase),
    Account(Account),
    Goal(Goal),
    Card(CreditCard),
    Recurrence(Recurrence),
}

pub async fn execute(
    services: &ServiceSet,
    command: FormCommand,
    origin: EntryOrigin,
) -> AppResult<Committed> {
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
        FormCommand::CardPurchase(request) => {
            services.cards.purchase(request, origin).await.map(Committed::Purchase)
        }
        money => record_money(services, money, origin).await.map(Committed::Entry),
    }
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
        other => Err(app::AppError::invalid(
            "form command",
            format!("{other:?}"),
            "a command that records money",
        )),
    }
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
    }
}
