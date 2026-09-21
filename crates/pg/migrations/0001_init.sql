-- finbot schema. One deployment serves one household (the couple), so the
-- household is a singleton row and other tables carry no household_id.
-- Amounts are integer cents. Dates that matter for accounting are `date`
-- values computed by the app in the household timezone; SQL never derives
-- them from now(), because UTC turns over at 21:00 in Sao Paulo.

create table household (
    id                smallint primary key default 1 check (id = 1),
    telegram_chat_id  bigint unique,
    cycle_start_day   smallint not null default 1 check (cycle_start_day between 1 and 31),
    daily_report_time time not null default '21:00',
    created_at        timestamptz not null default now()
);
insert into household default values;

create table members (
    id               uuid primary key default uuidv7(),
    telegram_user_id bigint not null unique,
    display_name     text not null check (length(display_name) between 1 and 64),
    dm_chat_id       bigint,
    receives_backups boolean not null default true,
    created_at       timestamptz not null default now()
);

create table accounts (
    id                    uuid primary key default uuidv7(),
    name                  text not null check (length(name) between 1 and 40),
    kind                  text not null check (kind in ('checking', 'savings', 'cash', 'pot')),
    initial_balance_cents bigint not null default 0,
    opened_on             date not null,
    archived_at           timestamptz,
    created_at            timestamptz not null default now()
);
create unique index accounts_active_name on accounts (lower(name)) where archived_at is null;

-- A goal is the target of one pot account ("caixinha").
create table goals (
    id           uuid primary key default uuidv7(),
    account_id   uuid not null unique references accounts (id),
    target_cents bigint not null check (target_cents > 0),
    target_date  date,
    created_at   timestamptz not null default now()
);

create table categories (
    id          uuid primary key default uuidv7(),
    name        text not null check (length(name) between 1 and 32),
    kind        text not null check (kind in ('expense', 'income')),
    emoji       text,
    archived_at timestamptz,
    created_at  timestamptz not null default now()
);
create unique index categories_active_name on categories (kind, lower(name)) where archived_at is null;

insert into categories (name, kind, emoji) values
    ('alimentação', 'expense', '🍽️'),
    ('mercado', 'expense', '🛒'),
    ('casa', 'expense', '🏠'),
    ('transporte', 'expense', '🚗'),
    ('saúde', 'expense', '💊'),
    ('lazer', 'expense', '🎉'),
    ('educação', 'expense', '📚'),
    ('assinaturas', 'expense', '📺'),
    ('pets', 'expense', '🐾'),
    ('outros', 'expense', '📦'),
    ('salário', 'income', '💰'),
    ('extra', 'income', '✨'),
    ('rendimentos', 'income', '📈');

create table credit_cards (
    id                         uuid primary key default uuidv7(),
    name                       text not null check (length(name) between 1 and 40),
    closing_day                smallint not null check (closing_day between 1 and 31),
    due_day                    smallint not null check (due_day between 1 and 31),
    closing_day_goes_next      boolean not null default true,
    limit_cents                bigint check (limit_cents > 0),
    default_payment_account_id uuid references accounts (id),
    archived_at                timestamptz,
    created_at                 timestamptz not null default now()
);
create unique index credit_cards_active_name on credit_cards (lower(name)) where archived_at is null;

-- Invoice dates are stored (and editable) because banks move them around
-- weekends and holidays. Status (open/closed/paid) is derived, never stored.
create table card_invoices (
    id              uuid primary key default uuidv7(),
    card_id         uuid not null references credit_cards (id),
    reference_month date not null check (extract(day from reference_month) = 1),
    closing_date    date not null,
    due_date        date not null check (due_date > closing_date),
    created_at      timestamptz not null default now(),
    unique (card_id, reference_month)
);

create table card_purchases (
    id                   uuid primary key default uuidv7(),
    card_id              uuid not null references credit_cards (id),
    description          text not null default '',
    category_id          uuid references categories (id),
    total_cents          bigint not null check (total_cents > 0),
    installment_count    smallint not null check (installment_count between 1 and 48),
    first_installment_no smallint not null default 1
        check (first_installment_no between 1 and installment_count),
    purchased_on         date not null,
    created_by           uuid references members (id),
    created_at           timestamptz not null default now(),
    deleted_at           timestamptz
);

create table recurrences (
    id                uuid primary key default uuidv7(),
    kind              text not null check (kind in ('income', 'expense')),
    amount_cents      bigint not null check (amount_cents > 0),
    description       text not null default '',
    category_id       uuid references categories (id),
    account_id        uuid references accounts (id),
    card_id           uuid references credit_cards (id),
    day_of_month      smallint not null check (day_of_month between 1 and 31),
    mode              text not null default 'auto' check (mode in ('auto', 'confirm')),
    active            boolean not null default true,
    starts_on         date not null,
    last_generated_on date,
    created_at        timestamptz not null default now(),
    check ((account_id is null) <> (card_id is null)),
    check (kind = 'expense' or card_id is null)
);

create table ledger_entries (
    id                 uuid primary key default uuidv7(),
    kind               text not null check (kind in (
        'income', 'expense', 'transfer', 'card_installment', 'card_credit',
        'invoice_payment', 'refund', 'adjust_in', 'adjust_out')),
    amount_cents       bigint not null check (amount_cents > 0),
    description        text not null default '',
    category_id        uuid references categories (id),
    account_id         uuid references accounts (id),
    counter_account_id uuid references accounts (id),
    card_purchase_id   uuid references card_purchases (id),
    installment_no     smallint,
    invoice_id         uuid references card_invoices (id),
    accounting_date    date not null,
    recurrence_id      uuid references recurrences (id),
    created_by         uuid references members (id),
    created_at         timestamptz not null default now(),
    deleted_at         timestamptz,
    constraint ledger_entries_shape check (
        case kind
            when 'transfer' then
                account_id is not null and counter_account_id is not null
                and account_id <> counter_account_id
                and category_id is null and invoice_id is null and card_purchase_id is null
            when 'card_installment' then
                account_id is null and counter_account_id is null
                and card_purchase_id is not null and invoice_id is not null and installment_no is not null
            when 'card_credit' then
                account_id is null and counter_account_id is null and invoice_id is not null
            when 'invoice_payment' then
                account_id is not null and counter_account_id is null
                and invoice_id is not null and category_id is null and card_purchase_id is null
            else
                account_id is not null and counter_account_id is null
                and invoice_id is null and card_purchase_id is null
        end
    )
);
-- Not partial on deleted_at: an auto-generated entry the couple undid must
-- never be generated again.
create unique index ledger_entries_recurrence_day
    on ledger_entries (recurrence_id, accounting_date) where recurrence_id is not null;
create index ledger_entries_accounting_date on ledger_entries (accounting_date) where deleted_at is null;
create index ledger_entries_account on ledger_entries (account_id) where deleted_at is null;
create index ledger_entries_counter_account on ledger_entries (counter_account_id) where deleted_at is null;
create index ledger_entries_invoice on ledger_entries (invoice_id) where deleted_at is null;
create index ledger_entries_created_by on ledger_entries (created_by, created_at desc);

-- Idempotency keys: a bot flow's draft id or a REST Idempotency-Key. Inserted
-- in the same transaction as the command, so a double tap saves once.
create table committed_drafts (
    draft_id     uuid primary key,
    committed_at timestamptz not null default now()
);

create table category_budgets (
    id          uuid primary key default uuidv7(),
    category_id uuid not null unique references categories (id),
    limit_cents bigint not null check (limit_cents > 0),
    created_at  timestamptz not null default now()
);

create table budget_alerts (
    budget_id     uuid not null references category_budgets (id) on delete cascade,
    cycle_start   date not null,
    threshold_pct smallint not null check (threshold_pct in (80, 100)),
    sent_at       timestamptz not null default now(),
    primary key (budget_id, cycle_start, threshold_pct)
);

-- In-progress guided flows, keyed by (chat, user) so both spouses can type
-- at the same time in the same group.
create table chat_flows (
    chat_id           bigint not null,
    user_id           bigint not null,
    flow              jsonb not null,
    prompt_message_id bigint,
    draft_id          uuid not null,
    expires_at        timestamptz not null,
    primary key (chat_id, user_id)
);

create table bot_state (
    key   text primary key,
    value bigint not null
);

create table job_runs (
    job        text not null,
    run_date   date not null,
    status     text not null check (status in ('running', 'succeeded', 'failed')),
    attempts   integer not null default 1,
    updated_at timestamptz not null default now(),
    primary key (job, run_date)
);

create table api_keys (
    id           uuid primary key default uuidv7(),
    name         text not null unique check (length(name) between 1 and 64),
    token_sha256 bytea not null unique check (length(token_sha256) = 32),
    created_at   timestamptz not null default now(),
    revoked_at   timestamptz,
    last_used_at timestamptz
);
