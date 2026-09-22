-- Recurrences that end, such as financing or loans paid monthly from an
-- account (/parcelas). first_installment_no is the installment paid on the
-- first due date, above 1 for a plan already under way.
alter table recurrences
    add column installment_count smallint,
    add column first_installment_no smallint,
    add constraint recurrences_plan_shape check (
        (installment_count is null and first_installment_no is null)
        or (installment_count between 1 and 480
            and first_installment_no between 1 and installment_count));
