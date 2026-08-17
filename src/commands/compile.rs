//! `fsy compile` — collect source files and POST `/api/v1/compile`.

use std::path::Path;

use anyhow::{Context, Result};
use clap::Args;

use crate::client::{has_http_status, FormsyClient};
use crate::collect::collect_source_files;
use crate::commands::CompileInputArgs;
use crate::models::{CompileRequest, TaskSourcePayload};

#[derive(Args, Debug)]
pub struct CompileCmd {
    #[command(flatten)]
    pub input: CompileInputArgs,

    /// Print the raw server JSON response instead of a summary
    #[arg(long)]
    pub json: bool,
}

pub fn run(client: &FormsyClient, cmd: &CompileCmd) -> Result<()> {
    let mut request = build_request(&cmd.input, Vec::new())?;
    let can_reuse_snapshot = request.revision.is_some() && request.task_source.is_some();
    if can_reuse_snapshot {
        if cmd.json {
            match client.compile_json(&request) {
                Ok(value) => {
                    println!("{}", serde_json::to_string_pretty(&value)?);
                    return Ok(());
                }
                Err(error) if has_http_status(&error, 404) => {}
                Err(error) => return Err(error),
            }
        } else {
            match client.compile(&request) {
                Ok(response) => {
                    print_compile_summary(&response);
                    return Ok(());
                }
                Err(error) if has_http_status(&error, 404) => {}
                Err(error) => return Err(error),
            }
        }
        eprintln!(
            "[info] snapshot {}@{} is not compiled; collecting repository source",
            request.repo_id,
            request.revision.as_deref().unwrap_or_default()
        );
    }

    let files = collect_source_files(Path::new(&cmd.input.repo_root), &cmd.input.extensions)
        .context("collecting source files")?;
    if files.is_empty() {
        return Err(anyhow::anyhow!(
            "no eligible source files found under {:?} (extensions: {:?})",
            cmd.input.repo_root,
            cmd.input.extensions
        ));
    }
    eprintln!(
        "[info] collected {} files from {:?}",
        files.len(),
        cmd.input.repo_root
    );

    request.files = files;

    if cmd.json {
        let value = client.compile_json(&request)?;
        println!("{}", serde_json::to_string_pretty(&value)?);
        return Ok(());
    }

    let resp = client.compile(&request)?;
    print_compile_summary(&resp);
    Ok(())
}

fn print_compile_summary(resp: &crate::models::CompileResponse) {
    println!(
        "[ok] compile repo_id={} revision={} parsed_file_count={}",
        resp.repo_id, resp.revision, resp.parsed_file_count
    );
}

/// Build a `CompileRequest` from shared input args (used by `search` too).
pub fn build_request(
    input: &CompileInputArgs,
    files: Vec<crate::models::SourceFilePayload>,
) -> Result<CompileRequest> {
    let mut request = CompileRequest::new(input.repo_id.clone(), files);
    request.mode = input.mode.clone();
    request.revision = input.revision.clone();
    request.test_file_mutation_policy = input.test_file_mutation_policy.clone();
    match (&input.task_id, &input.task_file) {
        (Some(task_id), Some(task_file)) => {
            let full_task_description = std::fs::read_to_string(task_file)
                .with_context(|| format!("reading task source file {task_file:?}"))?;
            if full_task_description.trim().is_empty() {
                return Err(anyhow::anyhow!("task source file must not be empty"));
            }
            request.task_source = Some(TaskSourcePayload {
                task_id: task_id.clone(),
                task_revision: input.task_revision,
                full_task_description,
            });
        }
        (None, None) => {}
        _ => {
            return Err(anyhow::anyhow!(
                "--task-id and --task-file must be supplied together"
            ))
        }
    }
    Ok(request)
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::build_request;
    use crate::commands::CompileInputArgs;
    use crate::models::SourceFilePayload;

    #[test]
    fn task_file_becomes_the_complete_typed_task_source() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("fsy-task-source-{nonce}.txt"));
        std::fs::write(&path, "Fix decode behavior.\n\nPreserve compatibility.")
            .expect("write task fixture");
        let input = CompileInputArgs {
            repo_id: "repo-1".to_string(),
            repo_root: ".".to_string(),
            extensions: vec!["py".to_string()],
            mode: "merge".to_string(),
            revision: Some("rev-1".to_string()),
            task_id: Some("task-1".to_string()),
            task_revision: 4,
            task_file: Some(path.to_string_lossy().into_owned()),
            test_file_mutation_policy: "prohibited".to_string(),
        };
        let files = vec![SourceFilePayload {
            path: "module.py".to_string(),
            content: "value = 1\n".to_string(),
            language: "python".to_string(),
        }];

        let request = build_request(&input, files).expect("valid compile request");
        std::fs::remove_file(path).expect("remove task fixture");

        let task_source = request.task_source.as_ref().expect("task source");
        assert_eq!(task_source.task_id, "task-1");
        assert_eq!(task_source.task_revision, 4);
        assert_eq!(
            task_source.full_task_description,
            "Fix decode behavior.\n\nPreserve compatibility."
        );
        let serialized = serde_json::to_value(request).expect("serialize request");
        assert!(serialized.get("query").is_none());
        assert!(serialized.get("enable_w2").is_none());
        assert_eq!(serialized["test_file_mutation_policy"], "prohibited");
    }
}
