//! Serde models for the `/api/v1/compile` and `/api/v1/query` endpoints.
//!
//! Request structs carry every field the server accepts (see
//! `packages/server/src/formsy/server/models.py`). Optional fields use
//! `skip_serializing_if = "Option::is_none"` so we only send what's set, matching the
//! lean payloads the e2e Python script sends.
//!
//! Response structs deliberately parse only the fields the CLI needs to display; the
//! large, frequently-evolving payloads (`context_package`, `bundle`, `query_plan`, ...)
//! are captured into a `serde_json::Value` catch-all so server-side model additions never
//! break deserialization.

// These structs model an external API; their fields are read by serde and intended for
// future CLI use, so silence dead-code warnings rather than delete them.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Shared / request payloads
// ---------------------------------------------------------------------------

/// One source file in a `CompileRequest`. Mirrors `SourceFilePayload`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceFilePayload {
    pub path: String,
    pub content: String,
    #[serde(
        default = "default_language",
        skip_serializing_if = "is_default_language"
    )]
    pub language: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskSourcePayload {
    pub task_id: String,
    pub task_revision: u32,
    pub full_task_description: String,
}

fn default_language() -> String {
    "python".to_string()
}

fn is_default_language(lang: &str) -> bool {
    lang == "python"
}

/// `CompileRequest`. `files` is required and must be non-empty (server validates).
#[derive(Debug, Clone, Serialize)]
pub struct CompileRequest {
    pub repo_id: String,
    pub files: Vec<SourceFilePayload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    #[serde(default = "default_mode", skip_serializing_if = "is_default_mode")]
    pub mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_source: Option<TaskSourcePayload>,
    #[serde(
        default = "default_test_file_mutation_policy",
        skip_serializing_if = "is_unspecified_test_file_mutation_policy"
    )]
    pub test_file_mutation_policy: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<String>,
}

fn default_mode() -> String {
    "merge".to_string()
}

fn is_default_mode(mode: &str) -> bool {
    mode == "merge"
}

fn default_test_file_mutation_policy() -> String {
    "unspecified".to_string()
}

fn is_unspecified_test_file_mutation_policy(policy: &str) -> bool {
    policy == "unspecified"
}

impl CompileRequest {
    /// Build a minimal compile request: just `repo_id` + `files`.
    pub fn new(repo_id: impl Into<String>, files: Vec<SourceFilePayload>) -> Self {
        Self {
            repo_id: repo_id.into(),
            files,
            revision: None,
            mode: default_mode(),
            task_source: None,
            test_file_mutation_policy: default_test_file_mutation_policy(),
            metadata: BTreeMap::new(),
            owner_id: None,
        }
    }
}

/// `QueryRequest`. `repo_id` + `query` are required.
#[derive(Debug, Clone, Serialize)]
pub struct QueryRequest {
    pub repo_id: String,
    pub query: String,
    #[serde(default = "default_intent", skip_serializing_if = "is_default_intent")]
    pub intent: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    #[serde(default = "default_budget", skip_serializing_if = "is_default_budget")]
    pub budget: u32,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<String>,
    #[serde(
        default = "default_response_format",
        skip_serializing_if = "is_default_response_format"
    )]
    pub response_format: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_revision: Option<u32>,
}

fn default_intent() -> String {
    "general".to_string()
}

fn is_default_intent(intent: &str) -> bool {
    intent == "general"
}

fn default_budget() -> u32 {
    4000
}

fn is_default_budget(b: &u32) -> bool {
    *b == 4000
}

fn default_response_format() -> String {
    "bundle".to_string()
}

fn is_default_response_format(f: &str) -> bool {
    f == "bundle"
}

impl QueryRequest {
    pub fn new(repo_id: impl Into<String>, query: impl Into<String>) -> Self {
        Self {
            repo_id: repo_id.into(),
            query: query.into(),
            intent: default_intent(),
            revision: None,
            budget: default_budget(),
            metadata: BTreeMap::new(),
            owner_id: None,
            response_format: default_response_format(),
            task_id: None,
            task_revision: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Responses — parse only what the CLI displays; flatten the rest into `extra`.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Default)]
pub struct CompileResponse {
    pub repo_id: String,
    pub revision: String,
    pub parsed_file_count: i64,
    #[serde(default)]
    pub scan_stats: Option<Value>,
    #[serde(default)]
    pub store_stats: Option<Value>,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QueryResponse {
    #[serde(default)]
    pub repo_id: String,
    #[serde(default)]
    pub revision: String,
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub extra_context: String,
    #[serde(default)]
    pub retrieval_state: String,
    #[serde(default)]
    pub preferred_next_step: String,
    #[serde(default)]
    pub coverage: String,
    #[serde(default)]
    pub agent_guidance_text: String,
    #[serde(default)]
    pub error_code: String,
    /// Everything else (`context_package`, `bundle`, `query_plan`, `matches`,
    /// `test_constraints`, `guidance`, `query_profile`, `store_stats`, ...).
    /// Kept as raw JSON so the CLI never breaks on server-side field additions.
    #[serde(flatten)]
    pub extra: Value,
}

#[cfg(test)]
mod tests {
    use super::QueryResponse;

    #[test]
    fn query_response_reads_extra_context_from_minimal_response() {
        let response: QueryResponse = serde_json::from_value(serde_json::json!({
            "extra_context": "retrieved context"
        }))
        .expect("minimal query response should deserialize");

        assert_eq!(response.extra_context, "retrieved context");
        assert!(response.repo_id.is_empty());
        assert!(response.revision.is_empty());
        assert!(response.query.is_empty());
    }
}
