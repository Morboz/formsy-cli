//! `fsy` — Rust CLI for formsy.server `/api/v1/compile` + `/api/v1/query`.
//!
//! See `README.md` for usage. Subcommand logic lives in `commands/`.

mod capabilities;
mod client;
mod collect;
mod commands;
mod models;

use std::time::Duration;

use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::client::FormsyClient;
use crate::commands::{compile::CompileCmd, query::QueryCmd, search::SearchCmd, GlobalArgs};

#[derive(Parser, Debug)]
#[command(
    name = "fsy",
    version = concat!(
        env!("CARGO_PKG_VERSION"),
        " commit=",
        env!("FSY_BUILD_GIT_COMMIT"),
        " dirty=",
        env!("FSY_BUILD_GIT_DIRTY"),
        " target=",
        env!("FSY_BUILD_TARGET")
    ),
    about = "Rust CLI client for formsy.server compile/query endpoints",
    long_about = None
)]
struct Cli {
    #[command(flatten)]
    global: GlobalArgs,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Print immutable build identity and supported protocol capabilities
    Capabilities(capabilities::CapabilitiesCmd),
    /// Collect source files and POST /api/v1/compile
    Compile(CompileCmd),
    /// POST /api/v1/query
    Query(QueryCmd),
    /// Compile then query in one shot (compile → query)
    Search(SearchCmd),
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let client = FormsyClient::new(
        cli.global.base_url.clone(),
        cli.global.api_key_option(),
        Duration::from_secs(cli.global.timeout),
    )?;

    match cli.command {
        Command::Capabilities(cmd) => capabilities::run(&cmd)?,
        Command::Compile(cmd) => commands::compile::run(&client, &cmd)?,
        Command::Query(cmd) => commands::query::run(&client, &cmd)?,
        Command::Search(cmd) => commands::search::run(&client, &cmd)?,
    }
    Ok(())
}
