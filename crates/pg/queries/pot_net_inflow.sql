-- Transfers into pots minus transfers out of pots in the period.
select coalesce(
           sum(case when destination.kind = 'pot' then entries.amount_cents else 0 end)
         - sum(case when source.kind = 'pot' then entries.amount_cents else 0 end), 0)::bigint as "net!"
from ledger_entries entries
join accounts source on source.id = entries.account_id
join accounts destination on destination.id = entries.counter_account_id
where entries.deleted_at is null and entries.kind = 'transfer'
  and entries.accounting_date >= $1 and entries.accounting_date < $2
