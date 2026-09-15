//! Task 3 of plan 02-07: the end-to-end dry run, driving the compiled `nr-stop-harness` binary
//! through `NR_STOP_FACTS_FIXTURE` exactly the way plan 02-08's two rig invocations will, minus
//! the rig. This is the test that would have caught every integration mistake in tasks 1 and 2
//! (it did: see `crates/stop-harness/src/characterise.rs::pin_for_characterisation`, added while
//! writing this file because `cross_core_offset_ns`'s unconditional pin made `characterise` fail
//! on every macOS host, fixture or not, with no way for this test to ever pass), and it is the
//! reason the macOS leg of CI is worth having.
//!
//! Host-matrix discipline (`.planning/STATE.md`, 2026-09-07 incident): `ci.yml` runs this test
//! matrix on both `ubuntu-latest` and `macos-latest`. Every assertion below is on structure, on
//! recomputed checksums, and on the presence of required phrases, never on a wall-clock-derived
//! number, a thread-scheduling-order artifact, or a whole-file snapshot.

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use nr_manifest::RunManifest;

/// The same real, committed rig capture `crates/stop-harness/tests/capture.rs` uses, and the same
/// derivation from it. Duplicated rather than shared through a `tests/common` module: this is
/// only the second use, and the project's own rule is to copy twice before abstracting (see
/// `crates/stop-harness/src/rundir.rs`'s module doc for the precedent this follows).
const RIG_AS_FOUND: &str = include_str!("../../capture/tests/fixtures/probe-sysfs-tuning.txt");

/// `0`/`1`, not the harness's real `7`/`8` CLI defaults: `FixtureFacts`'s cpuidle lookup only
/// ever serves `cpu0`'s own sysfs path (`crates/capture/src/sources.rs::lookup`), so a per-cpu
/// idle-state check against any other single cpu reads `Unavailable` under a fixture regardless
/// of what the fixture text says. Test construction only; the harness's own real defaults are
/// unchanged.
const HOT_CPU: &str = "0";
const ABORT_CPU: &str = "1";

fn fully_passing_facts_text() -> String {
    let governor_and_target = RIG_AS_FOUND
        .replace(".governor=powersave", ".governor=performance")
        .replace(
            "intel_pstate.no_turbo=unavailable",
            "intel_pstate.no_turbo=1",
        )
        .replace(
            "systemd.default_target=graphical.target",
            "systemd.default_target=multi-user.target",
        );
    let with_thermal_margin: String = governor_and_target
        .lines()
        .map(|line| match line.split_once('=') {
            Some((key, _)) if key.starts_with("thermal.") => format!("{key}=40000"),
            _ => line.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "{with_thermal_margin}\n\
         ssh.active_sessions=0\n\
         service.gdm.service.ActiveState=inactive\n\
         graphical.sessions=0\n\
         login.local_sessions=\n\
         cpu.isolated=0-1\n\
         service.rt-tuning.service.ActiveState=active\n"
    )
}

fn nr_stop_harness() -> Command {
    Command::cargo_bin("nr-stop-harness").expect("nr-stop-harness binary is built")
}

/// The only immediate subdirectory of `root` other than `exclude`. Naming-convention-agnostic on
/// purpose: a directory name embeds the run's own UTC start date, which this test must not
/// otherwise depend on.
fn only_other_subdirectory(root: &Path, exclude: &Path) -> PathBuf {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(root)
        .expect("read_dir measurements_root")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir() && path != exclude)
        .collect();
    assert_eq!(
        dirs.len(),
        1,
        "expected exactly one other run directory under {}, found {dirs:?}",
        root.display()
    );
    dirs.remove(0)
}

fn only_subdirectory(root: &Path) -> PathBuf {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(root)
        .expect("read_dir measurements_root")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    assert_eq!(
        dirs.len(),
        1,
        "expected exactly one run directory under {}, found {dirs:?}",
        root.display()
    );
    dirs.remove(0)
}

/// Everything the three behaviours in this file need, built once: both of plan 02-08's rig
/// invocations, chained exactly as it will chain them (the abort-latency run's
/// `--characterisation-tsv` points at the characterise run's own output), entirely under
/// `NR_STOP_FACTS_FIXTURE`. Built once and shared across assertions, matching
/// `crates/stop-harness/tests/capture.rs::build_completed_run`'s own reason for doing so: this
/// spawns two real subprocesses, and the point under test is the same completed directory for all
/// three behaviours, not three independently synthesised ones.
struct DryRun {
    _tmp: tempfile::TempDir,
    run_dir: PathBuf,
    manifest: RunManifest,
}

fn produced_abort_latency_run() -> DryRun {
    let tmp = tempfile::tempdir().expect("tempdir");
    let fixture_path = tmp.path().join("fixture-facts.txt");
    std::fs::write(&fixture_path, fully_passing_facts_text()).expect("write fixture facts");
    let measurements_root = tmp.path().join("measurements");

    // --sys-root exists precisely so a test can supply a clocksource without a real /sys
    // (main.rs's own doc comment on the flag). Populated so this run proves every D-35 figure,
    // not just the two that need no filesystem stand-in.
    let sys_root = tmp.path().join("sys-root");
    let clocksource_dir = sys_root.join("devices/system/clocksource/clocksource0");
    std::fs::create_dir_all(&clocksource_dir).expect("mkdir fake sys_root");
    std::fs::write(clocksource_dir.join("current_clocksource"), "tsc\n")
        .expect("write fake clocksource");

    let characterise_output = nr_stop_harness()
        .env("NR_STOP_FACTS_FIXTURE", &fixture_path)
        .args([
            "characterise",
            "--cpu-a",
            HOT_CPU,
            "--cpu-b",
            ABORT_CPU,
            "--iterations",
            "1000",
            "--rounds",
            "50",
            "--rig-slug",
            "precision3591",
        ])
        .arg("--sys-root")
        .arg(&sys_root)
        .arg("--measurements-root")
        .arg(&measurements_root)
        .output()
        .expect("spawn nr-stop-harness characterise");
    assert!(
        characterise_output.status.success(),
        "a fixture-backed characterise run must succeed on every CI host: stdout={}\nstderr={}",
        String::from_utf8_lossy(&characterise_output.stdout),
        String::from_utf8_lossy(&characterise_output.stderr)
    );
    let characterise_dir = only_subdirectory(&measurements_root);
    let characterisation_tsv = characterise_dir.join("clock-characterisation.tsv");
    assert!(
        characterisation_tsv.is_file(),
        "characterise must leave its own clock-characterisation.tsv"
    );

    let abort_output = nr_stop_harness()
        .env("NR_STOP_FACTS_FIXTURE", &fixture_path)
        .args([
            "abort-latency",
            "--period-ns",
            "1000000",
            "--trials",
            "200",
            "--hot-cpu",
            HOT_CPU,
            "--abort-cpu",
            ABORT_CPU,
            "--rig-slug",
            "precision3591",
        ])
        .arg("--measurements-root")
        .arg(&measurements_root)
        .arg("--characterisation-tsv")
        .arg(&characterisation_tsv)
        .output()
        .expect("spawn nr-stop-harness abort-latency");
    assert!(
        abort_output.status.success(),
        "a fixture-backed abort-latency run must succeed on every CI host: stdout={}\nstderr={}",
        String::from_utf8_lossy(&abort_output.stdout),
        String::from_utf8_lossy(&abort_output.stderr)
    );

    let run_dir = only_other_subdirectory(&measurements_root, &characterise_dir);
    let manifest_text =
        std::fs::read_to_string(run_dir.join("manifest.json")).expect("read manifest.json");
    let manifest: RunManifest = serde_json::from_str(&manifest_text).expect("parse manifest.json");

    DryRun {
        _tmp: tmp,
        run_dir,
        manifest,
    }
}

/// Behaviours: a_fixture_run_produces_a_complete_run_directory, the_fixture_run_is_marked_as_one,
/// and the_manifest_validates_and_the_checksums_match. One test, not three: all three examine the
/// same completed run directory, and splitting them would mean paying for two more subprocess
/// pairs to assert nothing new.
#[test]
fn a_fixture_run_produces_a_complete_directory_that_validates_and_is_marked_as_one() {
    let run = produced_abort_latency_run();

    // a_fixture_run_produces_a_complete_run_directory: the raw capture, the manifest, the
    // metrics entry and the report are all present.
    assert!(
        run.run_dir.join("abort-latency-1000000ns.tsv").is_file(),
        "the raw per-trial capture must exist"
    );
    assert!(
        run.run_dir.join("manifest.json").is_file(),
        "manifest.json must exist"
    );
    assert!(
        run.run_dir.join("metrics-entry.json").is_file(),
        "metrics-entry.json must exist"
    );
    assert!(
        run.run_dir.join("REPORT.md").is_file(),
        "REPORT.md must exist"
    );

    // the_fixture_run_is_marked_as_one: the manifest records the fixture seam by variable name,
    // so it can never be read as a rig capture.
    assert!(
        run.manifest
            .fixtures_used
            .iter()
            .any(|used| used == "NR_STOP_FACTS_FIXTURE"),
        "fixtures_used must name NR_STOP_FACTS_FIXTURE: {:?}",
        run.manifest.fixtures_used
    );

    // the_manifest_validates_and_the_checksums_match.
    nr_manifest::validate(&run.run_dir, &run.manifest)
        .map_err(|errors| format!("{errors:?}"))
        .expect("the manifest this harness produces must validate");
    for artifact in &run.manifest.artifacts {
        let path = run.run_dir.join(&artifact.path);
        let recomputed = nr_manifest::blake3_file(&path)
            .unwrap_or_else(|err| panic!("recompute blake3 for {}: {err}", path.display()));
        assert_eq!(
            artifact.blake3, recomputed,
            "{} checksum must recompute to the recorded value",
            artifact.path
        );
        let recomputed_bytes = std::fs::metadata(&path)
            .unwrap_or_else(|err| panic!("stat {}: {err}", path.display()))
            .len();
        assert_eq!(
            artifact.bytes, recomputed_bytes,
            "{} byte count must match the file on disk",
            artifact.path
        );
    }

    // Every file physically in the directory is either one of the three generated sidecars or
    // listed as an artifact: the same rule crates/stop-harness/tests/capture.rs's
    // a_completed_run_leaves_validating_evidence already checks at the library level, checked
    // here again through the real compiled binary.
    let known_non_capture = ["manifest.json", "metrics-entry.json", "REPORT.md"];
    for entry in std::fs::read_dir(&run.run_dir).expect("read_dir run_dir") {
        let entry = entry.expect("dir entry");
        let name = entry.file_name().to_string_lossy().into_owned();
        if known_non_capture.contains(&name.as_str()) {
            continue;
        }
        assert!(
            run.manifest.artifacts.iter().any(|a| a.path == name),
            "{name} exists on disk but is not listed in manifest.artifacts"
        );
    }

    // Structural report assertions only (host-matrix discipline): required phrases, not exact
    // wall-clock-derived numbers.
    let report = std::fs::read_to_string(run.run_dir.join("REPORT.md")).expect("read REPORT.md");
    assert!(report.contains("not combined"));
    assert!(report.contains("poll period"));
    assert!(report.contains("200"), "the trial count must appear");
    // A characterisation run was chained in via --characterisation-tsv, so both D-35 figures are
    // present rather than "unavailable"; the exact numbers are thread-scheduling-dependent and
    // deliberately not asserted.
    assert!(report.contains("cross-core propagation:"));
    assert!(report.contains("clock read overhead:"));
    assert!(
        !report.to_lowercase().contains("unavailable"),
        "a characterisation run was supplied; nothing should render as unavailable: {report}"
    );
}
