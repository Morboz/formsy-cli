//! `fsy query` — POST `/api/v1/query`.

use anyhow::Result;
use clap::Args;

use crate::client::FormsyClient;
use crate::commands::QueryInputArgs;
use crate::models::{QueryGuidance, QueryRequest};

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
    let request = build_request(&cmd.repo_id, &cmd.input)?;

    if cmd.json {
        let value = client.query_json(&request)?;
        println!("{}", serde_json::to_string_pretty(&value)?);
        return Ok(());
    }

    let resp = client.query(&request)?;
    if !resp.error_code.is_empty() {
        println!(
            "[warn] error_code={} retrieval_state={}",
            resp.error_code, resp.retrieval_state
        );
    }
    if let Some(guidance) = render_agent_guidance(&resp.guidance) {
        println!("[guidance]\n{guidance}");
    }
    if resp.extra_context.trim().is_empty() {
        println!("[ok] query returned empty extra_context");
    } else {
        println!("{}", resp.extra_context);
    }
    println!(
        "[status] coverage={} next={}",
        resp.coverage, resp.preferred_next_step
    );
    Ok(())
}

pub(crate) fn render_agent_guidance(guidance: &QueryGuidance) -> Option<String> {
    let mut lines = Vec::new();
    let identity = [
        (!guidance.decision.is_empty()).then(|| format!("decision={}", guidance.decision)),
        (!guidance.mode.is_empty()).then(|| format!("mode={}", guidance.mode)),
        (!guidance.code.is_empty()).then(|| format!("code={}", guidance.code)),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    if !identity.is_empty() {
        lines.push(identity.join(" "));
    }
    if !guidance.summary.is_empty() {
        lines.push(guidance.summary.clone());
    }
    if !guidance.fallback_path.is_empty() {
        lines.push(format!("fallback={}", guidance.fallback_path));
    }
    if !guidance.preferred_next_step.is_empty() {
        lines.push(format!("next={}", guidance.preferred_next_step));
    }
    lines.extend(
        guidance
            .required_next_actions
            .iter()
            .filter(|action| !action.trim().is_empty())
            .map(|action| format!("- {action}")),
    );
    (!lines.is_empty()).then(|| lines.join("\n"))
}

/// Build a `QueryRequest` from a repo id + shared query input (used by `search` too).
pub fn build_request(repo_id: &str, input: &QueryInputArgs) -> Result<QueryRequest> {
    let mut request = QueryRequest::new(repo_id.to_string(), input.query.clone());
    request.revision = input.revision.clone();
    if let Some(intent) = &input.intent {
        request.intent = intent.clone();
    }
    if let Some(budget) = input.budget {
        request.budget = budget;
    }
    if let Some(fmt) = &input.response_format {
        request.response_format = fmt.clone();
    }
    let task_id = input.task_id.clone().or_else(|| {
        std::env::var("FORMSY_TASK_ID")
            .ok()
            .filter(|value| !value.trim().is_empty())
    });
    let task_revision = input.task_revision.or_else(|| {
        std::env::var("FORMSY_TASK_REVISION")
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
    });
    if task_id.is_some() != task_revision.is_some() {
        return Err(anyhow::anyhow!(
            "task association requires both task id and task revision"
        ));
    }
    request.task_id = task_id;
    request.task_revision = task_revision;
    Ok(request)
}

#[cfg(test)]
mod tests {
    use super::{build_request, render_agent_guidance};
    use crate::commands::QueryInputArgs;
    use crate::models::QueryResponse;

    #[test]
    fn explicit_task_association_is_sent_as_a_typed_pair() {
        let input = QueryInputArgs {
            query: "find the decoder".to_string(),
            revision: Some("rev-1".to_string()),
            intent: None,
            budget: None,
            response_format: None,
            task_id: Some("task-1".to_string()),
            task_revision: Some(3),
        };

        let request = build_request("repo-1", &input).expect("valid query request");

        assert_eq!(request.task_id.as_deref(), Some("task-1"));
        assert_eq!(request.task_revision, Some(3));
        assert_eq!(request.revision.as_deref(), Some("rev-1"));
    }

    #[test]
    fn renders_only_query_specific_guidance() {
        let response: QueryResponse = serde_json::from_value(serde_json::json!({
            "guidance": {
                "mode": "codegraph_unavailable_source_fallback",
                "decision": "STOP_EXPLORATION",
                "summary": "Use the compiled source fallback.",
                "fallback_path": "compiled_source_plus_direct_query_matches",
                "preferred_next_step": "bounded_source_inspection",
                "required_next_actions": [
                    "Read the ranked source files returned by context_search."
                ],
                "task_contract": {
                    "behavior": ["A large compile-time contract must not be repeated."]
                },
                "test_context": {
                    "source_obligations": ["Another compile-time payload."]
                }
            }
        }))
        .expect("query response");

        let rendered = render_agent_guidance(&response.guidance).expect("guidance");
        assert!(rendered.contains("decision=STOP_EXPLORATION"));
        assert!(rendered.contains("mode=codegraph_unavailable_source_fallback"));
        assert!(rendered.contains("next=bounded_source_inspection"));
        assert!(rendered.contains("Read the ranked source files"));
        assert!(!rendered.contains("large compile-time contract"));
        assert!(!rendered.contains("source_obligations"));
    }

    #[test]
    fn compile_time_only_guidance_is_not_repeated() {
        let response: QueryResponse = serde_json::from_value(serde_json::json!({
            "guidance": {
                "task_contract": {"behavior": ["Preserve compatibility."]},
                "test_context": {"file_mutation_policy": "prohibited"}
            }
        }))
        .expect("query response");

        assert_eq!(render_agent_guidance(&response.guidance), None);
    }
}
