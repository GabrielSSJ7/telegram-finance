insert into ledger_entries
    (kind, amount_cents, description, category_id, account_id, counter_account_id, invoice_id, accounting_date, created_by)
values ($1, $2, $3, $4, $5, $6, $7, $8, $9)
returning id, kind, amount_cents, description, category_id, account_id, counter_account_id,
          card_purchase_id, installment_no, invoice_id, accounting_date, created_by, created_at, deleted_at
