use std::collections::BTreeMap;

use async_trait::async_trait;
use chrono::NaiveDate;
use domain::{AccountKind, Cents, EntryKind};

use super::{InMemoryStore, MemoryState};
use crate::model::{AccountId, CategoryId, LedgerEntry, MemberId, PeriodFlow};
use crate::ports::{ReportStore, StoreResult};

type FlowKey = (Option<CategoryId>, Option<MemberId>, EntryKind);

#[async_trait]
impl ReportStore for InMemoryStore {
    async fn period_flows(
        &self,
        from: NaiveDate,
        to_exclusive: NaiveDate,
    ) -> StoreResult<Vec<PeriodFlow>> {
        let state = self.lock();
        let mut totals: BTreeMap<FlowKey, Cents> = BTreeMap::new();
        for entry in live_between(&state, from, to_exclusive) {
            *totals.entry((entry.category_id, entry.created_by, entry.kind)).or_default() +=
                entry.amount;
        }
        let flows = totals.into_iter().map(|((category_id, created_by, kind), total)| PeriodFlow {
            category_id,
            created_by,
            kind,
            total,
        });
        Ok(flows.collect())
    }

    async fn pot_net_inflow(&self, from: NaiveDate, to_exclusive: NaiveDate) -> StoreResult<Cents> {
        let state = self.lock();
        let is_pot = |id: Option<AccountId>| {
            state
                .accounts
                .iter()
                .any(|account| Some(account.id) == id && account.kind == AccountKind::Pot)
        };
        let transfers = live_between(&state, from, to_exclusive)
            .filter(|entry| entry.kind == EntryKind::Transfer);
        let net = transfers.map(|entry| {
            let into = if is_pot(entry.counter_account_id) { entry.amount } else { Cents::ZERO };
            let out_of = if is_pot(entry.account_id) { entry.amount } else { Cents::ZERO };
            into - out_of
        });
        Ok(net.sum())
    }
}

fn live_between(
    state: &MemoryState,
    from: NaiveDate,
    to_exclusive: NaiveDate,
) -> impl Iterator<Item = &LedgerEntry> {
    let in_period = move |entry: &&LedgerEntry| {
        entry.accounting_date >= from && entry.accounting_date < to_exclusive
    };
    state.entries.iter().filter(|entry| !entry.deleted).filter(in_period)
}
