//! `fsy` — Rust CLI for formsy.server `/api/v1/compile` + `/api/v1/query`.
//!
//! See `README.md` for usage. Subcommand logic lives in `commands/`.

mod client;
mod collect;
mod commands;
mod models;

use std::time::Duration;

use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::client::FormsyClient;
use crate::commands::graph::{GetNeighborsCmd, GetNodeDetailCmd, SearchNodesCmd};
use crate::commands::{compile::CompileCmd, query::QueryCmd, search::SearchCmd, GlobalArgs};

#[derive(Parser, Debug)]
#[command(
    name = "fsy",
    version,
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
    /// Collect source files and POST /api/v1/compile
    Compile(CompileCmd),
    /// POST /api/v1/query
    Query(QueryCmd),
    /// Compile then query in one shot (compile → query)
    Search(SearchCmd),
    /// POST /api/v1/search_nodes — fuzzy-search graph symbols
    SearchNodes(SearchNodesCmd),
    /// POST /api/v1/get_neighbors — call-graph callers/callees of a node
    GetNeighbors(GetNeighborsCmd),
    /// POST /api/v1/get_node_detail — full detail for one node
    GetNodeDetail(GetNodeDetailCmd),
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let client = FormsyClient::new(
        cli.global.base_url.clone(),
        cli.global.api_key_option(),
        Duration::from_secs(cli.global.timeout),
    )?;

    match cli.command {
        Command::Compile(cmd) => commands::compile::run(&client, &cmd)?,
        Command::Query(cmd) => commands::query::run(&client, &cmd)?,
        Command::Search(cmd) => commands::search::run(&client, &cmd)?,
        Command::SearchNodes(cmd) => commands::graph::run_search_nodes(&client, &cmd)?,
        Command::GetNeighbors(cmd) => commands::graph::run_get_neighbors(&client, &cmd)?,
        Command::GetNodeDetail(cmd) => commands::graph::run_get_node_detail(&client, &cmd)?,
    }
    Ok(())
}
