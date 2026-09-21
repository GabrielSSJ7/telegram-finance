insert into card_purchases
    (card_id, description, category_id, total_cents, installment_count, first_installment_no, purchased_on, created_by)
values ($1, $2, $3, $4, $5, $6, $7, $8)
returning id, card_id, description, category_id, total_cents, installment_count, first_installment_no,
          purchased_on, created_by, deleted_at
