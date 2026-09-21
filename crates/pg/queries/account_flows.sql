-- Totals per (account, kind, role) of live entries up to a date; the app
-- applies the sign rules from domain::EntryKind::account_effect.
select account_id as "account_id!", kind as "kind!", 'primary' as "role!",
       sum(amount_cents)::bigint as "total!"
from ledger_entries
where deleted_at is null and accounting_date <= $1 and account_id is not null
group by account_id, kind
union all
select counter_account_id, kind, 'counter', sum(amount_cents)::bigint
from ledger_entries
where deleted_at is null and accounting_date <= $1 and counter_account_id is not null
group by counter_account_id, kind
