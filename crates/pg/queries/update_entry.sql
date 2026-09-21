-- Null parameters keep the current value.
update ledger_entries set
    amount_cents = coalesce($2, amount_cents),
    description = coalesce($3, description),
    category_id = coalesce($4, category_id),
    accounting_date = coalesce($5, accounting_date)
where id = $1 and deleted_at is null
returning id, kind, amount_cents, description, category_id, account_id, counter_account_id,
          accounting_date, created_by, created_at, deleted_at
