# finbot

Personal finance for a couple, run from a shared Telegram group. You record
money in and out with guided commands (`/gasto`, `/entrada`, ...). A Rust
service stores each entry in Postgres, sends a daily balance, and sends a
closing report at the end of each cycle (a "month" that starts on a day you
choose, such as payday). A REST API exposes the same data.

## Status

| Phase | Scope | State |
|---|---|---|
| 1 | Workspace and domain rules (money, cycles, card invoices, installments, balances) | done |
| 2 | Postgres schema, core services, REST API | done |
| 3 | Telegram bot MVP | done |
| 4 | Docker, Caddy, backups, deploy | done |
| 5 | Credit cards and invoices | done |
| 6 | Scheduler and reports | done |
| 7 | Budgets and goals | done |
| 8 | Edit and delete, CSV export, balance adjustment, settings in chat, hardening | done |

## Layout

```
crates/
  domain/   pure finance rules, no I/O
  app/      use cases (services) over store ports; `test-support` feature
            ships the in-memory fake and the store contract suite
  pg/       Postgres adapter (sqlx), migrations/, queries/
  http/     REST API (axum, problem+json errors, OpenAPI)
  telegram/ bot: long-poll loop, guided flows (pure state machines),
            pt-BR cards, frankenstein client behind a project trait
  finbot/   binary: config, wiring, CLI
```

The in-memory fake and Postgres run the same store contract
(`app::fakes::contract`), so service tests on the fake stay honest.

## Development

Needs Rust 1.95+, Docker, and `sqlx-cli` 0.9 for `make sqlx-prepare`.

```sh
make test          # every test (starts a throwaway Postgres on :54329)
make coverage      # tests under coverage; fails below 90% of lines
make lint          # rustfmt + clippy (warnings are errors)
make deny          # licenses and advisories
make dev           # API from source on :8080, Swagger at /docs
make sqlx-prepare  # refresh .sqlx/ after changing a query
```

## CLI

```sh
finbot serve                    # migrate, then run the API, the bot and the scheduler
finbot migrate                  # apply migrations only
finbot api-key create <name>    # prints the token once
finbot api-key revoke <name>
finbot healthcheck              # exit 0 when /healthz answers 200
finbot run-job <job> [--date D] # run a scheduled job now
```

## Scheduled messages

When the bot runs, a scheduler checks every minute (household timezone) and
runs each job once per day, recorded in `job_runs` so restarts never repeat
a message:

| Job | When | What |
|---|---|---|
| recurrences | 06:00 | Records due recurring entries (backfills up to 3 months); bills set to "ask first" get [Registrar] [Pular] buttons |
| invoice_events | 09:00 | Invoice closed today; unpaid invoice due in 3 days or today |
| yesterday_report | yesterday's summary time, any hour (default 09:00) | All of yesterday's entries, the cycle so far, balances, cards, goals, what is coming |
| today_report | today's summary time, 19:00 or later (default 21:00) | The same for the day that is ending, once most of it is recorded |
| cycle_report | yesterday's summary time, first day of a cycle | Closing of the financial month: income, spending, savings rate, categories, essential vs other vs saved, per person |
| backup_watch | 10:00 | Private-chat warning when no backup succeeded in 26 hours |

`finbot run-job today-report --date 2026-10-05` runs one job immediately.
Both summary times are set with `/config` in the group.

## Deploy

Production runs from `compose.yml`: Caddy (HTTPS), finbot, Postgres 18 and a
backup sidecar. Images are built by GitHub Actions on `v*` tags and pushed to
GHCR; the VPS only pulls.

- [docs/deploy-hostinger.md](docs/deploy-hostinger.md): VPS setup and releases
- [docs/deploy-shared-nginx.md](docs/deploy-shared-nginx.md): same VPS as another
  stack whose nginx owns ports 80/443 (no Caddy)
- [docs/backup-restore.md](docs/backup-restore.md): encrypted backups and restore
- [docs/setup-telegram.md](docs/setup-telegram.md): BotFather and group setup

## Configuration

See `.env.example`. Any secret `X` can be given as `X_FILE` (Docker secrets).
Telegram setup: [docs/setup-telegram.md](docs/setup-telegram.md).

## Bot commands

| Command | What it does |
|---|---|
| `/gasto` | Guided expense: value, description, category, account or card (with installments), date. A card plan already under way: type `3/10` at the installments step, the value is then per installment and the date is the original purchase's |
| `/entrada` | Guided income |
| `/transferir` | Move money between accounts |
| `/guardar`, `/resgatar` | Move money into or out of a goal's pot |
| `/pagarfatura` | Pay (part of) a card invoice from an account |
| `/estorno` | Refund back to an account or a card invoice |
| `/fatura`, `/cartoes` | Card invoices (open, due, future installments); cards |
| `/novaconta`, `/novameta`, `/novocartao` | Create an account, a savings goal (optional deadline shows the monthly pace) or a card |
| `/novacategoria` | Create an expense or income category, with an optional emoji (`/nova-categoria` also works when typed) |
| `/saldo`, `/resumo`, `/ontem`, `/contas`, `/metas`, `/categorias` | Reports (`/resumo` and `/ontem` are today's and yesterday's summaries on demand; `/resumo 15/09` shows any earlier day) |
| `/recorrente`, `/recorrentes` | Create a monthly entry (salary, rent, subscription), optionally with a number of installments (financing, loan) and how many are already paid; list and deactivate |
| `/parcelas` | Card purchases and financings still being paid: paid so far, what is left, monthly installment and last month |
| `/orcamento`, `/orcamentos` | Set (or remove) a category's limit per cycle; see how much of each is used. Alerts at 80% and 100% |
| `/mes` | The current cycle so far |
| `/custodevida [mm/aaaa]` | Basic cost of living: essential spending so far and still coming, the average of the last 3 cycles, share of income and the 6-month emergency reserve |
| `/essenciais` | Mark which expense categories are essential (casa, mercado, saúde, transporte and educação start marked) |
| `/desfazer` | Undo your own last entry |
| `/ultimos` | Last 10 entries with ✏️ edit (value, description, category or date) and 🗑️ delete; only the author (or anyone, for automatic entries) |
| `/ajuste` | Make an account match the bank: type the real balance, the difference is recorded as an adjustment (not income or spending) |
| `/extrato [mm/aaaa]` | Spending and income per category for the cycle; each category button lists its entries |
| `/exportar [mm/aaaa]` | The cycle's entries as a CSV spreadsheet (`;`, decimal comma) |
| `/config` | Day the cycle starts; times of yesterday's summary (any hour) and today's (19:00 or later) |
| `/cancelar`, `/ajuda` | Cancel the current form, list commands |

## REST API

`/api/v1/*` needs `Authorization: Bearer fbk_…`. Errors are
`application/problem+json`. POSTs that move money (entries, goal deposits
and withdrawals, card purchases, credits and payments, reconciliations)
accept an `Idempotency-Key` (UUID): a retry with the same key returns 409
instead of saving twice. Amounts are integer cents; dates are ISO 8601.

| Resource | Endpoints |
|---|---|
| Accounts | `GET/POST /accounts`, `DELETE /accounts/{id}`, `GET /accounts/balances`, `POST /accounts/{id}/reconcile` |
| Entries | `GET/POST /entries` (filters: dates, kind, category, account, card), `GET/PATCH/DELETE /entries/{id}` |
| Categories | `GET/POST /categories`, `PATCH /categories/{id}` (`essential`), `DELETE /categories/{id}` |
| Cards | `GET/POST /cards`, `DELETE /cards/{id}`, `GET /cards/summaries`, `GET /cards/{id}/invoices`, `POST /cards/{id}/purchases`, `DELETE /card-purchases/{id}`, `POST /cards/{id}/credits`, `POST /invoices/{id}/payments` |
| Goals | `GET/POST /goals`, `PUT /goals/{id}/target`, `POST /goals/{id}/deposits`, `POST /goals/{id}/withdrawals` |
| Installments | `GET /installments` |
| Planning | `GET/POST /recurrences` (`installment_count`, `installments_paid`), `DELETE /recurrences/{id}`, `GET /budgets`, `PUT/DELETE /budgets/{category_id}` |
| Reports | `GET /reports/daily?date=`, `GET /reports/cycle?date=`, `GET /reports/living-cost?date=` |
| Exports | `GET /exports/entries.csv` (cycle containing `date`, default today; or `from` and `to`) |
| Household | `GET/PATCH /settings`, `GET /members` |

`GET /healthz` needs no token. With `SWAGGER_ENABLED=true`, the full
OpenAPI document is browsable at `/docs`.
