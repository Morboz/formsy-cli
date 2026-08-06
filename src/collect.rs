//! Collect source files under a directory, mirroring
//! `e2e_server_compile_query.py::collect_source_files`:
//!   - recursive glob by extension (default `*.py`)
//!   - `path` is repo-relative posix
//!   - `language = "python"` (or the extension itself for non-py)
//!   - `is_test = "test" in filename.lower()`
//!
//! No `.gitignore` filtering — same as the Python reference, by design (simple + matches
//! the e2e baseline).

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use walkdir::WalkDir;

use crate::models::SourceFilePayload;

/// Collect every file under `root` whose extension is in `extensions` (lowercase, no dot,
/// e.g. `["py"]`). Files are returned sorted by relative path for stable ordering.
pub fn collect_source_files(
    root: &Path,
    extensions: &[String],
) -> Result<Vec<SourceFilePayload>> {
    let root = root
        .canonicalize()
        .with_context(|| format!("repo-root {:?} does not exist", root))?;
    if !root.is_dir() {
        return Err(anyhow!("repo-root {:?} is not a directory", root));
    }

    let exts: Vec<String> = extensions
        .iter()
        .map(|e| e.trim().trim_start_matches('.').to_ascii_lowercase())
        .filter(|e| !e.is_empty())
        .collect();
    if exts.is_empty() {
        return Err(anyhow!("no extensions given (use --extensions py)"));
    }

    let mut paths: Vec<PathBuf> = WalkDir::new(&root)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|x| x.to_str())
                .map(|x| exts.iter().any(|want| want == &x.to_ascii_lowercase()))
                .unwrap_or(false)
        })
        .map(|e| e.into_path())
        .collect();
    paths.sort();

    let mut files = Vec::with_capacity(paths.len());
    for path in paths {
        let rel = path
            .strip_prefix(&root)
            .map_err(|e| anyhow!("strip_prefix failed for {path:?}: {e}"))?;
        let rel_posix = rel.to_str().ok_or_else(|| {
            anyhow!("path {rel:?} is not valid UTF-8 (repo-relative posix required)")
        })?;

        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read {path:?}"))?;

        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        let is_test = file_name.contains("test");

        let ext = path
            .extension()
            .and_then(|x| x.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let language = if ext == "py" {
            "python".to_string()
        } else {
            ext
        };

        files.push(SourceFilePayload {
            path: rel_posix.to_string(),
            content,
            language,
            is_test,
        });
    }

    Ok(files)
}
