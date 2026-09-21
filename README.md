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
| 7 | Budgets and goals | next |
| 8 | Edit flows, CSV export, hardening | |

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
finbot serve                    # migrate, then serve the API (and bot, later)
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
| daily_report | report time (default 21:00) | Today's entries, the cycle so far, balances, cards, goals, what is coming |
| cycle_report | report time, first day of a cycle | Closing of the financial month: income, spending, savings rate, categories, per person |
| backup_watch | 10:00 | Private-chat warning when no backup succeeded in 26 hours |

`finbot run-job daily-report --date 2026-10-05` runs one job immediately.

## Deploy

Production runs from `compose.yml`: Caddy (HTTPS), finbot, Postgres 18 and a
backup sidecar. Images are built by GitHub Actions on `v*` tags and pushed to
GHCR; the VPS only pulls.

- [docs/deploy-hostinger.md](docs/deploy-hostinger.md): VPS setup and releases
- [docs/backup-restore.md](docs/backup-restore.md): encrypted backups and restore
- [docs/setup-telegram.md](docs/setup-telegram.md): BotFather and group setup

## Configuration

See `.env.example`. Any secret `X` can be given as `X_FILE` (Docker secrets).
Telegram setup: [docs/setup-telegram.md](docs/setup-telegram.md).

## Bot commands

| Command | What it does |
|---|---|
| `/gasto` | Guided expense: value, description, category, account or card (with installments), date |
| `/entrada` | Guided income |
| `/transferir` | Move money between accounts |
| `/guardar`, `/resgatar` | Move money into or out of a goal's pot |
| `/pagarfatura` | Pay (part of) a card invoice from an account |
| `/estorno` | Refund back to an account or a card invoice |
| `/fatura`, `/cartoes` | Card invoices (open, due, future installments); cards |
| `/novaconta`, `/novameta`, `/novocartao` | Create an account, a savings goal or a card |
| `/saldo`, `/resumo`, `/contas`, `/metas`, `/categorias` | Reports (`/resumo` is the daily report on demand) |
| `/recorrente`, `/recorrentes` | Create a monthly entry (salary, rent, subscription); list and deactivate |
| `/desfazer` | Undo your own last entry |
| `/cancelar`, `/ajuda` | Cancel the current form, list commands |

## REST API

`/api/v1/*` needs `Authorization: Bearer fbk_…`. Errors are
`application/problem+json`. POSTs that create entries accept an
`Idempotency-Key` (UUID): a retry with the same key returns 409 instead of
saving twice. Amounts are integer cents.
