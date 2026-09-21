use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "finbot", version, about = "Couple finance Telegram bot and REST API")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Command {
    /// Apply migrations, then run the HTTP API (and the bot, when configured).
    Serve,
    /// Apply pending database migrations and exit.
    Migrate,
    /// Exit 0 when `/healthz` answers 200; used by the container healthcheck.
    Healthcheck {
        #[arg(long, default_value = "127.0.0.1:8080")]
        address: String,
    },
    /// Manage REST API keys.
    #[command(subcommand)]
    ApiKey(ApiKeyCommand),
    /// Run one scheduled job now (sends real Telegram messages).
    RunJob {
        /// recurrences, invoice-events, daily-report, cycle-report or backup-watch
        job: String,
        /// Local date to run for (YYYY-MM-DD); defaults to today.
        #[arg(long)]
        date: Option<chrono::NaiveDate>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum ApiKeyCommand {
    /// Create a key and print its token once.
    Create { name: String },
    /// Revoke the active key with this name.
    Revoke { name: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Command {
        Cli::try_parse_from(std::iter::once("finbot").chain(args.iter().copied())).unwrap().command
    }

    #[test]
    fn parses_every_command() {
        assert_eq!(parse(&["serve"]), Command::Serve);
        assert_eq!(parse(&["migrate"]), Command::Migrate);
        assert_eq!(
            parse(&["healthcheck"]),
            Command::Healthcheck { address: "127.0.0.1:8080".into() }
        );
        let create = parse(&["api-key", "create", "dashboard"]);
        assert_eq!(create, Command::ApiKey(ApiKeyCommand::Create { name: "dashboard".into() }));
        let job = parse(&["run-job", "daily-report", "--date", "2026-10-05"]);
        let date = chrono::NaiveDate::from_ymd_opt(2026, 10, 5);
        assert_eq!(job, Command::RunJob { job: "daily-report".into(), date });
        let revoke = parse(&["api-key", "revoke", "dashboard"]);
        assert_eq!(revoke, Command::ApiKey(ApiKeyCommand::Revoke { name: "dashboard".into() }));
    }

    #[test]
    fn rejects_unknown_command() {
        assert!(Cli::try_parse_from(["finbot", "explode"]).is_err());
    }
}
