//! Embeds the commit this binary was BUILT from, so harness identity describes the
//! executable rather than whatever checkout happens to be the process's working
//! directory at run time.
//!
//! Runs at compile time, in this crate's own source directory, so it reads the
//! checkout that is actually being compiled. `crates/cli/src/cmd/run.rs::
//! harness_info` reads these three `env!` values instead of shelling out to git at
//! run time; see finding 6 of `01-EXTERNAL-AUDIT.md` for why that mattered (a run
//! launched by `systemd-run` with no working directory inside the checkout recorded
//! `git_sha: "unknown"` alongside `git_dirty: false`, asserting a clean tree nobody
//! observed).

fn main() {
    let sha = git(&["rev-parse", "HEAD"]);
    let dirty = git(&["status", "--porcelain"]).map(|s| !s.trim().is_empty());
    match (&sha, dirty) {
        (Some(sha), Some(dirty)) => {
            println!("cargo:rustc-env=NR_BUILD_GIT_SHA={sha}");
            println!("cargo:rustc-env=NR_BUILD_GIT_DIRTY={dirty}");
            println!("cargo:rustc-env=NR_BUILD_GIT_SHA_SOURCE=build-time");
        }
        _ => {
            println!("cargo:rustc-env=NR_BUILD_GIT_SHA=unavailable-at-build-time");
            println!("cargo:rustc-env=NR_BUILD_GIT_DIRTY=false");
            println!("cargo:rustc-env=NR_BUILD_GIT_SHA_SOURCE=unavailable");
        }
    }
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-changed=../../.git/index");
}

/// Mirrors the shape of `crates/cli/src/cmd/run.rs::git_output`: `None` on any
/// failure to spawn git or a non-zero exit, never a panic. A build script that
/// cannot spawn git (or is building outside a checkout entirely) must degrade to
/// the `unavailable` branch above, not fail the build.
fn git(args: &[&str]) -> Option<String> {
    std::process::Command::new("git")
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
}
