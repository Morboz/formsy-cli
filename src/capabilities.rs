use anyhow::Result;
use clap::Args;
use serde::Serialize;

pub const CAPABILITY_SCHEMA_VERSION: &str = "formsy.cli_capabilities.v1";
pub const CAPABILITIES: &[&str] = &[
    "compile.task_source.v1",
    "compile.test_file_mutation_policy.v1",
    "graph.get_neighbors.v1",
    "graph.get_node_detail.v1",
    "graph.search_nodes.v1",
    "query.task_identity.v1",
    "source.auto_extensions.v1",
];

#[derive(Args, Debug)]
pub struct CapabilitiesCmd {
    /// Emit the machine-readable capability document
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct CapabilityDocument {
    pub schema_version: &'static str,
    pub cli_version: &'static str,
    pub git_commit: &'static str,
    pub git_dirty: &'static str,
    pub target: &'static str,
    pub capabilities: &'static [&'static str],
}

impl CapabilityDocument {
    pub fn current() -> Self {
        Self {
            schema_version: CAPABILITY_SCHEMA_VERSION,
            cli_version: env!("CARGO_PKG_VERSION"),
            git_commit: env!("FSY_BUILD_GIT_COMMIT"),
            git_dirty: env!("FSY_BUILD_GIT_DIRTY"),
            target: env!("FSY_BUILD_TARGET"),
            capabilities: CAPABILITIES,
        }
    }
}

pub fn run(cmd: &CapabilitiesCmd) -> Result<()> {
    let document = CapabilityDocument::current();
    if cmd.json {
        println!("{}", serde_json::to_string_pretty(&document)?);
    } else {
        println!(
            "fsy {} commit={} dirty={} target={}",
            document.cli_version, document.git_commit, document.git_dirty, document.target
        );
        for capability in document.capabilities {
            println!("- {capability}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{CapabilityDocument, CAPABILITIES, CAPABILITY_SCHEMA_VERSION};

    #[test]
    fn capability_document_is_machine_stable_and_build_identified() {
        let document = CapabilityDocument::current();
        let value = serde_json::to_value(&document).expect("serialize capabilities");

        assert_eq!(value["schema_version"], CAPABILITY_SCHEMA_VERSION);
        assert_eq!(value["cli_version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(value["git_commit"], env!("FSY_BUILD_GIT_COMMIT"));
        assert_eq!(value["git_dirty"], env!("FSY_BUILD_GIT_DIRTY"));
        assert_eq!(value["target"], env!("FSY_BUILD_TARGET"));
        assert_eq!(value["capabilities"], serde_json::json!(CAPABILITIES));
    }
}
