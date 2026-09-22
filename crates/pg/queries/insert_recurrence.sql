insert into recurrences
    (kind, amount_cents, description, category_id, account_id, card_id, day_of_month, mode, starts_on,
     installment_count, first_installment_no)
values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
returning id, kind, amount_cents, description, category_id, account_id, card_id, day_of_month, mode,
          active, starts_on, last_generated_on, installment_count, first_installment_no
