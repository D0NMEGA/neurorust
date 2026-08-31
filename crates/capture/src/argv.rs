//! Argv relativization, carried forward from plan 01-03's `deferred-items.md`
//! (Decision B): `nr_manifest::ToolInvocation::argv`'s doc comment states that any
//! output-file path in a recorded argv is relative to the run directory, "the
//! rewrite happens where argv is captured, in `nr-capture`". None of this plan's
//! three tasks shell out to a tool themselves (that is nr-cli's job, plan 01-07,
//! per this plan's own `<interfaces>` block: `// ... the measurement runs ...`
//! happens between two `interference::snapshot` calls, outside this crate), so
//! there is no `ToolInvocation` built here to relativize inline. This module is the
//! standalone, tested utility nr-cli calls at the point it does capture an argv,
//! so the actual rewrite still lives in `nr-capture` as the doc comment promises.

use std::path::Path;

/// Rewrites every `argv` token that is an absolute path under `run_dir` to be
/// relative to it, so a recorded `ToolInvocation.argv` is a command a third party
/// can paste and run from inside the published run directory, rather than an
/// unusable absolute `/home/...` path. Handles both a bare absolute-path token and
/// a `--flag=/abs/path` token; anything else (a flag with no path, a relative path,
/// a path outside `run_dir`) passes through unchanged.
pub fn relativize_argv(run_dir: &Path, argv: &[String]) -> Vec<String> {
    argv.iter()
        .map(|token| relativize_token(run_dir, token))
        .collect()
}

fn relativize_token(run_dir: &Path, token: &str) -> String {
    if let Some((flag, value)) = token.split_once('=') {
        if flag.starts_with('-') {
            return format!("{flag}={}", relativize_path_str(run_dir, value));
        }
    }
    relativize_path_str(run_dir, token)
}

fn relativize_path_str(run_dir: &Path, candidate: &str) -> String {
    let path = Path::new(candidate);
    if !path.is_absolute() {
        return candidate.to_string();
    }
    match path.strip_prefix(run_dir) {
        Ok(relative) => relative.to_string_lossy().into_owned(),
        Err(_) => candidate.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relativizes_output_file_paths_under_the_run_directory() {
        let run_dir = Path::new("/home/d0nmega/measurements/2026-09-01-precision3591-headline-001");
        let argv = vec![
            "cyclictest".to_string(),
            "--mainaffinity=0,1".to_string(),
            "--affinity=6-11".to_string(),
            format!(
                "--histfile={}/cyclictest-rt-isolated-idle-10m.hist",
                run_dir.display()
            ),
            format!("--json={}/results.json", run_dir.display()),
        ];

        let relativized = relativize_argv(run_dir, &argv);

        assert_eq!(
            relativized,
            vec![
                "cyclictest".to_string(),
                "--mainaffinity=0,1".to_string(),
                "--affinity=6-11".to_string(),
                "--histfile=cyclictest-rt-isolated-idle-10m.hist".to_string(),
                "--json=results.json".to_string(),
            ]
        );
    }

    #[test]
    fn leaves_paths_outside_the_run_directory_untouched() {
        let run_dir = Path::new("/home/d0nmega/measurements/run-001");
        let argv = vec!["--config=/etc/cyclictest.conf".to_string()];
        assert_eq!(relativize_argv(run_dir, &argv), argv);
    }

    #[test]
    fn leaves_relative_paths_and_bare_flags_untouched() {
        let run_dir = Path::new("/home/d0nmega/measurements/run-001");
        let argv = vec![
            "--threads".to_string(),
            "--priority=99".to_string(),
            "--histfile=already-relative.hist".to_string(),
        ];
        assert_eq!(relativize_argv(run_dir, &argv), argv);
    }
}
