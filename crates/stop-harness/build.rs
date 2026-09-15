//! Embeds the commit this binary was BUILT from, so harness identity describes the executable
//! rather than whatever checkout happens to be the process's working directory at run time.
//!
//! This is a deliberate copy of `crates/cli/build.rs`, not a shared dependency: the project's
//! own rule is to copy twice before abstracting, and this is forty lines. A change to one should
//! be made to both.
//!
//! Runs at compile time, in this crate's own source directory, so it reads the checkout that is
//! actually being compiled, rather than shelling out to git at run time; see finding 6 of
//! `01-EXTERNAL-AUDIT.md` for why that mattered elsewhere (a run launched with no working
//! directory inside the checkout recorded `git_sha: "unknown"` alongside `git_dirty: false`,
//! asserting a clean tree nobody observed).
//!
//! A build machine can also have no git checkout to read at all, rather than merely the wrong
//! one: the reference rig's `~/neurorust` is an rsync mirror with no `.git`, so `git rev-parse
//! HEAD` fails there in exactly the place a stale-binary check needs it to succeed. For that
//! case, `scripts/nr-push-to-rig.sh` stamps the revision it is pushing into `.git-sha`, which
//! this build reads and records under its own source value, `pushed-stamp`: an asserted
//! revision, labelled as asserted, never presented as the build-time observation git itself
//! could not make (D-29).

fn main() {
    let sha = git(&["rev-parse", "HEAD"]);
    let dirty = git(&["status", "--porcelain"]).map(|s| !s.trim().is_empty());
    match (&sha, dirty) {
        (Some(sha), Some(dirty)) => {
            println!("cargo:rustc-env=NR_BUILD_GIT_SHA={sha}");
            println!("cargo:rustc-env=NR_BUILD_GIT_DIRTY={dirty}");
            println!("cargo:rustc-env=NR_BUILD_GIT_SHA_SOURCE=build-time");
        }
        _ => match read_stamp() {
            Some(stamp) => {
                println!("cargo:rustc-env=NR_BUILD_GIT_SHA={}", stamp.sha);
                println!("cargo:rustc-env=NR_BUILD_GIT_DIRTY={}", stamp.dirty);
                println!("cargo:rustc-env=NR_BUILD_GIT_SHA_SOURCE=pushed-stamp");
            }
            None => {
                println!("cargo:rustc-env=NR_BUILD_GIT_SHA=unavailable-at-build-time");
                println!("cargo:rustc-env=NR_BUILD_GIT_DIRTY=false");
                println!("cargo:rustc-env=NR_BUILD_GIT_SHA_SOURCE=unavailable");
            }
        },
    }
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-changed=../../.git/index");
    println!("cargo:rerun-if-changed=../../.git-sha");
}

/// Mirrors the shape of `crates/cli/src/cmd/run.rs::git_output`: `None` on any failure to spawn
/// git or a non-zero exit, never a panic. A build script that cannot spawn git (or is building
/// outside a checkout entirely) must degrade to the `unavailable` branch above, not fail the
/// build.
fn git(args: &[&str]) -> Option<String> {
    std::process::Command::new("git")
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// The revision `scripts/nr-push-to-rig.sh` stamped into the source tree it pushed.
struct Stamp {
    sha: String,
    dirty: bool,
}

/// Reads `../../.git-sha`, written by `scripts/nr-push-to-rig.sh`. Returns `None` on a missing
/// or unreadable file, or on a line shape it does not recognise: a half-read or hand-edited
/// stamp must never become a confident sha, so any deviation from exactly what the script writes
/// falls through to the `unavailable` branch in `main` rather than reporting a value nobody
/// asserted.
fn read_stamp() -> Option<Stamp> {
    let text = std::fs::read_to_string("../../.git-sha").ok()?;
    let mut sha = None;
    let mut dirty = None;
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("sha=") {
            sha = Some(value.to_string());
        } else if let Some(value) = line.strip_prefix("dirty=") {
            dirty = Some(value.to_string());
        }
    }
    let sha = sha.filter(|s| {
        s.len() == 40
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })?;
    let dirty = match dirty.as_deref() {
        Some("true") => true,
        Some("false") => false,
        _ => return None,
    };
    Some(Stamp { sha, dirty })
}
