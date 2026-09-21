-- Pot account and goal in one statement (called inside a transaction).
with pot as (
    insert into accounts (name, kind, initial_balance_cents, opened_on) values ($1, $2, $3, $4)
    returning id, name, kind, initial_balance_cents, opened_on, archived_at
), goal as (
    insert into goals (account_id, target_cents, target_date)
    select id, $5, $6 from pot
    returning id, target_cents, target_date
)
select goal.id as "goal_id!", goal.target_cents as "target_cents!", goal.target_date,
       pot.id as "id!", pot.name as "name!", pot.kind as "kind!",
       pot.initial_balance_cents as "initial_balance_cents!", pot.opened_on as "opened_on!",
       pot.archived_at
from goal, pot
