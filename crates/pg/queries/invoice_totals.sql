-- Invoices of one card with the live sums the settlement rules need.
select invoices.id, invoices.card_id, invoices.reference_month, invoices.closing_date, invoices.due_date,
       coalesce(sum(entries.amount_cents) filter (where entries.kind = 'card_installment'), 0)::bigint as "charges!",
       coalesce(sum(entries.amount_cents) filter (where entries.kind = 'card_credit'), 0)::bigint as "credits!",
       coalesce(sum(entries.amount_cents) filter (where entries.kind = 'invoice_payment'), 0)::bigint as "payments!"
from card_invoices invoices
left join ledger_entries entries on entries.invoice_id = invoices.id and entries.deleted_at is null
where invoices.card_id = $1
group by invoices.id
order by invoices.closing_date, invoices.id
