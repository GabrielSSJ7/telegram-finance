-- Essential categories make up the household's basic cost of living
-- (/custodevida). Only expense categories can be essential.
alter table categories add column essential boolean not null default false;
alter table categories add constraint categories_essential_is_expense
    check (not essential or kind = 'expense');

-- The usual needs among the seeded categories; the couple adjusts them
-- with /essenciais.
update categories set essential = true
 where kind = 'expense'
   and archived_at is null
   and lower(name) in ('casa', 'mercado', 'saúde', 'transporte', 'educação');
