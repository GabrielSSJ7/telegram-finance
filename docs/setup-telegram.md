# Telegram setup

finbot talks to Telegram with long polling, so the bot needs no public URL.
Setup has three parts: create the bot, set it up in your group, then tell
finbot who is allowed to use it.

## 1. Create the bot (BotFather)

1. In Telegram, open **@BotFather** and send `/newbot`. Pick a name and a
   username. BotFather replies with the **token** (`123456:ABC...`).
   Treat it like a password.
2. Send `/setprivacy`, choose the bot, then **Disable**. Guided answers
   such as `10,50` are plain messages; with privacy on, the bot would not
   see them.
3. Optional: `/setdescription` and `/setuserpic`.

finbot publishes its own command menu (`/gasto`, `/saldo`, ...) at
startup, so `/setcommands` is not needed.

## 2. Find your Telegram user ids

Only listed ids can use the bot. To find yours, message **@userinfobot**.
Each spouse does this and notes the number.

## 3. Configure finbot

In the server's environment (see `.env.example`):

```sh
TELEGRAM_BOT_TOKEN_FILE=/run/secrets/telegram_bot_token   # or TELEGRAM_BOT_TOKEN=...
ALLOWED_TELEGRAM_USER_IDS=111111111,222222222
```

Restart finbot. `/healthz` now shows a `telegram` check.

## 4. Add the bot to your group

1. Create a group with the two of you (or use an existing one) and add the
   bot. **Re-add it if you changed privacy after adding it**; Telegram only
   applies the privacy change to groups joined afterwards.
2. One of you sends `/start` in the group. This binds finbot to that group;
   any other group is ignored, and the bot leaves groups it is added to by
   anyone not on the list.
3. Back in BotFather, send `/setjoingroups` → **Disable**, so nobody can add
   the bot anywhere else.
4. Each of you opens a private chat with the bot and sends `/start`. That is
   where the daily encrypted backups are delivered.

## 5. First steps in the group

```
/novaconta   → Nubank, Conta corrente, saldo atual
/novameta    → Casa própria, 100.000, já guardado 20.000
/gasto       → valor, descrição, categoria, conta ou cartão, (parcelas), data
/novocartao  → nome, dia de fechamento, dia de vencimento
/recorrente  → salário todo dia 5, aluguel todo dia 10...
/saldo, /resumo, /fatura
```

The daily summary arrives at 21:00 (change it with the API's
`PATCH /api/v1/settings`), and the closing of the financial month on the
first day of each cycle.

Each confirmation has a **↩️ Desfazer** button; only the person who
recorded the entry can use it. `/desfazer` undoes your own last entry.

## How the bot behaves

- Both of you can fill forms at the same time; each form belongs to the
  person who started it, and buttons on the other person's form are refused.
- A form expires after 30 minutes without an answer. `/cancelar` drops it.
- Tapping **Confirmar** twice saves once.
