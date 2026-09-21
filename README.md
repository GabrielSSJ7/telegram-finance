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
| 2 | Postgres schema, core services, REST API | next |
| 3 | Telegram bot MVP | |
| 4 | Docker, Caddy, backups, deploy | |
| 5 | Credit cards and invoices | |
| 6 | Scheduler and reports | |
| 7 | Budgets and goals | |
| 8 | Edit flows, CSV export, hardening | |

## Layout

```
crates/
  domain/   pure finance rules, no I/O
```

## Development

```sh
make test   # every test in the workspace
make lint   # rustfmt + clippy (warnings are errors)
make deny   # licenses and advisories
```

Rust 1.95 or newer.
