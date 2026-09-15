//! Integration coverage for `nrmeasure verify` (the D-13 blocking provenance gate), driven
//! against the real compiled binary via `assert_cmd` so exit codes and stdout text are
//! exercised exactly as CI observes them.
//!
//! Every test builds its own temporary tree (never touching the real `measurements/`), so
//! `git -C <tempdir> ls-files` always fails (the temp directory is outside any git repository)
//! and `nrmeasure verify`'s check 2 exercises its documented fallback: a plain recursive walk
//! skipping only `.git` and `target`.

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use nr_manifest::{
    ArtifactKind, ArtifactRecord, AttemptFailure, AttemptRecord, AttemptStatus, HarnessInfo,
    InstrumentClass, RequestedRun, RunClass, StorageLocation,
};
use serde_json::Value;
use time::OffsetDateTime;

/// A complete, valid, harness-generated manifest (plan 01-03's human-reviewed worked example),
/// whose one artifact record's blake3 is the real committed histogram capture's own digest.
const MINIMAL_MANIFEST: &str = include_str!("../../manifest/tests/fixtures/minimal-manifest.json");

/// Byte-identical to `measurements/2026-08-28-precision3591/cyclictest-rt-isolated-idle-10m.hist`
/// (confirmed via `diff`); this copy lives under a `tests/fixtures/` exempt tree already.
const HIST_BYTES: &[u8] =
    include_bytes!("../../histogram/tests/fixtures/cyclictest-rt-isolated-idle-10m.hist");

fn nrmeasure() -> Command {
    Command::cargo_bin("nrmeasure").expect("nrmeasure binary is built")
}

fn base_cmd(root: &Path, measurements: &Path) -> Command {
    let mut cmd = nrmeasure();
    cmd.arg("verify")
        .arg("--root")
        .arg(root)
        .arg("--measurements")
        .arg(measurements);
    cmd
}

/// Writes a complete, valid run directory named `run_id` under `measurements_root`: a manifest
/// derived from [`MINIMAL_MANIFEST`] (with `run_id` overridden to match the directory) plus its
/// one real, checksum-matching capture file.
fn write_valid_run(measurements_root: &Path, run_id: &str) {
    let run_dir = measurements_root.join(run_id);
    fs::create_dir_all(&run_dir).expect("mkdir run dir");

    let mut manifest: Value = serde_json::from_str(MINIMAL_MANIFEST).expect("fixture parses");
    manifest["run_id"] = Value::String(run_id.to_string());
    fs::write(
        run_dir.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).expect("serialise manifest"),
    )
    .expect("write manifest.json");

    fs::write(
        run_dir.join("cyclictest-rt-isolated-idle-10m.hist"),
        HIST_BYTES,
    )
    .expect("write capture");
}

/// Writes a run directory identical to [`write_valid_run`], except its cyclictest histogram
/// artifact is syntactically malformed (missing the required "# Histogram" header) while its
/// checksum in the manifest still matches the file on disk: the exact "correctly hashed but
/// malformed capture" case finding 6 of `01-EXTERNAL-AUDIT.md` names. A blake3 mismatch would
/// trip check 2 instead (`verify_rejects_checksum_mismatch` already covers that); this fixture
/// is built to pass checksum verification and fail only at histogram-parse time.
fn write_run_with_malformed_hist(measurements_root: &Path, run_id: &str) {
    let run_dir = measurements_root.join(run_id);
    fs::create_dir_all(&run_dir).expect("mkdir run dir");

    let garbage: &[u8] = b"this is not a cyclictest histogram file\n";
    let hist_path = run_dir.join("cyclictest-rt-isolated-idle-10m.hist");
    fs::write(&hist_path, garbage).expect("write malformed capture");
    let garbage_blake3 = nr_manifest::blake3_file(&hist_path).expect("hash the malformed capture");

    let mut manifest: Value = serde_json::from_str(MINIMAL_MANIFEST).expect("fixture parses");
    manifest["run_id"] = Value::String(run_id.to_string());
    manifest["artifacts"][0]["bytes"] = Value::from(garbage.len() as u64);
    manifest["artifacts"][0]["blake3"] = Value::String(garbage_blake3);
    fs::write(
        run_dir.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).expect("serialise manifest"),
    )
    .expect("write manifest.json");
}

fn manifest_missing_key(key: &str) -> String {
    let mut manifest: Value = serde_json::from_str(MINIMAL_MANIFEST).expect("fixture parses");
    manifest
        .as_object_mut()
        .expect("manifest is a JSON object")
        .remove(key);
    serde_json::to_string_pretty(&manifest).expect("serialise manifest")
}

#[test]
fn verify_accepts_valid_run() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    fs::create_dir_all(&measurements).expect("mkdir measurements");
    write_valid_run(&measurements, "2026-08-30-precision3591-recon-001");

    let output = base_cmd(temp.path(), &measurements)
        .output()
        .expect("run verify");

    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn verify_rejects_orphan_capture() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    let orphan_dir = measurements.join("2026-08-30-precision3591-orphan");
    fs::create_dir_all(&orphan_dir).expect("mkdir orphan dir");
    fs::write(orphan_dir.join("cyclictest.hist"), HIST_BYTES).expect("write orphan capture");
    // Deliberately no manifest.json: this is the orphan capture D-13 must catch.

    let output = base_cmd(temp.path(), &measurements)
        .output()
        .expect("run verify");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("2026-08-30-precision3591-orphan"),
        "stdout should name the orphan directory: {stdout}"
    );
}

#[test]
fn verify_rejects_checksum_mismatch() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    fs::create_dir_all(&measurements).expect("mkdir measurements");
    write_valid_run(&measurements, "2026-08-30-precision3591-recon-001");

    let hist_path = measurements
        .join("2026-08-30-precision3591-recon-001")
        .join("cyclictest-rt-isolated-idle-10m.hist");
    let mut tampered = fs::read(&hist_path).expect("read capture");
    tampered.push(b'\n');
    fs::write(&hist_path, tampered).expect("tamper capture");

    let output = base_cmd(temp.path(), &measurements)
        .output()
        .expect("run verify");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("checksum mismatch"), "stdout: {stdout}");
    assert!(
        stdout.contains("cyclictest-rt-isolated-idle-10m.hist"),
        "stdout should name the file: {stdout}"
    );
    assert!(
        stdout.contains("recorded"),
        "stdout should name the recorded hash: {stdout}"
    );
    assert!(
        stdout.contains("computed"),
        "stdout should name the computed hash: {stdout}"
    );
}

#[test]
fn verify_rejects_missing_required_field() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    let run_dir = measurements.join("2026-08-30-precision3591-recon-001");
    fs::create_dir_all(&run_dir).expect("mkdir run dir");
    fs::write(run_dir.join("manifest.json"), manifest_missing_key("host"))
        .expect("write manifest.json");
    fs::write(
        run_dir.join("cyclictest-rt-isolated-idle-10m.hist"),
        HIST_BYTES,
    )
    .expect("write capture");

    let output = base_cmd(temp.path(), &measurements)
        .output()
        .expect("run verify");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("host"),
        "stdout should name the missing field: {stdout}"
    );
}

/// Distinct from `verify_rejects_checksum_mismatch`: here the artifact file the manifest names
/// never existed at all, rather than existing with different content. Required by this
/// execution's own success criteria alongside the plan's four named failure modes.
#[test]
fn verify_rejects_manifest_naming_a_nonexistent_file() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    let run_dir = measurements.join("2026-08-30-precision3591-recon-001");
    fs::create_dir_all(&run_dir).expect("mkdir run dir");

    let manifest: Value = serde_json::from_str(MINIMAL_MANIFEST).expect("fixture parses");
    fs::write(
        run_dir.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).expect("serialise manifest"),
    )
    .expect("write manifest.json");
    // Deliberately no cyclictest-rt-isolated-idle-10m.hist: the manifest names a file that was
    // never written.

    let output = base_cmd(temp.path(), &measurements)
        .output()
        .expect("run verify");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("cyclictest-rt-isolated-idle-10m.hist"),
        "stdout should name the missing artifact file: {stdout}"
    );
    assert!(
        stdout.contains("does not exist"),
        "stdout should say the named file does not exist: {stdout}"
    );
}

#[test]
fn verify_rejects_stray_capture_outside_measurements() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    fs::create_dir_all(&measurements).expect("mkdir measurements");
    fs::write(temp.path().join("stray.hist"), b"not a real capture").expect("write stray file");

    let output = base_cmd(temp.path(), &measurements)
        .output()
        .expect("run verify");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("stray.hist"), "stdout: {stdout}");
}

#[test]
fn verify_allows_exempt_trees() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    fs::create_dir_all(&measurements).expect("mkdir measurements");

    let fixtures_dir = temp.path().join("crates/histogram/tests/fixtures");
    fs::create_dir_all(&fixtures_dir).expect("mkdir fixtures dir");
    fs::write(fixtures_dir.join("sample.hist"), b"fixture data").expect("write fixture");

    let recon_dir = temp.path().join("docs/rig/recon-2026-09-01");
    fs::create_dir_all(&recon_dir).expect("mkdir recon dir");
    fs::write(recon_dir.join("probe-foo.hist"), b"probe data").expect("write probe");

    let output = base_cmd(temp.path(), &measurements)
        .output()
        .expect("run verify");

    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn verify_rejects_unprefixed_file_in_exempt_tree() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    fs::create_dir_all(&measurements).expect("mkdir measurements");

    let recon_dir = temp.path().join("docs/rig/recon-2026-09-01");
    fs::create_dir_all(&recon_dir).expect("mkdir recon dir");
    fs::write(recon_dir.join("results.hist"), b"not prefixed").expect("write file");

    let output = base_cmd(temp.path(), &measurements)
        .output()
        .expect("run verify");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("results.hist"), "stdout: {stdout}");
}

#[test]
fn verify_reports_all_failures() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    fs::create_dir_all(&measurements).expect("mkdir measurements");

    // Problem 1: an orphan capture (no manifest.json).
    let orphan_dir = measurements.join("2026-08-30-precision3591-orphan");
    fs::create_dir_all(&orphan_dir).expect("mkdir orphan dir");
    fs::write(orphan_dir.join("cyclictest.hist"), HIST_BYTES).expect("write orphan capture");

    // Problem 2: a stray capture outside measurements/ and outside the exempt trees.
    fs::write(temp.path().join("stray.hist"), b"stray").expect("write stray file");

    // Problem 3: a manifest missing a required field.
    let broken_dir = measurements.join("2026-08-30-precision3591-broken");
    fs::create_dir_all(&broken_dir).expect("mkdir broken dir");
    fs::write(
        broken_dir.join("manifest.json"),
        manifest_missing_key("kernel"),
    )
    .expect("write manifest.json");

    let output = base_cmd(temp.path(), &measurements)
        .output()
        .expect("run verify");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("precision3591-orphan"),
        "missing orphan problem: {stdout}"
    );
    assert!(
        stdout.contains("stray.hist"),
        "missing stray problem: {stdout}"
    );
    assert!(
        stdout.contains("kernel"),
        "missing missing-field problem: {stdout}"
    );
}

#[test]
fn verify_write_index_is_idempotent() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    fs::create_dir_all(&measurements).expect("mkdir measurements");
    write_valid_run(&measurements, "2026-08-30-precision3591-recon-001");

    let output = base_cmd(temp.path(), &measurements)
        .arg("--write-index")
        .output()
        .expect("run verify --write-index");
    assert!(output.status.success());
    let first = fs::read_to_string(measurements.join("INDEX.md")).expect("read INDEX.md");

    let output2 = base_cmd(temp.path(), &measurements)
        .arg("--write-index")
        .output()
        .expect("run verify --write-index again");
    assert!(output2.status.success());
    let second = fs::read_to_string(measurements.join("INDEX.md")).expect("read INDEX.md again");

    assert_eq!(
        first, second,
        "a second --write-index run must produce no diff"
    );
}

#[test]
fn verify_check_index_detects_drift() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    fs::create_dir_all(&measurements).expect("mkdir measurements");
    write_valid_run(&measurements, "2026-08-30-precision3591-recon-001");

    let output = base_cmd(temp.path(), &measurements)
        .arg("--write-index")
        .output()
        .expect("run verify --write-index");
    assert!(output.status.success());

    let index_path = measurements.join("INDEX.md");
    let mut tampered = fs::read_to_string(&index_path).expect("read INDEX.md");
    tampered.push_str("\n| hand-edited row that should never appear |\n");
    fs::write(&index_path, tampered).expect("tamper INDEX.md");

    let output2 = base_cmd(temp.path(), &measurements)
        .arg("--check-index")
        .output()
        .expect("run verify --check-index");

    assert!(!output2.status.success());
    let stdout = String::from_utf8_lossy(&output2.stdout);
    assert!(stdout.contains("INDEX.md"), "stdout: {stdout}");
}

// ---------------------------------------------------------------------------------
// Finding 6, first half (01-19 task 1): a value that could not be computed is
// `unavailable`, never a substituted zero.
// ---------------------------------------------------------------------------------

#[test]
fn unparseable_capture_is_reported_unavailable_not_zero() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    fs::create_dir_all(&measurements).expect("mkdir measurements");
    write_run_with_malformed_hist(&measurements, "2026-08-30-precision3591-malformed");

    let output = base_cmd(temp.path(), &measurements)
        .arg("--write-index")
        .output()
        .expect("run verify --write-index");
    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let index = fs::read_to_string(measurements.join("INDEX.md")).expect("read INDEX.md");
    let row = index
        .lines()
        .find(|line| line.contains("2026-08-30-precision3591-malformed"))
        .expect("row for the malformed run");
    assert!(
        row.contains("unavailable"),
        "expected unavailable in the row, got: {row}"
    );
    assert!(
        !row.contains("| 0 |"),
        "a parse failure must never render as a zero: {row}"
    );
}

#[test]
fn unparseable_capture_fails_strict_verification() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    fs::create_dir_all(&measurements).expect("mkdir measurements");
    write_run_with_malformed_hist(&measurements, "2026-08-30-precision3591-malformed");

    let output = base_cmd(temp.path(), &measurements)
        .arg("--strict")
        .output()
        .expect("run verify --strict");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("2026-08-30-precision3591-malformed"),
        "stdout should name the run: {stdout}"
    );
    assert!(
        stdout.contains("failed to parse"),
        "stdout should name the parse failure: {stdout}"
    );
}

#[test]
fn unparseable_capture_passes_non_strict_verification() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    fs::create_dir_all(&measurements).expect("mkdir measurements");
    write_run_with_malformed_hist(&measurements, "2026-08-30-precision3591-malformed");

    let output = base_cmd(temp.path(), &measurements)
        .output()
        .expect("run verify");

    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

// ---------------------------------------------------------------------------------
// Finding 6, second half (01-19 task 2): strict verification re-derives the
// published numbers from the run's own raw capture.
// ---------------------------------------------------------------------------------

/// Recursively copies a real committed run directory (e.g.
/// `measurements/2026-09-01-precision3591-calibration-clean`) into `dest`, so a strict-mode
/// re-derivation test can edit exactly one committed value and leave everything else,
/// including the real manifest and the real raw capture, untouched. Never writes back into the
/// real `measurements/` tree: `dest` is always a tempdir path.
fn copy_real_run(run_id: &str, dest: &Path) {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let src = repo_root.join("measurements").join(run_id);
    copy_dir_recursive(&src, dest);
}

fn copy_dir_recursive(src: &Path, dest: &Path) {
    fs::create_dir_all(dest).expect("mkdir dest");
    for entry in fs::read_dir(src).expect("read src dir") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if path.is_dir() {
            copy_dir_recursive(&path, &dest_path);
        } else {
            fs::copy(&path, &dest_path).expect("copy file");
        }
    }
}

#[test]
fn strict_rederives_hist_tsv() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    fs::create_dir_all(&measurements).expect("mkdir measurements");
    let run_id = "2026-09-01-precision3591-calibration-clean";
    let run_dir = measurements.join(run_id);
    copy_real_run(run_id, &run_dir);

    let hist_tsv_path = run_dir.join("hist.tsv");
    let original = fs::read_to_string(&hist_tsv_path).expect("read hist.tsv");
    let mut lines: Vec<&str> = original.lines().collect();
    let (bin, count) = lines[0].split_once('\t').expect("tab-separated line");
    let tampered_count: u64 = count.parse::<u64>().expect("numeric count") + 1;
    let tampered_first_line = format!("{bin}\t{tampered_count}");
    lines[0] = tampered_first_line.as_str();
    fs::write(&hist_tsv_path, lines.join("\n") + "\n").expect("write tampered hist.tsv");

    let output = base_cmd(temp.path(), &measurements)
        .arg("--strict")
        .output()
        .expect("run verify --strict");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("hist.tsv"), "stdout: {stdout}");
    assert!(
        stdout.contains("re-derived hist.tsv"),
        "stdout should name the disagreement: {stdout}"
    );
}

#[test]
fn strict_rederives_report_results() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    fs::create_dir_all(&measurements).expect("mkdir measurements");
    let run_id = "2026-09-01-precision3591-calibration-clean";
    let run_dir = measurements.join(run_id);
    copy_real_run(run_id, &run_dir);

    let report_path = run_dir.join("REPORT.md");
    let original = fs::read_to_string(&report_path).expect("read REPORT.md");
    let tampered = original.replace("| p99 | 9 |", "| p99 | 999 |");
    assert_ne!(
        original, tampered,
        "the fixture must actually contain the row this test edits"
    );
    fs::write(&report_path, tampered).expect("write tampered REPORT.md");

    let output = base_cmd(temp.path(), &measurements)
        .arg("--strict")
        .output()
        .expect("run verify --strict");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("REPORT.md"), "stdout: {stdout}");
    assert!(stdout.contains("p99"), "stdout: {stdout}");
    assert!(
        stdout.contains("999"),
        "stdout should name the published figure: {stdout}"
    );
}

#[test]
fn strict_records_a_reconstructed_run_as_not_rederivable() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    fs::create_dir_all(&measurements).expect("mkdir measurements");
    let run_id = "2026-08-28-precision3591";
    copy_real_run(run_id, &measurements.join(run_id));

    let output = base_cmd(temp.path(), &measurements)
        .arg("--strict")
        .output()
        .expect("run verify --strict");

    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("not re-derivable: reconstructed run has no generated report"),
        "stdout: {stdout}"
    );
}

#[test]
fn strict_refuses_to_guess_a_missing_histogram_bound() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    fs::create_dir_all(&measurements).expect("mkdir measurements");
    let run_id = "2026-09-01-precision3591-calibration-clean";
    let run_dir = measurements.join(run_id);
    copy_real_run(run_id, &run_dir);

    let manifest_path = run_dir.join("manifest.json");
    let mut manifest: Value =
        serde_json::from_str(&fs::read_to_string(&manifest_path).expect("read manifest"))
            .expect("parse manifest");
    manifest["tools"] = Value::Array(vec![]);
    fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).expect("serialise manifest"),
    )
    .expect("write manifest.json");

    let output = base_cmd(temp.path(), &measurements)
        .arg("--strict")
        .output()
        .expect("run verify --strict");

    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("not re-derivable: no recorded --histogram bound"),
        "stdout: {stdout}"
    );
}

// ---------------------------------------------------------------------------------
// Finding 7 (01-17): a failed attempt is a first-class published outcome. A
// directory holding only an ATTEMPT.json (no manifest.json) must verify under
// --strict when status is failed, and must not when status is in-progress or the
// preserved array omits a capture-shaped file actually present.
// ---------------------------------------------------------------------------------

fn sample_harness() -> HarnessInfo {
    HarnessInfo {
        version: "0.1.0".to_string(),
        git_sha: "0".repeat(40),
        git_dirty: false,
        git_sha_source: None,
        executable_blake3: None,
        executable_bytes: None,
        invoked_from_git_sha: None,
    }
}

fn sample_requested() -> RequestedRun {
    RequestedRun {
        run_class: RunClass::Recon,
        instrument_class: InstrumentClass::HeadlineSeries,
        cpus: "6-11".to_string(),
        main_cpus: "0,1".to_string(),
        duration_seconds: 60,
        with_hwlatdetect: false,
        hwlatdetect_duration_seconds: None,
    }
}

fn write_attempt_record(run_dir: &Path, record: &AttemptRecord) {
    fs::write(
        run_dir.join("ATTEMPT.json"),
        serde_json::to_string_pretty(record).expect("serialise ATTEMPT.json"),
    )
    .expect("write ATTEMPT.json");
}

/// Writes a directory holding only an `ATTEMPT.json` (`status: failed`, no
/// `manifest.json`) with one preserved capture-shaped file whose recorded blake3
/// matches the file on disk: the shape task 1 introduced, that `verify --strict`
/// must accept.
fn write_failed_attempt(measurements_root: &Path, run_id: &str) -> AttemptRecord {
    let run_dir = measurements_root.join(run_id);
    fs::create_dir_all(&run_dir).expect("mkdir run dir");

    let hist_path = run_dir.join("cyclictest.hist");
    fs::write(&hist_path, b"this is not a valid cyclictest histogram\n")
        .expect("write preserved capture");
    let blake3 = nr_manifest::blake3_file(&hist_path).expect("hash the preserved capture");
    let bytes = fs::metadata(&hist_path)
        .expect("stat the preserved capture")
        .len();

    let record = AttemptRecord {
        schema_version: nr_manifest::ATTEMPT_SCHEMA_VERSION,
        run_id: run_id.to_string(),
        utc_start: OffsetDateTime::now_utc(),
        utc_end: Some(OffsetDateTime::now_utc()),
        status: AttemptStatus::Failed,
        harness: sample_harness(),
        requested: sample_requested(),
        tools: Vec::new(),
        preserved: vec![ArtifactRecord {
            path: "cyclictest.hist".to_string(),
            bytes,
            blake3,
            kind: ArtifactKind::CyclictestHist,
            stored: StorageLocation::InRepo,
        }],
        failure: Some(AttemptFailure {
            stage: "parse".to_string(),
            message: "failed to parse cyclictest's .hist output: missing # Histogram header"
                .to_string(),
        }),
        usable_for_numerical_analysis: false,
    };
    write_attempt_record(&run_dir, &record);
    record
}

#[test]
fn verify_accepts_a_failed_attempt_directory() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    fs::create_dir_all(&measurements).expect("mkdir measurements");
    write_failed_attempt(&measurements, "2026-09-06-precision3591-recon");

    let output = base_cmd(temp.path(), &measurements)
        .arg("--strict")
        .output()
        .expect("run verify --strict");

    assert!(
        output.status.success(),
        "a failed attempt directory must pass --strict: stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn verify_rejects_a_directory_with_neither_record() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    let run_dir = measurements.join("2026-09-06-precision3591-neither");
    fs::create_dir_all(&run_dir).expect("mkdir run dir");
    fs::write(run_dir.join("cyclictest.hist"), HIST_BYTES).expect("write capture");
    // Deliberately no manifest.json and no ATTEMPT.json.

    let output = base_cmd(temp.path(), &measurements)
        .arg("--strict")
        .output()
        .expect("run verify --strict");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("2026-09-06-precision3591-neither"),
        "stdout should name the directory: {stdout}"
    );
}

#[test]
fn verify_rejects_an_unchecksummed_preserved_file() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    fs::create_dir_all(&measurements).expect("mkdir measurements");
    write_failed_attempt(&measurements, "2026-09-06-precision3591-recon");

    // A second capture-shaped file the preserved array does not name: a partial
    // capture must still be checksummed evidence, not exempted by directory alone.
    fs::write(
        measurements
            .join("2026-09-06-precision3591-recon")
            .join("cyclictest.json"),
        b"{}",
    )
    .expect("write unaccounted capture");

    let output = base_cmd(temp.path(), &measurements)
        .arg("--strict")
        .output()
        .expect("run verify --strict");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("cyclictest.json"),
        "stdout should name the unaccounted file: {stdout}"
    );
}

#[test]
fn verify_rejects_an_in_progress_attempt() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    let run_id = "2026-09-06-precision3591-recon";
    let run_dir = measurements.join(run_id);
    fs::create_dir_all(&run_dir).expect("mkdir run dir");

    let record = AttemptRecord {
        schema_version: nr_manifest::ATTEMPT_SCHEMA_VERSION,
        run_id: run_id.to_string(),
        utc_start: OffsetDateTime::now_utc(),
        utc_end: None,
        status: AttemptStatus::InProgress,
        harness: sample_harness(),
        requested: sample_requested(),
        tools: Vec::new(),
        preserved: Vec::new(),
        failure: None,
        usable_for_numerical_analysis: false,
    };
    write_attempt_record(&run_dir, &record);

    let output = base_cmd(temp.path(), &measurements)
        .arg("--strict")
        .output()
        .expect("run verify --strict");

    assert!(
        !output.status.success(),
        "a run left in-progress must fail strict verification, not be committable"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(run_id),
        "stdout should name the run: {stdout}"
    );
    assert!(
        stdout.to_lowercase().contains("in-progress")
            || stdout.to_lowercase().contains("unfinished"),
        "stdout should describe the run as unfinished: {stdout}"
    );
}

// ---------------------------------------------------------------------------------
// C1 / B5 (01-26): strict verification re-derives every published firmware figure from
// the run's own `rtla-hwnoise.txt`, using the existing `nr_capture::hwnoise` parser, and
// a requested CPU with no row fails the coverage claim instead of being explained away.
// ---------------------------------------------------------------------------------

#[test]
fn firmware_figures_rederive_from_the_committed_captures() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    fs::create_dir_all(&measurements).expect("mkdir measurements");
    for run_id in [
        "2026-09-06-precision3591-screen-02",
        "2026-09-06-precision3591-screen-03",
        "2026-09-06-precision3591-screen-04",
        "2026-09-07-precision3591-screen",
    ] {
        copy_real_run(run_id, &measurements.join(run_id));
    }

    let output = base_cmd(temp.path(), &measurements)
        .arg("--strict")
        .output()
        .expect("run verify --strict");

    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("4 firmware re-derived"),
        "stdout should count all four committed firmware screens re-derived: {stdout}"
    );
}

#[test]
fn a_hand_edited_report_maximum_fails_strict() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    fs::create_dir_all(&measurements).expect("mkdir measurements");
    let run_id = "2026-09-06-precision3591-screen-03";
    let run_dir = measurements.join(run_id);
    copy_real_run(run_id, &run_dir);

    let report_path = run_dir.join("REPORT.md");
    let original = fs::read_to_string(&report_path).expect("read REPORT.md");
    let real_line = "maximum: 1 us (the largest Max Single value (one-shot hardware-noise \
                      event) across the observed CPUs' final rtla hwnoise rows)";
    let tampered = original.replace(real_line, "maximum: 999 us (tampered)");
    assert_ne!(
        original, tampered,
        "the fixture must actually carry the firmware maximum line this test edits"
    );
    fs::write(&report_path, tampered).expect("write tampered REPORT.md");

    let output = base_cmd(temp.path(), &measurements)
        .arg("--strict")
        .output()
        .expect("run verify --strict");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(run_id),
        "stdout should name the run directory: {stdout}"
    );
    assert!(stdout.contains("maximum"), "stdout: {stdout}");
    assert!(
        stdout.contains("999 us"),
        "stdout should name the published value: {stdout}"
    );
    assert!(
        stdout.contains("re-derived 1 us"),
        "stdout should name the re-derived value: {stdout}"
    );
}

#[test]
fn a_hand_edited_manifest_maximum_fails_strict() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    fs::create_dir_all(&measurements).expect("mkdir measurements");
    let run_id = "2026-09-06-precision3591-screen-03";
    let run_dir = measurements.join(run_id);
    copy_real_run(run_id, &run_dir);

    let manifest_path = run_dir.join("manifest.json");
    let mut manifest: Value =
        serde_json::from_str(&fs::read_to_string(&manifest_path).expect("read manifest"))
            .expect("parse manifest");
    assert_eq!(
        manifest["firmware_screens"][0]["max_us"],
        Value::from(1),
        "the fixture must actually carry the real published max_us this test edits"
    );
    manifest["firmware_screens"][0]["max_us"] = Value::from(999);
    fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).expect("serialise manifest"),
    )
    .expect("write manifest.json");

    let output = base_cmd(temp.path(), &measurements)
        .arg("--strict")
        .output()
        .expect("run verify --strict");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(run_id),
        "stdout should name the run directory: {stdout}"
    );
    assert!(stdout.contains("maximum"), "stdout: {stdout}");
    assert!(
        stdout.contains("999 us"),
        "stdout should name the published value: {stdout}"
    );
    assert!(
        stdout.contains("re-derived 1 us"),
        "stdout should name the re-derived value: {stdout}"
    );
}

#[test]
fn a_requested_cpu_with_no_row_fails_coverage() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    fs::create_dir_all(&measurements).expect("mkdir measurements");
    let run_id = "2026-09-06-precision3591-screen-03";
    let run_dir = measurements.join(run_id);
    copy_real_run(run_id, &run_dir);

    let capture_path = run_dir.join("rtla-hwnoise.txt");
    let original = fs::read_to_string(&capture_path).expect("read rtla-hwnoise.txt");
    // Every header, duration and redraw-marker line starts with a token other than the
    // literal "11", so filtering on the row's own first token removes exactly cpu 11's rows
    // (every redraw block's) and nothing else, leaving cpus 6-10 fully intact.
    let stripped: String = original
        .lines()
        .filter(|line| line.split_whitespace().next() != Some("11"))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    assert_ne!(
        original, stripped,
        "the fixture must actually carry cpu 11 rows to strip"
    );
    fs::write(&capture_path, stripped).expect("write stripped capture");

    let output = base_cmd(temp.path(), &measurements)
        .arg("--strict")
        .output()
        .expect("run verify --strict");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(run_id),
        "stdout should name the run directory: {stdout}"
    );
    assert!(
        stdout.contains("cpu 11"),
        "stdout should name cpu 11: {stdout}"
    );
    assert!(
        stdout.contains("coverage cannot be confirmed"),
        "stdout: {stdout}"
    );
}

#[test]
fn an_unparsable_firmware_capture_is_a_problem_not_a_skip() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    fs::create_dir_all(&measurements).expect("mkdir measurements");
    let run_id = "2026-09-06-precision3591-screen-03";
    let run_dir = measurements.join(run_id);
    copy_real_run(run_id, &run_dir);

    let capture_path = run_dir.join("rtla-hwnoise.txt");
    fs::write(&capture_path, b"this is not rtla hwnoise output\n").expect("truncate capture");

    let output = base_cmd(temp.path(), &measurements)
        .arg("--strict")
        .output()
        .expect("run verify --strict");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(run_id),
        "stdout should name the run directory: {stdout}"
    );
    assert!(
        stdout.contains("rtla-hwnoise.txt"),
        "stdout should name the file: {stdout}"
    );
    assert!(
        stdout.contains("no 'rtla hwnoise' column header found"),
        "stdout should name the parser error, never a silent pass: {stdout}"
    );
}

#[test]
fn a_run_with_no_firmware_screen_is_neither_checked_nor_failed() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    fs::create_dir_all(&measurements).expect("mkdir measurements");
    let run_id = "2026-09-01-precision3591-calibration-clean";
    copy_real_run(run_id, &measurements.join(run_id));

    let output = base_cmd(temp.path(), &measurements)
        .arg("--strict")
        .output()
        .expect("run verify --strict");

    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("0 firmware re-derived, 0 firmware not re-derivable"),
        "a cyclictest-only run must be counted in neither firmware bucket: {stdout}"
    );
}

// ---------------------------------------------------------------------------------
// --rewrite-reports (01-26 task 3): regenerate a published REPORT.md rendering from its
// own manifest and raw capture, rather than hand-editing generated evidence.
// ---------------------------------------------------------------------------------

/// The committed `REPORT.md` this test tampers is itself stale relative to the current
/// renderer (it predates plan 01-24's Series admission section and this plan's own counter
/// table rename), so the fix under test cannot be "matches the byte-for-byte original": that
/// original was never current either. What must hold is that the tampered placeholder is gone,
/// the fresh rendering uses today's renderer, and a second pass converges to a stable output
/// rather than merely changing it again.
#[test]
fn rewrite_reports_regenerates_a_stale_report() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    fs::create_dir_all(&measurements).expect("mkdir measurements");
    let run_id = "2026-09-01-precision3591-calibration-clean";
    let run_dir = measurements.join(run_id);
    copy_real_run(run_id, &run_dir);

    let report_path = run_dir.join("REPORT.md");
    fs::write(
        &report_path,
        "this is a stale rendering, not the real one\n",
    )
    .expect("tamper REPORT.md");

    let output = base_cmd(temp.path(), &measurements)
        .arg("--rewrite-reports")
        .output()
        .expect("run verify --rewrite-reports");

    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("wrote"), "stdout: {stdout}");
    assert!(stdout.contains("1 written"), "stdout: {stdout}");

    let rewritten = fs::read_to_string(&report_path).expect("read rewritten REPORT.md");
    assert!(
        !rewritten.contains("this is a stale rendering"),
        "the tampered placeholder must be gone: {rewritten}"
    );
    assert!(
        rewritten.contains("| cpu | cal ipis | tlb ipis | res ipis | irqs |"),
        "the regenerated report must use the current renderer: {rewritten}"
    );

    // A second pass over the now-current rendering must be a true no-op: the rewrite
    // converged to a stable output rather than merely producing a different one.
    let second = base_cmd(temp.path(), &measurements)
        .arg("--rewrite-reports")
        .output()
        .expect("run verify --rewrite-reports again");
    assert!(second.status.success());
    let second_stdout = String::from_utf8_lossy(&second.stdout);
    assert!(
        second_stdout.contains("0 written"),
        "a second pass over the freshly rewritten report must write nothing: {second_stdout}"
    );
}

#[test]
fn rewrite_reports_is_idempotent() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    fs::create_dir_all(&measurements).expect("mkdir measurements");
    let run_id = "2026-09-01-precision3591-calibration-clean";
    copy_real_run(run_id, &measurements.join(run_id));

    let first = base_cmd(temp.path(), &measurements)
        .arg("--rewrite-reports")
        .output()
        .expect("run verify --rewrite-reports");
    assert!(first.status.success());

    let second = base_cmd(temp.path(), &measurements)
        .arg("--rewrite-reports")
        .output()
        .expect("run verify --rewrite-reports again");
    assert!(second.status.success());
    let stdout = String::from_utf8_lossy(&second.stdout);
    assert!(
        stdout.contains("0 written"),
        "a second run over an already-current report must write nothing: {stdout}"
    );
}

#[test]
fn rewrite_reports_refuses_to_combine_with_strict() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    fs::create_dir_all(&measurements).expect("mkdir measurements");

    let output = base_cmd(temp.path(), &measurements)
        .arg("--strict")
        .arg("--rewrite-reports")
        .output()
        .expect("run verify --strict --rewrite-reports");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("mutually exclusive"),
        "stderr should name the conflict: {stderr}"
    );
}

#[test]
fn rewrite_reports_skips_a_reconstructed_run() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    fs::create_dir_all(&measurements).expect("mkdir measurements");
    let run_id = "2026-08-28-precision3591";
    copy_real_run(run_id, &measurements.join(run_id));

    let output = base_cmd(temp.path(), &measurements)
        .arg("--rewrite-reports")
        .output()
        .expect("run verify --rewrite-reports");

    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("skipped: reconstructed run has no generated report"),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("0 written"), "stdout: {stdout}");
}

// ---------------------------------------------------------------------------------
// Plan 02-07, task 3: the stray-capture scan learns the two STOP-07 capture filename
// shapes, and a stop-harness-generated manifest (no cyclictest tool invocation at
// all) lands in check_derived_figures' not-re-derivable bucket as a note, never a
// --strict problem.
// ---------------------------------------------------------------------------------

/// Writes a run directory shaped like a `nr-stop-harness abort-latency` run: the one tool
/// invocation is named `nr-stop-harness`, never `cyclictest`, and the one artifact is
/// `kind: "other"`, naming an `abort-latency*.tsv` file rather than a `.hist`. Derived from
/// [`MINIMAL_MANIFEST`] by mutation, matching every other manifest-shaping helper in this file,
/// rather than a second fixture file for one narrow shape.
fn write_stop_harness_run(measurements_root: &Path, run_id: &str) -> Vec<u8> {
    let run_dir = measurements_root.join(run_id);
    fs::create_dir_all(&run_dir).expect("mkdir run dir");

    let capture_bytes = b"trial\tphase_ns\tabort_raw_ns\tobserved_raw_ns\tlatency_ns\n\
                           0\t100\t1000\t1030\t30\n"
        .to_vec();
    let capture_path = run_dir.join("abort-latency-33000ns.tsv");
    fs::write(&capture_path, &capture_bytes).expect("write raw capture");
    let blake3 = nr_manifest::blake3_file(&capture_path).expect("hash raw capture");

    let mut manifest: Value = serde_json::from_str(MINIMAL_MANIFEST).expect("fixture parses");
    manifest["run_id"] = Value::String(run_id.to_string());
    manifest["run_class"] = Value::String("headline".to_string());
    manifest["tools"] = serde_json::json!([{
        "name": "nr-stop-harness",
        "version": "0.1.0",
        "argv": ["nr-stop-harness", "abort-latency", "--period-ns", "33000"],
        "exit_code": 0,
        "artifact_paths": []
    }]);
    manifest["artifacts"] = serde_json::json!([{
        "path": "abort-latency-33000ns.tsv",
        "bytes": capture_bytes.len(),
        "blake3": blake3,
        "kind": "other",
        "stored": "in-repo"
    }]);
    manifest["excluded_from_series"] = Value::Bool(true);
    manifest["exclusion_reason"] = Value::String("excluded_from_series: true per D-37".to_string());
    fs::write(
        run_dir.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).expect("serialise manifest"),
    )
    .expect("write manifest.json");

    capture_bytes
}

/// Behavior: a_stray_capture_outside_a_run_directory_is_caught, for the `abort-latency*.tsv`
/// shape specifically (the existing `verify_rejects_stray_capture_outside_measurements` already
/// covers `*.hist`; the two new glob entries are the actual change under test here).
#[test]
fn verify_rejects_a_stray_stop_harness_capture() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    fs::create_dir_all(&measurements).expect("mkdir measurements");
    fs::write(
        temp.path().join("abort-latency-33000ns.tsv"),
        b"trial\tphase_ns\tabort_raw_ns\tobserved_raw_ns\tlatency_ns\n",
    )
    .expect("write stray capture");

    let output = base_cmd(temp.path(), &measurements)
        .output()
        .expect("run verify");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("abort-latency-33000ns.tsv"),
        "stdout should name the stray file: {stdout}"
    );
}

/// Behavior: a_capture_listed_as_an_artifact_is_not_a_stray. The same filename shape, this time
/// inside a run directory whose manifest lists it with a matching checksum, produces no problem.
#[test]
fn a_listed_stop_harness_capture_is_not_a_stray() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    fs::create_dir_all(&measurements).expect("mkdir measurements");
    write_stop_harness_run(&measurements, "2026-09-20-precision3591-headline");

    let output = base_cmd(temp.path(), &measurements)
        .output()
        .expect("run verify");

    assert!(
        output.status.success(),
        "a listed abort-latency*.tsv must not be flagged as a stray: stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Confirms, empirically rather than by reading `recorded_histogram_bound`'s code alone, what
/// `check_derived_figures` does with a stop-harness-generated manifest: its one tool invocation
/// is named `nr-stop-harness`, never `cyclictest`, so `recorded_histogram_bound` finds no
/// `--histogram=` argument to parse and the run lands in the not-re-derivable bucket as a note,
/// the same bucket `strict_refuses_to_guess_a_missing_histogram_bound` already exercises for a
/// cyclictest-shaped manifest with its tools array emptied. `verify.rs` needed no code change for
/// this: `check_derived_figures` already asks "was a --histogram bound recorded" before it asks
/// "is there a CyclictestHist artifact", and a stop-harness manifest fails the first, cleaner
/// question before the second ever matters.
#[test]
fn strict_records_a_stop_harness_run_as_not_rederivable() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    fs::create_dir_all(&measurements).expect("mkdir measurements");
    write_stop_harness_run(&measurements, "2026-09-20-precision3591-headline");

    let output = base_cmd(temp.path(), &measurements)
        .arg("--strict")
        .output()
        .expect("run verify --strict");

    assert!(
        output.status.success(),
        "a stop-harness manifest must pass --strict, not-re-derivable is not a problem: \
         stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("not re-derivable: no recorded --histogram bound"),
        "stdout: {stdout}"
    );
}
