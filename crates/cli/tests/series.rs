//! Integration coverage for `nrmeasure series` (BENCH-08, D-11), driven against the real
//! compiled binary via `assert_cmd` so exit codes and stdout text are exercised exactly as CI
//! observes them. Every test builds its own temporary tree; none touches the real
//! `measurements/` or `metrics/`.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::Output;

use assert_cmd::Command;
use serde_json::{Value, json};

/// A complete, valid, harness-generated manifest (plan 01-03's human-reviewed worked example).
const MINIMAL_MANIFEST: &str = include_str!("../../manifest/tests/fixtures/minimal-manifest.json");

/// Byte-identical to `measurements/2026-08-28-precision3591/cyclictest-rt-isolated-idle-10m.hist`.
const HIST_BYTES: &[u8] =
    include_bytes!("../../histogram/tests/fixtures/cyclictest-rt-isolated-idle-10m.hist");

/// A real, committed `rtla hwnoise` 60s probe capture (plan 01-05's recon), reused here as the
/// firmware screen's raw capture.
const HWNOISE_BYTES: &[u8] = include_bytes!("../../capture/tests/fixtures/rtla-hwnoise-probe.txt");

fn nrmeasure() -> Command {
    Command::cargo_bin("nrmeasure").expect("nrmeasure binary is built")
}

fn run_series(measurements: &Path, metrics: &Path, extra_args: &[&str]) -> Output {
    let mut cmd = nrmeasure();
    cmd.arg("series")
        .arg("--measurements")
        .arg(measurements)
        .arg("--metrics")
        .arg(metrics);
    for arg in extra_args {
        cmd.arg(arg);
    }
    cmd.output().expect("run nrmeasure series")
}

fn base_manifest(run_id: &str) -> Value {
    let mut manifest: Value = serde_json::from_str(MINIMAL_MANIFEST).expect("fixture parses");
    manifest["run_id"] = Value::String(run_id.to_string());
    manifest
}

/// Writes a run directory carrying a real cyclictest capture and a real rtla hwnoise capture,
/// so `--append` finds exactly two stages: `cyclictest.wakeup_latency` and
/// `rtla_hwnoise.hardware_noise`. `mutate` receives the manifest after both artifacts and the
/// one firmware screen are wired in, and may override any field.
fn write_cyclictest_and_hwnoise_run(
    measurements_root: &Path,
    run_id: &str,
    mutate: impl FnOnce(&mut Value),
) {
    let run_dir = measurements_root.join(run_id);
    fs::create_dir_all(&run_dir).expect("mkdir run dir");
    fs::write(
        run_dir.join("cyclictest-rt-isolated-idle-10m.hist"),
        HIST_BYTES,
    )
    .expect("write cyclictest capture");
    fs::write(run_dir.join("rtla-hwnoise.txt"), HWNOISE_BYTES).expect("write hwnoise capture");

    let hwnoise_blake3 = "0".repeat(64);
    let mut manifest = base_manifest(run_id);
    manifest["artifacts"] = json!([
        {
            "path": "cyclictest-rt-isolated-idle-10m.hist",
            "bytes": HIST_BYTES.len(),
            "blake3": "495c45891c866a627334e74d810a5377be684beb6d8e8a17b7853f8c4cb9c88f",
            "kind": "cyclictest-hist",
            "stored": "in-repo"
        },
        {
            "path": "rtla-hwnoise.txt",
            "bytes": HWNOISE_BYTES.len(),
            "blake3": hwnoise_blake3,
            "kind": "rtla-hwnoise",
            "stored": "in-repo"
        }
    ]);
    manifest["firmware_screens"] = json!([firmware_screen_json()]);

    mutate(&mut manifest);

    fs::write(
        run_dir.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).expect("serialise manifest"),
    )
    .expect("write manifest.json");
}

/// Writes a run directory carrying only the rtla hwnoise capture, so its one appended stage
/// entry (`rtla_hwnoise.hardware_noise`) has a null `p99_us`: there is no cyclictest capture to
/// produce one.
fn write_hwnoise_only_run(measurements_root: &Path, run_id: &str, mutate: impl FnOnce(&mut Value)) {
    let run_dir = measurements_root.join(run_id);
    fs::create_dir_all(&run_dir).expect("mkdir run dir");
    fs::write(run_dir.join("rtla-hwnoise.txt"), HWNOISE_BYTES).expect("write hwnoise capture");

    let hwnoise_blake3 = "0".repeat(64);
    let mut manifest = base_manifest(run_id);
    manifest["artifacts"] = json!([
        {
            "path": "rtla-hwnoise.txt",
            "bytes": HWNOISE_BYTES.len(),
            "blake3": hwnoise_blake3,
            "kind": "rtla-hwnoise",
            "stored": "in-repo"
        }
    ]);
    manifest["firmware_screens"] = json!([firmware_screen_json()]);

    mutate(&mut manifest);

    fs::write(
        run_dir.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).expect("serialise manifest"),
    )
    .expect("write manifest.json");
}

fn firmware_screen_json() -> Value {
    json!({
        "instrument": "rtla-hwnoise",
        "tool_version": "7.0.12",
        "argv": ["rtla", "hwnoise", "-c", "6-11", "-H", "0-5", "-P", "f:99", "-d", "60s"],
        "requested_cpus": [6, 7, 8, 9, 10, 11],
        "observed_cpus": [6, 7, 8, 9, 10, 11],
        "per_cpu_exposure_seconds": [],
        "max_us": 1,
        "max_population": "the largest Max Single value across the observed CPUs' final rtla hwnoise rows",
        "events_recorded": 22
    })
}

/// Writes a run directory holding only a failed `ATTEMPT.json` (plan 01-17), no `manifest.json`.
fn write_failed_attempt(measurements_root: &Path, run_id: &str, stage: &str) {
    let run_dir = measurements_root.join(run_id);
    fs::create_dir_all(&run_dir).expect("mkdir run dir");

    let git_sha = "0".repeat(40);
    let attempt = json!({
        "schema_version": 1,
        "run_id": run_id,
        "utc_start": "2026-09-01T00:00:00Z",
        "utc_end": "2026-09-01T00:05:00Z",
        "status": "failed",
        "harness": {
            "version": "0.1.0",
            "git_sha": git_sha,
            "git_dirty": false
        },
        "requested": {
            "run_class": "recon",
            "instrument_class": "headline-series",
            "cpus": "6-11",
            "main_cpus": "0,1",
            "duration_seconds": 60,
            "with_hwlatdetect": false
        },
        "tools": [],
        "preserved": [],
        "failure": {
            "stage": stage,
            "message": "cyclictest exited with a non-zero status"
        },
        "usable_for_numerical_analysis": false
    });

    fs::write(
        run_dir.join("ATTEMPT.json"),
        serde_json::to_string_pretty(&attempt).expect("serialise ATTEMPT.json"),
    )
    .expect("write ATTEMPT.json");
}

fn load_json(path: &Path) -> Value {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|err| panic!("{} is not valid JSON: {err}", path.display()))
}

fn load_series_json(metrics: &Path) -> Value {
    load_json(&metrics.join("latency-series.json"))
}

fn load_coverage_json(metrics: &Path) -> Value {
    load_json(&metrics.join("coverage.json"))
}

fn load_baseline_json(metrics: &Path) -> Value {
    load_json(&metrics.join("baseline.json"))
}

fn write_baseline(metrics: &Path, baseline: Value) {
    fs::create_dir_all(metrics).expect("mkdir metrics");
    fs::write(
        metrics.join("baseline.json"),
        serde_json::to_string_pretty(&baseline).expect("serialise baseline"),
    )
    .expect("write baseline.json");
}

fn empty_baseline_json() -> Value {
    json!({
        "schema_version": 1,
        "thresholds": {
            "p99": {"relative_pct": 20.0, "absolute_us": 2},
            "max": {"relative_pct": 50.0, "absolute_us": 100}
        },
        "note": "test fixture: empty baseline",
        "entries": []
    })
}

fn find_stage<'a>(series: &'a Value, stage: &str) -> &'a Value {
    series["entries"]
        .as_array()
        .expect("entries is an array")
        .iter()
        .find(|entry| entry["stage"] == stage)
        .unwrap_or_else(|| panic!("no entry for stage {stage:?} in {series:#?}"))
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

// ---------------------------------------------------------------------------------
// --append
// ---------------------------------------------------------------------------------

/// A run directory with a cyclictest capture and an rtla hwnoise capture appends two
/// `StageMetrics` entries, one per stage.
#[test]
fn series_append_adds_one_entry_per_stage() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    let metrics = temp.path().join("metrics");
    fs::create_dir_all(&measurements).expect("mkdir measurements");

    write_cyclictest_and_hwnoise_run(&measurements, "2026-08-31-precision3591-recon-001", |_| {});

    let output = run_series(&measurements, &metrics, &["--append"]);
    assert!(output.status.success(), "stderr={}", stderr_of(&output));

    let series = load_series_json(&metrics);
    let entries = series["entries"].as_array().expect("entries array");
    assert_eq!(
        entries.len(),
        2,
        "expected one entry per stage, got {entries:#?}"
    );

    let stages: Vec<&str> = entries
        .iter()
        .map(|e| e["stage"].as_str().unwrap())
        .collect();
    assert!(stages.contains(&"cyclictest.wakeup_latency"), "{stages:?}");
    assert!(
        stages.contains(&"rtla_hwnoise.hardware_noise"),
        "{stages:?}"
    );
}

/// The hwnoise entry has `p50_us`, `p95_us`, `p99_us` and `p999_us` all null, and a
/// `population` string naming what the maximum is over.
#[test]
fn firmware_stage_records_null_percentiles() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    let metrics = temp.path().join("metrics");
    fs::create_dir_all(&measurements).expect("mkdir measurements");

    write_cyclictest_and_hwnoise_run(&measurements, "2026-08-31-precision3591-recon-002", |_| {});

    let output = run_series(&measurements, &metrics, &["--append"]);
    assert!(output.status.success(), "stderr={}", stderr_of(&output));

    let series = load_series_json(&metrics);
    let hwnoise_entry = find_stage(&series, "rtla_hwnoise.hardware_noise");

    for field in ["p50_us", "p95_us", "p99_us", "p999_us"] {
        assert!(
            hwnoise_entry[field].is_null(),
            "{field} should be null, got {:?}",
            hwnoise_entry[field]
        );
    }
    let population = hwnoise_entry["population"]
        .as_str()
        .expect("population is a string");
    assert!(
        !population.is_empty(),
        "population must name what max_us is over"
    );
}

/// No percentile field on a firmware stage equals `max_us`, asserted directly.
#[test]
fn firmware_stage_never_copies_the_maximum() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    let metrics = temp.path().join("metrics");
    fs::create_dir_all(&measurements).expect("mkdir measurements");

    write_cyclictest_and_hwnoise_run(&measurements, "2026-08-31-precision3591-recon-003", |_| {});

    let output = run_series(&measurements, &metrics, &["--append"]);
    assert!(output.status.success(), "stderr={}", stderr_of(&output));

    let series = load_series_json(&metrics);
    let hwnoise_entry = find_stage(&series, "rtla_hwnoise.hardware_noise");
    let max_us = hwnoise_entry["max_us"].as_u64();
    assert!(
        max_us.is_some(),
        "the firmware stage must still carry a real maximum"
    );

    for field in ["p50_us", "p95_us", "p99_us", "p999_us"] {
        assert_ne!(
            hwnoise_entry[field].as_u64(),
            max_us,
            "{field} must never equal max_us"
        );
    }
}

/// A run whose `instrument_class` is investigation is never appended, and the command prints
/// the skipped run id.
#[test]
fn series_skips_investigation_runs() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    let metrics = temp.path().join("metrics");
    fs::create_dir_all(&measurements).expect("mkdir measurements");

    write_cyclictest_and_hwnoise_run(
        &measurements,
        "2026-08-31-precision3591-investigation-004",
        |manifest| {
            manifest["instrument_class"] = Value::String("investigation".to_string());
        },
    );

    let output = run_series(&measurements, &metrics, &["--append"]);
    assert!(output.status.success(), "stderr={}", stderr_of(&output));
    let stdout = stdout_of(&output);
    assert!(
        stdout.contains("2026-08-31-precision3591-investigation-004"),
        "stdout={stdout}"
    );

    let series = load_series_json(&metrics);
    assert_eq!(series["entries"].as_array().unwrap().len(), 0);
}

/// A directory holding only an `ATTEMPT.json` is never appended and is reported as skipped
/// with its failure stage.
#[test]
fn series_skips_failed_attempts() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    let metrics = temp.path().join("metrics");
    fs::create_dir_all(&measurements).expect("mkdir measurements");

    write_failed_attempt(
        &measurements,
        "2026-09-01-precision3591-recon-005",
        "tool-exit",
    );

    let output = run_series(&measurements, &metrics, &["--append"]);
    assert!(output.status.success(), "stderr={}", stderr_of(&output));
    let stdout = stdout_of(&output);
    assert!(stdout.contains("tool-exit"), "stdout={stdout}");
    assert!(
        stdout.contains("2026-09-01-precision3591-recon-005"),
        "stdout={stdout}"
    );

    let series = load_series_json(&metrics);
    assert_eq!(series["entries"].as_array().unwrap().len(), 0);
}

/// Re-running against an unchanged tree appends nothing and exits 0.
#[test]
fn series_skips_already_recorded() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    let metrics = temp.path().join("metrics");
    fs::create_dir_all(&measurements).expect("mkdir measurements");
    write_cyclictest_and_hwnoise_run(&measurements, "2026-08-31-precision3591-recon-006", |_| {});

    let first = run_series(&measurements, &metrics, &["--append"]);
    assert!(first.status.success(), "stderr={}", stderr_of(&first));
    let count_after_first = load_series_json(&metrics)["entries"]
        .as_array()
        .unwrap()
        .len();

    let second = run_series(&measurements, &metrics, &["--append"]);
    assert!(second.status.success(), "stderr={}", stderr_of(&second));
    let count_after_second = load_series_json(&metrics)["entries"]
        .as_array()
        .unwrap()
        .len();

    assert_eq!(count_after_first, 2);
    assert_eq!(
        count_after_first, count_after_second,
        "re-running --append against an unchanged tree must not duplicate entries"
    );
}

/// Appending an entry three ISO weeks after the last one writes two Gap entries with reason
/// `no-run-recorded`.
#[test]
fn series_records_gap_for_missed_weeks() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    let metrics = temp.path().join("metrics");
    fs::create_dir_all(&measurements).expect("mkdir measurements");

    write_cyclictest_and_hwnoise_run(
        &measurements,
        "2026-08-31-precision3591-recon-007",
        |manifest| manifest["utc_start"] = Value::String("2026-08-31T06:00:00Z".to_string()),
    );
    write_cyclictest_and_hwnoise_run(
        &measurements,
        "2026-09-21-precision3591-recon-008",
        |manifest| manifest["utc_start"] = Value::String("2026-09-21T06:00:00Z".to_string()),
    );

    let output = run_series(&measurements, &metrics, &["--append"]);
    assert!(output.status.success(), "stderr={}", stderr_of(&output));

    let coverage = load_coverage_json(&metrics);
    let weeks = coverage["weeks"].as_array().expect("weeks array");
    assert_eq!(weeks.len(), 4, "{weeks:#?}");

    // 2026-08-31 and 2026-09-21 are exactly 21 days (3 ISO weeks) apart: W36 and W39, with W37
    // and W38 as the two missed weeks in between.
    assert_eq!(weeks[0]["run"]["iso_week"], "2026-W36");
    assert_eq!(weeks[1]["gap"]["iso_week"], "2026-W37");
    assert_eq!(weeks[1]["gap"]["reason"], "no-run-recorded");
    assert_eq!(weeks[2]["gap"]["iso_week"], "2026-W38");
    assert_eq!(weeks[2]["gap"]["reason"], "no-run-recorded");
    assert_eq!(weeks[3]["run"]["iso_week"], "2026-W39");
}

/// `--record-refusal <check>` writes a Gap entry whose reason names that check.
#[test]
fn series_records_gap_for_refused_run() {
    let temp = tempfile::tempdir().expect("tempdir");
    let metrics = temp.path().join("metrics");

    let output = nrmeasure()
        .arg("series")
        .arg("--metrics")
        .arg(&metrics)
        .arg("--record-refusal")
        .arg("governor-is-performance-on-all-cpus")
        .output()
        .expect("run series --record-refusal");
    assert!(output.status.success(), "stderr={}", stderr_of(&output));

    let coverage = load_coverage_json(&metrics);
    let weeks = coverage["weeks"].as_array().expect("weeks array");
    assert_eq!(weeks.len(), 1, "{weeks:#?}");
    assert_eq!(
        weeks[0]["gap"]["reason"]["refused-on-precondition"]["check"],
        "governor-is-performance-on-all-cpus"
    );
}

/// No code path writes a `StageMetrics` entry for a week with no run directory: every Gap
/// entry carries only `iso_week` and `reason` (never a metric field), and the coverage record's
/// total `run_ids` count matches the number of real runs, never the number of weeks spanned.
#[test]
fn series_never_backfills() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    let metrics = temp.path().join("metrics");
    fs::create_dir_all(&measurements).expect("mkdir measurements");

    write_cyclictest_and_hwnoise_run(
        &measurements,
        "2026-08-31-precision3591-recon-009",
        |manifest| manifest["utc_start"] = Value::String("2026-08-31T06:00:00Z".to_string()),
    );
    write_cyclictest_and_hwnoise_run(
        &measurements,
        "2026-09-21-precision3591-recon-010",
        |manifest| manifest["utc_start"] = Value::String("2026-09-21T06:00:00Z".to_string()),
    );

    let output = run_series(&measurements, &metrics, &["--append"]);
    assert!(output.status.success(), "stderr={}", stderr_of(&output));

    let coverage = load_coverage_json(&metrics);
    let weeks = coverage["weeks"].as_array().expect("weeks array");
    let gaps: Vec<&Value> = weeks
        .iter()
        .filter(|week| week.get("gap").is_some())
        .collect();
    assert_eq!(gaps.len(), 2, "{weeks:#?}");
    for gap in &gaps {
        let fields: BTreeSet<&str> = gap["gap"]
            .as_object()
            .expect("a gap entry is an object")
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            fields,
            BTreeSet::from(["iso_week", "reason"]),
            "a Gap entry must carry no metric field: {gap:?}"
        );
    }

    let total_run_ids: usize = weeks
        .iter()
        .filter_map(|week| week["run"]["run_ids"].as_array())
        .map(|ids| ids.len())
        .sum();
    assert_eq!(
        total_run_ids, 2,
        "exactly the two real runs, never the weeks spanned"
    );
}

// ---------------------------------------------------------------------------------
// --compare
// ---------------------------------------------------------------------------------

/// `--compare` prints the RegressionVerdict for each entry and exits non-zero only on Fail.
#[test]
fn series_compare_reports_verdict() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    let metrics = temp.path().join("metrics");
    fs::create_dir_all(&measurements).expect("mkdir measurements");
    write_cyclictest_and_hwnoise_run(&measurements, "2026-08-31-precision3591-recon-011", |_| {});

    let append_output = run_series(&measurements, &metrics, &["--append"]);
    assert!(
        append_output.status.success(),
        "stderr={}",
        stderr_of(&append_output)
    );

    // Zero tolerance: any real cyclictest p99 (always greater than 1us) fails.
    write_baseline(
        &metrics,
        json!({
            "schema_version": 1,
            "thresholds": {
                "p99": {"relative_pct": 0.0, "absolute_us": 0},
                "max": {"relative_pct": 0.0, "absolute_us": 0}
            },
            "note": "test fixture: zero tolerance",
            "entries": [
                {
                    "rig_slug": "precision3591",
                    "run_class": "recon",
                    "stage": "cyclictest.wakeup_latency",
                    "p99_us": 1,
                    "max_us": 1
                }
            ]
        }),
    );

    let output = run_series(&measurements, &metrics, &["--compare"]);
    let stdout = stdout_of(&output);
    assert_eq!(output.status.code(), Some(1), "stdout={stdout}");
    assert!(stdout.contains("FAIL"), "stdout={stdout}");
    assert!(
        stdout.contains("cyclictest.wakeup_latency"),
        "stdout={stdout}"
    );
}

/// With an empty baseline, `--compare` prints `NoComparableBaseline` and exits 0.
#[test]
fn series_compare_neutral_on_no_baseline() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    let metrics = temp.path().join("metrics");
    fs::create_dir_all(&measurements).expect("mkdir measurements");
    write_cyclictest_and_hwnoise_run(&measurements, "2026-08-31-precision3591-recon-012", |_| {});

    let append_output = run_series(&measurements, &metrics, &["--append"]);
    assert!(
        append_output.status.success(),
        "stderr={}",
        stderr_of(&append_output)
    );
    write_baseline(&metrics, empty_baseline_json());

    let output = run_series(&measurements, &metrics, &["--compare"]);
    let stdout = stdout_of(&output);
    assert_eq!(output.status.code(), Some(0), "stdout={stdout}");
    assert!(stdout.contains("NoComparableBaseline"), "stdout={stdout}");
}

/// An entry whose `p99_us` is null is reported as skipped with a stated reason and does not
/// fail the comparison.
#[test]
fn series_compare_skips_a_null_statistic() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    let metrics = temp.path().join("metrics");
    fs::create_dir_all(&measurements).expect("mkdir measurements");
    write_hwnoise_only_run(&measurements, "2026-08-31-precision3591-recon-013", |_| {});

    let append_output = run_series(&measurements, &metrics, &["--append"]);
    assert!(
        append_output.status.success(),
        "stderr={}",
        stderr_of(&append_output)
    );
    write_baseline(&metrics, empty_baseline_json());

    let output = run_series(&measurements, &metrics, &["--compare"]);
    let stdout = stdout_of(&output);
    assert_eq!(output.status.code(), Some(0), "stdout={stdout}");
    assert!(stdout.contains("no statistic"), "stdout={stdout}");
    assert!(!stdout.contains("FAIL"), "stdout={stdout}");
}

// ---------------------------------------------------------------------------------
// --seed-baseline
// ---------------------------------------------------------------------------------

/// `--seed-baseline` refuses a contaminated run.
#[test]
fn seed_baseline_refuses_a_contaminated_run() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    let metrics = temp.path().join("metrics");
    fs::create_dir_all(&measurements).expect("mkdir measurements");

    write_cyclictest_and_hwnoise_run(
        &measurements,
        "2026-08-31-precision3591-recon-014",
        |manifest| manifest["interference"]["verdict"] = Value::String("contaminated".to_string()),
    );

    let append_output = run_series(&measurements, &metrics, &["--append"]);
    assert!(
        append_output.status.success(),
        "stderr={}",
        stderr_of(&append_output)
    );

    let output = run_series(
        &measurements,
        &metrics,
        &["--seed-baseline", "2026-08-31-precision3591-recon-014"],
    );
    assert_ne!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        stdout_of(&output),
        stderr_of(&output)
    );
}

/// `--seed-baseline` refuses an investigation run. Crafted directly into `latency-series.json`
/// (bypassing `--append`, which already excludes `instrument_class` investigation on its own),
/// mirroring the known pre-fix data shape recorded in STATE.md where a run's own
/// `excluded_from_series` predates the D-28 admission fix and reads false even though its
/// `instrument_class` is investigation: seed-baseline must refuse on `instrument_class` alone,
/// not only on `excluded_from_series`.
#[test]
fn seed_baseline_refuses_an_investigation_run() {
    let temp = tempfile::tempdir().expect("tempdir");
    let metrics = temp.path().join("metrics");
    fs::create_dir_all(&metrics).expect("mkdir metrics");

    let manifest_blake3 = "a".repeat(64);
    let series = json!({
        "schema_version": 1,
        "entries": [{
            "run_id": "2026-09-07-precision3591-investigation",
            "run_class": "investigation",
            "instrument_class": "investigation",
            "utc_start": "2026-09-07T00:00:00Z",
            "iso_week": "2026-W36",
            "rig_slug": "precision3591",
            "tool": "cyclictest",
            "stage": "cyclictest.wakeup_latency",
            "sample_count": 1000,
            "overflow_count": 0,
            "p50_us": 2,
            "p95_us": 4,
            "p99_us": 8,
            "p999_us": 10,
            "max_us": 20,
            "population": "scheduling wakeups, binned samples plus overflows",
            "contamination_verdict": "clean",
            "excluded_from_series": false,
            "exclusion_reason": null,
            "manifest_blake3": manifest_blake3
        }]
    });
    fs::write(
        metrics.join("latency-series.json"),
        serde_json::to_string_pretty(&series).expect("serialise series"),
    )
    .expect("write latency-series.json");

    let output = nrmeasure()
        .arg("series")
        .arg("--metrics")
        .arg(&metrics)
        .arg("--seed-baseline")
        .arg("2026-09-07-precision3591-investigation")
        .output()
        .expect("run series --seed-baseline");
    assert_ne!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        stdout_of(&output),
        stderr_of(&output)
    );
}

/// `--seed-baseline` refuses a run whose only stage has a null p99.
#[test]
fn seed_baseline_refuses_a_run_whose_only_stage_has_null_p99() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    let metrics = temp.path().join("metrics");
    fs::create_dir_all(&measurements).expect("mkdir measurements");
    write_hwnoise_only_run(&measurements, "2026-08-31-precision3591-recon-015", |_| {});

    let append_output = run_series(&measurements, &metrics, &["--append"]);
    assert!(
        append_output.status.success(),
        "stderr={}",
        stderr_of(&append_output)
    );
    write_baseline(&metrics, empty_baseline_json());

    let output = run_series(
        &measurements,
        &metrics,
        &["--seed-baseline", "2026-08-31-precision3591-recon-015"],
    );
    assert_ne!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        stdout_of(&output),
        stderr_of(&output)
    );
}

/// A clean run seeds its cyclictest stage into the baseline and skips its null-percentile
/// hwnoise stage without refusing the whole command. This is the exact shape of the real
/// PLAT-03 headline run task 3 seeds the baseline from: a percentile-bearing stage next to a
/// firmware/SMI stage that has none.
#[test]
fn seed_baseline_seeds_a_clean_run_and_skips_its_null_stage() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    let metrics = temp.path().join("metrics");
    fs::create_dir_all(&measurements).expect("mkdir measurements");
    write_cyclictest_and_hwnoise_run(
        &measurements,
        "2026-08-31-precision3591-headline-016",
        |manifest| manifest["run_class"] = Value::String("headline".to_string()),
    );

    let append_output = run_series(&measurements, &metrics, &["--append"]);
    assert!(
        append_output.status.success(),
        "stderr={}",
        stderr_of(&append_output)
    );
    write_baseline(&metrics, empty_baseline_json());

    let output = run_series(
        &measurements,
        &metrics,
        &["--seed-baseline", "2026-08-31-precision3591-headline-016"],
    );
    let stdout = stdout_of(&output);
    assert_eq!(output.status.code(), Some(0), "stdout={stdout}");

    let baseline = load_baseline_json(&metrics);
    let entries = baseline["entries"].as_array().expect("entries array");
    assert_eq!(entries.len(), 1, "{entries:#?}");
    assert_eq!(entries[0]["stage"], "cyclictest.wakeup_latency");
    assert_eq!(entries[0]["run_class"], "headline");

    assert!(
        stdout.contains("rtla_hwnoise.hardware_noise") && stdout.contains("skipped"),
        "stdout={stdout}"
    );
}
