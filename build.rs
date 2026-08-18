use std::env;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=FSY_BUILD_GIT_COMMIT");
    println!("cargo:rerun-if-env-changed=FSY_BUILD_GIT_DIRTY");
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/index");
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=Cargo.toml");

    let commit = env::var("FSY_BUILD_GIT_COMMIT")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| git_stdout(&["rev-parse", "HEAD"]))
        .unwrap_or_else(|| "unknown".to_string());
    let dirty = env::var("FSY_BUILD_GIT_DIRTY")
        .ok()
        .filter(|value| matches!(value.as_str(), "true" | "false"))
        .unwrap_or_else(|| {
            git_stdout(&["status", "--porcelain", "--untracked-files=no"])
                .map(|output| (!output.is_empty()).to_string())
                .unwrap_or_else(|| "unknown".to_string())
        });
    let target = env::var("TARGET").unwrap_or_else(|_| "unknown".to_string());

    println!("cargo:rustc-env=FSY_BUILD_GIT_COMMIT={commit}");
    println!("cargo:rustc-env=FSY_BUILD_GIT_DIRTY={dirty}");
    println!("cargo:rustc-env=FSY_BUILD_TARGET={target}");
}

fn git_stdout(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout)
        .ok()
        .map(|value| value.trim().to_string())
}
