//! `nrmeasure series`: turns a set of committed run directories into the BENCH-08 weekly
//! metrics JSON, and compares it against the reviewed baseline (D-11).
//!
//! Three exclusions apply to `--append`, all load-bearing:
//!
//!   1. A run whose `instrument_class` is `investigation` is never appended. RESEARCH.md
//!      pattern 2 is explicit that an investigation run carries tracer overhead and must never
//!      feed the regression series. The skipped run id is printed so the exclusion is visible.
//!   2. A directory holding only a failed `ATTEMPT.json` (plan 01-17) is never appended. It is
//!      printed as skipped with its failure stage; it already appears in `measurements/INDEX.md`
//!      (BENCH-06).
//!   3. A run whose `excluded_from_series` is true IS appended, with that flag and its reason
//!      carried through, so it appears in the record but is skipped by `--compare`. That is the
//!      other half of BENCH-06: the losing run is in the file, it just does not move the
//!      baseline.
//!
//! `--append` also feeds [`nr_metrics::coverage`], so a machine that was off for two weeks
//! produces two recorded gaps rather than a silent hole (D-08). Nothing here ever writes a
//! metric value for a week with no run directory; [`nr_metrics::coverage::record_gaps`] cannot
//! represent one.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Args as ClapArgs;
use nr_capture::hwnoise;
use nr_manifest::{
    ArtifactKind, AttemptRecord, AttemptStatus, ContaminationVerdict, InstrumentClass, RunManifest,
};
use nr_metrics::baseline::{Baseline, BaselineEntry, RegressionVerdict, compare};
use nr_metrics::coverage::{Coverage, GapReason, WeekRecord, record_gap, record_gaps};
use nr_metrics::series::{MetricsSeries, StageMetrics, append};
use time::OffsetDateTime;

#[derive(ClapArgs, Debug)]
pub struct Args {
    /// Root directory containing run directories to fold into the series.
    #[arg(long = "measurements", default_value = "./measurements")]
    pub measurements_root: PathBuf,

    /// Root directory holding latency-series.json, coverage.json and baseline.json.
    #[arg(long = "metrics", default_value = "./metrics")]
    pub metrics_root: PathBuf,

    /// Scan run directories and append any not yet in the series.
    #[arg(long)]
    pub append: bool,

    /// Compare every entry against the baseline and print a verdict per entry.
    #[arg(long)]
    pub compare: bool,

    /// Override the baseline path. CI reads the baseline at the event's own base revision
    /// rather than the working tree; see `.github/workflows/regression.yml`.
    #[arg(long)]
    pub baseline: Option<PathBuf>,

    /// Record the current ISO week as a coverage gap caused by a precondition refusal (D-08).
    #[arg(long = "record-refusal", value_name = "CHECK")]
    pub record_refusal: Option<String>,

    /// Write the named run's series entries into metrics/baseline.json, keyed by
    /// (rig_slug, run_class, stage).
    #[arg(long = "seed-baseline", value_name = "RUN_ID")]
    pub seed_baseline: Option<String>,
}

pub fn run(args: Args) -> Result<i32> {
    if let Some(check) = &args.record_refusal {
        return run_record_refusal(&args.metrics_root, check);
    }
    if let Some(run_id) = &args.seed_baseline {
        return run_seed_baseline(&args.metrics_root, run_id);
    }
    if !args.append && !args.compare {
        anyhow::bail!(
            "nrmeasure series: specify at least one of --append, --compare, --record-refusal \
             or --seed-baseline"
        );
    }

    let mut exit_code = 0;
    if args.append {
        exit_code = exit_code.max(run_append(&args.measurements_root, &args.metrics_root)?);
    }
    if args.compare {
        exit_code = exit_code.max(run_compare(&args.metrics_root, args.baseline.as_deref())?);
    }
    Ok(exit_code)
}

// ---------------------------------------------------------------------------------
// --append
// ---------------------------------------------------------------------------------

fn run_append(measurements_root: &Path, metrics_root: &Path) -> Result<i32> {
    let series_path = metrics_root.join("latency-series.json");
    let coverage_path = metrics_root.join("coverage.json");

    let mut series = load_series(&series_path)?;
    let mut coverage = load_coverage(&coverage_path)?;

    let already_recorded: HashSet<String> =
        series.entries.iter().map(|e| e.run_id.clone()).collect();

    let mut dirs: Vec<PathBuf> = fs::read_dir(measurements_root)
        .map(|entries| {
            entries
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .filter(|path| path.is_dir())
                .collect()
        })
        .unwrap_or_default();
    dirs.sort();

    let mut appended = 0usize;
    let mut skipped_investigation = 0usize;
    let mut skipped_failed_attempts = 0usize;
    let mut skipped_already_recorded = 0usize;

    for dir in dirs {
        let manifest: Option<RunManifest> = fs::read_to_string(dir.join("manifest.json"))
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok());

        let Some(manifest) = manifest else {
            if let Ok(text) = fs::read_to_string(dir.join("ATTEMPT.json")) {
                if let Ok(attempt) = serde_json::from_str::<AttemptRecord>(&text) {
                    if attempt.status == AttemptStatus::Failed {
                        let stage = attempt
                            .failure
                            .as_ref()
                            .map(|failure| failure.stage.as_str())
                            .unwrap_or("unknown");
                        println!(
                            "skipped {}: failed attempt at stage {stage}",
                            attempt.run_id
                        );
                        skipped_failed_attempts += 1;
                    }
                }
            }
            continue;
        };

        if already_recorded.contains(&manifest.run_id) {
            skipped_already_recorded += 1;
            continue;
        }

        if manifest.instrument_class == InstrumentClass::Investigation {
            println!(
                "skipped {}: instrument_class is investigation, per RESEARCH.md pattern 2",
                manifest.run_id
            );
            skipped_investigation += 1;
            continue;
        }

        let entries = build_stage_entries(&dir, &manifest)?;
        if entries.is_empty() {
            continue;
        }

        let gap_probe = entries[0].clone();
        let iso_week = entries[0].iso_week.clone();
        let run_id = manifest.run_id.clone();
        for entry in entries {
            append(&mut series, entry).context("appending a series entry")?;
        }
        record_gaps(&mut coverage, &gap_probe);
        record_run_week(&mut coverage, &iso_week, &run_id);
        appended += 1;
    }

    save_series(&series_path, &series)?;
    save_coverage(&coverage_path, &coverage)?;

    println!(
        "series --append: {appended} run(s) appended, {skipped_investigation} investigation \
         run(s) skipped, {skipped_failed_attempts} failed attempt(s) skipped, \
         {skipped_already_recorded} already recorded"
    );

    Ok(0)
}

/// Builds one [`StageMetrics`] per capture this run's manifest carries: a cyclictest histogram,
/// each `rtla-hwnoise` firmware screen, and an SMI count bracket, in that order. A stage with
/// nothing to compute a maximum from (no artifact, an unparsable capture, or an SMI read that
/// never happened) contributes no entry at all: "a run that produced no maximum is a run that
/// produced nothing," never a fabricated zero.
fn build_stage_entries(run_dir: &Path, manifest: &RunManifest) -> Result<Vec<StageMetrics>> {
    let mut entries = Vec::new();
    let manifest_blake3 = nr_manifest::blake3_file(&run_dir.join("manifest.json"))
        .context("hashing manifest.json for the series entry")?;
    let iso_week = iso_week_label(manifest.utc_start);

    if let Some(artifact) = manifest
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == ArtifactKind::CyclictestHist)
    {
        let hist_path = run_dir.join(&artifact.path);
        let bound = recorded_histogram_bound(manifest);
        match nr_histogram::hist::parse_hist_file(&hist_path, bound) {
            Ok(hist_run) => match hist_run.percentiles(&[0.5, 0.95, 0.99, 0.999]) {
                Ok(percentiles) => {
                    let [p50, p95, p99, p999] = [
                        percentiles.values[0].1,
                        percentiles.values[1].1,
                        percentiles.values[2].1,
                        percentiles.values[3].1,
                    ];
                    entries.push(StageMetrics {
                        sample_count: percentiles.total_samples,
                        overflow_count: percentiles.overflow_samples,
                        p50_us: Some(p50),
                        p95_us: Some(p95),
                        p99_us: Some(p99),
                        p999_us: Some(p999),
                        max_us: percentiles.max_us,
                        population: "scheduling wakeups, binned samples plus overflows".to_string(),
                        ..base_stage_metrics(
                            manifest,
                            &iso_week,
                            &manifest_blake3,
                            "cyclictest.wakeup_latency",
                            "cyclictest",
                        )
                    });
                }
                Err(err) => println!(
                    "{}: failed to compute cyclictest percentiles: {err}",
                    run_dir.display()
                ),
            },
            Err(err) => println!(
                "{}: failed to parse {}: {err}",
                run_dir.display(),
                hist_path.display()
            ),
        }
    }

    for screen in &manifest.firmware_screens {
        if screen.instrument != "rtla-hwnoise" {
            println!(
                "{}: skipped firmware screen: instrument {:?} has no parser yet",
                run_dir.display(),
                screen.instrument
            );
            continue;
        }
        let Some(artifact) = manifest
            .artifacts
            .iter()
            .find(|artifact| artifact.kind == ArtifactKind::RtlaHwnoise)
        else {
            println!(
                "{}: manifest carries a firmware screen but no rtla-hwnoise artifact",
                run_dir.display()
            );
            continue;
        };
        let capture_path = run_dir.join(&artifact.path);
        let parsed = match hwnoise::parse_hwnoise_file(&capture_path, &screen.requested_cpus) {
            Ok(parsed) => parsed,
            Err(err) => {
                println!(
                    "{}: failed to parse {}: {err}",
                    run_dir.display(),
                    capture_path.display()
                );
                continue;
            }
        };
        let observed_rows: Vec<_> = parsed
            .rows
            .iter()
            .filter(|row| parsed.observed_cpus.contains(&row.cpu))
            .collect();
        let Some(max_us) = observed_rows.iter().map(|row| row.max_single_us).max() else {
            println!(
                "{}: rtla-hwnoise screen produced no observed rows; nothing to report",
                run_dir.display()
            );
            continue;
        };
        let sample_count: u64 = observed_rows.iter().map(|row| row.hw_count).sum();
        entries.push(StageMetrics {
            sample_count,
            overflow_count: 0,
            max_us,
            population: "hardware noise reported per CPU over the sampling window".to_string(),
            ..base_stage_metrics(
                manifest,
                &iso_week,
                &manifest_blake3,
                "rtla_hwnoise.hardware_noise",
                "rtla-hwnoise",
            )
        });
    }

    if let Some(smi) = &manifest.smi_counts {
        if smi.unavailable_reason.is_none() {
            if let Some(max_us) = smi.delta.iter().map(|counter| counter.count).max() {
                entries.push(StageMetrics {
                    sample_count: smi.delta.len() as u64,
                    overflow_count: 0,
                    max_us,
                    population: "SMI count per isolated CPU over the run".to_string(),
                    ..base_stage_metrics(
                        manifest,
                        &iso_week,
                        &manifest_blake3,
                        "smi.count",
                        "rdmsr",
                    )
                });
            }
        }
    }

    Ok(entries)
}

/// The fields shared by every stage entry of one run, with the percentiles null and `max_us`
/// zero: the caller overrides whichever of those the stage actually produces.
fn base_stage_metrics(
    manifest: &RunManifest,
    iso_week: &str,
    manifest_blake3: &str,
    stage: &str,
    tool: &str,
) -> StageMetrics {
    StageMetrics {
        run_id: manifest.run_id.clone(),
        run_class: manifest.run_class.clone(),
        instrument_class: manifest.instrument_class.clone(),
        utc_start: manifest.utc_start,
        iso_week: iso_week.to_string(),
        rig_slug: manifest.host.rig_slug.clone(),
        tool: tool.to_string(),
        stage: stage.to_string(),
        sample_count: 0,
        overflow_count: 0,
        p50_us: None,
        p95_us: None,
        p99_us: None,
        p999_us: None,
        max_us: 0,
        population: String::new(),
        contamination_verdict: manifest.interference.verdict.clone(),
        excluded_from_series: manifest.excluded_from_series,
        exclusion_reason: manifest.exclusion_reason.clone(),
        manifest_blake3: manifest_blake3.to_string(),
    }
}

/// Parses the `--histogram=<n>` bound out of the `cyclictest` tool invocation's recorded argv.
/// `None` when there is no such invocation or flag, in which case the parser derives the bound
/// from the data instead (`nr_histogram::hist::parse_hist`'s documented fallback). Mirrors
/// `crates/cli/src/cmd/verify.rs`'s private helper of the same name; kept as a second small copy
/// here rather than shared, since the two crates' own build order argues against a plan-scoped
/// refactor to pull it out.
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

/// Merges `run_id` into the coverage record's own week entry for `iso_week`, creating one if the
/// last recorded week does not already match. Coverage tracks one entry per week regardless of
/// how many stages a run contributed, so two runs in the same week share one `Run` entry rather
/// than producing two.
fn record_run_week(coverage: &mut Coverage, iso_week: &str, run_id: &str) {
    if let Some(WeekRecord::Run {
        iso_week: existing_week,
        run_ids,
    }) = coverage.weeks.last_mut()
    {
        if existing_week == iso_week {
            run_ids.push(run_id.to_string());
            return;
        }
    }
    coverage.weeks.push(WeekRecord::Run {
        iso_week: iso_week.to_string(),
        run_ids: vec![run_id.to_string()],
    });
}

// ---------------------------------------------------------------------------------
// --compare
// ---------------------------------------------------------------------------------

fn run_compare(metrics_root: &Path, baseline_override: Option<&Path>) -> Result<i32> {
    let series = load_series(&metrics_root.join("latency-series.json"))?;

    let baseline_path = baseline_override
        .map(Path::to_path_buf)
        .unwrap_or_else(|| metrics_root.join("baseline.json"));
    let baseline = Baseline::load(&baseline_path)
        .with_context(|| format!("failed to load baseline from {}", baseline_path.display()))?;

    let mut any_fail = false;
    for entry in &series.entries {
        if entry.excluded_from_series {
            println!(
                "{} {}: excluded from series ({}), not compared",
                entry.run_id,
                entry.stage,
                entry
                    .exclusion_reason
                    .as_deref()
                    .unwrap_or("no reason recorded")
            );
            continue;
        }

        let verdict = compare(entry, &baseline);
        print_verdict(entry, &verdict);
        if matches!(verdict, RegressionVerdict::Fail { .. }) {
            any_fail = true;
        }
    }

    Ok(if any_fail { 1 } else { 0 })
}

fn print_verdict(entry: &StageMetrics, verdict: &RegressionVerdict) {
    match verdict {
        RegressionVerdict::Pass {
            p99_headroom_us,
            max_headroom_us,
        } => println!(
            "{} {}: pass (p99 headroom {p99_headroom_us} us, max headroom {max_headroom_us} us)",
            entry.run_id, entry.stage
        ),
        RegressionVerdict::Fail { failures } => {
            for failure in failures {
                println!(
                    "{} {}: FAIL {:?} observed {} us against threshold {} us",
                    entry.run_id,
                    entry.stage,
                    failure.metric,
                    failure.observed_us,
                    failure.threshold_us
                );
            }
        }
        RegressionVerdict::SkippedContaminated { reason } => println!(
            "{} {}: skipped, contaminated: {reason}",
            entry.run_id, entry.stage
        ),
        RegressionVerdict::SkippedUncalibrated { reason } => println!(
            "{} {}: skipped, uncalibrated: {reason}",
            entry.run_id, entry.stage
        ),
        RegressionVerdict::SkippedNoStatistic { reason } => println!(
            "{} {}: skipped, no statistic: {reason}",
            entry.run_id, entry.stage
        ),
        RegressionVerdict::NoComparableBaseline {
            stage,
            run_class,
            rig_slug,
        } => println!(
            "{} {}: NoComparableBaseline (rig {rig_slug:?}, run_class {run_class:?}, stage {stage:?})",
            entry.run_id, entry.stage
        ),
    }
}

// ---------------------------------------------------------------------------------
// --record-refusal
// ---------------------------------------------------------------------------------

fn run_record_refusal(metrics_root: &Path, check: &str) -> Result<i32> {
    let coverage_path = metrics_root.join("coverage.json");
    let mut coverage = load_coverage(&coverage_path)?;

    let week = iso_week_label(OffsetDateTime::now_utc());
    record_gap(
        &mut coverage,
        week.clone(),
        GapReason::RefusedOnPrecondition {
            check: check.to_string(),
        },
    );

    save_coverage(&coverage_path, &coverage)?;
    println!("series --record-refusal: recorded a coverage gap for {week} ({check})");
    Ok(0)
}

// ---------------------------------------------------------------------------------
// --seed-baseline
// ---------------------------------------------------------------------------------

/// Copies one already-appended run's series entries into `metrics/baseline.json`, keyed by
/// (rig_slug, run_class, stage). Refuses the whole operation when the run is excluded from the
/// series, contaminated, or was taken under `instrument_class` investigation: a baseline seeded
/// from any of those would silently license every future regression. Independently, a stage
/// whose `p99_us` is null is skipped rather than seeded (a threshold on a statistic that does
/// not exist can never fire), without refusing the stages that do have one: the real headline
/// run this seeds from always carries a null-percentile SMI stage alongside its cyclictest one.
fn run_seed_baseline(metrics_root: &Path, run_id: &str) -> Result<i32> {
    let series = load_series(&metrics_root.join("latency-series.json"))?;

    let run_entries: Vec<&StageMetrics> = series
        .entries
        .iter()
        .filter(|entry| entry.run_id == run_id)
        .collect();
    if run_entries.is_empty() {
        eprintln!(
            "series --seed-baseline: no entries recorded for run {run_id:?}; run --append first"
        );
        return Ok(1);
    }
    if run_entries.iter().any(|entry| entry.excluded_from_series) {
        eprintln!("series --seed-baseline: refusing: {run_id} is excluded from the series");
        return Ok(1);
    }
    if run_entries
        .iter()
        .any(|entry| entry.contamination_verdict == ContaminationVerdict::Contaminated)
    {
        eprintln!("series --seed-baseline: refusing: {run_id} is contaminated");
        return Ok(1);
    }
    if run_entries
        .iter()
        .any(|entry| entry.instrument_class == InstrumentClass::Investigation)
    {
        eprintln!(
            "series --seed-baseline: refusing: {run_id} was taken under instrument_class \
             investigation"
        );
        return Ok(1);
    }

    let baseline_path = metrics_root.join("baseline.json");
    let mut baseline = Baseline::load(&baseline_path)
        .with_context(|| format!("failed to load baseline from {}", baseline_path.display()))?;

    let mut seeded = 0usize;
    for entry in &run_entries {
        let Some(p99_us) = entry.p99_us else {
            println!(
                "series --seed-baseline: skipped stage {}: no p99 statistic to threshold \
                 against",
                entry.stage
            );
            continue;
        };
        baseline.entries.retain(|existing| {
            !(existing.rig_slug == entry.rig_slug
                && existing.run_class == entry.run_class
                && existing.stage == entry.stage)
        });
        baseline.entries.push(BaselineEntry {
            rig_slug: entry.rig_slug.clone(),
            run_class: entry.run_class.clone(),
            stage: entry.stage.clone(),
            p99_us,
            max_us: entry.max_us,
        });
        seeded += 1;
        println!(
            "series --seed-baseline: seeded {} / {:?} / {}",
            entry.rig_slug, entry.run_class, entry.stage
        );
    }

    if seeded == 0 {
        eprintln!(
            "series --seed-baseline: nothing seeded from {run_id}: every stage lacks a p99 \
             statistic"
        );
        return Ok(1);
    }

    save_baseline(&baseline_path, &baseline)?;
    Ok(0)
}

// ---------------------------------------------------------------------------------
// Shared load/save helpers.
// ---------------------------------------------------------------------------------

fn iso_week_label(dt: OffsetDateTime) -> String {
    let (year, week, _) = dt.to_iso_week_date();
    format!("{year}-W{week:02}")
}

fn load_series(path: &Path) -> Result<MetricsSeries> {
    match fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text)
            .with_context(|| format!("failed to parse {}", path.display())),
        Err(_) => Ok(MetricsSeries {
            schema_version: nr_metrics::series::SCHEMA_VERSION,
            entries: Vec::new(),
        }),
    }
}

fn save_series(path: &Path, series: &MetricsSeries) -> Result<()> {
    write_pretty_json(path, series)
}

fn load_coverage(path: &Path) -> Result<Coverage> {
    match fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text)
            .with_context(|| format!("failed to parse {}", path.display())),
        Err(_) => Ok(Coverage::new()),
    }
}

fn save_coverage(path: &Path, coverage: &Coverage) -> Result<()> {
    write_pretty_json(path, coverage)
}

fn save_baseline(path: &Path, baseline: &Baseline) -> Result<()> {
    write_pretty_json(path, baseline)
}

fn write_pretty_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(value)
        .with_context(|| format!("failed to serialise {}", path.display()))?;
    fs::write(path, json).with_context(|| format!("failed to write {}", path.display()))
}
