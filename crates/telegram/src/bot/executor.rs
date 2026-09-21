//! Runs a confirmed form against the services.

use app::AppResult;
use app::model::{Account, Goal, LedgerEntry};
use app::services::{EntryOrigin, ServiceSet};

use crate::flows::{FormCommand, FormKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Committed {
    Entry(LedgerEntry),
    Account(Account),
    Goal(Goal),
}

pub async fn execute(
    services: &ServiceSet,
    command: FormCommand,
    origin: EntryOrigin,
) -> AppResult<Committed> {
    match command {
        FormCommand::Record(request) => {
            services.ledger.record(request, origin).await.map(Committed::Entry)
        }
        FormCommand::PotDeposit(request) => {
            services.goals.deposit(request, origin).await.map(Committed::Entry)
        }
        FormCommand::PotWithdraw(request) => {
            services.goals.withdraw(request, origin).await.map(Committed::Entry)
        }
        FormCommand::OpenAccount(request) => {
            services.accounts.open(request).await.map(Committed::Account)
        }
        FormCommand::CreateGoal(request) => {
            services.goals.create(request).await.map(Committed::Goal)
        }
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
    }
}
