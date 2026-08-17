//! Repository-aware source collection for `compile` and `search`.
//!
//! With no explicit extension filter, the collector sends source languages that
//! CodeGraph can parse.  In a Git worktree it uses Git's own tracked/untracked
//! inventory, so ignored dependency and build directories never enter the
//! compile request.  Non-Git directories use the same ignore semantics through
//! the `ignore` walker.

use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, Context, Result};
use ignore::WalkBuilder;

use crate::models::SourceFilePayload;

/// Extensions parsed by the CodeGraph backend. This list defines automatic
/// discovery only; an explicit `--extensions` value remains an escape hatch for
/// a backend with newer language support.
const CODEGRAPH_LANGUAGES: &[(&str, &str)] = &[
    ("py", "python"),
    ("ts", "typescript"),
    ("tsx", "tsx"),
    ("js", "javascript"),
    ("jsx", "jsx"),
    ("go", "go"),
    ("rs", "rust"),
    ("java", "java"),
    ("c", "c"),
    ("h", "c"),
    ("cpp", "cpp"),
    ("hpp", "cpp"),
    ("cc", "cpp"),
    ("cxx", "cpp"),
    ("cs", "csharp"),
    ("php", "php"),
    ("rb", "ruby"),
    ("swift", "swift"),
    ("kt", "kotlin"),
    ("kts", "kotlin"),
    ("dart", "dart"),
];

/// Collect source files from the current repository inventory.
///
/// An empty `extensions` slice means automatic CodeGraph-language discovery.
/// Non-empty values restrict collection to those extensions (lowercase or
/// mixed-case, with or without a leading dot).
pub fn collect_source_files(root: &Path, extensions: &[String]) -> Result<Vec<SourceFilePayload>> {
    let root = root
        .canonicalize()
        .with_context(|| format!("repo-root {:?} does not exist", root))?;
    if !root.is_dir() {
        return Err(anyhow!("repo-root {:?} is not a directory", root));
    }

    let requested_extensions: HashSet<String> = extensions
        .iter()
        .map(|e| e.trim().trim_start_matches('.').to_ascii_lowercase())
        .filter(|e| !e.is_empty())
        .collect();

    let mut paths = match collect_git_inventory(&root)? {
        Some(paths) => paths,
        None => collect_ignored_aware_inventory(&root),
    };
    paths.retain(|path| {
        let Some(ext) = normalized_extension(path) else {
            return false;
        };
        if requested_extensions.is_empty() {
            language_for_extension(&ext).is_some()
        } else {
            requested_extensions.contains(&ext)
        }
    });
    paths.sort();
    paths.dedup();

    let mut files = Vec::with_capacity(paths.len());
    for path in paths {
        let rel = path
            .strip_prefix(&root)
            .map_err(|e| anyhow!("strip_prefix failed for {path:?}: {e}"))?;
        let rel_posix = rel.to_str().ok_or_else(|| {
            anyhow!("path {rel:?} is not valid UTF-8 (repo-relative posix required)")
        })?;
        let ext = normalized_extension(&path)
            .expect("eligible source paths always have a UTF-8 extension");
        let content =
            std::fs::read_to_string(&path).with_context(|| format!("failed to read {path:?}"))?;

        files.push(SourceFilePayload {
            path: rel_posix.replace(std::path::MAIN_SEPARATOR, "/"),
            content,
            language: language_for_extension(&ext).unwrap_or(&ext).to_string(),
        });
    }

    Ok(files)
}

fn normalized_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
}

fn language_for_extension(extension: &str) -> Option<&'static str> {
    CODEGRAPH_LANGUAGES
        .iter()
        .find_map(|(ext, language)| (*ext == extension).then_some(*language))
}

/// Ask Git for tracked files plus non-ignored working-tree files. `None` means
/// the directory is not a Git worktree (or Git is unavailable), in which case
/// the caller uses the ignore-aware filesystem fallback.
fn collect_git_inventory(root: &Path) -> Result<Option<Vec<PathBuf>>> {
    let output = match Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ])
        .output()
    {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).context("running git ls-files"),
    };
    if !output.status.success() {
        return Ok(None);
    }

    let mut paths = Vec::new();
    for raw_path in output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|p| !p.is_empty())
    {
        let relative =
            std::str::from_utf8(raw_path).context("git reported a path that is not valid UTF-8")?;
        let relative = Path::new(relative);
        if relative.is_absolute()
            || relative
                .components()
                .any(|component| matches!(component, Component::ParentDir))
        {
            return Err(anyhow!("git reported path outside repo-root: {relative:?}"));
        }
        let path = root.join(relative);
        if std::fs::symlink_metadata(&path)
            .map(|metadata| metadata.file_type().is_file())
            .unwrap_or(false)
        {
            paths.push(path);
        }
    }
    Ok(Some(paths))
}

fn collect_ignored_aware_inventory(root: &Path) -> Vec<PathBuf> {
    let mut builder = WalkBuilder::new(root);
    builder
        .standard_filters(true)
        .require_git(false)
        .follow_links(false);
    builder
        .build()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_type()
                .map(|kind| kind.is_file())
                .unwrap_or(false)
        })
        .map(|entry| entry.into_path())
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::collect_source_files;

    fn temp_dir(label: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("fsy-collect-{label}-{nonce}"));
        fs::create_dir_all(&path).expect("create fixture root");
        path
    }

    fn write(root: &std::path::Path, relative: &str, content: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("fixture parent")).expect("create parent");
        fs::write(path, content).expect("write fixture");
    }

    #[test]
    fn auto_discovery_uses_git_inventory_and_supported_languages() {
        let root = temp_dir("git-auto");
        assert!(Command::new("git")
            .args(["init", "-q"])
            .current_dir(&root)
            .status()
            .expect("run git init")
            .success());
        write(&root, ".gitignore", "node_modules/\n");
        write(&root, "src/main.js", "export const value = 1;\n");
        write(&root, "src/extra.ts", "export const extra: number = 2;\n");
        write(&root, "tool.py", "value = 3\n");
        write(&root, "node_modules/vendor.js", "ignored();\n");
        write(&root, "README.md", "not a CodeGraph language\n");
        assert!(Command::new("git")
            .args(["add", ".gitignore", "src/main.js", "tool.py"])
            .current_dir(&root)
            .status()
            .expect("run git add")
            .success());

        let files = collect_source_files(&root, &[]).expect("collect source files");
        let paths: Vec<_> = files.iter().map(|file| file.path.as_str()).collect();
        assert_eq!(paths, ["src/extra.ts", "src/main.js", "tool.py"]);
        assert_eq!(files[0].language, "typescript");
        assert_eq!(files[1].language, "javascript");
        assert_eq!(files[2].language, "python");
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    fn explicit_extensions_restrict_and_can_override_auto_languages() {
        let root = temp_dir("explicit");
        write(&root, ".gitignore", "ignored/\n");
        write(&root, "README.md", "include me\n");
        write(&root, "main.js", "exclude me\n");
        write(&root, "ignored/notes.md", "ignore me\n");

        let files =
            collect_source_files(&root, &[".MD".to_string()]).expect("collect explicit extension");
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "README.md");
        assert_eq!(files[0].language, "md");
        fs::remove_dir_all(root).expect("remove fixture");
    }
}
