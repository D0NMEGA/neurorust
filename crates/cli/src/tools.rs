//! External tool process execution (`cyclictest`, `hwlatdetect`).
//!
//! Every invocation uses `Command::new(path)` with an explicit `Vec<String>`
//! argument vector and never a shell, so a hostile rig slug or note can never
//! become an argument injection (T-1-09). Tool paths are resolved once per run
//! and their version captured up front via [`resolve_and_verify`] ("fail early if
//! a tool is missing"); the captured version string is then reused when the tool
//! is actually invoked with its real argument vector, so a tool is never spawned
//! twice just to learn its own version.

use std::path::{Path, PathBuf};
use std::process::Command;

use nr_manifest::ToolInvocation;
use thiserror::Error;

/// Overrides the resolved path to `cyclictest`. Test-only seam: `crates/cli/tests/
/// run_pipeline.rs` points this at `fake-cyclictest.sh` so the pipeline runs on a
/// dev host with no rig.
pub const CYCLICTEST_PATH_ENV: &str = "NRMEASURE_CYCLICTEST";

/// Overrides the resolved path to `hwlatdetect`. Same test-only purpose as
/// [`CYCLICTEST_PATH_ENV`].
pub const HWLATDETECT_PATH_ENV: &str = "NRMEASURE_HWLATDETECT";

#[derive(Debug, Error)]
pub enum ToolError {
    #[error("{name} not found at {path}: {source}", path = .path.display())]
    Missing {
        name: String,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// One completed tool invocation: the manifest record plus the raw stdout/stderr
/// bytes for the caller to inspect (e.g. `hwlatdetect`'s report is its own stdout).
pub struct ToolOutput {
    pub invocation: ToolInvocation,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// Resolves the path to one external tool: the named environment variable first
/// (the fake-binary seam tests use), falling back to the bare command name for
/// the shell's own `PATH` resolution otherwise.
pub fn resolve_tool_path(env_var: &str, default_name: &str) -> PathBuf {
    std::env::var(env_var)
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(default_name))
}

/// Confirms `path` can be spawned at all and returns its self-reported version.
/// A tool that fails to spawn is a hard error ("fail early if a tool is
/// missing"); a tool that spawns but exits non-zero for its own version flag is
/// not treated as missing, since not every build recognises the same version
/// flag the same way.
pub fn resolve_and_verify(
    name: &str,
    path: &Path,
    version_args: &[&str],
) -> Result<String, ToolError> {
    let output = Command::new(path)
        .args(version_args)
        .output()
        .map_err(|source| ToolError::Missing {
            name: name.to_string(),
            path: path.to_path_buf(),
            source,
        })?;
    Ok(first_reported_line(&output.stdout, &output.stderr))
}

fn first_reported_line(stdout: &[u8], stderr: &[u8]) -> String {
    let stdout_text = String::from_utf8_lossy(stdout);
    let text = if stdout_text.trim().is_empty() {
        String::from_utf8_lossy(stderr).into_owned()
    } else {
        stdout_text.into_owned()
    };
    let first = text.lines().next().unwrap_or_default().trim().to_string();
    if first.is_empty() {
        "version unknown".to_string()
    } else {
        first
    }
}

/// Runs one tool to completion with the given argument vector, built as a
/// `Vec<String>` and passed to `Command::args` verbatim, never through a shell.
/// `version` is the string [`resolve_and_verify`] already captured for this tool.
///
/// A non-zero exit is not an error here: BENCH-06 requires a tool failure to be
/// recorded, not hidden, so the caller (not this function) decides what a
/// non-zero `exit_code` in the returned [`ToolInvocation`] means for the run.
pub fn run_tool(
    name: &str,
    path: &Path,
    version: &str,
    argv: Vec<String>,
) -> Result<ToolOutput, ToolError> {
    let output = Command::new(path)
        .args(&argv)
        .output()
        .map_err(|source| ToolError::Missing {
            name: name.to_string(),
            path: path.to_path_buf(),
            source,
        })?;

    let mut full_argv = vec![name.to_string()];
    full_argv.extend(argv);

    Ok(ToolOutput {
        invocation: ToolInvocation {
            name: name.to_string(),
            version: version.to_string(),
            argv: full_argv,
            exit_code: output.status.code().unwrap_or(-1),
            // Populated later in run.rs::execute, once the artifacts this
            // invocation's output became are known.
            artifact_paths: Vec::new(),
        },
        stdout: output.stdout,
        stderr: output.stderr,
    })
}
