//! finbot entry point. Parses the command line, sets up logging and runs
//! the chosen command; the work lives in the library modules.

use clap::Parser;
use finbot::cli::Cli;
use finbot::commands;
use finbot::config::{AppConfig, ProcessEnvironment};
use finbot::logging;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let config = AppConfig::load(&ProcessEnvironment)?;
    logging::init(config.log_format);
    commands::run(cli.command, config).await
}
