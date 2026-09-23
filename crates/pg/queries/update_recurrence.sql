update recurrences
   set amount_cents = coalesce($2, amount_cents),
       day_of_month = coalesce($3, day_of_month),
       mode = coalesce($4, mode)
 where id = $1 and active
returning id, kind, amount_cents, description, category_id, account_id, card_id, day_of_month, mode,
          active, starts_on, last_generated_on, installment_count, first_installment_no
