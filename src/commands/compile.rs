//! `fsy compile` — collect source files and POST `/api/v1/compile`.

use std::path::Path;

use anyhow::{Context, Result};
use clap::Args;

use crate::client::FormsyClient;
use crate::collect::collect_source_files;
use crate::commands::CompileInputArgs;
use crate::models::CompileRequest;

#[derive(Args, Debug)]
pub struct CompileCmd {
    #[command(flatten)]
    pub input: CompileInputArgs,

    /// Print the raw server JSON response instead of a summary
    #[arg(long)]
    pub json: bool,
}

pub fn run(client: &FormsyClient, cmd: &CompileCmd) -> Result<()> {
    let files = collect_source_files(Path::new(&cmd.input.repo_root), &cmd.input.extensions)
        .context("collecting source files")?;
    if files.is_empty() {
        return Err(anyhow::anyhow!(
            "no eligible source files found under {:?} (extensions: {:?})",
            cmd.input.repo_root,
            cmd.input.extensions
        ));
    }
    eprintln!("[info] collected {} files from {:?}", files.len(), cmd.input.repo_root);

    let mut request = CompileRequest::new(cmd.input.repo_id.clone(), files);
    request.mode = cmd.input.mode.clone();
    request.enable_w2 = !cmd.input.no_w2;
    request.revision = cmd.input.revision.clone();
    request.query = cmd.input.query.clone();

    if cmd.json {
        let value = client.compile_json(&request)?;
        println!("{}", serde_json::to_string_pretty(&value)?);
        return Ok(());
    }

    let resp = client.compile(&request)?;
    println!(
        "[ok] compile repo_id={} revision={} parsed_file_count={}",
        resp.repo_id, resp.revision, resp.parsed_file_count
    );
    Ok(())
}

/// Build a `CompileRequest` from shared input args (used by `search` too).
pub fn build_request(input: &CompileInputArgs, files: Vec<crate::models::SourceFilePayload>) -> CompileRequest {
    let mut request = CompileRequest::new(input.repo_id.clone(), files);
    request.mode = input.mode.clone();
    request.enable_w2 = !input.no_w2;
    request.revision = input.revision.clone();
    request.query = input.query.clone();
    request
}
