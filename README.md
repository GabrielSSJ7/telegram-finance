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
| 3 | Telegram bot MVP | next |
| 4 | Docker, Caddy, backups, deploy | |
| 5 | Credit cards and invoices | |
| 6 | Scheduler and reports | |
| 7 | Budgets and goals | |
| 8 | Edit flows, CSV export, hardening | |

## Layout

```
crates/
  domain/   pure finance rules, no I/O
  app/      use cases (services) over store ports; `test-support` feature
            ships the in-memory fake and the store contract suite
  pg/       Postgres adapter (sqlx), migrations/, queries/
  http/     REST API (axum, problem+json errors, OpenAPI)
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
```

## Configuration

See `.env.example`. Any secret `X` can be given as `X_FILE` (Docker secrets).

## REST API

`/api/v1/*` needs `Authorization: Bearer fbk_…`. Errors are
`application/problem+json`. POSTs that create entries accept an
`Idempotency-Key` (UUID): a retry with the same key returns 409 instead of
saving twice. Amounts are integer cents.
