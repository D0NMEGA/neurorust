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
use serde_json::Value;

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
