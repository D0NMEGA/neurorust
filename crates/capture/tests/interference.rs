//! Integration tests for the D-15 interference diff and contamination verdict,
//! driven by the real rig-captured `/proc/interrupts` and two derived after-run
//! variants (see `fixtures/README.md`), plus (D-24) the D-17 calibration pair's real
//! committed captures under `measurements/`.

use std::path::Path;
use std::time::Duration;

use nr_capture::interference::{self, Thresholds};
use nr_histogram::hist::{CyclictestRun, parse_hist, parse_hist_file};
use nr_manifest::{ContaminationVerdict, CpuCounter, InterferenceSnapshot, RunManifest};

const BEFORE: &str = include_str!("fixtures/probe-proc-interrupts.txt");
const AFTER_CLEAN: &str = include_str!("fixtures/proc-interrupts-after-clean.txt");
const AFTER_CONTAMINATED: &str = include_str!("fixtures/proc-interrupts-after-contaminated.txt");

const ISOLATED_CPUS: [u32; 6] = [6, 7, 8, 9, 10, 11];
const ONE_HOUR: Duration = Duration::from_secs(3600);

/// D-24: `verdict` now always computes tail metrics from a real histogram, even for a
/// test that is only exercising the interference-counter (`Calibrated`/`Uncalibrated`)
/// paths below. A minimal, valid, single-thread run is enough for those; the tests
/// that actually prove the D-24 metrics separate real data are further down this file.
fn sample_cyclictest_run() -> CyclictestRun {
    let hist = "# Histogram\n\
                000001 000001\n\
                000002 000002\n\
                # Min Latencies: 00001\n\
                # Avg Latencies: 00001\n\
                # Max Latencies: 00002\n\
                # Histogram Overflows: 00000\n\
                # Histogram Overflow at cycle number:\n\
                # Thread 0: \n";
    parse_hist(hist, Some(400)).expect("the minimal fixture parses")
}

fn calibrated_thresholds() -> Thresholds {
    let json = r#"{
        "schema_version": 1,
        "status": "calibrated",
        "calibration": {
            "derived_from": [
                "2026-09-01-precision3591-calibration-clean-001",
                "2026-09-01-precision3591-calibration-contaminated-001"
            ],
            "note": "test fixture, not a real calibration"
        },
        "per_run_hour": {
            "cal_delta_max": 5000.0,
            "tlb_delta_max": 5000.0,
            "res_delta_max": 5000.0,
            "device_irq_delta_max": 1000000.0,
            "context_switch_delta_max": 5000.0
        }
    }"#;
    Thresholds::parse(json).expect("valid calibrated thresholds")
}

fn find(counters: &[CpuCounter], cpu: u32) -> u64 {
    counters
        .iter()
        .find(|c| c.cpu == cpu)
        .unwrap_or_else(|| panic!("no counter for cpu{cpu}"))
        .count
}

#[test]
fn parses_real_proc_interrupts() {
    let snapshot =
        interference::snapshot_from_text(BEFORE, &ISOLATED_CPUS).expect("parses the real fixture");
    assert_eq!(snapshot.isolated_cpus, ISOLATED_CPUS.to_vec());

    // CAL and TLB, column index 6 (cpu6), read directly off the committed fixture.
    assert_eq!(find(&snapshot.cal_ipis, 6), 22789);
    assert_eq!(find(&snapshot.tlb_ipis, 6), 2);
    assert_eq!(find(&snapshot.cal_ipis, 11), 22782);
}

#[test]
fn delta_is_per_isolated_cpu() {
    let before = interference::snapshot_from_text(BEFORE, &ISOLATED_CPUS).expect("parses");
    let after = interference::snapshot_from_text(AFTER_CLEAN, &ISOLATED_CPUS).expect("parses");
    let outcome = interference::verdict(
        before,
        after,
        &sample_cyclictest_run(),
        &Thresholds::Uncalibrated,
        ONE_HOUR,
    )
    .expect("verdict computes");

    // The clean arm's per-cpu CAL bumps (42, 47, 39, 51, 44, 48) are all distinct;
    // reproducing them exactly proves the delta is computed per isolated CPU, not
    // summed or averaged machine wide.
    let expected = [(6, 42), (7, 47), (8, 39), (9, 51), (10, 44), (11, 48)];
    for (cpu, expected_delta) in expected {
        assert_eq!(
            find(&outcome.pair.delta.cal_ipis, cpu),
            expected_delta,
            "cpu{cpu} CAL delta"
        );
    }
}

/// The 2026-09-07 rescheduling-IPI rename (`01-REVIEW-2026-09-06.md` finding B4) carries a
/// `#[serde(alias = "context_switches")]`, not a schema-version bump, specifically so every
/// manifest committed under the old key keeps deserialising. This constructs the shape by hand
/// rather than loading a committed manifest file: the point is the field's own serde contract,
/// independent of whichever real manifests happen to predate or postdate the rename.
#[test]
fn a_committed_manifest_with_the_old_key_still_deserialises() {
    let json = r#"{
        "isolated_cpus": [6, 7],
        "cal_ipis": [{ "cpu": 6, "count": 1 }, { "cpu": 7, "count": 2 }],
        "tlb_ipis": [{ "cpu": 6, "count": 3 }, { "cpu": 7, "count": 4 }],
        "context_switches": [{ "cpu": 6, "count": 5 }, { "cpu": 7, "count": 6 }],
        "irqs": [{ "cpu": 6, "count": 7 }, { "cpu": 7, "count": 8 }]
    }"#;
    let snapshot: InterferenceSnapshot =
        serde_json::from_str(json).expect("a manifest using the old key still deserialises");
    assert_eq!(find(&snapshot.rescheduling_ipis, 6), 5);
    assert_eq!(find(&snapshot.rescheduling_ipis, 7), 6);
}

/// The other half of the same contract: a value built and serialised today emits the new key
/// and never the old one, so a fresh manifest is unambiguous about what the field holds.
#[test]
fn a_new_manifest_serialises_the_new_key() {
    let snapshot = InterferenceSnapshot {
        isolated_cpus: vec![6],
        cal_ipis: vec![],
        tlb_ipis: vec![],
        rescheduling_ipis: vec![CpuCounter { cpu: 6, count: 9 }],
        irqs: vec![],
    };
    let json = serde_json::to_string(&snapshot).expect("a snapshot always serialises");
    assert!(json.contains("\"rescheduling_ipis\""), "got: {json}");
    assert!(
        !json.contains("context_switches"),
        "a freshly serialised snapshot must never emit the retired key: {json}"
    );
}

#[test]
fn verdict_clean() {
    let before = interference::snapshot_from_text(BEFORE, &ISOLATED_CPUS).expect("parses");
    let after = interference::snapshot_from_text(AFTER_CLEAN, &ISOLATED_CPUS).expect("parses");
    let outcome = interference::verdict(
        before,
        after,
        &sample_cyclictest_run(),
        &calibrated_thresholds(),
        ONE_HOUR,
    )
    .expect("verdict computes");

    assert_eq!(outcome.pair.verdict, ContaminationVerdict::Clean);
    assert!(outcome.reason.is_none());
}

#[test]
fn verdict_contaminated() {
    let before = interference::snapshot_from_text(BEFORE, &ISOLATED_CPUS).expect("parses");
    let after =
        interference::snapshot_from_text(AFTER_CONTAMINATED, &ISOLATED_CPUS).expect("parses");
    let outcome = interference::verdict(
        before,
        after,
        &sample_cyclictest_run(),
        &calibrated_thresholds(),
        ONE_HOUR,
    )
    .expect("verdict computes");

    assert_eq!(outcome.pair.verdict, ContaminationVerdict::Contaminated);
    let reason = outcome
        .reason
        .expect("a Contaminated verdict carries a reason");
    assert!(reason.contains("CAL"), "reason should name CAL: {reason}");
    assert!(reason.contains("cpu"), "reason should name a cpu: {reason}");
}

#[test]
fn verdict_uncalibrated() {
    let before = interference::snapshot_from_text(BEFORE, &ISOLATED_CPUS).expect("parses");
    let after =
        interference::snapshot_from_text(AFTER_CONTAMINATED, &ISOLATED_CPUS).expect("parses");

    // Even a deeply contaminated delta never yields Clean or Contaminated against
    // the uncalibrated thresholds.
    let outcome = interference::verdict(
        before,
        after,
        &sample_cyclictest_run(),
        &Thresholds::Uncalibrated,
        ONE_HOUR,
    )
    .expect("verdict computes");
    assert_eq!(outcome.pair.verdict, ContaminationVerdict::Uncalibrated);
    assert_eq!(outcome.pair.thresholds_provisional, Some(false));
    assert!(outcome.reason.is_none());

    // The shipped config file is D-24 provisional (the D-17 calibration pair is n=2,
    // not a calibration set), never Uncalibrated and never plain Calibrated; see
    // `real_clean_arm_scores_clean_under_the_shipped_provisional_thresholds` and its
    // contaminated twin below for what it actually does with real data.
    let shipped = shipped_thresholds();
    assert!(matches!(shipped, Thresholds::Provisional(_)));
}

#[test]
fn thresholds_require_calibration_provenance() {
    let json = r#"{
        "schema_version": 1,
        "status": "calibrated",
        "calibration": { "derived_from": [], "note": "no runs named" },
        "per_run_hour": {
            "cal_delta_max": 5000.0,
            "tlb_delta_max": 5000.0,
            "res_delta_max": 5000.0,
            "device_irq_delta_max": 1000000.0,
            "context_switch_delta_max": 5000.0
        }
    }"#;
    let result = Thresholds::parse(json);
    assert!(
        result.is_err(),
        "a calibrated status with an empty derived_from must fail to load"
    );
}

#[test]
fn contaminated_run_is_not_dropped() {
    let before = interference::snapshot_from_text(BEFORE, &ISOLATED_CPUS).expect("parses");
    let after =
        interference::snapshot_from_text(AFTER_CONTAMINATED, &ISOLATED_CPUS).expect("parses");
    let outcome = interference::verdict(
        before,
        after,
        &sample_cyclictest_run(),
        &calibrated_thresholds(),
        ONE_HOUR,
    )
    .expect("verdict computes");

    // BENCH-06/D-15: a Contaminated verdict still carries a complete, usable
    // before/after/delta record. Dropping it is a decision this crate refuses to
    // make; excluding it from the headline series is the caller's job.
    assert_eq!(outcome.pair.verdict, ContaminationVerdict::Contaminated);
    assert_eq!(outcome.pair.before.isolated_cpus, ISOLATED_CPUS.to_vec());
    assert_eq!(outcome.pair.after.isolated_cpus, ISOLATED_CPUS.to_vec());
    assert_eq!(outcome.pair.delta.cal_ipis.len(), ISOLATED_CPUS.len());
    assert!(outcome.reason.is_some());
}

// ---------------------------------------------------------------------------------
// D-24: the tail-based metrics, proven against the two REAL committed D-17
// calibration captures under `measurements/`, not a synthetic fixture. The point is
// that this works on real data.
// ---------------------------------------------------------------------------------

const CLEAN_HIST: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../measurements/2026-09-01-precision3591-calibration-clean/cyclictest.hist"
);
const CLEAN_MANIFEST: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../measurements/2026-09-01-precision3591-calibration-clean/manifest.json"
);
const CONTAMINATED_HIST: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../measurements/2026-09-02-precision3591-calibration-contaminated/cyclictest.hist"
);
const CONTAMINATED_MANIFEST: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../measurements/2026-09-02-precision3591-calibration-contaminated/manifest.json"
);

/// Declared cyclictest duration for the D-17 clean arm: 3600s, its manifest's
/// `tools[0].argv` names `--duration=3600`. Used exactly as `nrmeasure run` would use
/// it.
const CLEAN_DURATION: Duration = Duration::from_secs(3600);
/// Declared cyclictest duration for the D-17 contaminated arm: 900s (`--duration=900`).
const CONTAMINATED_DURATION: Duration = Duration::from_secs(900);

fn load_real_run(hist_path: &str) -> CyclictestRun {
    // Some(400): the real captures were taken with cyclictest's `--histogram=400`;
    // declaring the same bound here reproduces the original run's overflow accounting
    // exactly, rather than deriving a bound from whatever the highest observed bin
    // happens to be.
    parse_hist_file(Path::new(hist_path), Some(400)).expect("a committed real .hist parses")
}

/// The `before`/`after` interference snapshots recorded in the real manifest at
/// capture time, reused here rather than duplicated by hand. `interference::verdict`
/// is called fresh below with these two plus the real histogram, so nothing about the
/// manifest's OWN (necessarily absent, D-24 postdates these captures) `tail_metrics`/
/// `thresholds_provisional` fields is ever read back; this only proves those two
/// fields are `Option` for exactly the right reason (a historical manifest still
/// deserializes).
fn load_real_snapshots(manifest_path: &str) -> (InterferenceSnapshot, InterferenceSnapshot) {
    let text =
        std::fs::read_to_string(manifest_path).expect("a committed real manifest.json reads");
    let manifest: RunManifest = serde_json::from_str(&text).expect(
        "a committed real manifest.json deserializes even though it predates D-24's tail_metrics/\
         thresholds_provisional fields",
    );
    assert!(
        manifest.interference.tail_metrics.is_none(),
        "this historical manifest predates D-24 and must not carry tail_metrics"
    );
    (manifest.interference.before, manifest.interference.after)
}

fn shipped_thresholds() -> Thresholds {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../config/contamination-thresholds.json"
    );
    Thresholds::load(Path::new(path)).expect("config/contamination-thresholds.json loads")
}

/// The three D-24 metrics, computed from the real clean arm's own histogram, match the
/// values this plan's own evidence table documents (tail excursion ratio 8.7,
/// thread-max spread 76.9%, zero overflow).
#[test]
fn tail_metrics_from_real_clean_capture_match_the_documented_values() {
    let run = load_real_run(CLEAN_HIST);
    let (before, after) = load_real_snapshots(CLEAN_MANIFEST);
    let outcome = interference::verdict(
        before,
        after,
        &run,
        &Thresholds::Uncalibrated,
        CLEAN_DURATION,
    )
    .expect("verdict computes on a real capture");
    let tail = outcome
        .pair
        .tail_metrics
        .expect("a freshly computed verdict always populates tail_metrics");

    assert!(
        (tail.tail_excursion_ratio - 8.6667).abs() < 0.01,
        "clean tail_excursion_ratio: {}",
        tail.tail_excursion_ratio
    );
    assert!(
        (tail.thread_max_spread - 0.7692).abs() < 0.001,
        "clean thread_max_spread: {}",
        tail.thread_max_spread
    );
    assert_eq!(
        tail.overflow_rate_per_s, 0.0,
        "the clean arm has zero overflow samples"
    );
}

/// The three D-24 metrics, computed from the real contaminated arm's own histogram,
/// match the values this plan's own evidence table documents (tail excursion ratio
/// 428.4, thread-max spread 3.6%, overflow rate 0.8822/s).
#[test]
fn tail_metrics_from_real_contaminated_capture_match_the_documented_values() {
    let run = load_real_run(CONTAMINATED_HIST);
    let (before, after) = load_real_snapshots(CONTAMINATED_MANIFEST);
    let outcome = interference::verdict(
        before,
        after,
        &run,
        &Thresholds::Uncalibrated,
        CONTAMINATED_DURATION,
    )
    .expect("verdict computes on a real capture");
    let tail = outcome
        .pair
        .tail_metrics
        .expect("a freshly computed verdict always populates tail_metrics");

    assert!(
        (tail.tail_excursion_ratio - 428.44).abs() < 0.1,
        "contaminated tail_excursion_ratio: {}",
        tail.tail_excursion_ratio
    );
    assert!(
        (tail.thread_max_spread - 0.0358).abs() < 0.001,
        "contaminated thread_max_spread: {}",
        tail.thread_max_spread
    );
    assert!(
        (tail.overflow_rate_per_s - 0.8822).abs() < 0.001,
        "contaminated overflow_rate_per_s: {}",
        tail.overflow_rate_per_s
    );
}

/// The whole point of this plan: the clean arm of the D-17 pair scores Clean under the
/// shipped, provisional D-24 thresholds.
#[test]
fn real_clean_arm_scores_clean_under_the_shipped_provisional_thresholds() {
    let run = load_real_run(CLEAN_HIST);
    let (before, after) = load_real_snapshots(CLEAN_MANIFEST);
    let thresholds = shipped_thresholds();
    assert!(
        matches!(thresholds, Thresholds::Provisional(_)),
        "config/contamination-thresholds.json must ship provisional, got {thresholds:?}"
    );

    let outcome = interference::verdict(before, after, &run, &thresholds, CLEAN_DURATION)
        .expect("verdict computes on a real capture");

    assert_eq!(outcome.pair.verdict, ContaminationVerdict::Clean);
    assert_eq!(outcome.pair.thresholds_provisional, Some(true));
    assert!(outcome.reason.is_none());
}

/// The other half of the point: the contaminated arm of the same pair scores
/// Contaminated under the same shipped thresholds, which is exactly what the current
/// (pre-D-24) interference-counter detector fails to do (it is blind to this run; see
/// the module documentation in `crates/capture/src/interference.rs`).
#[test]
fn real_contaminated_arm_scores_contaminated_under_the_shipped_provisional_thresholds() {
    let run = load_real_run(CONTAMINATED_HIST);
    let (before, after) = load_real_snapshots(CONTAMINATED_MANIFEST);
    let thresholds = shipped_thresholds();

    let outcome = interference::verdict(before, after, &run, &thresholds, CONTAMINATED_DURATION)
        .expect("verdict computes on a real capture");

    assert_eq!(outcome.pair.verdict, ContaminationVerdict::Contaminated);
    assert_eq!(outcome.pair.thresholds_provisional, Some(true));
    let reason = outcome
        .reason
        .expect("a Contaminated verdict carries a reason");
    assert!(
        reason.contains("excursion") && reason.contains("spread"),
        "reason should name both metrics: {reason}"
    );
}

/// Plan 01-11 task 3's named acceptance test: the D-17 calibration pair (the two real
/// captures `config/contamination-thresholds.json`'s `calibration.derived_from` names)
/// separates under the shipped thresholds. This ties together the two assertions
/// proven separately above (clean scores Clean, contaminated scores Contaminated) into
/// the one check the plan's own verification command names, so
/// `cargo test -p nr-capture calibration_pair_separates` selects and passes something
/// real rather than a substring match on nothing.
#[test]
fn calibration_pair_separates() {
    let thresholds = shipped_thresholds();
    assert!(
        matches!(thresholds, Thresholds::Provisional(_)),
        "config/contamination-thresholds.json must ship provisional, got {thresholds:?}"
    );

    let clean_run = load_real_run(CLEAN_HIST);
    let (clean_before, clean_after) = load_real_snapshots(CLEAN_MANIFEST);
    let clean = interference::verdict(
        clean_before,
        clean_after,
        &clean_run,
        &thresholds,
        CLEAN_DURATION,
    )
    .expect("verdict computes on the real clean arm");
    assert_eq!(
        clean.pair.verdict,
        ContaminationVerdict::Clean,
        "the clean arm of the D-17 pair must read Clean under the shipped thresholds"
    );

    let contaminated_run = load_real_run(CONTAMINATED_HIST);
    let (contaminated_before, contaminated_after) = load_real_snapshots(CONTAMINATED_MANIFEST);
    let contaminated = interference::verdict(
        contaminated_before,
        contaminated_after,
        &contaminated_run,
        &thresholds,
        CONTAMINATED_DURATION,
    )
    .expect("verdict computes on the real contaminated arm");
    assert_eq!(
        contaminated.pair.verdict,
        ContaminationVerdict::Contaminated,
        "the contaminated arm of the D-17 pair must read Contaminated under the shipped thresholds"
    );
}
