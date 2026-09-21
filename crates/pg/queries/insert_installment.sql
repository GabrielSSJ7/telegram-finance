insert into ledger_entries
    (kind, amount_cents, description, category_id, card_purchase_id, installment_no, invoice_id, accounting_date, created_by)
values ('card_installment', $1, $2, $3, $4, $5, $6, $7, $8)
