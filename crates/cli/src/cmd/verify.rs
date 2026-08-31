//! `nrmeasure verify`: the D-13 blocking provenance gate.
//!
//! Three checks, run over the whole tree, every failure collected before exiting (never
//! stopping at the first):
//!
//!   1. Every immediate subdirectory of `measurements/` has a readable `manifest.json` that
//!      deserialises into a [`RunManifest`] and passes [`nr_manifest::validate`].
//!   2. No capture-shaped file exists anywhere in the git-tracked tree except inside a run
//!      directory that lists it (by path and by blake3), or under one of two narrow exempt
//!      trees: `crates/*/tests/fixtures/` or `docs/rig/recon-*/` with a filename beginning
//!      `probe-`. This is the T-1-05 mitigation: scoped by what a file *is*, not only by
//!      where it sits, so moving a figure out of `measurements/` does not evade the gate.
//!   3. `measurements/INDEX.md`, generated from every manifest, either gets written
//!      (`--write-index`) or compared against what is on disk (`--check-index`).
//!
//! Exit code 0 on success, 1 on any check failure.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::Args as ClapArgs;
use nr_manifest::{ArtifactKind, RunManifest};
use nr_metrics::index::{RunSummary, render_index};
use time::macros::format_description;

#[derive(ClapArgs, Debug)]
pub struct Args {
    /// Repository root, used to scope the stray-capture scan (check 2).
    #[arg(long, default_value = ".")]
    pub root: PathBuf,

    /// Root directory containing run directories to verify.
    #[arg(long = "measurements", default_value = "./measurements")]
    pub measurements_root: PathBuf,

    /// Treat every detected problem as a failure. The CI gate always passes this; printed in
    /// the summary line so a reader can confirm the gate ran in its blocking configuration.
    #[arg(long)]
    pub strict: bool,

    /// Regenerate measurements/INDEX.md from every manifest and write it.
    #[arg(long)]
    pub write_index: bool,

    /// Fail if measurements/INDEX.md differs from the generated form. Never auto-repairs: a
    /// hand-edited index (the easy way to make a losing configuration disappear from the
    /// published surface) must fail the build rather than be silently regenerated.
    #[arg(long)]
    pub check_index: bool,
}

/// A file anywhere in the tree matching one of these glob-like patterns (a single `*` standing
/// in for any run of characters) is capture-shaped: raw instrument output that must either be
/// checksummed inside a valid run directory or live in one of the two exempt trees below.
const CAPTURE_GLOBS: &[&str] = &[
    "*.hist",
    "cyclictest*.json",
    "hwlatdetect*.txt",
    "*.trace.dat",
    "trace-*.txt",
    "timerlat*.txt",
    "osnoise*.txt",
];

/// Exempt tree (b): `crates/*/tests/fixtures/`. Test input, not a published figure.
const EXEMPT_FIXTURES_MARKERS: (&str, &str) = ("tests", "fixtures");

/// Exempt tree (c): `docs/rig/recon-*/` with a filename beginning `probe-`. Rig-recon probes,
/// documented as never a measurement (see `crates/capture/tests/fixtures/README.md`).
const RECON_DIR_PREFIX: &str = "recon-";
const RECON_PROBE_PREFIX: &str = "probe-";

pub fn run(args: Args) -> anyhow::Result<i32> {
    let root_abs = args
        .root
        .canonicalize()
        .with_context(|| format!("failed to resolve --root {}", args.root.display()))?;
    let measurements_abs = resolve_absolute(&root_abs, &args.measurements_root);
    let measurements_abs = measurements_abs.canonicalize().unwrap_or(measurements_abs);
    let measurements_rel = measurements_abs
        .strip_prefix(&root_abs)
        .unwrap_or(&measurements_abs)
        .to_path_buf();

    let mut problems: Vec<String> = Vec::new();

    let (run_problems, run_dir_count, loaded) = check_run_directories(&measurements_abs);
    problems.extend(run_problems);

    let stray_problems = check_stray_captures(&root_abs, &measurements_rel, &loaded);
    problems.extend(stray_problems);

    if args.write_index || args.check_index {
        let index_text = render_index(&build_summaries(&measurements_abs, &loaded));
        let index_path = measurements_abs.join("INDEX.md");

        if args.write_index {
            std::fs::create_dir_all(&measurements_abs).with_context(|| {
                format!(
                    "failed to create {} before writing INDEX.md",
                    measurements_abs.display()
                )
            })?;
            std::fs::write(&index_path, &index_text)
                .with_context(|| format!("failed to write {}", index_path.display()))?;
        }

        if args.check_index {
            match std::fs::read_to_string(&index_path) {
                Ok(on_disk) if on_disk == index_text => {}
                Ok(_) => problems.push(format!(
                    "{}: does not match the generated index (run with --write-index to regenerate)",
                    index_path.display()
                )),
                Err(_) => problems.push(format!(
                    "{}: missing; run with --write-index to generate it",
                    index_path.display()
                )),
            }
        }
    }

    let strict_note = if args.strict { " (strict)" } else { "" };
    println!(
        "verify: {run_dir_count} run directories, {} problems{strict_note}",
        problems.len()
    );
    for problem in &problems {
        println!("{problem}");
    }

    Ok(if problems.is_empty() { 0 } else { 1 })
}

fn resolve_absolute(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}

// ---------------------------------------------------------------------------------
// Check 1: every run directory is complete.
// ---------------------------------------------------------------------------------

/// Validates every immediate subdirectory of `measurements_root`. Returns every problem found
/// (never stopping at the first), the number of run directories examined, and every manifest
/// that at least *deserialised* successfully (regardless of whether `nr_manifest::validate`
/// also found a problem with it), keyed by the run directory's own name. Check 2 uses this map
/// to avoid re-reporting a directory whose manifest could not be loaded at all.
fn check_run_directories(
    measurements_root: &Path,
) -> (Vec<String>, usize, HashMap<String, RunManifest>) {
    let mut problems = Vec::new();
    let mut loaded = HashMap::new();
    let mut run_dir_count = 0usize;

    let Ok(entries) = std::fs::read_dir(measurements_root) else {
        // measurements/ does not exist yet on a fresh checkout. Nothing to check; not itself a
        // problem (an empty tree has no orphan captures either).
        return (problems, run_dir_count, loaded);
    };

    let mut dirs: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    dirs.sort();

    for dir in dirs {
        run_dir_count += 1;
        let name = dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let manifest_path = dir.join("manifest.json");

        let text = match std::fs::read_to_string(&manifest_path) {
            Ok(text) => text,
            Err(_) => {
                problems.push(format!(
                    "{}: missing or unreadable manifest.json (orphan capture)",
                    dir.display()
                ));
                continue;
            }
        };

        let manifest: RunManifest = match serde_json::from_str(&text) {
            Ok(manifest) => manifest,
            Err(err) => {
                problems.push(format!(
                    "{}: manifest.json failed to parse: {err}",
                    manifest_path.display()
                ));
                continue;
            }
        };

        if let Err(errors) = nr_manifest::validate(&dir, &manifest) {
            for err in errors {
                problems.push(format!("{}: {err}", dir.display()));
            }
        }

        loaded.insert(name, manifest);
    }

    (problems, run_dir_count, loaded)
}

// ---------------------------------------------------------------------------------
// Check 2: no stray captures anywhere in the tree.
// ---------------------------------------------------------------------------------

/// Lists every file `git` considers part of the tree (tracked, or untracked but not
/// `.gitignore`d), relative to `root`. Falls back to a plain recursive walk (skipping `.git`
/// and `target`) when `root` is not inside a git repository, so the check still works on a
/// bare checkout.
fn list_tracked_files(root: &Path) -> Vec<PathBuf> {
    git_ls_files(root).unwrap_or_else(|| walk_fallback(root))
}

fn git_ls_files(root: &Path) -> Option<Vec<PathBuf>> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "--cached", "--others", "--exclude-standard"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Some(
        text.lines()
            .filter(|line| !line.trim().is_empty())
            .map(PathBuf::from)
            .collect(),
    )
}

fn walk_fallback(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk_dir(root, root, &mut out);
    out
}

fn walk_dir(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(|entry| entry.ok()) {
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if path.is_dir() {
            if name_str == ".git" || name_str == "target" {
                continue;
            }
            walk_dir(root, &path, out);
        } else if let Ok(rel) = path.strip_prefix(root) {
            out.push(rel.to_path_buf());
        }
    }
}

fn is_capture_shaped(filename: &str) -> bool {
    CAPTURE_GLOBS
        .iter()
        .any(|pattern| matches_single_star_glob(pattern, filename))
}

/// Matches `pattern` (containing at most one `*`, standing in for any run of characters, zero
/// or more) against `filename`. Every entry in [`CAPTURE_GLOBS`] has this shape, so a
/// hand-rolled prefix/suffix check is sufficient and avoids a glob-matching dependency.
fn matches_single_star_glob(pattern: &str, filename: &str) -> bool {
    match pattern.split_once('*') {
        Some((prefix, suffix)) => {
            filename.len() >= prefix.len() + suffix.len()
                && filename.starts_with(prefix)
                && filename.ends_with(suffix)
        }
        None => pattern == filename,
    }
}

/// Exempt tree (b): a path shaped `crates/<any>/tests/fixtures/...`.
fn is_under_crate_fixtures(rel_path: &Path) -> bool {
    let comps: Vec<String> = rel_path
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    let (tests_marker, fixtures_marker) = EXEMPT_FIXTURES_MARKERS;
    comps.len() >= 4
        && comps[0] == "crates"
        && comps[2] == tests_marker
        && comps[3] == fixtures_marker
}

/// Exempt tree (c): a path shaped `docs/rig/recon-*/probe-...`.
fn is_exempt_recon_probe(rel_path: &Path) -> bool {
    let comps: Vec<String> = rel_path
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    if comps.len() < 3 || comps[0] != "docs" || comps[1] != "rig" {
        return false;
    }
    if !comps[2].starts_with(RECON_DIR_PREFIX) {
        return false;
    }
    rel_path
        .file_name()
        .map(|f| f.to_string_lossy().starts_with(RECON_PROBE_PREFIX))
        .unwrap_or(false)
}

/// Whether `abs_path` (a capture-shaped file physically present on disk, at `rel_in_run`
/// relative to its run directory) is listed in `manifest.artifacts` with a matching path AND a
/// matching blake3 (a file present on disk but absent from, or disagreeing with, the manifest
/// fails either way).
fn artifact_listed(manifest: &RunManifest, abs_path: &Path, rel_in_run: &Path) -> bool {
    let rel_str = rel_in_run.to_string_lossy();
    let Ok(actual_blake3) = nr_manifest::blake3_file(abs_path) else {
        return false;
    };
    manifest
        .artifacts
        .iter()
        .any(|artifact| artifact.path == rel_str && artifact.blake3 == actual_blake3)
}

fn check_stray_captures(
    root: &Path,
    measurements_rel: &Path,
    loaded: &HashMap<String, RunManifest>,
) -> Vec<String> {
    let mut problems = Vec::new();

    for rel in list_tracked_files(root) {
        let Some(filename) = rel.file_name().map(|f| f.to_string_lossy().into_owned()) else {
            continue;
        };
        if !is_capture_shaped(&filename) {
            continue;
        }

        if let Ok(rel_to_measurements) = rel.strip_prefix(measurements_rel) {
            let mut comps = rel_to_measurements.components();
            if let Some(run_dir_component) = comps.next() {
                let run_dir_name = run_dir_component.as_os_str().to_string_lossy().into_owned();
                if let Some(manifest) = loaded.get(&run_dir_name) {
                    let rel_in_run = rel_to_measurements
                        .strip_prefix(run_dir_component.as_os_str())
                        .unwrap_or(rel_to_measurements);
                    let abs = root.join(&rel);
                    if !artifact_listed(manifest, &abs, rel_in_run) {
                        problems.push(format!(
                            "{}: capture-shaped file is not listed (with a matching checksum) \
                             in {}'s manifest",
                            rel.display(),
                            run_dir_name
                        ));
                    }
                }
                // If the run directory has no loaded manifest, check 1 already reported its
                // root cause (missing or unparsable manifest.json); no further per-file noise.
                continue;
            }
        }

        if is_under_crate_fixtures(&rel) || is_exempt_recon_probe(&rel) {
            continue;
        }

        problems.push(format!(
            "{}: capture-shaped file outside measurements/ and outside the exempt trees \
             (crates/*/tests/fixtures/ or docs/rig/recon-*/ with a probe- prefix)",
            rel.display()
        ));
    }

    problems
}

// ---------------------------------------------------------------------------------
// Check 3: the index.
// ---------------------------------------------------------------------------------

fn build_summaries(
    measurements_root: &Path,
    loaded: &HashMap<String, RunManifest>,
) -> Vec<RunSummary> {
    let mut names: Vec<&String> = loaded.keys().collect();
    names.sort();

    names
        .into_iter()
        .map(|name| {
            let manifest = &loaded[name];
            let run_dir = measurements_root.join(name);
            let (p99_us, max_us) = compute_percentiles(&run_dir, manifest).unwrap_or((0, 0));
            let date_format = format_description!("[year]-[month]-[day]");
            let date = manifest
                .utc_start
                .date()
                .format(&date_format)
                .unwrap_or_default();

            RunSummary {
                date,
                run_id: manifest.run_id.clone(),
                run_class: manifest.run_class.clone(),
                instrument_class: manifest.instrument_class.clone(),
                provenance_tier: manifest.provenance_tier.clone(),
                verdict: manifest.interference.verdict.clone(),
                p99_us,
                max_us,
                in_series: !manifest.excluded_from_series,
                reason: manifest.exclusion_reason.clone(),
            }
        })
        .collect()
}

/// Best-effort: locates the run's cyclictest histogram artifact and computes p99/max from it.
/// A run with no such artifact, or one that fails to parse, contributes `None` rather than
/// failing the whole index render; the artifact's own presence and checksum are already
/// checked by [`check_run_directories`] and [`check_stray_captures`].
fn compute_percentiles(run_dir: &Path, manifest: &RunManifest) -> Option<(u64, u64)> {
    let artifact = manifest
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == ArtifactKind::CyclictestHist)?;
    let hist_path = run_dir.join(&artifact.path);
    let run = nr_histogram::hist::parse_hist_file(&hist_path, None).ok()?;
    let percentiles = run.percentiles(&[0.99]).ok()?;
    let p99_us = percentiles.values.first().map(|(_, v)| *v).unwrap_or(0);
    Some((p99_us, percentiles.max_us))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_single_star_glob_handles_prefix_and_suffix() {
        assert!(matches_single_star_glob("*.hist", "cyclictest.hist"));
        assert!(matches_single_star_glob(
            "cyclictest*.json",
            "cyclictest.json"
        ));
        assert!(matches_single_star_glob(
            "cyclictest*.json",
            "cyclictest-rt.json"
        ));
        assert!(!matches_single_star_glob("cyclictest*.json", "other.json"));
        assert!(!matches_single_star_glob("*.hist", "results.txt"));
    }

    #[test]
    fn is_capture_shaped_matches_every_declared_pattern() {
        for name in [
            "a.hist",
            "cyclictest-foo.json",
            "hwlatdetect-15m.txt",
            "run.trace.dat",
            "trace-001.txt",
            "timerlat-top.txt",
            "osnoise-top.txt",
        ] {
            assert!(is_capture_shaped(name), "{name} should be capture-shaped");
        }
        assert!(!is_capture_shaped("README.md"));
        assert!(!is_capture_shaped("manifest.json"));
    }

    #[test]
    fn is_under_crate_fixtures_matches_the_documented_shape() {
        assert!(is_under_crate_fixtures(Path::new(
            "crates/histogram/tests/fixtures/sample.hist"
        )));
        assert!(!is_under_crate_fixtures(Path::new(
            "crates/histogram/tests/sample.hist"
        )));
        assert!(!is_under_crate_fixtures(Path::new("docs/sample.hist")));
    }

    #[test]
    fn is_exempt_recon_probe_requires_the_probe_prefix() {
        assert!(is_exempt_recon_probe(Path::new(
            "docs/rig/recon-2026-09-01/probe-foo.hist"
        )));
        assert!(!is_exempt_recon_probe(Path::new(
            "docs/rig/recon-2026-09-01/results.hist"
        )));
        assert!(!is_exempt_recon_probe(Path::new(
            "docs/rig/other-2026-09-01/probe-foo.hist"
        )));
    }
}
