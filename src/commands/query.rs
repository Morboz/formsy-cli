//! `fsy query` — POST `/api/v1/query`.

use anyhow::Result;
use clap::Args;

use crate::client::FormsyClient;
use crate::commands::QueryInputArgs;
use crate::models::QueryRequest;

#[derive(Args, Debug)]
pub struct QueryCmd {
    /// External repository identifier
    #[arg(long)]
    pub repo_id: String,

    #[command(flatten)]
    pub input: QueryInputArgs,

    /// Print the raw server JSON response instead of a summary
    #[arg(long)]
    pub json: bool,
}

pub fn run(client: &FormsyClient, cmd: &QueryCmd) -> Result<()> {
    let request = build_request(&cmd.repo_id, &cmd.input);

    if cmd.json {
        let value = client.query_json(&request)?;
        println!("{}", serde_json::to_string_pretty(&value)?);
        return Ok(());
    }

    let resp = client.query(&request)?;
    if !resp.error_code.is_empty() {
        println!("[warn] error_code={} retrieval_state={}", resp.error_code, resp.retrieval_state);
    }
    if !resp.agent_guidance_text.is_empty() {
        println!("[guidance] {}", resp.agent_guidance_text);
    }
    if resp.extra_context.trim().is_empty() {
        println!("[ok] query returned empty extra_context (coverage={}, next={})",
            resp.coverage, resp.preferred_next_step);
    } else {
        println!("{}", resp.extra_context);
    }
    Ok(())
}

/// Build a `QueryRequest` from a repo id + shared query input (used by `search` too).
pub fn build_request(repo_id: &str, input: &QueryInputArgs) -> QueryRequest {
    let mut request = QueryRequest::new(repo_id.to_string(), input.query.clone());
    request.revision = input.revision.clone();
    request.enable_profiling = input.enable_profiling;
    request.profiling_top_n = input.profiling_top_n;
    if let Some(intent) = &input.intent {
        request.intent = intent.clone();
    }
    if let Some(budget) = input.budget {
        request.budget = budget;
    }
    if let Some(fmt) = &input.response_format {
        request.response_format = fmt.clone();
    }
    request
}
