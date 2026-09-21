-- Inserts the invoice unless one exists for (card, reference month); the
-- second branch returns the existing row, keeping its stored dates.
with inserted as (
    insert into card_invoices (card_id, reference_month, closing_date, due_date)
    values ($1, $2, $3, $4)
    on conflict (card_id, reference_month) do nothing
    returning id, card_id, reference_month, closing_date, due_date
)
select id as "id!", card_id as "card_id!", reference_month as "reference_month!",
       closing_date as "closing_date!", due_date as "due_date!"
from inserted
union all
select id, card_id, reference_month, closing_date, due_date
from card_invoices
where card_id = $1 and reference_month = $2
limit 1
