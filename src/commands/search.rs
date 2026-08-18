//! `fsy search` — combined compile → query, mirroring the e2e Python script's flow.
//!
//! Collects source files under `--repo-root`, POSTs `/api/v1/compile`, takes the returned
//! revision, then POSTs `/api/v1/query` against that revision and prints a summary.
//!
//! `search` defines its own flat arg set (rather than flattening `CompileInputArgs` +
//! `QueryInputArgs`) because the two share flag names (`--revision`, `--query`) that would
//! otherwise collide. `--revision` here targets the query leg; the compile leg always
//! compiles "latest" and the returned revision is reused for query unless overridden.

use std::path::Path;

use anyhow::{Context, Result};
use clap::Args;

use crate::client::FormsyClient;
use crate::collect::collect_source_files;
use crate::commands::compile::build_request as build_compile_request;
use crate::commands::query::{build_request as build_query_request, render_agent_guidance};

#[derive(Args, Debug)]
pub struct SearchCmd {
    /// External repository identifier
    #[arg(long)]
    pub repo_id: String,

    /// Directory whose source files are collected and ingested (compile leg)
    #[arg(long)]
    pub repo_root: String,

    /// Natural-language repository query (query leg)
    #[arg(long)]
    pub query: String,

    /// Optional extension filter (compile leg; default: auto-detect CodeGraph languages)
    #[arg(long, value_delimiter = ',')]
    pub extensions: Vec<String>,

    /// How the compile leg updates the in-memory repo snapshot
    #[arg(long, value_parser = ["merge", "replace"], default_value = "merge")]
    pub mode: String,

    /// Stable upstream task identifier; requires --task-file
    #[arg(long)]
    pub task_id: Option<String>,

    /// Exact upstream task revision
    #[arg(long, default_value_t = 1)]
    pub task_revision: u32,

    /// UTF-8 file containing the complete upstream task source
    #[arg(long)]
    pub task_file: Option<String>,

    /// Caller policy for modifying repository test files
    #[arg(
        long,
        value_parser = ["allowed", "prohibited", "required", "unspecified"],
        default_value = "unspecified"
    )]
    pub test_file_mutation_policy: String,

    /// Override the revision used for the query leg (defaults to the revision returned by compile)
    #[arg(long)]
    pub revision: Option<String>,

    /// Query intent used to steer retrieval (query leg)
    #[arg(
        long,
        value_parser = [
            "general",
            "symbol_definition",
            "file",
            "call_flow",
            "tests",
            "behavior",
            "regression"
        ]
    )]
    pub intent: Option<String>,

    /// Context token budget (query leg)
    #[arg(long)]
    pub budget: Option<u32>,

    /// Response format for the query leg: structured bundle (default) or legacy JSON
    #[arg(long, value_parser = ["bundle", "legacy"])]
    pub response_format: Option<String>,

    /// Print the raw server JSON response of the query leg instead of a summary
    #[arg(long)]
    pub json: bool,
}

/// Adaptor exposing the compile-leg flags as `CompileInputArgs` expects.
fn compile_view(cmd: &SearchCmd) -> crate::commands::CompileInputArgs {
    crate::commands::CompileInputArgs {
        repo_id: cmd.repo_id.clone(),
        repo_root: cmd.repo_root.clone(),
        extensions: cmd.extensions.clone(),
        mode: cmd.mode.clone(),
        // search's --revision targets the query leg, not the compile leg.
        revision: None,
        task_id: cmd.task_id.clone(),
        task_revision: cmd.task_revision,
        task_file: cmd.task_file.clone(),
        test_file_mutation_policy: cmd.test_file_mutation_policy.clone(),
    }
}

/// Adaptor exposing the query-leg flags as `QueryInputArgs` expects.
fn query_view(
    cmd: &SearchCmd,
    resolved_revision: Option<String>,
) -> crate::commands::QueryInputArgs {
    crate::commands::QueryInputArgs {
        query: cmd.query.clone(),
        revision: resolved_revision,
        intent: cmd.intent.clone(),
        budget: cmd.budget,
        response_format: cmd.response_format.clone(),
        task_id: cmd.task_id.clone(),
        task_revision: cmd.task_id.as_ref().map(|_| cmd.task_revision),
    }
}

pub fn run(client: &FormsyClient, cmd: &SearchCmd) -> Result<()> {
    let files = collect_source_files(Path::new(&cmd.repo_root), &cmd.extensions)
        .context("collecting source files")?;
    if files.is_empty() {
        return Err(anyhow::anyhow!(
            "no eligible source files found under {:?} (extensions: {:?})",
            cmd.repo_root,
            cmd.extensions
        ));
    }
    eprintln!(
        "[info] collected {} files from {:?}",
        files.len(),
        cmd.repo_root
    );

    // --- compile ---
    let compile_req = build_compile_request(&compile_view(cmd), files)?;
    let compile_resp = client.compile(&compile_req)?;
    eprintln!(
        "[ok] compile repo_id={} revision={} parsed_file_count={}",
        compile_resp.repo_id, compile_resp.revision, compile_resp.parsed_file_count
    );

    // --- query (against the freshly compiled revision, unless --revision overrides) ---
    let resolved_revision = cmd
        .revision
        .clone()
        .or_else(|| Some(compile_resp.revision.clone()));
    let query_input = query_view(cmd, resolved_revision);

    if cmd.json {
        let query_req = build_query_request(&cmd.repo_id, &query_input)?;
        let value = client.query_json(&query_req)?;
        println!("{}", serde_json::to_string_pretty(&value)?);
        return Ok(());
    }

    let query_req = build_query_request(&cmd.repo_id, &query_input)?;
    let query_resp = client.query(&query_req)?;
    if !query_resp.error_code.is_empty() {
        eprintln!(
            "[warn] query error_code={} retrieval_state={}",
            query_resp.error_code, query_resp.retrieval_state
        );
    }
    if let Some(guidance) = render_agent_guidance(&query_resp.guidance) {
        eprintln!("[guidance]\n{guidance}");
    }
    if query_resp.extra_context.trim().is_empty() {
        println!("[ok] query returned empty extra_context");
    } else {
        println!("{}", query_resp.extra_context);
    }
    println!(
        "[status] coverage={} next={}",
        query_resp.coverage, query_resp.preferred_next_step
    );
    Ok(())
}
