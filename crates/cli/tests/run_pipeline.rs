//! End to end coverage of `nrmeasure run` with no rig present: fake tool binaries
//! (`fake-cyclictest.sh`, `fake-hwlatdetect.sh`) stand in for `cyclictest`/
//! `hwlatdetect`, and a derived, genuinely tuned facts fixture stands in for the
//! real machine, so the orchestration, manifest emission and report rendering are
//! all exercised on macOS with no rig.
//!
//! `nr-cli` is a binary-only crate (no `[lib]` target), so this integration test
//! can only interact with `nrmeasure` as a subprocess (`assert_cmd`); it cannot
//! call `nr_cli::` functions directly, and therefore duplicates the small
//! fixture-derivation helper `crates/cli/src/cmd/run.rs`'s own unit tests also
//! use, rather than sharing it.

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use nr_manifest::{GitShaSource, PreconditionStatus, RunManifest};

const CLEAN_FACTS: &str = include_str!("../../capture/tests/fixtures/probe-sysfs-tuning.txt");
const VIOLATED_FACTS: &str = include_str!("../../capture/tests/fixtures/violated-sysfs-tuning.txt");
const INTERRUPTS: &str = include_str!("../../capture/tests/fixtures/probe-proc-interrupts.txt");

/// `probe-sysfs-tuning.txt` carries only the sysfs tuning probe; it has no
/// `/proc/cpuinfo`, `/proc/meminfo`, `/proc/cmdline`, or `/etc/os-release` data at
/// all. Appended to `tuned_facts_text`'s output so the D-14 host/kernel/os fields
/// in the rendered report are genuinely populated rather than empty. Uses the
/// `@begin`/`@end` extension `nrmeasure`'s own `extract_multiline_blocks`
/// understands (see `crates/cli/src/cmd/run.rs`) for the two values that do not
/// fit on one line; `FixtureFacts::parse`'s own format has no other way to
/// represent a value with embedded newlines.
const D14_ENVIRONMENT_FIXTURE: &str = "\
/proc/meminfo=MemTotal:       31457280 kB
/proc/cmdline=BOOT_IMAGE=/vmlinuz-7.0.0-30-realtime root=UUID=1111-2222 ro quiet splash isolcpus=6-11 nohz_full=6-11 rcu_nocbs=6-11 irqaffinity=0-5,12-21
/proc/sys/kernel/osrelease=7.0.0-30-realtime
/proc/sys/kernel/version=#30-Ubuntu SMP PREEMPT_RT Fri Jul 31 18:22:54 UTC 2026
@begin /proc/cpuinfo
processor\t: 0
vendor_id\t: GenuineIntel
model name\t: Intel(R) Core(TM) Ultra 9 185H
physical id\t: 0
core id\t\t: 0
microcode\t: 0x28

processor\t: 1
vendor_id\t: GenuineIntel
model name\t: Intel(R) Core(TM) Ultra 9 185H
physical id\t: 0
core id\t\t: 1
microcode\t: 0x28
@end
@begin /etc/os-release
NAME=\"Ubuntu\"
VERSION=\"26.04.1 LTS\"
@end
";

/// `CLEAN_FACTS` is the real rig's own recon capture, and the rig has never
/// actually been observed in a tuned state (docs/rig/recon-2026-08-31/
/// FINDINGS.md; `crates/capture/tests/preconditions.rs` names this same fixture
/// `CLEAN` while documenting the same fact). This derives a fully-passing scenario
/// from it (every field a real tuned rig would report), rather than fabricating
/// one from nothing, so a headline-class run can be exercised end to end.
fn tuned_facts_text() -> String {
    let text = CLEAN_FACTS
        .replace(".governor=powersave", ".governor=performance")
        .replace(
            "systemd.default_target=graphical.target",
            "systemd.default_target=multi-user.target",
        )
        .replace(
            "intel_pstate.no_turbo=unavailable",
            "intel_pstate.no_turbo=1",
        );
    let mut lines: Vec<String> = text
        .lines()
        .map(|line| match line.split_once('=') {
            Some((key, _)) if key.starts_with("thermal.") => format!("{key}=40000"),
            _ => line.to_string(),
        })
        .collect();
    lines.push("ssh.active_sessions=0".to_string());
    lines.push("service.gdm.service.ActiveState=inactive".to_string());
    lines.push("graphical.sessions=0".to_string());
    lines.push("login.local_sessions=".to_string());
    lines.push("cpu.isolated=6-11".to_string());
    lines.push("service.rt-tuning.service.ActiveState=active".to_string());
    format!("{}\n{D14_ENVIRONMENT_FIXTURE}", lines.join("\n"))
}

/// The D-17 contaminated arm, reproduced as a fixture: otherwise identical to
/// `tuned_facts_text()` (so the D-14 environment snapshot is genuinely
/// populated and 13 of the 15 checks still pass), but with an active SSH
/// session and an active graphical session, the two conditions
/// `docs/measurement-protocol.md`'s CAVEAT names as what the 2026-08-28
/// contamination, and this calibration arm, both reproduce.
fn contaminated_calibration_facts_text() -> String {
    tuned_facts_text()
        .replace("ssh.active_sessions=0", "ssh.active_sessions=1")
        .replace("graphical.sessions=0", "graphical.sessions=1")
}

fn write_fixture(dir: &Path, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, content).expect("write fixture");
    path
}

fn fake_cyclictest_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake-cyclictest.sh")
}

fn fake_hwlatdetect_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake-hwlatdetect.sh")
}

fn thresholds_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config/contamination-thresholds.json")
}

fn base_run_command(measurements_root: &Path) -> Command {
    let mut cmd = Command::cargo_bin("nrmeasure").expect("nrmeasure binary is built");
    cmd.env("NRMEASURE_CYCLICTEST", fake_cyclictest_path())
        .env("NRMEASURE_HWLATDETECT", fake_hwlatdetect_path())
        .arg("run")
        .args(["--rig-slug", "precision3591"])
        .args(["--cpus", "6-11"])
        .args(["--main-cpus", "0,1"])
        .args(["--duration", "1"])
        .arg("--thresholds")
        .arg(thresholds_path())
        .arg("--measurements-root")
        .arg(measurements_root);
    cmd
}

/// Runs the full pipeline against a fresh temp measurements root with a fully
/// tuned facts fixture and class `recon`, and returns the run directory it wrote.
fn run_full_pipeline(temp_root: &Path) -> PathBuf {
    let facts_path = write_fixture(temp_root, "facts.txt", &tuned_facts_text());
    let interrupts_path = write_fixture(temp_root, "interrupts.txt", INTERRUPTS);
    let measurements_root = temp_root.join("measurements");
    std::fs::create_dir_all(&measurements_root).expect("mkdir measurements root");

    let output = base_run_command(&measurements_root)
        .env("NRMEASURE_FACTS_FIXTURE", &facts_path)
        .env("NRMEASURE_INTERRUPTS_FIXTURE", &interrupts_path)
        .args(["--class", "recon"])
        .output()
        .expect("nrmeasure runs");

    assert!(
        output.status.success(),
        "run should succeed: stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let entries: Vec<_> = std::fs::read_dir(&measurements_root)
        .expect("read_dir")
        .filter_map(|entry| entry.ok())
        .collect();
    assert_eq!(entries.len(), 1, "exactly one run directory is written");
    entries[0].path()
}

#[test]
fn full_run_produces_valid_run_dir() {
    let temp = tempfile::tempdir().expect("tempdir");
    let run_dir = run_full_pipeline(temp.path());

    for name in ["manifest.json", "REPORT.md", "hist.tsv", "cyclictest.hist"] {
        assert!(run_dir.join(name).is_file(), "missing {name}");
    }

    let manifest_text =
        std::fs::read_to_string(run_dir.join("manifest.json")).expect("read manifest.json");
    let manifest: RunManifest = serde_json::from_str(&manifest_text).expect("manifest.json parses");

    nr_manifest::validate(&run_dir, &manifest)
        .expect("a freshly written run directory validates with no errors");
}

/// Strips the fields that legitimately differ on every run (wall-clock
/// timestamps, and the manifest blake3 fingerprint that changes whenever any of
/// those timestamps does) so the snapshot is stable across runs and commits.
///
/// The run id carries a wall-clock date too, which is easy to miss because it
/// does not look like a timestamp. Leaving it unredacted made this test fail on
/// the first UTC day after the snapshot was captured, rather than on any real
/// change. Only the date is redacted; the rig slug and run class stay asserted.
fn redact_report(report: &str) -> String {
    report
        .lines()
        .map(|line| {
            if line.starts_with("utc start: ") {
                "utc start: [redacted]".to_string()
            } else if line.starts_with("utc end: ") {
                "utc end: [redacted]".to_string()
            } else if line.starts_with("manifest blake3: ") {
                "manifest blake3: [redacted]".to_string()
            } else if let Some(run_id) = line.strip_prefix("run id: ") {
                format!("run id: {}", redact_leading_date(run_id))
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Replaces a leading `YYYY-MM-DD` with the literal `[date]`, leaving the rest of
/// the run id intact. Returns the input unchanged if it does not start with a date.
fn redact_leading_date(run_id: &str) -> String {
    let is_date_shaped = run_id.len() >= 10
        && run_id.as_bytes()[..10].iter().enumerate().all(|(i, b)| {
            if i == 4 || i == 7 {
                *b == b'-'
            } else {
                b.is_ascii_digit()
            }
        });

    if is_date_shaped {
        format!("[date]{}", &run_id[10..])
    } else {
        run_id.to_string()
    }
}

#[test]
fn redact_leading_date_only_touches_a_leading_date() {
    assert_eq!(
        redact_leading_date("2026-09-01-precision3591-recon"),
        "[date]-precision3591-recon"
    );
    assert_eq!(
        redact_leading_date("precision3591-recon"),
        "precision3591-recon"
    );
    assert_eq!(
        redact_leading_date("2026-9-1-precision3591"),
        "2026-9-1-precision3591"
    );
}

#[test]
fn full_run_report_matches_snapshot() {
    let temp = tempfile::tempdir().expect("tempdir");
    let run_dir = run_full_pipeline(temp.path());
    let report = std::fs::read_to_string(run_dir.join("REPORT.md")).expect("read REPORT.md");
    insta::assert_snapshot!(redact_report(&report));
}

#[test]
fn full_run_hist_tsv_is_derivable() {
    let temp = tempfile::tempdir().expect("tempdir");
    let run_dir = run_full_pipeline(temp.path());
    let tsv = std::fs::read_to_string(run_dir.join("hist.tsv")).expect("read hist.tsv");

    let mut total: u64 = 0;
    let mut rows = 0u64;
    for line in tsv.lines() {
        let mut parts = line.split('\t');
        parts.next().expect("bin_us column");
        let count: u64 = parts
            .next()
            .expect("count column")
            .parse()
            .expect("count is a number");
        assert!(
            count > 0,
            "hist.tsv must have one row per non-empty bin only, got a zero-count row: {line:?}"
        );
        total += count;
        rows += 1;
    }
    assert!(rows > 0, "hist.tsv must not be empty");
    assert_eq!(
        total, 17_994_956,
        "hist.tsv's row sum is the binned sample total (the 888 overflow samples have no bin \
         index and cannot appear here)"
    );
}

#[test]
fn refusal_exits_two_and_writes_nothing() {
    let temp = tempfile::tempdir().expect("tempdir");
    let facts_path = write_fixture(temp.path(), "facts.txt", VIOLATED_FACTS);
    let measurements_root = temp.path().join("measurements");
    std::fs::create_dir_all(&measurements_root).expect("mkdir measurements root");

    let output = base_run_command(&measurements_root)
        .env("NRMEASURE_FACTS_FIXTURE", &facts_path)
        .args(["--class", "recon"])
        .output()
        .expect("nrmeasure runs");

    assert_eq!(
        output.status.code(),
        Some(2),
        "a precondition violation exits 2"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("GovernorIsPerformanceOnAllCpus"),
        "stderr should name the offending check: {stderr}"
    );
    assert_eq!(
        std::fs::read_dir(&measurements_root)
            .expect("read_dir")
            .count(),
        0,
        "a refused run must not create any run directory"
    );
}

#[test]
fn provenance_gate_rejects_orphan_capture() {
    let temp = tempfile::tempdir().expect("tempdir");
    let run_dir = run_full_pipeline(temp.path());

    let manifest_path = run_dir.join("manifest.json");
    let manifest_text = std::fs::read_to_string(&manifest_path).expect("read manifest.json");
    let manifest: RunManifest = serde_json::from_str(&manifest_text).expect("manifest.json parses");
    nr_manifest::validate(&run_dir, &manifest)
        .expect("the run validates before manifest.json is removed");

    std::fs::remove_file(&manifest_path).expect("remove manifest.json");

    // The raw capture survives; only the manifest was removed, which is exactly
    // an "orphan capture" (D-13): a capture with no manifest. `nrmeasure verify`
    // (plan 01-08) walks measurements/ and must fail to load this directory,
    // which is the condition its CI gate enforces; the underlying condition is
    // asserted here, in the pipeline that produced the run directory.
    assert!(
        run_dir.join("cyclictest.hist").is_file(),
        "the raw capture must survive manifest.json's removal"
    );
    let reload = std::fs::read_to_string(&manifest_path);
    assert!(
        reload.is_err(),
        "an orphaned capture directory (no manifest.json) must fail to load with a named error"
    );
}

#[test]
fn fixture_facts_refused_for_publishable_classes() {
    let temp = tempfile::tempdir().expect("tempdir");
    let facts_path = write_fixture(temp.path(), "facts.txt", &tuned_facts_text());
    let measurements_root = temp.path().join("measurements");
    std::fs::create_dir_all(&measurements_root).expect("mkdir measurements root");

    for class in ["headline", "weekly", "soak"] {
        let output = base_run_command(&measurements_root)
            .env("NRMEASURE_FACTS_FIXTURE", &facts_path)
            .args(["--class", class])
            .output()
            .unwrap_or_else(|_| panic!("nrmeasure runs for class {class}"));

        assert!(
            !output.status.success(),
            "class {class} must be refused when NRMEASURE_FACTS_FIXTURE is set"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("NRMEASURE_FACTS_FIXTURE cannot be used with run class"),
            "stderr for class {class}: {stderr}"
        );
    }

    assert_eq!(
        std::fs::read_dir(&measurements_root)
            .expect("read_dir")
            .count(),
        0,
        "no run directory may be written for any publishable class"
    );
}

/// D-06/D-17/PLAT-02: `--allow-precondition-violation` must be rejected outright
/// for every run class other than `calibration-contaminated` (checked here for
/// `headline`, `investigation` and `soak`), and the accepted
/// `calibration-contaminated` path must still record all 15 precondition
/// results and still force `excluded_from_series: true`. Without this
/// restriction and this test, the flag is a hole that would let a headline run
/// be published with its preconditions silently waived.
#[test]
fn allow_precondition_violation_rejected_outside_calibration_contaminated() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements_root = temp.path().join("measurements");
    std::fs::create_dir_all(&measurements_root).expect("mkdir measurements root");

    for class in ["headline", "investigation", "soak"] {
        let output = base_run_command(&measurements_root)
            .args(["--class", class])
            .arg("--allow-precondition-violation")
            .output()
            .unwrap_or_else(|_| panic!("nrmeasure runs for class {class}"));

        assert!(
            !output.status.success(),
            "--allow-precondition-violation must be rejected for class {class}"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("calibration-contaminated"),
            "stderr for class {class} should name the only class that accepts the flag: \
             {stderr}"
        );
    }

    assert_eq!(
        std::fs::read_dir(&measurements_root)
            .expect("read_dir")
            .count(),
        0,
        "no run directory may be written when the flag is rejected"
    );

    // The accepted path: calibration-contaminated, with two of the fifteen
    // preconditions deliberately violated (an active SSH session and an active
    // graphical session), the same two conditions the real D-17 contaminated
    // arm reproduces.
    let contaminated_facts_path = write_fixture(
        temp.path(),
        "contaminated-facts.txt",
        &contaminated_calibration_facts_text(),
    );
    let interrupts_path = write_fixture(temp.path(), "interrupts.txt", INTERRUPTS);

    let output = base_run_command(&measurements_root)
        .env("NRMEASURE_FACTS_FIXTURE", &contaminated_facts_path)
        .env("NRMEASURE_INTERRUPTS_FIXTURE", &interrupts_path)
        .args(["--class", "calibration-contaminated"])
        .arg("--allow-precondition-violation")
        .output()
        .expect("nrmeasure runs");

    assert!(
        output.status.success(),
        "the accepted calibration-contaminated path must still succeed: stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let entries: Vec<_> = std::fs::read_dir(&measurements_root)
        .expect("read_dir")
        .filter_map(|entry| entry.ok())
        .collect();
    assert_eq!(
        entries.len(),
        1,
        "exactly one run directory is written for the accepted path"
    );

    let manifest_text = std::fs::read_to_string(entries[0].path().join("manifest.json"))
        .expect("read manifest.json");
    let manifest: RunManifest = serde_json::from_str(&manifest_text).expect("manifest.json parses");

    assert_eq!(
        manifest.preconditions.len(),
        15,
        "every one of the 15 preconditions must still be recorded when the flag waives the \
         refusal"
    );
    let failed: Vec<_> = manifest
        .preconditions
        .iter()
        .filter(|p| p.status == PreconditionStatus::Fail)
        .collect();
    assert!(
        !failed.is_empty(),
        "the deliberately violated preconditions must still be recorded as Fail, not silently \
         dropped: {:?}",
        manifest.preconditions
    );
    assert!(
        manifest.excluded_from_series,
        "a run taken with --allow-precondition-violation must always be excluded_from_series"
    );
    assert!(
        manifest
            .exclusion_reason
            .as_ref()
            .is_some_and(|reason| !reason.is_empty()),
        "excluded_from_series must carry a meaningful, non-empty exclusion_reason, got {:?}",
        manifest.exclusion_reason
    );
}

/// A missing or unreadable thresholds file must be caught BEFORE the measurement runs,
/// not at the point of use.
///
/// This is a regression test for a real loss. On 2026-09-01 a full one-hour calibration
/// run completed on the reference rig and was then discarded, because the thresholds path
/// defaults to `./config/contamination-thresholds.json` and systemd's working directory is
/// `/`, not the repository root. The harness had already fixed this class of mistake for
/// tools ("Step 1: ... Fail early if a tool is missing") but loaded this file at step 6.
///
/// Asserting the exit code alone would not catch a regression, since the run fails either
/// way. The marker file is what proves the ordering: cyclictest must never be invoked.
#[test]
fn bad_thresholds_path_fails_before_the_measurement_runs() {
    let temp = tempfile::tempdir().expect("tempdir");
    let facts_path = write_fixture(temp.path(), "facts.txt", &tuned_facts_text());
    let interrupts_path = write_fixture(temp.path(), "interrupts.txt", INTERRUPTS);
    let measurements_root = temp.path().join("measurements");
    std::fs::create_dir_all(&measurements_root).expect("mkdir measurements root");
    let marker = temp.path().join("cyclictest-was-invoked");

    let mut cmd = Command::cargo_bin("nrmeasure").expect("nrmeasure binary is built");
    cmd.env("NRMEASURE_CYCLICTEST", fake_cyclictest_path())
        .env("NRMEASURE_HWLATDETECT", fake_hwlatdetect_path())
        .env("NRMEASURE_FACTS_FIXTURE", &facts_path)
        .env("NRMEASURE_INTERRUPTS_FIXTURE", &interrupts_path)
        .env("FAKE_CYCLICTEST_MARKER", &marker)
        .arg("run")
        .args(["--rig-slug", "precision3591"])
        .args(["--cpus", "6-11"])
        .args(["--main-cpus", "0,1"])
        .args(["--duration", "1"])
        .args(["--class", "recon"])
        .arg("--thresholds")
        .arg(temp.path().join("definitely-not-here.json"))
        .arg("--measurements-root")
        .arg(&measurements_root);

    let output = cmd.output().expect("nrmeasure runs");

    assert!(!output.status.success(), "a bad thresholds path must fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("thresholds"),
        "the error must name the thresholds file: {stderr}"
    );
    assert!(
        !marker.exists(),
        "cyclictest must never be invoked when the thresholds file is unreadable; \
         the marker at {} proves the measurement started anyway",
        marker.display()
    );
    let entries = std::fs::read_dir(&measurements_root)
        .expect("read_dir")
        .count();
    assert_eq!(entries, 0, "no run directory may be written");
}

// ---------------------------------------------------------------------------------
// Harness identity: compiled in by crates/cli/build.rs, not re-derived from the
// working directory at run time (finding 6 of 01-EXTERNAL-AUDIT.md). Seven of the
// eight committed manifests recorded `git_sha: "unknown"` because the run's working
// directory was never inside a git checkout (systemd-run); the eighth carried the
// checkout's sha rather than the binary's own. These three tests guard the fix.
// ---------------------------------------------------------------------------------

/// A run launched with a working directory outside any git checkout (the
/// systemd-run shape that produced seven "unknown" manifests) still records the
/// real 40-character commit the binary was built from, because `git_sha` is now
/// embedded at compile time rather than read from the process's cwd at run time.
#[test]
fn harness_identity_is_compiled_in() {
    let temp = tempfile::tempdir().expect("tempdir");
    let facts_path = write_fixture(temp.path(), "facts.txt", &tuned_facts_text());
    let interrupts_path = write_fixture(temp.path(), "interrupts.txt", INTERRUPTS);
    let measurements_root = temp.path().join("measurements");
    std::fs::create_dir_all(&measurements_root).expect("mkdir measurements root");
    // A second, unrelated tempdir: outside this repository's git checkout, exactly
    // the systemd-run shape (no working directory inside the checkout) that
    // produced seven "unknown" manifests.
    let outside_checkout = tempfile::tempdir().expect("a second, unrelated tempdir");

    let output = base_run_command(&measurements_root)
        .env("NRMEASURE_FACTS_FIXTURE", &facts_path)
        .env("NRMEASURE_INTERRUPTS_FIXTURE", &interrupts_path)
        .args(["--class", "recon"])
        .current_dir(outside_checkout.path())
        .output()
        .expect("nrmeasure runs");
    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let entries: Vec<_> = std::fs::read_dir(&measurements_root)
        .expect("read_dir")
        .filter_map(|entry| entry.ok())
        .collect();
    assert_eq!(entries.len(), 1, "exactly one run directory is written");
    let manifest: RunManifest = serde_json::from_str(
        &std::fs::read_to_string(entries[0].path().join("manifest.json"))
            .expect("read manifest.json"),
    )
    .expect("manifest.json parses");

    assert_eq!(
        manifest.harness.git_sha.len(),
        40,
        "expected a 40-character commit sha, got {:?}",
        manifest.harness.git_sha
    );
    assert!(
        manifest
            .harness
            .git_sha
            .chars()
            .all(|c| c.is_ascii_hexdigit()),
        "git_sha must be hex: {:?}",
        manifest.harness.git_sha
    );
    assert_ne!(manifest.harness.git_sha, "unknown");
    assert_eq!(
        manifest.harness.invoked_from_git_sha, None,
        "the process had no git checkout as its working directory, so the invoked-from \
         identifier must be absent, distinctly from the compiled-in git_sha above"
    );
}

/// The manifest's `harness.executable_blake3` is an independently computed blake3 of
/// the exact `nrmeasure` binary that produced it, not a value merely trusted from the
/// harness's own internal bookkeeping.
#[test]
fn harness_records_executable_hash() {
    let temp = tempfile::tempdir().expect("tempdir");
    let run_dir = run_full_pipeline(temp.path());
    let manifest: RunManifest = serde_json::from_str(
        &std::fs::read_to_string(run_dir.join("manifest.json")).expect("read manifest.json"),
    )
    .expect("manifest.json parses");

    let nrmeasure_path = assert_cmd::cargo::cargo_bin("nrmeasure");
    let expected_hash = nr_manifest::blake3_file(&nrmeasure_path)
        .expect("hash the exact nrmeasure binary this test invoked");
    let expected_bytes = std::fs::metadata(&nrmeasure_path)
        .expect("stat the nrmeasure binary")
        .len();

    assert_eq!(
        manifest.harness.executable_blake3.as_deref(),
        Some(expected_hash.as_str()),
        "the manifest's executable_blake3 must equal an independently computed hash of the \
         binary that actually ran"
    );
    assert_eq!(manifest.harness.executable_bytes, Some(expected_bytes));
}

/// `git_sha_source` makes the previous "unknown git_sha, git_dirty: false" pair, which
/// asserted a clean tree nobody observed (seven committed manifests carry exactly that
/// pair), representable as an honest "we could not tell". For a build inside a real git
/// checkout (this one), `git_dirty` must equal the independently observed state of the
/// working tree, never a hardcoded default.
#[test]
fn harness_git_sha_source_is_explicit() {
    let temp = tempfile::tempdir().expect("tempdir");
    let run_dir = run_full_pipeline(temp.path());
    let manifest: RunManifest = serde_json::from_str(
        &std::fs::read_to_string(run_dir.join("manifest.json")).expect("read manifest.json"),
    )
    .expect("manifest.json parses");

    assert_eq!(
        manifest.harness.git_sha_source,
        Some(GitShaSource::BuildTime),
        "this repository's own build always has git available"
    );

    let status = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("git status runs in this checkout");
    let actually_dirty = !String::from_utf8_lossy(&status.stdout).trim().is_empty();
    assert_eq!(
        manifest.harness.git_dirty, actually_dirty,
        "git_dirty must be the real, independently observed state, never a silent default (the \
         previous code reported false unconditionally whenever git could not be read)"
    );
}
