pub mod compile;
pub mod query;
pub mod search;

use clap::Args;

/// Global HTTP options shared by every subcommand. Placed on the top-level CLI via
/// `#[command(flatten)]`, so `--base-url` / `--api-key` / `--timeout` work before the
/// subcommand (e.g. `fsy --base-url http://x:9000 query ...`).
#[derive(Args, Debug, Clone)]
pub struct GlobalArgs {
    /// Running formsy.server base URL
    #[arg(long, global = true, default_value = "http://127.0.0.1:8080")]
    pub base_url: String,

    /// API key sent as `Authorization: Bearer <KEY>` (omit to send none)
    #[arg(long, global = true, default_value = "fsy_test_key_dev_only_12345678")]
    pub api_key: String,

    /// Per-request HTTP timeout in seconds
    #[arg(long, global = true, default_value_t = 1200)]
    pub timeout: u64,
}

impl GlobalArgs {
    pub fn api_key_option(&self) -> Option<String> {
        // Treat empty string as "no key" (lets users pass --api-key "" to disable).
        if self.api_key.trim().is_empty() {
            None
        } else {
            Some(self.api_key.clone())
        }
    }
}

/// Options shared by `compile` and `search` (both collect source files).
#[derive(Args, Debug, Clone)]
pub struct CompileInputArgs {
    /// External repository identifier
    #[arg(long)]
    pub repo_id: String,

    /// Directory whose source files are collected and ingested
    #[arg(long)]
    pub repo_root: String,

    /// Comma-separated file extensions to ingest (default: py)
    #[arg(long, value_delimiter = ',', default_value = "py")]
    pub extensions: Vec<String>,

    /// How this compile updates the in-memory repo snapshot
    #[arg(long, value_parser = ["merge", "replace"], default_value = "merge")]
    pub mode: String,

    /// Logical revision label (defaults to latest on the server)
    #[arg(long)]
    pub revision: Option<String>,

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
}

/// Options shared by `query` and `search`.
#[derive(Args, Debug, Clone)]
pub struct QueryInputArgs {
    /// Natural-language repository query
    #[arg(long)]
    pub query: String,

    /// Logical revision label (defaults to latest compiled revision)
    #[arg(long)]
    pub revision: Option<String>,

    /// Query intent used to steer retrieval
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

    /// Context token budget
    #[arg(long)]
    pub budget: Option<u32>,

    /// Response format: structured bundle (default) or legacy ContextPackage JSON
    #[arg(long, value_parser = ["bundle", "legacy"])]
    pub response_format: Option<String>,

    /// Task authority association; defaults to FORMSY_TASK_ID when set
    #[arg(long)]
    pub task_id: Option<String>,

    /// Exact task revision; defaults to FORMSY_TASK_REVISION when set
    #[arg(long)]
    pub task_revision: Option<u32>,
}
