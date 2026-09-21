select category_id, created_by, kind as "kind!", sum(amount_cents)::bigint as "total!"
from ledger_entries
where deleted_at is null and accounting_date >= $1 and accounting_date < $2
group by category_id, created_by, kind
order by kind, category_id, created_by
