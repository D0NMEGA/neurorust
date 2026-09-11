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
use nr_manifest::{ArtifactKind, GitShaSource, PreconditionStatus, RunManifest};

const CLEAN_FACTS: &str = include_str!("../../capture/tests/fixtures/probe-sysfs-tuning.txt");
const VIOLATED_FACTS: &str = include_str!("../../capture/tests/fixtures/violated-sysfs-tuning.txt");
const INTERRUPTS: &str = include_str!("../../capture/tests/fixtures/probe-proc-interrupts.txt");

/// `MSR_SMI_COUNT` on cpus 6-11, the hex lines only from
/// `docs/rig/recon-2026-09-05/probe-rdmsr-smi-count.txt`. That probe's separate `-d`
/// (decimal) lines for cpu 6 and cpu 11 are deliberately excluded: the harness's own
/// invocation (`rdmsr -p <cpu> 0x34`, never `-d`) always produces hex, and
/// `parse_rdmsr_value` always parses as hex, so including a decimal "4006" line would
/// silently be read as 0x4006 instead of the real 0xfa6 reading.
const SMI_COUNTS: &str = "\
cpu 6  0x34 = fa6
cpu 7  0x34 = fa6
cpu 8  0x34 = fa6
cpu 9  0x34 = fa6
cpu 10 0x34 = fa6
cpu 11 0x34 = fa6
";

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
    // `DeepCstatesDisabled` now reads every target CPU's own cpuidle tree (finding 8,
    // 01-EXTERNAL-AUDIT.md) rather than cpu0's alone. `CLEAN_FACTS` only ever probed
    // cpu0 (POLL, C1E; no C6/C10 registered, the `intel_idle.max_cstate=1` rationale),
    // so a genuinely tuned reading of the isolated cores (6-11, matching `--cpus 6-11`
    // in `base_run_command`) must be supplied explicitly here, or every full-pipeline
    // test below would see `Unavailable` (no data was ever read for those CPUs) rather
    // than the intended `Pass`.
    for cpu in 6..=11 {
        lines.push(format!(
            "/sys/devices/system/cpu/cpu{cpu}/cpuidle/state0/name=POLL"
        ));
        lines.push(format!(
            "/sys/devices/system/cpu/cpu{cpu}/cpuidle/state0/disable=0"
        ));
        lines.push(format!(
            "/sys/devices/system/cpu/cpu{cpu}/cpuidle/state1/name=C1E"
        ));
        lines.push(format!(
            "/sys/devices/system/cpu/cpu{cpu}/cpuidle/state1/disable=0"
        ));
    }
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

fn fake_rtla_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake-rtla.sh")
}

fn thresholds_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config/contamination-thresholds.json")
}

fn base_run_command(measurements_root: &Path) -> Command {
    let mut cmd = Command::cargo_bin("nrmeasure").expect("nrmeasure binary is built");
    cmd.env("NRMEASURE_CYCLICTEST", fake_cyclictest_path())
        .env("NRMEASURE_HWLATDETECT", fake_hwlatdetect_path())
        .env("NRMEASURE_RTLA", fake_rtla_path())
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
/// timestamps, the manifest blake3 fingerprint that changes whenever any of
/// those timestamps does, and the D-24 overflow rate) so the snapshot is stable
/// across runs and commits.
///
/// The run id carries a wall-clock date too, which is easy to miss because it
/// does not look like a timestamp. Leaving it unredacted made this test fail on
/// the first UTC day after the snapshot was captured, rather than on any real
/// change. Only the date is redacted; the rig slug and run class stay asserted.
///
/// The overflow rate is redacted for a different reason (finding 7 of
/// `01-EXTERNAL-AUDIT.md`): it is now `overflow_count / cyclictest window's
/// measured elapsed time`, and that elapsed time is real subprocess wall-clock
/// duration, not the fixed `--duration` this fixture's fake tool used to be
/// divided by. A few milliseconds of scheduling noise around an
/// almost-instant fake tool changes the fourth significant figure between
/// runs; the other tail metrics on the same table row (tail excursion ratio,
/// thread-max spread) do not depend on elapsed time and stay asserted.
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
            } else if line.starts_with("| overflow rate |") {
                "| overflow rate | [redacted]/s |".to_string()
            } else if line.starts_with("| smi-delta |") && line.ends_with("| unavailable |") {
                // Why the host cannot read MSR_SMI_COUNT is host-specific, and the CI
                // matrix runs this on two hosts that fail differently: macOS refuses at
                // the platform check ("requires Linux"), while a Linux runner gets past
                // that and fails to spawn rdmsr, which is not installed there. Both are
                // correct and both are "unavailable"; only the sentence differs. Asserting
                // on the sentence made this snapshot unsatisfiable on one of the two
                // platforms no matter which host recorded it, and it was recorded on the
                // dev host, so ci has been red on ubuntu-latest since 2026-09-07.
                //
                // The verdict is kept because it is portable and is what the row means.
                // Only the `unavailable` case is redacted: a real SMI reading stays
                // visible, so this cannot hide a regression that turns a count into
                // something else.
                "| smi-delta | [redacted: host-specific reason] | unavailable |".to_string()
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

/// The facts fixture and the interrupts fixture must be refused identically for
/// headline, weekly and soak: reusing one fixture text for both the before and the
/// after interference snapshot makes every delta it produces exactly zero, which
/// reads as a perfectly quiet machine rather than as a value that was never measured.
/// Finding 8 of `01-EXTERNAL-AUDIT.md`.
#[test]
fn interrupts_fixture_is_refused_for_publishable_classes() {
    let temp = tempfile::tempdir().expect("tempdir");
    let interrupts_path = write_fixture(temp.path(), "interrupts.txt", INTERRUPTS);
    let measurements_root = temp.path().join("measurements");
    std::fs::create_dir_all(&measurements_root).expect("mkdir measurements root");

    for class in ["headline", "weekly", "soak"] {
        let output = base_run_command(&measurements_root)
            .env("NRMEASURE_INTERRUPTS_FIXTURE", &interrupts_path)
            .args(["--class", class])
            .output()
            .unwrap_or_else(|_| panic!("nrmeasure runs for class {class}"));

        assert!(
            !output.status.success(),
            "class {class} must be refused when NRMEASURE_INTERRUPTS_FIXTURE is set"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("NRMEASURE_INTERRUPTS_FIXTURE cannot be used with run class"),
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

/// Runs a `calibration-clean`-class measurement with both fixture seams active (a
/// class where a fixture is legitimately allowed) and returns the resulting manifest.
/// Task 2 of finding 8, `01-EXTERNAL-AUDIT.md`: `fixtures_used`, `exclusion_reason`
/// and the verdict's own reason must all name the fixtures used, and the run must
/// always be forced out of the series.
fn run_fixture_driven_calibration(temp_root: &Path) -> RunManifest {
    let facts_path = write_fixture(temp_root, "facts.txt", &tuned_facts_text());
    let interrupts_path = write_fixture(temp_root, "interrupts.txt", INTERRUPTS);
    let measurements_root = temp_root.join("measurements");
    std::fs::create_dir_all(&measurements_root).expect("mkdir measurements root");

    let output = base_run_command(&measurements_root)
        .env("NRMEASURE_FACTS_FIXTURE", &facts_path)
        .env("NRMEASURE_INTERRUPTS_FIXTURE", &interrupts_path)
        .args(["--class", "calibration-clean"])
        .output()
        .expect("nrmeasure runs");

    assert!(
        output.status.success(),
        "a fixture-driven calibration-clean run must still succeed: stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let entries: Vec<_> = std::fs::read_dir(&measurements_root)
        .expect("read_dir")
        .filter_map(|entry| entry.ok())
        .collect();
    assert_eq!(entries.len(), 1, "exactly one run directory is written");

    let manifest_text = std::fs::read_to_string(entries[0].path().join("manifest.json"))
        .expect("read manifest.json");
    serde_json::from_str(&manifest_text).expect("manifest.json parses")
}

#[test]
fn fixture_run_records_the_fixture_it_used() {
    let temp = tempfile::tempdir().expect("tempdir");
    let manifest = run_fixture_driven_calibration(temp.path());
    assert!(
        manifest
            .fixtures_used
            .iter()
            .any(|f| f == "NRMEASURE_FACTS_FIXTURE"),
        "fixtures_used should name NRMEASURE_FACTS_FIXTURE: {:?}",
        manifest.fixtures_used
    );
    assert!(
        manifest
            .fixtures_used
            .iter()
            .any(|f| f == "NRMEASURE_INTERRUPTS_FIXTURE"),
        "fixtures_used should name NRMEASURE_INTERRUPTS_FIXTURE: {:?}",
        manifest.fixtures_used
    );
}

#[test]
fn fixture_run_is_excluded_from_series() {
    let temp = tempfile::tempdir().expect("tempdir");
    let manifest = run_fixture_driven_calibration(temp.path());
    assert!(
        manifest.excluded_from_series,
        "a fixture-driven run must always be excluded_from_series"
    );
    let reason = manifest
        .exclusion_reason
        .as_ref()
        .expect("exclusion_reason must be present");
    assert!(
        reason.contains("NRMEASURE_FACTS_FIXTURE")
            || reason.contains("NRMEASURE_INTERRUPTS_FIXTURE"),
        "exclusion_reason should name a fixture: {reason}"
    );
}

/// The reason explains WHY the deltas are zero (a fixture text read repeatedly),
/// rather than merely naming the fixture, so a reader cannot mistake a fabricated
/// zero for evidence of a quiet machine.
#[test]
fn fixture_run_verdict_states_the_fixture() {
    let temp = tempfile::tempdir().expect("tempdir");
    let manifest = run_fixture_driven_calibration(temp.path());
    let reason = manifest
        .exclusion_reason
        .as_ref()
        .expect("exclusion_reason must be present");
    assert!(
        reason.contains("zero"),
        "the reason should explain that fixture-driven deltas are zero by construction, not \
         evidence of a quiet machine: {reason}"
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

/// `crates/cli/build.rs` only reaches for the `.git-sha` pushed-stamp fallback when
/// `git rev-parse HEAD` itself fails, which it never does inside this repository's
/// own checkout. Guards against the D-29 fallback silently becoming the default here
/// (which would hide a build machine's genuine `pushed-stamp`/`unavailable` provenance
/// behind an always-true `build-time`).
#[test]
fn a_dev_host_build_still_records_build_time() {
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
    assert_eq!(
        manifest.harness.git_sha.len(),
        40,
        "expected a full 40-character sha, got {:?}",
        manifest.harness.git_sha
    );
    assert!(
        manifest
            .harness
            .git_sha
            .chars()
            .all(|c| c.is_ascii_hexdigit()),
        "expected the sha to be hex, got {:?}",
        manifest.harness.git_sha
    );
}

// ---------------------------------------------------------------------------------
// Recorded argv and artifact_paths: the argv is byte for byte what ran, and every
// output-file path in it maps to a named artifact (finding 6 of
// 01-EXTERNAL-AUDIT.md). Relativizing argv against the run directory used to match
// nothing, because the tools actually wrote into a scratch tempdir under /tmp.
// ---------------------------------------------------------------------------------

/// The manifest's cyclictest argv is exactly what the process received, byte for
/// byte, with no rewriting attempted against it.
#[test]
fn argv_is_recorded_as_executed() {
    let temp = tempfile::tempdir().expect("tempdir");
    let facts_path = write_fixture(temp.path(), "facts.txt", &tuned_facts_text());
    let interrupts_path = write_fixture(temp.path(), "interrupts.txt", INTERRUPTS);
    let measurements_root = temp.path().join("measurements");
    std::fs::create_dir_all(&measurements_root).expect("mkdir measurements root");
    let argv_log = temp.path().join("cyclictest-argv.log");

    let output = base_run_command(&measurements_root)
        .env("NRMEASURE_FACTS_FIXTURE", &facts_path)
        .env("NRMEASURE_INTERRUPTS_FIXTURE", &interrupts_path)
        .env("FAKE_CYCLICTEST_ARGV_FILE", &argv_log)
        .args(["--class", "recon"])
        .output()
        .expect("nrmeasure runs");
    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let executed_argv: Vec<String> = std::fs::read_to_string(&argv_log)
        .expect("the fake cyclictest logged the argv it received")
        .lines()
        .map(|s| s.to_string())
        .collect();
    assert!(
        !executed_argv.is_empty(),
        "the fake tool must have logged something"
    );

    let entries: Vec<_> = std::fs::read_dir(&measurements_root)
        .expect("read_dir")
        .filter_map(|entry| entry.ok())
        .collect();
    let run_dir = entries[0].path();
    let manifest: RunManifest = serde_json::from_str(
        &std::fs::read_to_string(run_dir.join("manifest.json")).expect("read manifest.json"),
    )
    .expect("manifest.json parses");
    let cyclictest = manifest
        .tools
        .iter()
        .find(|t| t.name == "cyclictest")
        .expect("cyclictest ran");

    // tools[].argv is [name, ...args]; the log captured only the args.
    assert_eq!(
        &cyclictest.argv[1..],
        executed_argv.as_slice(),
        "the recorded argv must be byte for byte what the process actually received"
    );

    let histfile_arg = cyclictest
        .argv
        .iter()
        .find(|a| a.starts_with("--histfile="))
        .expect("--histfile is always passed");
    let histfile_value = histfile_arg.strip_prefix("--histfile=").unwrap();
    assert!(
        Path::new(histfile_value).is_absolute(),
        "the scratch path cyclictest actually wrote into is absolute: {histfile_value:?}"
    );
}

/// Every entry of `tools[].artifact_paths` names an artifact that actually exists in
/// this manifest's own `artifacts` array, and its `executed_path` actually appears
/// in that tool's own recorded argv.
#[test]
fn artifact_path_mapping_names_every_output() {
    let temp = tempfile::tempdir().expect("tempdir");
    let run_dir = run_full_pipeline(temp.path());
    let manifest: RunManifest = serde_json::from_str(
        &std::fs::read_to_string(run_dir.join("manifest.json")).expect("read manifest.json"),
    )
    .expect("manifest.json parses");

    let artifact_names: Vec<&str> = manifest.artifacts.iter().map(|a| a.path.as_str()).collect();
    let mut total_mappings = 0usize;
    for tool in &manifest.tools {
        for mapping in &tool.artifact_paths {
            total_mappings += 1;
            assert!(
                artifact_names.contains(&mapping.artifact_path.as_str()),
                "{}'s artifact_paths names {:?}, which is not in artifacts: {artifact_names:?}",
                tool.name,
                mapping.artifact_path
            );
            assert!(
                tool.argv.iter().any(|a| a.contains(&mapping.executed_path)),
                "executed_path {:?} does not appear in {}'s own argv: {:?}",
                mapping.executed_path,
                tool.name,
                tool.argv
            );
        }
    }
    assert!(
        total_mappings > 0,
        "the pipeline must produce at least one artifact_paths mapping"
    );

    let cyclictest = manifest
        .tools
        .iter()
        .find(|t| t.name == "cyclictest")
        .expect("cyclictest ran");
    assert!(
        cyclictest
            .artifact_paths
            .iter()
            .any(|m| m.artifact_path == "cyclictest.hist"),
        "cyclictest.hist must be mapped: {:?}",
        cyclictest.artifact_paths
    );
}

/// An argv element (or an `artifact_paths.executed_path`) under the process's home
/// directory is recorded with the home prefix replaced by the literal token
/// `[redacted]` (T-1-06, T-1-48): `scripts/nr-run-measurement` passes
/// `--measurements-root /home/<user>/neurorust/measurements` on every rig
/// invocation, and the scratch directory the tools actually write into would carry
/// the same exposure if `$TMPDIR` (or an equivalent) ever pointed under home.
#[test]
fn argv_redacts_a_home_directory_prefix() {
    let temp = tempfile::tempdir().expect("tempdir");
    let facts_path = write_fixture(temp.path(), "facts.txt", &tuned_facts_text());
    let interrupts_path = write_fixture(temp.path(), "interrupts.txt", INTERRUPTS);

    // A directory shaped like a real home directory, forced as $HOME for the
    // subprocess. The run directory (and therefore every path the tools write
    // into, now that they write straight into it rather than a scratch tempdir)
    // is rooted under it here, matching the real exposure: every rig invocation
    // of scripts/nr-run-measurement passes
    // --measurements-root /home/<user>/neurorust/measurements.
    let fake_home = temp.path().join("home-precision3591-fake");
    let measurements_root = fake_home.join("neurorust").join("measurements");
    std::fs::create_dir_all(&measurements_root).expect("mkdir measurements root");

    let output = base_run_command(&measurements_root)
        .env("NRMEASURE_FACTS_FIXTURE", &facts_path)
        .env("NRMEASURE_INTERRUPTS_FIXTURE", &interrupts_path)
        .env("HOME", &fake_home)
        .args(["--class", "recon"])
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
    let manifest: RunManifest = serde_json::from_str(
        &std::fs::read_to_string(entries[0].path().join("manifest.json"))
            .expect("read manifest.json"),
    )
    .expect("manifest.json parses");
    let cyclictest = manifest
        .tools
        .iter()
        .find(|t| t.name == "cyclictest")
        .expect("cyclictest ran");

    let fake_home_str = fake_home.display().to_string();
    let histfile_arg = cyclictest
        .argv
        .iter()
        .find(|a| a.starts_with("--histfile="))
        .expect("--histfile is always passed");
    assert!(
        histfile_arg.starts_with("--histfile=[redacted]"),
        "expected the home-directory prefix to be redacted, got {histfile_arg:?}"
    );
    assert!(
        !histfile_arg.contains(&fake_home_str),
        "the real home path must not survive redaction: {histfile_arg:?}"
    );

    let mapping = cyclictest
        .artifact_paths
        .iter()
        .find(|m| m.artifact_path == "cyclictest.hist")
        .expect("cyclictest.hist mapping present");
    assert!(
        mapping.executed_path.starts_with("[redacted]"),
        "executed_path must also be redacted: {:?}",
        mapping.executed_path
    );
    assert!(!mapping.executed_path.contains(&fake_home_str));
}

/// Plan 01-23, T-1-95: `scripts/nr-run-measurement` launches this binary as a root
/// `systemd-run` transient unit with no `User=`/`PAMName=`, whose environment does not
/// reliably set `HOME` to the operator's real home directory. Three committed manifests
/// leaked `/home/<user>/...` unredacted this way before it was caught, even though
/// `argv_redacts_a_home_directory_prefix` above was green throughout: that test forces
/// `HOME` via `.env(...)`, so it never exercised the real gap. Here `HOME` is set to a
/// directory unrelated to where the run directory actually lives (standing in for "unset
/// or wrong"), so only the measurements-root-derived prefix can save the redaction.
#[test]
fn argv_redacts_the_conventional_home_shape_even_when_home_does_not_match_it() {
    let temp = tempfile::tempdir().expect("tempdir");
    let facts_path = write_fixture(temp.path(), "facts.txt", &tuned_facts_text());
    let interrupts_path = write_fixture(temp.path(), "interrupts.txt", INTERRUPTS);

    // Mirrors the real rig's own layout (`/home/d0nmega/neurorust/measurements`): a user
    // directory literally named `home`, not the fake_home style of
    // `argv_redacts_a_home_directory_prefix` above, which sits under a bare tempdir name
    // that just happens to be pointed to by `$HOME`. Here `$HOME` points somewhere
    // unrelated, standing in for the real production gap (unset or wrong under a
    // `systemd-run` transient unit), so only the structural `/home/<user>` match can save
    // the redaction.
    let home_like = temp.path().join("home").join("d0nmega-fake");
    let real_root = home_like.join("neurorust");
    let measurements_root = real_root.join("measurements");
    std::fs::create_dir_all(&measurements_root).expect("mkdir measurements root");
    let wrong_home = temp.path().join("root");

    let output = base_run_command(&measurements_root)
        .env("NRMEASURE_FACTS_FIXTURE", &facts_path)
        .env("NRMEASURE_INTERRUPTS_FIXTURE", &interrupts_path)
        .env("HOME", &wrong_home)
        .args(["--class", "recon"])
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
    let manifest: RunManifest = serde_json::from_str(
        &std::fs::read_to_string(entries[0].path().join("manifest.json"))
            .expect("read manifest.json"),
    )
    .expect("manifest.json parses");
    let cyclictest = manifest
        .tools
        .iter()
        .find(|t| t.name == "cyclictest")
        .expect("cyclictest ran");

    let home_like_str = home_like.display().to_string();
    let histfile_arg = cyclictest
        .argv
        .iter()
        .find(|a| a.starts_with("--histfile="))
        .expect("--histfile is always passed");
    assert!(
        histfile_arg.starts_with("--histfile=[redacted]"),
        "expected redaction even though HOME does not match the real home-shaped path: \
         {histfile_arg:?}"
    );
    assert!(
        !histfile_arg.contains(&home_like_str),
        "the identity-bearing home-shaped path must not survive when HOME is wrong: \
         {histfile_arg:?}"
    );
    assert!(
        histfile_arg.contains("neurorust"),
        "only the home-shaped prefix is redacted, not the whole path: {histfile_arg:?}"
    );

    let mapping = cyclictest
        .artifact_paths
        .iter()
        .find(|m| m.artifact_path == "cyclictest.hist")
        .expect("cyclictest.hist mapping present");
    assert!(mapping.executed_path.starts_with("[redacted]"));
    assert!(!mapping.executed_path.contains(&home_like_str));
}

// ---------------------------------------------------------------------------------
// The durable attempt record (finding 7 of 01-EXTERNAL-AUDIT.md, first half): the
// run directory and ATTEMPT.json exist before the first instrument starts, and no
// failure between then and the manifest write can delete the evidence. Raw output
// used to live in a `tempfile::tempdir()` until parsing, reconciliation and the
// contamination verdict had all succeeded, so any failure among them deleted the
// capture and an hour of rig time survived only as a line on stderr.
// ---------------------------------------------------------------------------------

/// While a deliberately slow cyclictest is still running, the run directory and an
/// `ATTEMPT.json` with `status: in-progress` are already on disk, and no
/// `manifest.json` exists yet. Polls for a sentinel the fake tool touches before it
/// sleeps, and confirms the child process has not exited yet, rather than asserting
/// on timing alone.
#[test]
fn attempt_record_exists_before_any_tool_runs() {
    let temp = tempfile::tempdir().expect("tempdir");
    let facts_path = write_fixture(temp.path(), "facts.txt", &tuned_facts_text());
    let interrupts_path = write_fixture(temp.path(), "interrupts.txt", INTERRUPTS);
    let measurements_root = temp.path().join("measurements");
    std::fs::create_dir_all(&measurements_root).expect("mkdir measurements root");
    let sentinel = temp.path().join("cyclictest-started");

    // Spawned directly via std::process::Command (not assert_cmd::Command, whose
    // own spawn() is private): this test needs to observe on-disk state while the
    // subprocess is still running, which Command::output()/assert_cmd's own
    // helpers cannot do, since both block until the process exits.
    let mut cmd = std::process::Command::new(assert_cmd::cargo::cargo_bin("nrmeasure"));
    cmd.env("NRMEASURE_CYCLICTEST", fake_cyclictest_path())
        .env("NRMEASURE_HWLATDETECT", fake_hwlatdetect_path())
        .env("NRMEASURE_FACTS_FIXTURE", &facts_path)
        .env("NRMEASURE_INTERRUPTS_FIXTURE", &interrupts_path)
        .env("FAKE_CYCLICTEST_SENTINEL", &sentinel)
        .env("FAKE_CYCLICTEST_SLEEP_SECONDS", "2")
        .arg("run")
        .args(["--rig-slug", "precision3591"])
        .args(["--cpus", "6-11"])
        .args(["--main-cpus", "0,1"])
        .args(["--duration", "1"])
        .arg("--thresholds")
        .arg(thresholds_path())
        .arg("--measurements-root")
        .arg(&measurements_root)
        .args(["--class", "recon"]);

    let mut child = cmd.spawn().expect("nrmeasure spawns");

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !sentinel.exists() {
        assert!(
            std::time::Instant::now() < deadline,
            "cyclictest never started: no sentinel at {} after 5s",
            sentinel.display()
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    assert!(
        child.try_wait().expect("try_wait must not error").is_none(),
        "the fake tool must still be sleeping (and nrmeasure still running) the moment the \
         sentinel is observed"
    );

    let entries: Vec<_> = std::fs::read_dir(&measurements_root)
        .expect("read_dir")
        .filter_map(|entry| entry.ok())
        .collect();
    assert_eq!(
        entries.len(),
        1,
        "the run directory must already exist while the tool is still running"
    );
    let run_dir = entries[0].path();
    assert!(
        !run_dir.join("manifest.json").is_file(),
        "no manifest.json may exist while the tool is still running"
    );

    let attempt: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(run_dir.join("ATTEMPT.json"))
            .expect("ATTEMPT.json exists before the tool exits"),
    )
    .expect("ATTEMPT.json parses as JSON");
    assert_eq!(attempt["status"], "in-progress");

    let status = child.wait().expect("nrmeasure exits");
    assert!(status.success(), "the run should finish successfully");
}

/// A fake cyclictest that writes an unparseable `.hist` leaves the run directory on
/// disk containing that `.hist`, the tool's stderr, and an `ATTEMPT.json` with
/// `status: failed`, `failure.stage: "parse"`, and `usable_for_numerical_analysis:
/// false`.
#[test]
fn failed_parse_preserves_raw_output() {
    let temp = tempfile::tempdir().expect("tempdir");
    let facts_path = write_fixture(temp.path(), "facts.txt", &tuned_facts_text());
    let interrupts_path = write_fixture(temp.path(), "interrupts.txt", INTERRUPTS);
    let measurements_root = temp.path().join("measurements");
    std::fs::create_dir_all(&measurements_root).expect("mkdir measurements root");

    let output = base_run_command(&measurements_root)
        .env("NRMEASURE_FACTS_FIXTURE", &facts_path)
        .env("NRMEASURE_INTERRUPTS_FIXTURE", &interrupts_path)
        .env("FAKE_CYCLICTEST_BAD_HIST", "1")
        .env(
            "FAKE_CYCLICTEST_STDERR",
            "cyclictest: a fabricated warning for the test",
        )
        .args(["--class", "recon"])
        .output()
        .expect("nrmeasure runs");

    assert!(
        !output.status.success(),
        "a parse failure must not exit 0: stdout={}\nstderr={}",
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
        "the run directory must survive the parse failure"
    );
    let run_dir = entries[0].path();

    assert!(
        run_dir.join("cyclictest.hist").is_file(),
        "the raw (unparseable) capture must survive"
    );
    assert!(
        run_dir.join("cyclictest.stderr.txt").is_file(),
        "cyclictest's stderr must be preserved"
    );
    let stderr_text =
        std::fs::read_to_string(run_dir.join("cyclictest.stderr.txt")).expect("read stderr");
    assert!(stderr_text.contains("a fabricated warning for the test"));

    let attempt: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(run_dir.join("ATTEMPT.json")).expect("read ATTEMPT.json"),
    )
    .expect("ATTEMPT.json parses");
    assert_eq!(attempt["status"], "failed");
    assert_eq!(attempt["failure"]["stage"], "parse");
    assert_eq!(attempt["usable_for_numerical_analysis"], false);
}

/// The same failed-parse directory contains no `manifest.json`, so a failed
/// attempt can never be mistaken for a measurement.
#[test]
fn failed_attempt_has_no_manifest() {
    let temp = tempfile::tempdir().expect("tempdir");
    let facts_path = write_fixture(temp.path(), "facts.txt", &tuned_facts_text());
    let interrupts_path = write_fixture(temp.path(), "interrupts.txt", INTERRUPTS);
    let measurements_root = temp.path().join("measurements");
    std::fs::create_dir_all(&measurements_root).expect("mkdir measurements root");

    let output = base_run_command(&measurements_root)
        .env("NRMEASURE_FACTS_FIXTURE", &facts_path)
        .env("NRMEASURE_INTERRUPTS_FIXTURE", &interrupts_path)
        .env("FAKE_CYCLICTEST_BAD_HIST", "1")
        .args(["--class", "recon"])
        .output()
        .expect("nrmeasure runs");
    assert!(!output.status.success());

    let entries: Vec<_> = std::fs::read_dir(&measurements_root)
        .expect("read_dir")
        .filter_map(|entry| entry.ok())
        .collect();
    assert_eq!(entries.len(), 1);
    let run_dir = entries[0].path();
    assert!(
        !run_dir.join("manifest.json").exists(),
        "a failed attempt must never carry a manifest.json"
    );
}

/// A normal, successful run leaves `ATTEMPT.json` with `status: completed` and
/// `usable_for_numerical_analysis: true` beside `manifest.json`.
#[test]
fn successful_run_marks_the_attempt_completed() {
    let temp = tempfile::tempdir().expect("tempdir");
    let run_dir = run_full_pipeline(temp.path());

    assert!(run_dir.join("manifest.json").is_file());
    let attempt: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(run_dir.join("ATTEMPT.json")).expect("read ATTEMPT.json"),
    )
    .expect("ATTEMPT.json parses");
    assert_eq!(attempt["status"], "completed");
    assert_eq!(attempt["usable_for_numerical_analysis"], true);
    assert!(
        attempt["failure"].is_null(),
        "a completed attempt must carry no failure: {:?}",
        attempt["failure"]
    );
}

/// Every file named in a failed attempt's `ATTEMPT.json` `preserved` array exists
/// and its recorded blake3 matches the file on disk.
#[test]
fn preserved_artifacts_carry_checksums() {
    let temp = tempfile::tempdir().expect("tempdir");
    let facts_path = write_fixture(temp.path(), "facts.txt", &tuned_facts_text());
    let interrupts_path = write_fixture(temp.path(), "interrupts.txt", INTERRUPTS);
    let measurements_root = temp.path().join("measurements");
    std::fs::create_dir_all(&measurements_root).expect("mkdir measurements root");

    let output = base_run_command(&measurements_root)
        .env("NRMEASURE_FACTS_FIXTURE", &facts_path)
        .env("NRMEASURE_INTERRUPTS_FIXTURE", &interrupts_path)
        .env("FAKE_CYCLICTEST_BAD_HIST", "1")
        .env(
            "FAKE_CYCLICTEST_STDERR",
            "cyclictest: another fabricated warning",
        )
        .args(["--class", "recon"])
        .output()
        .expect("nrmeasure runs");
    assert!(!output.status.success());

    let entries: Vec<_> = std::fs::read_dir(&measurements_root)
        .expect("read_dir")
        .filter_map(|entry| entry.ok())
        .collect();
    let run_dir = entries[0].path();
    let attempt: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(run_dir.join("ATTEMPT.json")).expect("read ATTEMPT.json"),
    )
    .expect("ATTEMPT.json parses");

    let preserved = attempt["preserved"]
        .as_array()
        .expect("preserved is a JSON array");
    assert!(
        !preserved.is_empty(),
        "a failed attempt must preserve at least the raw capture"
    );
    for entry in preserved {
        let path = entry["path"].as_str().expect("path is a string");
        let recorded_blake3 = entry["blake3"].as_str().expect("blake3 is a string");
        let file_path = run_dir.join(path);
        assert!(
            file_path.is_file(),
            "{path} named in preserved must exist on disk"
        );
        let actual =
            nr_manifest::blake3_file(&file_path).expect("hash the file named in preserved");
        assert_eq!(
            &actual, recorded_blake3,
            "{path}'s recorded blake3 must match the file on disk"
        );
    }
}

// ---------------------------------------------------------------------------------
// Interference counters bracket one instrument each, measured on a monotonic clock
// (finding 7 of 01-EXTERNAL-AUDIT.md, second half). The counters used to be sampled
// once before cyclictest and once after everything (including a second
// instrument), while the per-run-hour normalisation divided by the requested
// cyclictest --duration alone.
// ---------------------------------------------------------------------------------

/// A run with both cyclictest and hwlatdetect records two `InstrumentWindow`
/// entries, each with its own before, after and measured elapsed time.
#[test]
fn interference_windows_are_per_instrument() {
    let temp = tempfile::tempdir().expect("tempdir");
    let facts_path = write_fixture(temp.path(), "facts.txt", &tuned_facts_text());
    let interrupts_path = write_fixture(temp.path(), "interrupts.txt", INTERRUPTS);
    let measurements_root = temp.path().join("measurements");
    std::fs::create_dir_all(&measurements_root).expect("mkdir measurements root");

    let output = base_run_command(&measurements_root)
        .env("NRMEASURE_FACTS_FIXTURE", &facts_path)
        .env("NRMEASURE_INTERRUPTS_FIXTURE", &interrupts_path)
        .args(["--class", "recon"])
        .arg("--with-hwlatdetect")
        .args(["--hwlatdetect-duration", "1"])
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
    let manifest: RunManifest = serde_json::from_str(
        &std::fs::read_to_string(entries[0].path().join("manifest.json"))
            .expect("read manifest.json"),
    )
    .expect("manifest.json parses");

    let windows = &manifest.interference.windows;
    assert_eq!(
        windows.len(),
        2,
        "cyclictest and hwlatdetect must each get their own window: {windows:?}"
    );
    assert_eq!(windows[0].instrument, "cyclictest");
    assert_eq!(windows[1].instrument, "hwlatdetect");
    assert!(windows[0].elapsed_seconds >= 0.0);
    assert!(windows[1].elapsed_seconds >= 0.0);
    assert_eq!(windows[0].requested_seconds, Some(1));
    assert_eq!(windows[1].requested_seconds, Some(1));
}

/// The contamination verdict's denominator equals the cyclictest window's
/// measured `elapsed_seconds`, not `--duration`. The fake cyclictest here always
/// returns the same real capture (888 overflow samples) almost instantly, so a
/// verdict still normalising by `--duration` (1 second, per `base_run_command`)
/// would compute an overflow rate near 888/s; normalising by the truly measured,
/// near-zero elapsed time produces a rate many times larger.
#[test]
fn verdict_normalises_by_measured_cyclictest_elapsed() {
    let temp = tempfile::tempdir().expect("tempdir");
    let run_dir = run_full_pipeline(temp.path());
    let manifest: RunManifest = serde_json::from_str(
        &std::fs::read_to_string(run_dir.join("manifest.json")).expect("read manifest.json"),
    )
    .expect("manifest.json parses");

    let tail = manifest
        .interference
        .tail_metrics
        .as_ref()
        .expect("tail metrics are always populated for a harness-generated run");
    let cyclictest_elapsed = manifest.interference.windows[0].elapsed_seconds;
    assert!(
        cyclictest_elapsed > 0.0,
        "a real subprocess must take a measurable, nonzero amount of time"
    );

    let expected_rate = 888.0 / cyclictest_elapsed;
    assert!(
        (tail.overflow_rate_per_s - expected_rate).abs() < expected_rate.max(1.0) * 0.05,
        "overflow_rate_per_s {} does not match overflow_count / windows[0].elapsed_seconds \
         ({expected_rate}); the verdict must normalise by the cyclictest window's measured \
         elapsed time",
        tail.overflow_rate_per_s
    );
    assert!(
        (tail.overflow_rate_per_s - 888.0).abs() > 1.0,
        "overflow_rate_per_s must not equal overflow_count / --duration (the old, wrong \
         denominator this plan replaces): got {}",
        tail.overflow_rate_per_s
    );
}

/// The sum of the windows' `elapsed_seconds` is strictly less than the whole-run
/// wall clock, proving parsing, reconciliation, and the environment snapshot all
/// fall outside every measurement window.
#[test]
fn parsing_is_outside_every_measurement_window() {
    let temp = tempfile::tempdir().expect("tempdir");
    let run_dir = run_full_pipeline(temp.path());
    let manifest: RunManifest = serde_json::from_str(
        &std::fs::read_to_string(run_dir.join("manifest.json")).expect("read manifest.json"),
    )
    .expect("manifest.json parses");

    assert!(!manifest.interference.windows.is_empty());
    let total_window_seconds: f64 = manifest
        .interference
        .windows
        .iter()
        .map(|w| w.elapsed_seconds)
        .sum();
    let whole_run_seconds = (manifest.utc_end - manifest.utc_start).as_seconds_f64();

    assert!(
        total_window_seconds < whole_run_seconds,
        "the sum of the windows' elapsed_seconds ({total_window_seconds}) must be strictly less \
         than the whole-run wall clock ({whole_run_seconds}s), proving parsing and snapshotting \
         fall outside every measurement window"
    );
}

// ---------------------------------------------------------------------------------
// rtla hwnoise: the D-27 replacement for hwlatdetect on the isolated cores. One osnoise
// sampling thread per requested CPU, rather than hwlatdetect's single non-migrating thread
// that isolcpus keeps off cpus 6-11 entirely. Plan 01-21.
// ---------------------------------------------------------------------------------

/// Runs the pipeline with `--with-hwnoise` against the committed 2026-09-05 probe (served by
/// `fake-rtla.sh`) and returns the run directory and parsed manifest.
fn run_hwnoise_pipeline(temp_root: &Path) -> (PathBuf, RunManifest) {
    let facts_path = write_fixture(temp_root, "facts.txt", &tuned_facts_text());
    let interrupts_path = write_fixture(temp_root, "interrupts.txt", INTERRUPTS);
    let measurements_root = temp_root.join("measurements");
    std::fs::create_dir_all(&measurements_root).expect("mkdir measurements root");

    let output = base_run_command(&measurements_root)
        .env("NRMEASURE_FACTS_FIXTURE", &facts_path)
        .env("NRMEASURE_INTERRUPTS_FIXTURE", &interrupts_path)
        .args(["--class", "recon"])
        .arg("--with-hwnoise")
        .args(["--hwnoise-cpus", "6-11"])
        .args(["--hwnoise-duration", "60"])
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
    let run_dir = entries[0].path();
    let manifest: RunManifest = serde_json::from_str(
        &std::fs::read_to_string(run_dir.join("manifest.json")).expect("read manifest.json"),
    )
    .expect("manifest.json parses");
    (run_dir, manifest)
}

/// `--dry-run` prints the exact declared invocation, built by the same function the real
/// execution path calls, so the two can never drift apart.
#[test]
fn hwnoise_argv_matches_the_declared_invocation() {
    let temp = tempfile::tempdir().expect("tempdir");
    let facts_path = write_fixture(temp.path(), "facts.txt", &tuned_facts_text());
    let measurements_root = temp.path().join("measurements");
    std::fs::create_dir_all(&measurements_root).expect("mkdir measurements root");

    let output = base_run_command(&measurements_root)
        .env("NRMEASURE_FACTS_FIXTURE", &facts_path)
        .args(["--class", "recon"])
        .arg("--with-hwnoise")
        .args(["--hwnoise-cpus", "6-11"])
        .args(["--hwnoise-housekeeping", "0-5"])
        .args(["--hwnoise-duration", "900"])
        .arg("--dry-run")
        .output()
        .expect("nrmeasure runs");

    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("would run: rtla hwnoise -c 6-11 -H 0-5 -P f:99 -d 900s"),
        "expected the declared invocation in dry-run output: {stdout}"
    );
}

/// The run directory contains `rtla-hwnoise.txt`, listed in the manifest artifacts with kind
/// `rtla-hwnoise` and a matching blake3.
#[test]
fn hwnoise_capture_is_placed_and_checksummed() {
    let temp = tempfile::tempdir().expect("tempdir");
    let (run_dir, manifest) = run_hwnoise_pipeline(temp.path());

    let capture_path = run_dir.join("rtla-hwnoise.txt");
    assert!(capture_path.is_file(), "rtla-hwnoise.txt must be written");

    let artifact = manifest
        .artifacts
        .iter()
        .find(|a| a.path == "rtla-hwnoise.txt")
        .expect("rtla-hwnoise.txt must be a recorded artifact");
    assert_eq!(artifact.kind, ArtifactKind::RtlaHwnoise);

    let expected_blake3 = nr_manifest::blake3_file(&capture_path).expect("hash the capture");
    assert_eq!(artifact.blake3, expected_blake3);
}

/// The manifest's `firmware_screens[0]` names the requested CPU list and the CPUs that
/// actually produced rows; the committed probe covers every one of cpus 6-11.
#[test]
fn hwnoise_screen_records_requested_and_observed_cpus() {
    let temp = tempfile::tempdir().expect("tempdir");
    let (_run_dir, manifest) = run_hwnoise_pipeline(temp.path());

    assert_eq!(
        manifest.firmware_screens.len(),
        1,
        "exactly one firmware screen ran: {:?}",
        manifest.firmware_screens
    );
    let screen = &manifest.firmware_screens[0];
    assert_eq!(screen.instrument, "rtla-hwnoise");
    assert_eq!(screen.requested_cpus, vec![6, 7, 8, 9, 10, 11]);
    assert_eq!(screen.observed_cpus, vec![6, 7, 8, 9, 10, 11]);
    assert_eq!(
        screen.max_us,
        Some(1),
        "the probe's final Max Single is 1us on every cpu"
    );
    assert!(
        !screen.per_cpu_exposure_seconds.is_empty(),
        "runtime-derived exposure must be populated from the probe's Runtime column"
    );
}

/// Passing both `--with-hwnoise` and `--with-hwlatdetect` exits non-zero with a message
/// naming both flags, and writes nothing.
#[test]
fn hwnoise_and_hwlatdetect_are_mutually_exclusive() {
    let temp = tempfile::tempdir().expect("tempdir");
    let measurements_root = temp.path().join("measurements");
    std::fs::create_dir_all(&measurements_root).expect("mkdir measurements root");

    let output = base_run_command(&measurements_root)
        .args(["--class", "recon"])
        .arg("--with-hwnoise")
        .arg("--with-hwlatdetect")
        .output()
        .expect("nrmeasure runs");

    assert!(!output.status.success(), "the combination must be refused");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--with-hwnoise") && stderr.contains("--with-hwlatdetect"),
        "stderr should name both flags: {stderr}"
    );
    assert_eq!(
        std::fs::read_dir(&measurements_root)
            .expect("read_dir")
            .count(),
        0,
        "no run directory may be written when the flags conflict"
    );
}

/// The manifest carries an `InstrumentWindow` whose `instrument` is `rtla-hwnoise`,
/// bracketed separately from cyclictest's own window.
#[test]
fn hwnoise_gets_its_own_interference_window() {
    let temp = tempfile::tempdir().expect("tempdir");
    let (_run_dir, manifest) = run_hwnoise_pipeline(temp.path());

    let windows = &manifest.interference.windows;
    assert_eq!(
        windows.len(),
        2,
        "cyclictest and rtla hwnoise must each get their own window: {windows:?}"
    );
    assert_eq!(windows[0].instrument, "cyclictest");
    assert_eq!(windows[1].instrument, "rtla-hwnoise");
}

/// A requested CPU absent from the observed list (sampled but silent, or never sampled at
/// all) prints a warning naming it on stderr, without failing the run.
#[test]
fn hwnoise_warns_on_uncovered_requested_cpus() {
    let temp = tempfile::tempdir().expect("tempdir");
    let facts_path = write_fixture(temp.path(), "facts.txt", &tuned_facts_text());
    let interrupts_path = write_fixture(temp.path(), "interrupts.txt", INTERRUPTS);
    let measurements_root = temp.path().join("measurements");
    std::fs::create_dir_all(&measurements_root).expect("mkdir measurements root");

    // The committed probe only ever names cpus 6-11; requesting 12 alongside them asks for
    // one this fixture cannot cover.
    let output = base_run_command(&measurements_root)
        .env("NRMEASURE_FACTS_FIXTURE", &facts_path)
        .env("NRMEASURE_INTERRUPTS_FIXTURE", &interrupts_path)
        .args(["--class", "recon"])
        .arg("--with-hwnoise")
        .args(["--hwnoise-cpus", "6-12"])
        .args(["--hwnoise-duration", "60"])
        .output()
        .expect("nrmeasure runs");

    assert!(
        output.status.success(),
        "an uncovered cpu is a warning, not a failure: stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("12"),
        "stderr should name the uncovered cpu 12: {stderr}"
    );
}

/// A `recon`-class run driven by `NRMEASURE_SMI_FIXTURE` (alongside the facts and
/// interrupts fixtures every fixture-driven test in this suite already needs, since
/// neither has a live path on macOS). Class `recon` rather than `headline`/`weekly`/
/// `soak`: those three classes already refuse the facts and interrupts fixtures
/// outright (finding 8, `01-EXTERNAL-AUDIT.md`), independent of what this test is
/// actually exercising.
fn run_smi_fixture_pipeline(temp_root: &Path) -> RunManifest {
    let facts_path = write_fixture(temp_root, "facts.txt", &tuned_facts_text());
    let interrupts_path = write_fixture(temp_root, "interrupts.txt", INTERRUPTS);
    let smi_path = write_fixture(temp_root, "smi.txt", SMI_COUNTS);
    let measurements_root = temp_root.join("measurements");
    std::fs::create_dir_all(&measurements_root).expect("mkdir measurements root");

    let output = base_run_command(&measurements_root)
        .env("NRMEASURE_FACTS_FIXTURE", &facts_path)
        .env("NRMEASURE_INTERRUPTS_FIXTURE", &interrupts_path)
        .env("NRMEASURE_SMI_FIXTURE", &smi_path)
        .args(["--class", "recon"])
        .output()
        .expect("nrmeasure runs");

    assert!(
        output.status.success(),
        "a run driven by the SMI fixture must still succeed: stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let entries: Vec<_> = std::fs::read_dir(&measurements_root)
        .expect("read_dir")
        .filter_map(|entry| entry.ok())
        .collect();
    assert_eq!(entries.len(), 1, "exactly one run directory is written");

    let manifest_text = std::fs::read_to_string(entries[0].path().join("manifest.json"))
        .expect("read manifest.json");
    serde_json::from_str(&manifest_text).expect("manifest.json parses")
}

/// D-27: `MSR_SMI_COUNT` brackets every run, `before` and `after`, for every isolated
/// CPU, with a `delta` genuinely computed as their difference. The difference is zero
/// here because a fixture-driven read sees the same text for both snapshots, exactly
/// like every other fixture seam in this suite; that is the real, correct answer for
/// two identical readings, not a fabricated one.
#[test]
fn smi_counts_bracket_the_run() {
    let temp = tempfile::tempdir().expect("tempdir");
    let manifest = run_smi_fixture_pipeline(temp.path());

    let smi = manifest
        .smi_counts
        .as_ref()
        .expect("a run must always carry smi_counts, fixture-driven or not");
    assert_eq!(smi.register, "0x34");
    assert!(
        smi.unavailable_reason.is_none(),
        "every requested cpu is covered by the fixture: {:?}",
        smi.unavailable_reason
    );

    for cpu in 6..=11u32 {
        let before = smi.before.iter().find(|c| c.cpu == cpu).unwrap_or_else(|| {
            panic!("cpu {cpu} missing from smi_counts.before: {:?}", smi.before)
        });
        let after =
            smi.after.iter().find(|c| c.cpu == cpu).unwrap_or_else(|| {
                panic!("cpu {cpu} missing from smi_counts.after: {:?}", smi.after)
            });
        let delta =
            smi.delta.iter().find(|c| c.cpu == cpu).unwrap_or_else(|| {
                panic!("cpu {cpu} missing from smi_counts.delta: {:?}", smi.delta)
            });

        assert_eq!(
            before.count, 4006,
            "cpu {cpu}: the fixture's 0xfa6 reading is 4006 decimal"
        );
        assert_eq!(
            after.count, before.count,
            "cpu {cpu}: one fixture text is read once and reused for both snapshots"
        );
        assert_eq!(
            delta.count,
            after.count.saturating_sub(before.count),
            "cpu {cpu}: delta must be the real after-minus-before difference"
        );
    }
}

/// A run driven by `NRMEASURE_SMI_FIXTURE` records the seam in `fixtures_used` and is
/// forced `excluded_from_series`, the same rule plan 01-18 applied to the facts and
/// interrupts fixture seams (finding 8, `01-EXTERNAL-AUDIT.md`): fixture-driven data
/// must never reach the series as if it were a real measurement.
#[test]
fn smi_fixture_forces_exclusion() {
    let temp = tempfile::tempdir().expect("tempdir");
    let manifest = run_smi_fixture_pipeline(temp.path());

    assert!(
        manifest
            .fixtures_used
            .iter()
            .any(|f| f == "NRMEASURE_SMI_FIXTURE"),
        "fixtures_used should name NRMEASURE_SMI_FIXTURE: {:?}",
        manifest.fixtures_used
    );
    assert!(
        manifest.excluded_from_series,
        "a run driven by the SMI fixture seam must be excluded_from_series"
    );
    let reason = manifest
        .exclusion_reason
        .as_ref()
        .expect("exclusion_reason must be present");
    assert!(
        reason.contains("NRMEASURE_SMI_FIXTURE"),
        "exclusion_reason should name the SMI fixture: {reason}"
    );
}
