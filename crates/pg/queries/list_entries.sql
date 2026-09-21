-- Optional filters: a null parameter matches everything.
select id, kind, amount_cents, description, category_id, account_id, counter_account_id,
       accounting_date, created_by, created_at, deleted_at
from ledger_entries
where deleted_at is null
  and ($1::date is null or accounting_date >= $1)
  and ($2::date is null or accounting_date <= $2)
  and ($3::text is null or kind = $3)
  and ($4::uuid is null or account_id = $4 or counter_account_id = $4)
  and ($5::uuid is null or category_id = $5)
  and ($6::uuid is null or created_by = $6)
order by accounting_date desc, id desc
limit $7
