insert into ledger_entries
    (kind, amount_cents, description, category_id, account_id, counter_account_id, accounting_date, created_by)
values ($1, $2, $3, $4, $5, $6, $7, $8)
returning id, kind, amount_cents, description, category_id, account_id, counter_account_id,
          accounting_date, created_by, created_at, deleted_at
