//! Integration tests for the D-15 interference diff and contamination verdict,
//! driven by the real rig-captured `/proc/interrupts` and two derived after-run
//! variants (see `fixtures/README.md`).

use std::time::Duration;

use nr_capture::interference::{self, Thresholds};
use nr_manifest::{ContaminationVerdict, CpuCounter};

const BEFORE: &str = include_str!("fixtures/probe-proc-interrupts.txt");
const AFTER_CLEAN: &str = include_str!("fixtures/proc-interrupts-after-clean.txt");
const AFTER_CONTAMINATED: &str = include_str!("fixtures/proc-interrupts-after-contaminated.txt");

const ISOLATED_CPUS: [u32; 6] = [6, 7, 8, 9, 10, 11];
const ONE_HOUR: Duration = Duration::from_secs(3600);

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
    let outcome = interference::verdict(before, after, &Thresholds::Uncalibrated, ONE_HOUR);

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

#[test]
fn verdict_clean() {
    let before = interference::snapshot_from_text(BEFORE, &ISOLATED_CPUS).expect("parses");
    let after = interference::snapshot_from_text(AFTER_CLEAN, &ISOLATED_CPUS).expect("parses");
    let outcome = interference::verdict(before, after, &calibrated_thresholds(), ONE_HOUR);

    assert_eq!(outcome.pair.verdict, ContaminationVerdict::Clean);
    assert!(outcome.reason.is_none());
}

#[test]
fn verdict_contaminated() {
    let before = interference::snapshot_from_text(BEFORE, &ISOLATED_CPUS).expect("parses");
    let after =
        interference::snapshot_from_text(AFTER_CONTAMINATED, &ISOLATED_CPUS).expect("parses");
    let outcome = interference::verdict(before, after, &calibrated_thresholds(), ONE_HOUR);

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
    let outcome = interference::verdict(before, after, &Thresholds::Uncalibrated, ONE_HOUR);
    assert_eq!(outcome.pair.verdict, ContaminationVerdict::Uncalibrated);
    assert!(outcome.reason.is_none());

    // The shipped config file is itself uncalibrated.
    let shipped_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../config/contamination-thresholds.json"
    );
    let shipped = Thresholds::load(std::path::Path::new(shipped_path))
        .expect("config/contamination-thresholds.json loads");
    assert!(matches!(shipped, Thresholds::Uncalibrated));
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
    let outcome = interference::verdict(before, after, &calibrated_thresholds(), ONE_HOUR);

    // BENCH-06/D-15: a Contaminated verdict still carries a complete, usable
    // before/after/delta record. Dropping it is a decision this crate refuses to
    // make; excluding it from the headline series is the caller's job.
    assert_eq!(outcome.pair.verdict, ContaminationVerdict::Contaminated);
    assert_eq!(outcome.pair.before.isolated_cpus, ISOLATED_CPUS.to_vec());
    assert_eq!(outcome.pair.after.isolated_cpus, ISOLATED_CPUS.to_vec());
    assert_eq!(outcome.pair.delta.cal_ipis.len(), ISOLATED_CPUS.len());
    assert!(outcome.reason.is_some());
}
