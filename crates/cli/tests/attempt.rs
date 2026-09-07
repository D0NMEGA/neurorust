//! Integration coverage for `nrmeasure attempt` (closing out an orphaned in-progress
//! `ATTEMPT.json`), driven against the real compiled binary via `assert_cmd`.
//!
//! Plan 01-23 added this command after a killed `rtla` left
//! `measurements/2026-09-06-precision3591-screen`'s `ATTEMPT.json` stuck at
//! `status: in-progress`, which `nrmeasure verify --strict` correctly refuses to accept
//! (`crates/cli/tests/verify.rs::verify_rejects_an_in_progress_attempt`). These tests exercise
//! the other side: that this command is the honest way to close such a record out, and that
//! doing so makes the directory pass strict verification afterward.

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use nr_manifest::{
    AttemptRecord, AttemptStatus, HarnessInfo, InstrumentClass, RequestedRun, RunClass,
};
use serde_json::Value;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

fn nrmeasure() -> Command {
    Command::cargo_bin("nrmeasure").expect("nrmeasure binary is built")
}

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
        run_class: RunClass::Screen,
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

/// An in-progress attempt with one raw file already on disk that no `preserved` entry lists
/// yet, matching the real orphan: cyclictest ran to completion and left its raw output before
/// the process was killed mid-flight, but the harness's own exit paths never rewrote the
/// record to say so.
fn write_in_progress_attempt(
    measurements_root: &Path,
    run_id: &str,
) -> (std::path::PathBuf, OffsetDateTime) {
    let run_dir = measurements_root.join(run_id);
    fs::create_dir_all(&run_dir).expect("mkdir run dir");
    fs::write(
        run_dir.join("cyclictest.hist"),
        b"# Histogram\nfake but present\n",
    )
    .expect("write the orphaned raw capture");

    let utc_start = OffsetDateTime::now_utc();
    let record = AttemptRecord {
        schema_version: nr_manifest::ATTEMPT_SCHEMA_VERSION,
        run_id: run_id.to_string(),
        utc_start,
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
    (run_dir, utc_start)
}

fn read_attempt(run_dir: &Path) -> AttemptRecord {
    let text = fs::read_to_string(run_dir.join("ATTEMPT.json")).expect("read ATTEMPT.json");
    serde_json::from_str(&text).expect("parse ATTEMPT.json")
}

#[test]
fn attempt_fail_transitions_in_progress_to_failed_and_preserves_the_orphaned_file() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    fs::create_dir_all(&measurements).expect("mkdir measurements");
    let (run_dir, _) = write_in_progress_attempt(&measurements, "2026-09-06-precision3591-screen");

    let output = nrmeasure()
        .arg("attempt")
        .arg(&run_dir)
        .arg("--reason")
        .arg("systemd SIGTERM'd this attempt (the --hwnoise-duration timeout-bound defect)")
        .output()
        .expect("run nrmeasure attempt");

    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let record = read_attempt(&run_dir);
    assert_eq!(record.status, AttemptStatus::Failed);
    assert!(
        record.utc_end.is_some(),
        "utc_end must be set once the attempt is closed out"
    );
    let failure = record
        .failure
        .expect("a failed record must carry a failure");
    assert!(
        failure.message.contains("SIGTERM"),
        "reason must be recorded verbatim: {failure:?}"
    );
    assert!(
        failure.message.contains("nrmeasure attempt"),
        "the record must say it was corrected after the fact by this command: {failure:?}"
    );
    assert_eq!(
        record.preserved.len(),
        1,
        "the orphaned raw file must be preserved"
    );
    assert_eq!(record.preserved[0].path, "cyclictest.hist");
    assert!(!record.usable_for_numerical_analysis);
}

#[test]
fn attempt_fail_honors_an_explicit_utc_end_over_the_correction_time() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    fs::create_dir_all(&measurements).expect("mkdir measurements");
    let (run_dir, utc_start) =
        write_in_progress_attempt(&measurements, "2026-09-06-precision3591-screen");

    // The real documented bound: this rig's --hwnoise-duration timeout-bound defect killed the
    // process at exactly 660s after utc_start, well before "now".
    let documented_stop = utc_start + time::Duration::seconds(660);
    let stop_str = documented_stop.format(&Rfc3339).expect("format RFC 3339");

    let output = nrmeasure()
        .arg("attempt")
        .arg(&run_dir)
        .arg("--reason")
        .arg("killed at the documented 660s timeout bound")
        .arg("--utc-end")
        .arg(&stop_str)
        .output()
        .expect("run nrmeasure attempt");

    assert!(output.status.success());

    let record = read_attempt(&run_dir);
    let recorded_end = record.utc_end.expect("utc_end set");
    assert_eq!(
        recorded_end.unix_timestamp_nanos(),
        documented_stop.unix_timestamp_nanos(),
        "an explicit --utc-end must be recorded exactly, not replaced by the correction time"
    );
    assert!(
        record
            .failure
            .expect("failure present")
            .message
            .contains("the documented stop time"),
        "the record must say utc_end came from evidence, not from when the correction ran"
    );
}

#[test]
fn attempt_fail_refuses_a_record_that_already_reached_a_terminal_status() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    fs::create_dir_all(&measurements).expect("mkdir measurements");
    let run_dir = measurements.join("2026-09-06-precision3591-already-failed");
    fs::create_dir_all(&run_dir).expect("mkdir run dir");

    let record = AttemptRecord {
        schema_version: nr_manifest::ATTEMPT_SCHEMA_VERSION,
        run_id: "2026-09-06-precision3591-already-failed".to_string(),
        utc_start: OffsetDateTime::now_utc(),
        utc_end: Some(OffsetDateTime::now_utc()),
        status: AttemptStatus::Failed,
        harness: sample_harness(),
        requested: sample_requested(),
        tools: Vec::new(),
        preserved: Vec::new(),
        failure: Some(nr_manifest::AttemptFailure {
            stage: "parse".to_string(),
            message: "already recorded as failed".to_string(),
        }),
        usable_for_numerical_analysis: false,
    };
    write_attempt_record(&run_dir, &record);
    let before = fs::read_to_string(run_dir.join("ATTEMPT.json")).expect("read before");

    let output = nrmeasure()
        .arg("attempt")
        .arg(&run_dir)
        .arg("--reason")
        .arg("should not apply")
        .output()
        .expect("run nrmeasure attempt");

    assert!(
        !output.status.success(),
        "must refuse to overwrite a record that already reached a terminal status"
    );
    let after = fs::read_to_string(run_dir.join("ATTEMPT.json")).expect("read after");
    assert_eq!(
        before, after,
        "a refused correction must not touch the file"
    );
}

#[test]
fn attempt_fail_refuses_a_directory_that_already_carries_a_manifest() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    fs::create_dir_all(&measurements).expect("mkdir measurements");
    let (run_dir, _) =
        write_in_progress_attempt(&measurements, "2026-09-06-precision3591-has-manifest");
    // A manifest existing alongside an in-progress attempt is not a shape the real harness
    // produces, but the guard must hold regardless: manifest + synthesized failure must never
    // coexist (crates/manifest/src/attempt.rs's own contract).
    fs::write(run_dir.join("manifest.json"), Value::Null.to_string()).expect("write manifest");

    let output = nrmeasure()
        .arg("attempt")
        .arg(&run_dir)
        .arg("--reason")
        .arg("should not apply")
        .output()
        .expect("run nrmeasure attempt");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("manifest.json"),
        "stderr should explain the manifest.json conflict: {stderr}"
    );
}

#[test]
fn attempt_fail_then_verify_strict_accepts_the_directory() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements = temp.path().join("measurements");
    fs::create_dir_all(&measurements).expect("mkdir measurements");
    let (run_dir, _) = write_in_progress_attempt(&measurements, "2026-09-06-precision3591-screen");

    let fail_output = nrmeasure()
        .arg("attempt")
        .arg(&run_dir)
        .arg("--reason")
        .arg("systemd SIGTERM'd this attempt")
        .output()
        .expect("run nrmeasure attempt");
    assert!(fail_output.status.success());

    let verify_output = nrmeasure()
        .arg("verify")
        .arg("--root")
        .arg(temp.path())
        .arg("--measurements")
        .arg(&measurements)
        .arg("--strict")
        .output()
        .expect("run nrmeasure verify --strict");

    assert!(
        verify_output.status.success(),
        "closing out the orphaned attempt must make it pass strict verification: stdout={}\nstderr={}",
        String::from_utf8_lossy(&verify_output.stdout),
        String::from_utf8_lossy(&verify_output.stderr)
    );
}
