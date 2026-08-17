//! `fsy search-nodes` / `fsy get-neighbors` / `fsy get-node-detail` —
//! POST `/api/v1/search_nodes`, `/api/v1/get_neighbors`, `/api/v1/get_node_detail`.
//!
//! These mirror the agent tool names: locate symbols with `search-nodes`, walk
//! the call graph with `get-neighbors` (using node ids from `search-nodes`),
//! and read one node's signature/span with `get-node-detail`.

use anyhow::Result;
use clap::Args;

use crate::client::FormsyClient;
use crate::models::{
    GetNeighborsRequest, GetNodeDetailRequest, Neighbor, NodeSummary, SearchNodesRequest,
};

/// Options shared by the graph-inspection subcommands.
#[derive(Args, Debug, Clone)]
pub struct GraphInputArgs {
    /// Logical revision label (defaults to latest compiled revision)
    #[arg(long)]
    pub revision: Option<String>,
}

#[derive(Args, Debug)]
pub struct SearchNodesCmd {
    /// External repository identifier
    #[arg(long)]
    pub repo_id: String,

    /// Search keywords: a symbol name, file name, or short code terms
    #[arg(long)]
    pub query: String,

    /// Maximum number of nodes to return
    #[arg(long, default_value_t = 10)]
    pub limit: u32,

    #[command(flatten)]
    pub input: GraphInputArgs,

    /// Print the raw server JSON response instead of a summary
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct GetNeighborsCmd {
    /// External repository identifier
    #[arg(long)]
    pub repo_id: String,

    /// Node id from a previous `search-nodes` (or `get-neighbors`) result
    #[arg(long)]
    pub node_id: String,

    /// Traversal direction: callers (who calls it), callees (what it calls), or both
    #[arg(long, value_parser = ["callers", "callees", "both"], default_value = "both")]
    pub direction: String,

    /// Call-chain depth (1 = direct edges only)
    #[arg(long, default_value_t = 1)]
    pub max_depth: u32,

    #[command(flatten)]
    pub input: GraphInputArgs,

    /// Print the raw server JSON response instead of a summary
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct GetNodeDetailCmd {
    /// External repository identifier
    #[arg(long)]
    pub repo_id: String,

    /// Node id from a previous `search-nodes` (or `get-neighbors`) result
    #[arg(long)]
    pub node_id: String,

    #[command(flatten)]
    pub input: GraphInputArgs,

    /// Print the raw server JSON response instead of a summary
    #[arg(long)]
    pub json: bool,
}

pub fn run_search_nodes(client: &FormsyClient, cmd: &SearchNodesCmd) -> Result<()> {
    let mut request = SearchNodesRequest::new(cmd.repo_id.clone(), cmd.query.clone());
    request.limit = cmd.limit;
    request.revision = cmd.input.revision.clone();

    if cmd.json {
        let value = client.search_nodes_json(&request)?;
        println!("{}", serde_json::to_string_pretty(&value)?);
        return Ok(());
    }

    let resp = client.search_nodes(&request)?;
    if resp.nodes.is_empty() {
        println!("[ok] no nodes matched query {:?}", resp.query);
        return Ok(());
    }
    println!("[ok] {} node(s) matched {:?}", resp.nodes.len(), resp.query);
    for node in &resp.nodes {
        println!("{}", format_node_summary(node));
    }
    Ok(())
}

pub fn run_get_neighbors(client: &FormsyClient, cmd: &GetNeighborsCmd) -> Result<()> {
    let mut request = GetNeighborsRequest::new(cmd.repo_id.clone(), cmd.node_id.clone());
    request.direction = cmd.direction.clone();
    request.max_depth = cmd.max_depth;
    request.revision = cmd.input.revision.clone();

    if cmd.json {
        let value = client.get_neighbors_json(&request)?;
        println!("{}", serde_json::to_string_pretty(&value)?);
        return Ok(());
    }

    let resp = client.get_neighbors(&request)?;
    if resp.callers.is_empty() && resp.callees.is_empty() {
        println!("[ok] no {} found for node {:?}", resp.direction, resp.node_id);
        return Ok(());
    }
    print_neighbor_section("callers", &resp.callers);
    print_neighbor_section("callees", &resp.callees);
    Ok(())
}

pub fn run_get_node_detail(client: &FormsyClient, cmd: &GetNodeDetailCmd) -> Result<()> {
    let mut request = GetNodeDetailRequest::new(cmd.repo_id.clone(), cmd.node_id.clone());
    request.revision = cmd.input.revision.clone();

    if cmd.json {
        let value = client.get_node_detail_json(&request)?;
        println!("{}", serde_json::to_string_pretty(&value)?);
        return Ok(());
    }

    let resp = client.get_node_detail(&request)?;
    let node = resp.node;
    println!("[ok] {} {} ({})", node.kind, node.name, node.qualified_name);
    println!("  id:       {}", node.id);
    println!("  location: {}:{}-{}", node.file_path, node.start_line, node.end_line);
    println!("  language: {}", node.language);
    if !node.signature.is_empty() {
        println!("  signature: {}", node.signature);
    }
    if let Some(visibility) = &node.visibility {
        println!("  visibility: {visibility}");
    }
    let mut flags = Vec::new();
    if node.is_async {
        flags.push("async");
    }
    if node.is_static {
        flags.push("static");
    }
    if node.is_abstract {
        flags.push("abstract");
    }
    if !flags.is_empty() {
        println!("  flags:    {}", flags.join(", "));
    }
    if !node.docstring.is_empty() {
        println!("  docstring:");
        for line in node.docstring.lines() {
            println!("    {line}");
        }
    }
    Ok(())
}

/// One line per node: `id | kind | name | file:line | signature`.
fn format_node_summary(node: &NodeSummary) -> String {
    let signature = if node.signature.is_empty() {
        String::new()
    } else {
        format!(" | {}", node.signature)
    };
    format!(
        "{} | {} | {} | {}:{}{}",
        node.id, node.kind, node.qualified_name, node.file_path, node.start_line, signature
    )
}

/// One line per neighbor: `node | file | edge Lline`, mirroring the agent tool output.
fn format_neighbor(neighbor: &Neighbor) -> String {
    let line = match neighbor.edge_line {
        Some(line) => format!(" L{line}"),
        None => String::new(),
    };
    format!(
        "  {} | {} | {} | {}{}",
        neighbor.node.id, neighbor.node.name, neighbor.node.file_path, neighbor.edge_kind, line
    )
}

fn print_neighbor_section(label: &str, neighbors: &[Neighbor]) {
    if neighbors.is_empty() {
        return;
    }
    println!("=== {label} ({}) ===", neighbors.len());
    for neighbor in neighbors {
        println!("{}", format_neighbor(neighbor));
    }
}
