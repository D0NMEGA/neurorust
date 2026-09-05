//! `nrmeasure verify`: the D-13 blocking provenance gate.
//!
//! Four checks, run over the whole tree, every failure collected before exiting (never
//! stopping at the first):
//!
//!   1. Every immediate subdirectory of `measurements/` has either a readable `manifest.json`
//!      that deserialises into a [`RunManifest`] and passes [`nr_manifest::validate`], or a
//!      readable `ATTEMPT.json` (task 1's [`AttemptRecord`]) with `status: failed`: a run that
//!      failed before producing a measurement is still a first-class published outcome
//!      (BENCH-06, finding 7 of `01-EXTERNAL-AUDIT.md`). A directory carrying both a manifest
//!      and a failed attempt record, or one left at `status: in-progress`, is reported as a
//!      defect; a directory with neither is reported exactly as an orphan capture always has
//!      been.
//!   2. No capture-shaped file exists anywhere in the git-tracked tree except inside a run
//!      directory that lists it (by path and by blake3) in its `manifest.json` OR in a failed
//!      attempt's `ATTEMPT.json` `preserved` array, or under one of two narrow exempt trees:
//!      `crates/*/tests/fixtures/` or `docs/rig/recon-*/` with a filename beginning `probe-`.
//!      This is the T-1-05 mitigation: scoped by what a file *is*, not only by where it sits,
//!      so moving a figure out of `measurements/` does not evade the gate.
//!   3. `measurements/INDEX.md`, generated from every manifest and every failed attempt,
//!      either gets written (`--write-index`) or compared against what is on disk
//!      (`--check-index`). A run whose histogram cannot be parsed, or a failed attempt, renders
//!      `unavailable` in its p99/max columns there, never a substituted zero (T-1-52).
//!   4. `--strict` only: every harness-generated run's published `hist.tsv` and `REPORT.md`
//!      `## Results` figures are re-derived from the run's own raw capture and compared
//!      (T-1-53). A run with no recorded `--histogram` bound, or a reconstructed run with no
//!      generated report, is recorded as not re-derivable rather than guessed at (T-1-54,
//!      T-1-55).
//!
//! Exit code 0 on success, 1 on any check failure.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::Args as ClapArgs;
use nr_histogram::hist::CyclictestRun;
use nr_manifest::{ArtifactKind, AttemptRecord, AttemptStatus, ProvenanceTier, RunManifest};
use nr_metrics::index::{RunOutcome, RunSummary, render_index};
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
    "hwnoise*.txt",
    "rtla-hwnoise*.txt",
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

    // Computed unconditionally (not only under --write-index/--check-index): a malformed
    // capture must fail --strict even when the caller only wants the orphan/checksum checks.
    let (summaries, percentile_problems) = build_summaries(&measurements_abs, &loaded, args.strict);
    problems.extend(percentile_problems);

    if args.write_index || args.check_index {
        let index_text = render_index(&summaries);
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

    let mut derived_summary: Option<(usize, usize)> = None;
    let mut derived_notes: Vec<String> = Vec::new();
    if args.strict {
        let derived = check_derived_figures(&measurements_abs, &loaded);
        problems.extend(derived.problems);
        derived_notes = derived.notes;
        derived_summary = Some((derived.rederived, derived.not_rederivable));
    }

    let strict_note = if args.strict { " (strict)" } else { "" };
    let derived_note = derived_summary
        .map(|(rederived, not_rederivable)| {
            format!("{rederived} re-derived, {not_rederivable} not re-derivable, ")
        })
        .unwrap_or_default();
    println!(
        "verify: {run_dir_count} run directories, {derived_note}{} problems{strict_note}",
        problems.len()
    );
    for problem in &problems {
        println!("{problem}");
    }
    for note in &derived_notes {
        println!("{note}");
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

/// What [`check_run_directories`] found for one run directory that verifies at all: either a
/// normal manifest, or a published failed attempt (task 1's [`AttemptRecord`], `status:
/// failed`; BENCH-06, finding 7 of `01-EXTERNAL-AUDIT.md`). Shared by check 2 (the
/// stray-capture scan) and check 3 (the index), so both understand the second valid shape a run
/// directory can take.
enum RunDirRecord {
    /// Both variants boxed: `RunManifest` and `AttemptRecord` are each large enough that
    /// leaving either unboxed would size every `RunDirRecord` to its larger variant.
    Manifest(Box<RunManifest>),
    FailedAttempt(Box<AttemptRecord>),
}

/// The result of reading and parsing one JSON sidecar file (`manifest.json` or `ATTEMPT.json`)
/// out of a run directory. `Invalid` means the file exists but failed to parse; the caller has
/// already recorded that as a problem and must not also report the directory as if the file
/// were simply missing.
enum Loaded<T> {
    Absent,
    Invalid,
    Valid(T),
}

fn load_json<T: serde::de::DeserializeOwned>(
    path: &Path,
    what: &str,
    problems: &mut Vec<String>,
) -> Loaded<T> {
    match std::fs::read_to_string(path) {
        Ok(text) => match serde_json::from_str::<T>(&text) {
            Ok(value) => Loaded::Valid(value),
            Err(err) => {
                problems.push(format!("{}: {what} failed to parse: {err}", path.display()));
                Loaded::Invalid
            }
        },
        Err(_) => Loaded::Absent,
    }
}

/// Validates every immediate subdirectory of `measurements_root`. Returns every problem found
/// (never stopping at the first), the number of run directories examined, and every directory
/// that at least *deserialised* successfully into one of the two valid shapes (regardless of
/// whether `nr_manifest::validate` also found a problem with a manifest), keyed by the run
/// directory's own name. Checks 2 and 3 use this map to avoid re-reporting a directory whose
/// records could not be loaded at all.
fn check_run_directories(
    measurements_root: &Path,
) -> (Vec<String>, usize, HashMap<String, RunDirRecord>) {
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

        let manifest: Loaded<RunManifest> =
            load_json(&dir.join("manifest.json"), "manifest.json", &mut problems);
        let attempt: Loaded<AttemptRecord> =
            load_json(&dir.join("ATTEMPT.json"), "ATTEMPT.json", &mut problems);

        match (manifest, attempt) {
            (Loaded::Valid(manifest), Loaded::Valid(attempt))
                if attempt.status == AttemptStatus::Failed =>
            {
                problems.push(format!(
                    "{}: has both manifest.json and an ATTEMPT.json with status failed; a \
                     failed attempt must not carry a manifest",
                    dir.display()
                ));
                if let Err(errors) = nr_manifest::validate(&dir, &manifest) {
                    for err in errors {
                        problems.push(format!("{}: {err}", dir.display()));
                    }
                }
                loaded.insert(name, RunDirRecord::Manifest(Box::new(manifest)));
            }
            (Loaded::Valid(manifest), _) => {
                // Either no ATTEMPT.json at all (every manifest committed before this plan),
                // or one with status completed (the normal shape going forward): both are a
                // manifest-bearing directory, validated exactly as before.
                if let Err(errors) = nr_manifest::validate(&dir, &manifest) {
                    for err in errors {
                        problems.push(format!("{}: {err}", dir.display()));
                    }
                }
                loaded.insert(name, RunDirRecord::Manifest(Box::new(manifest)));
            }
            (Loaded::Absent, Loaded::Valid(attempt)) => match attempt.status {
                AttemptStatus::Failed => {
                    loaded.insert(name, RunDirRecord::FailedAttempt(Box::new(attempt)));
                }
                AttemptStatus::InProgress => {
                    problems.push(format!(
                        "{}: ATTEMPT.json has status in-progress; a run that never finished \
                         must not be committed",
                        dir.display()
                    ));
                }
                AttemptStatus::Completed => {
                    // A completed attempt with no manifest is inconsistent (the harness always
                    // writes manifest.json before marking an attempt completed); reported the
                    // same way an orphan capture always has been.
                    problems.push(format!(
                        "{}: missing or unreadable manifest.json (orphan capture)",
                        dir.display()
                    ));
                }
            },
            (Loaded::Absent, Loaded::Absent) => {
                problems.push(format!(
                    "{}: missing or unreadable manifest.json (orphan capture)",
                    dir.display()
                ));
            }
            (Loaded::Invalid, _) | (Loaded::Absent, Loaded::Invalid) => {
                // A parse-failure problem was already pushed by load_json for whichever file
                // was malformed; nothing further to report for this directory.
            }
        }
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
/// relative to its run directory) is listed with a matching path AND a matching blake3 in
/// `record`'s own artifact list: `manifest.artifacts` for a measured run, or a failed attempt's
/// `preserved` array (task 1). A file present on disk but absent from, or disagreeing with,
/// that list fails either way. The exemption is by record and checksum, not by directory, so a
/// partial capture preserved by a failed attempt is still checksummed evidence rather than an
/// unaccounted file (T-1-61).
fn artifact_listed(record: &RunDirRecord, abs_path: &Path, rel_in_run: &Path) -> bool {
    let rel_str = rel_in_run.to_string_lossy();
    let Ok(actual_blake3) = nr_manifest::blake3_file(abs_path) else {
        return false;
    };
    let artifacts = match record {
        RunDirRecord::Manifest(manifest) => &manifest.artifacts,
        RunDirRecord::FailedAttempt(attempt) => &attempt.preserved,
    };
    artifacts
        .iter()
        .any(|artifact| artifact.path == rel_str && artifact.blake3 == actual_blake3)
}

fn check_stray_captures(
    root: &Path,
    measurements_rel: &Path,
    loaded: &HashMap<String, RunDirRecord>,
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
                if let Some(record) = loaded.get(&run_dir_name) {
                    let rel_in_run = rel_to_measurements
                        .strip_prefix(run_dir_component.as_os_str())
                        .unwrap_or(rel_to_measurements);
                    let abs = root.join(&rel);
                    if !artifact_listed(record, &abs, rel_in_run) {
                        problems.push(format!(
                            "{}: capture-shaped file is not listed (with a matching checksum) \
                             in {}'s manifest or preserved attempt record",
                            rel.display(),
                            run_dir_name
                        ));
                    }
                }
                // If the run directory has no loaded record, check 1 already reported its root
                // cause (missing or unparsable manifest.json/ATTEMPT.json); no further per-file
                // noise.
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

/// The one [`compute_percentiles`] failure, and the one [`check_derived_figures`] outcome, that
/// is never a `--strict` problem: a manifest with nothing to compute from at all (for example a
/// firmware-screen-only run with no cyclictest capture), as opposed to a capture that exists
/// and fails to parse. Shared by both so the two agree on the exact wording.
const NO_HISTOGRAM_ARTIFACT: &str = "no cyclictest histogram artifact in this manifest";

/// Builds one [`RunSummary`] per loaded manifest, sorted by run directory name, plus the list
/// of `--strict`-mode problems found while doing it. Under `--strict`, a percentile computation
/// failure other than [`NO_HISTOGRAM_ARTIFACT`] is a problem: a malformed but correctly
/// checksummed capture must fail the blocking gate rather than silently render `unavailable`
/// and pass it. Finding 6 of `01-EXTERNAL-AUDIT.md`.
fn build_summaries(
    measurements_root: &Path,
    loaded: &HashMap<String, RunDirRecord>,
    strict: bool,
) -> (Vec<RunSummary>, Vec<String>) {
    let mut names: Vec<&String> = loaded.keys().collect();
    names.sort();
    let mut problems = Vec::new();

    let summaries = names
        .into_iter()
        .map(|name| match &loaded[name] {
            RunDirRecord::Manifest(manifest) => {
                let run_dir = measurements_root.join(name);
                let (p99_us, max_us) = match compute_percentiles(&run_dir, manifest) {
                    Ok((p99, max)) => (Some(p99), Some(max)),
                    Err(reason) => {
                        if strict && reason != NO_HISTOGRAM_ARTIFACT {
                            problems.push(format!("{}: {reason}", run_dir.display()));
                        }
                        (None, None)
                    }
                };
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
                    verdict: Some(manifest.interference.verdict.clone()),
                    p99_us,
                    max_us,
                    in_series: !manifest.excluded_from_series,
                    reason: manifest.exclusion_reason.clone(),
                    outcome: RunOutcome::Measured,
                }
            }
            // BENCH-06/finding 7: a failed attempt gets a row too, never a computed value it
            // never produced. There is no `strict`-mode percentile attempt here at all: a
            // failed attempt is not required to have a parseable (or any) histogram, so its
            // absence is not a problem to report, only unavailable data to render.
            RunDirRecord::FailedAttempt(attempt) => {
                let date_format = format_description!("[year]-[month]-[day]");
                let date = attempt
                    .utc_start
                    .date()
                    .format(&date_format)
                    .unwrap_or_default();
                let reason = attempt.failure.as_ref().map(|failure| {
                    format!("attempt failed at {}: {}", failure.stage, failure.message)
                });

                RunSummary {
                    date,
                    run_id: attempt.run_id.clone(),
                    run_class: attempt.requested.run_class.clone(),
                    instrument_class: attempt.requested.instrument_class.clone(),
                    provenance_tier: ProvenanceTier::HarnessGenerated,
                    verdict: None,
                    p99_us: None,
                    max_us: None,
                    in_series: false,
                    reason,
                    outcome: RunOutcome::FailedAttempt,
                }
            }
        })
        .collect();

    (summaries, problems)
}

/// Locates the run's cyclictest histogram artifact and computes p99 and max from it. Returns
/// the reason on failure rather than a substituted value: a run with no such artifact, or one
/// that fails to parse, is reported as such by the caller (`unavailable` in the index, a named
/// problem under `--strict`), never as a computed zero. A substituted zero is indistinguishable
/// from a run that really observed zero, and one of those is evidence while the other is a
/// parse failure. The artifact's own presence and checksum are already checked by
/// [`check_run_directories`] and [`check_stray_captures`]; this only asks whether the numbers
/// behind it are also derivable. Finding 6 of `01-EXTERNAL-AUDIT.md`.
fn compute_percentiles(run_dir: &Path, manifest: &RunManifest) -> Result<(u64, u64), String> {
    let artifact = manifest
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == ArtifactKind::CyclictestHist)
        .ok_or_else(|| NO_HISTOGRAM_ARTIFACT.to_string())?;
    let hist_path = run_dir.join(&artifact.path);
    let run = nr_histogram::hist::parse_hist_file(&hist_path, None)
        .map_err(|err| format!("failed to parse {}: {err}", hist_path.display()))?;
    let percentiles = run.percentiles(&[0.99]).map_err(|err| {
        format!(
            "failed to compute percentiles from {}: {err}",
            hist_path.display()
        )
    })?;
    let &(_, p99_us) = percentiles
        .values
        .first()
        .expect("percentiles() returns one value per requested quantile");
    Ok((p99_us, percentiles.max_us))
}

// ---------------------------------------------------------------------------------
// Check 4 (--strict only): re-derive every published number from the raw capture.
// ---------------------------------------------------------------------------------

/// The result of [`check_derived_figures`]: every disagreement found (a `--strict` problem),
/// every run recorded as not re-derivable (never a problem; printed for visibility, T-1-55),
/// and the two counts `verify`'s own summary line reports.
struct DerivedFiguresReport {
    problems: Vec<String>,
    notes: Vec<String>,
    rederived: usize,
    not_rederivable: usize,
}

/// Re-derives every harness-generated run's published `hist.tsv` and `REPORT.md` `## Results`
/// figures from its own raw capture and compares them. This is what makes a checksummed
/// capture mean something beyond "unchanged since it was written": the published numbers are
/// checked against the evidence rather than trusted because the harness wrote them (T-1-53).
///
/// A run is recorded as not re-derivable, rather than checked or failed, when:
///   - its `provenance_tier` is `reconstructed` (no generated report exists to check; T-1-55),
///     or
///   - its recorded argv carries no `--histogram=<n>` for the `cyclictest` invocation: guessing
///     a plausible-looking default bound would produce a re-derivation that agrees with itself
///     by construction and proves nothing (T-1-54).
///
/// A failed attempt (task 1's [`AttemptRecord`]) is skipped outright, counted in neither
/// bucket: it has no manifest and no generated report at all, which is a different thing from a
/// measured run this check cannot re-derive, not a variant of it.
fn check_derived_figures(
    measurements_root: &Path,
    loaded: &HashMap<String, RunDirRecord>,
) -> DerivedFiguresReport {
    let mut problems = Vec::new();
    let mut notes = Vec::new();
    let mut rederived = 0usize;
    let mut not_rederivable = 0usize;

    let mut names: Vec<&String> = loaded.keys().collect();
    names.sort();

    for name in names {
        let RunDirRecord::Manifest(manifest) = &loaded[name] else {
            continue;
        };
        let run_dir = measurements_root.join(name);

        if matches!(manifest.provenance_tier, ProvenanceTier::Reconstructed) {
            not_rederivable += 1;
            notes.push(format!(
                "{}: not re-derivable: reconstructed run has no generated report",
                run_dir.display()
            ));
            continue;
        }

        let Some(bound) = recorded_histogram_bound(manifest) else {
            not_rederivable += 1;
            notes.push(format!(
                "{}: not re-derivable: no recorded --histogram bound",
                run_dir.display()
            ));
            continue;
        };

        let Some(artifact) = manifest
            .artifacts
            .iter()
            .find(|artifact| artifact.kind == ArtifactKind::CyclictestHist)
        else {
            not_rederivable += 1;
            notes.push(format!(
                "{}: not re-derivable: {NO_HISTOGRAM_ARTIFACT}",
                run_dir.display()
            ));
            continue;
        };

        let hist_path = run_dir.join(&artifact.path);
        let run = match nr_histogram::hist::parse_hist_file(&hist_path, Some(bound)) {
            Ok(run) => run,
            Err(err) => {
                problems.push(format!(
                    "{}: failed to re-derive from its own capture: {err}",
                    hist_path.display()
                ));
                continue;
            }
        };
        rederived += 1;

        let hist_tsv_path = run_dir.join("hist.tsv");
        match std::fs::read_to_string(&hist_tsv_path) {
            Ok(committed) => {
                let regenerated = crate::cmd::run::format_hist_tsv(&run);
                if let Some((line_no, committed_line, regenerated_line)) =
                    first_differing_line(&committed, &regenerated)
                {
                    problems.push(format!(
                        "{}: disagrees with the re-derived hist.tsv at line {line_no}: \
                         committed={committed_line:?}, re-derived={regenerated_line:?}",
                        hist_tsv_path.display()
                    ));
                }
            }
            Err(_) => problems.push(format!(
                "{}: missing; cannot re-derive hist.tsv",
                hist_tsv_path.display()
            )),
        }

        let report_path = run_dir.join("REPORT.md");
        match std::fs::read_to_string(&report_path) {
            Ok(committed) => match parse_report_results(&committed) {
                Some(published) => {
                    problems.extend(compare_derived_results(&report_path, &published, &run));
                }
                None => problems.push(format!(
                    "{}: could not find a ## Results block to re-derive against",
                    report_path.display()
                )),
            },
            Err(_) => problems.push(format!(
                "{}: missing; cannot re-derive REPORT.md",
                report_path.display()
            )),
        }
    }

    DerivedFiguresReport {
        problems,
        notes,
        rederived,
        not_rederivable,
    }
}

/// Parses the `--histogram=<n>` bound out of the `cyclictest` tool invocation's recorded argv.
/// `None` when the tools array carries no `cyclictest` invocation, or that invocation carries
/// no `--histogram=` token: the caller must record the run as not re-derivable rather than
/// assume a bound (T-1-54). `run.rs` never emits any other flag form; only this one appears in
/// a real captured argv.
fn recorded_histogram_bound(manifest: &RunManifest) -> Option<u64> {
    manifest
        .tools
        .iter()
        .find(|tool| tool.name == "cyclictest")
        .and_then(|tool| {
            tool.argv
                .iter()
                .find_map(|arg| arg.strip_prefix("--histogram="))
        })
        .and_then(|value| value.parse::<u64>().ok())
}

/// The 1-based line number and the two differing lines, at the first place `committed` and
/// `regenerated` disagree; `None` when every line matches.
fn first_differing_line<'a>(
    committed: &'a str,
    regenerated: &'a str,
) -> Option<(usize, &'a str, &'a str)> {
    let mut committed_lines = committed.lines();
    let mut regenerated_lines = regenerated.lines();
    let mut line_no = 0usize;
    loop {
        line_no += 1;
        match (committed_lines.next(), regenerated_lines.next()) {
            (None, None) => return None,
            (a, b) if a == b => continue,
            (a, b) => return Some((line_no, a.unwrap_or(""), b.unwrap_or(""))),
        }
    }
}

/// The `## Results` block figures parsed out of a generated `REPORT.md`, by plain line
/// matching against the fixed shapes `render_results` (`nr_metrics::report`) emits: see this
/// plan's `<interfaces>` block for a worked example.
struct ReportResults {
    /// `(label, value_us)`, e.g. `("p99", 9)`, in file order.
    percentiles: Vec<(String, u64)>,
    sample_count: u64,
    overflow_count: u64,
    maximum_us: u64,
}

/// Parses a `## Results` block out of a generated `REPORT.md`. `None` when any of the three
/// scalar lines (`sample count:`, `overflow count:`, `maximum:`) could not be found; a real
/// generated report always has all three, and a hand-truncated one is the only way to hit this,
/// reported as its own problem by the caller.
fn parse_report_results(report_text: &str) -> Option<ReportResults> {
    const LABELS: [&str; 4] = ["p50", "p95", "p99", "p99.9"];
    let mut percentiles: Vec<(String, u64)> = Vec::new();
    let mut sample_count = None;
    let mut overflow_count = None;
    let mut maximum_us = None;

    for line in report_text.lines() {
        let trimmed = line.trim();
        if let Some(cells) = table_row_cells(trimmed) {
            if cells.len() == 2 && LABELS.contains(&cells[0]) {
                if let Ok(value) = cells[1].parse::<u64>() {
                    percentiles.push((cells[0].to_string(), value));
                }
            }
        } else if let Some(rest) = trimmed.strip_prefix("sample count:") {
            sample_count = rest.trim().parse::<u64>().ok();
        } else if let Some(rest) = trimmed.strip_prefix("overflow count:") {
            overflow_count = rest.trim().parse::<u64>().ok();
        } else if let Some(rest) = trimmed.strip_prefix("maximum:") {
            maximum_us = rest
                .trim()
                .strip_suffix("us")
                .and_then(|value| value.trim().parse::<u64>().ok());
        }
    }

    Some(ReportResults {
        percentiles,
        sample_count: sample_count?,
        overflow_count: overflow_count?,
        maximum_us: maximum_us?,
    })
}

/// Splits a markdown table row `| a | b |` into its trimmed cells. `None` for a line that is
/// not a pipe-delimited row at all.
fn table_row_cells(line: &str) -> Option<Vec<&str>> {
    let inner = line.strip_prefix('|')?.strip_suffix('|')?;
    Some(inner.split('|').map(str::trim).collect())
}

/// Compares the re-derived percentiles, sample count, overflow count and maximum against what
/// `REPORT.md` publishes, returning one problem string per disagreement (never stopping at the
/// first) so a reader sees every field that drifted, not just the earliest one.
fn compare_derived_results(
    report_path: &Path,
    published: &ReportResults,
    run: &CyclictestRun,
) -> Vec<String> {
    let mut problems = Vec::new();

    let percentiles = match run.percentiles(&[0.5, 0.95, 0.99, 0.999]) {
        Ok(percentiles) => percentiles,
        Err(err) => {
            problems.push(format!(
                "{}: failed to re-derive percentiles: {err}",
                report_path.display()
            ));
            return problems;
        }
    };

    const LABELS: [&str; 4] = ["p50", "p95", "p99", "p99.9"];
    for (label, &(_, derived_value)) in LABELS.iter().zip(percentiles.values.iter()) {
        let published_value = published
            .percentiles
            .iter()
            .find(|entry| entry.0.as_str() == *label)
            .map(|entry| entry.1);
        match published_value {
            Some(value) if value == derived_value => {}
            Some(value) => problems.push(format!(
                "{}: {label} disagrees: published {value}, re-derived {derived_value}",
                report_path.display()
            )),
            None => problems.push(format!(
                "{}: {label} is missing from the published Results block",
                report_path.display()
            )),
        }
    }

    if published.sample_count != percentiles.total_samples {
        problems.push(format!(
            "{}: sample count disagrees: published {}, re-derived {}",
            report_path.display(),
            published.sample_count,
            percentiles.total_samples
        ));
    }
    if published.overflow_count != percentiles.overflow_samples {
        problems.push(format!(
            "{}: overflow count disagrees: published {}, re-derived {}",
            report_path.display(),
            published.overflow_count,
            percentiles.overflow_samples
        ));
    }
    if published.maximum_us != percentiles.max_us {
        problems.push(format!(
            "{}: maximum disagrees: published {} us, re-derived {} us",
            report_path.display(),
            published.maximum_us,
            percentiles.max_us
        ));
    }

    problems
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
            "hwnoise-top.txt",
            "rtla-hwnoise-probe.txt",
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
