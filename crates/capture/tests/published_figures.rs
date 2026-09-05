//! Re-derives a figure quoted in prose from the committed artifact it is quoted
//! from, so prose and data cannot drift apart unnoticed.
//!
//! Finding 4 of `01-EXTERNAL-AUDIT.md`: an earlier version of `interference.rs`'s
//! module documentation (and `docs/measurement-protocol.md`'s "What the
//! contamination detector measures" section) claimed the D-17 contaminated
//! calibration arm recorded FEWER CAL/TLB/RES/device-IRQ counts than the clean
//! arm. That was backwards, and it could have been caught the day it was written:
//! the correct numbers were already sitting in the two committed calibration
//! manifests below. This test sums each counter family's recorded delta over the
//! isolated CPUs (6 to 11), normalises by the run's own wall-clock duration, and
//! asserts the eight published per-run-hour figures plus the two directional
//! claims the prose makes. See `.planning/phases/01-trustworthy-measurement/
//! 01-11-SUMMARY.md`, "Two recurring shapes worth a mechanical guard".
//!
//! The reference values (the auditor's own independent recomputation):
//!
//! | Counter    | Clean, 3600s | Contaminated, 900s | Clean/hour | Contaminated/hour |
//! |------------|-------------:|-------------------:|-----------:|------------------:|
//! | CAL        |            6 |                  12 |          6 |                 48 |
//! | TLB        |            6 |                   6 |          6 |                 24 |
//! | RES        |           55 |                  24 |         55 |                 96 |
//! | device IRQ |         1127 |                 211 |       1127 |                844 |

use nr_manifest::{CpuCounter, RunManifest};

const ISOLATED_CPUS: std::ops::RangeInclusive<u32> = 6..=11;

const CLEAN_MANIFEST: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../measurements/2026-09-01-precision3591-calibration-clean/manifest.json"
);
const CONTAMINATED_MANIFEST: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../measurements/2026-09-02-precision3591-calibration-contaminated/manifest.json"
);

fn load_manifest(path: &str) -> RunManifest {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read committed manifest {path}: {err}"));
    serde_json::from_str(&text)
        .unwrap_or_else(|err| panic!("committed manifest {path} failed to parse: {err}"))
}

fn sum_over_isolated_cpus(counters: &[CpuCounter]) -> u64 {
    counters
        .iter()
        .filter(|c| ISOLATED_CPUS.contains(&c.cpu))
        .map(|c| c.count)
        .sum()
}

/// The run's own recorded wall-clock duration, in hours. Plan 01-17 adds a
/// measured elapsed time to the manifest; until then, `utc_end - utc_start` is
/// the manifest's own denominator, the same one `nrmeasure run` itself computes
/// these two runs from.
fn duration_hours(manifest: &RunManifest) -> f64 {
    (manifest.utc_end - manifest.utc_start).as_seconds_f64() / 3600.0
}

fn per_hour(count: u64, hours: f64) -> f64 {
    count as f64 / hours
}

/// 1% tolerance: the denominator is wall-clock duration, which runs a few hundred
/// milliseconds past the requested cyclictest `--duration`, not exact equality on
/// the raw summed deltas above (which use `assert_eq!`).
fn assert_within_one_percent(actual: f64, expected: f64, label: &str) {
    let tolerance = expected.abs() * 0.01;
    assert!(
        (actual - expected).abs() <= tolerance,
        "{label}: expected {expected} within 1%, got {actual}"
    );
}

#[test]
fn quoted_per_run_hour_counters_match_the_committed_manifests() {
    let clean = load_manifest(CLEAN_MANIFEST);
    let contaminated = load_manifest(CONTAMINATED_MANIFEST);

    let clean_cal = sum_over_isolated_cpus(&clean.interference.delta.cal_ipis);
    let clean_tlb = sum_over_isolated_cpus(&clean.interference.delta.tlb_ipis);
    let clean_res = sum_over_isolated_cpus(&clean.interference.delta.context_switches);
    let clean_irq = sum_over_isolated_cpus(&clean.interference.delta.irqs);

    let contaminated_cal = sum_over_isolated_cpus(&contaminated.interference.delta.cal_ipis);
    let contaminated_tlb = sum_over_isolated_cpus(&contaminated.interference.delta.tlb_ipis);
    let contaminated_res =
        sum_over_isolated_cpus(&contaminated.interference.delta.context_switches);
    let contaminated_irq = sum_over_isolated_cpus(&contaminated.interference.delta.irqs);

    // Exact equality on the raw summed deltas.
    assert_eq!(clean_cal, 6, "clean CAL delta sum over cpu6-11");
    assert_eq!(clean_tlb, 6, "clean TLB delta sum over cpu6-11");
    assert_eq!(clean_res, 55, "clean RES delta sum over cpu6-11");
    assert_eq!(clean_irq, 1127, "clean device IRQ delta sum over cpu6-11");

    assert_eq!(
        contaminated_cal, 12,
        "contaminated CAL delta sum over cpu6-11"
    );
    assert_eq!(
        contaminated_tlb, 6,
        "contaminated TLB delta sum over cpu6-11"
    );
    assert_eq!(
        contaminated_res, 24,
        "contaminated RES delta sum over cpu6-11"
    );
    assert_eq!(
        contaminated_irq, 211,
        "contaminated device IRQ delta sum over cpu6-11"
    );

    let clean_hours = duration_hours(&clean);
    let contaminated_hours = duration_hours(&contaminated);

    let clean_cal_per_hour = per_hour(clean_cal, clean_hours);
    let clean_tlb_per_hour = per_hour(clean_tlb, clean_hours);
    let clean_res_per_hour = per_hour(clean_res, clean_hours);
    let clean_irq_per_hour = per_hour(clean_irq, clean_hours);

    let contaminated_cal_per_hour = per_hour(contaminated_cal, contaminated_hours);
    let contaminated_tlb_per_hour = per_hour(contaminated_tlb, contaminated_hours);
    let contaminated_res_per_hour = per_hour(contaminated_res, contaminated_hours);
    let contaminated_irq_per_hour = per_hour(contaminated_irq, contaminated_hours);

    // The eight published per-run-hour figures.
    assert_within_one_percent(clean_cal_per_hour, 6.0, "clean CAL/hour");
    assert_within_one_percent(clean_tlb_per_hour, 6.0, "clean TLB/hour");
    assert_within_one_percent(clean_res_per_hour, 55.0, "clean RES/hour");
    assert_within_one_percent(clean_irq_per_hour, 1127.0, "clean device IRQ/hour");

    assert_within_one_percent(contaminated_cal_per_hour, 48.0, "contaminated CAL/hour");
    assert_within_one_percent(contaminated_tlb_per_hour, 24.0, "contaminated TLB/hour");
    assert_within_one_percent(contaminated_res_per_hour, 96.0, "contaminated RES/hour");
    assert_within_one_percent(
        contaminated_irq_per_hour,
        844.0,
        "contaminated device IRQ/hour",
    );

    // The two directional claims the prose makes, which are what went wrong
    // before (finding 4): CAL, TLB and RES are all higher per run hour on the
    // contaminated arm, and only the aggregate device IRQ figure inverts. If a
    // future edit re-inserts the withdrawn "the contaminated arm recorded fewer
    // interrupts" claim, this is what contradicts it.
    assert!(
        contaminated_cal_per_hour > clean_cal_per_hour,
        "CAL must be higher per run hour on the contaminated arm: clean={clean_cal_per_hour} \
         contaminated={contaminated_cal_per_hour}"
    );
    assert!(
        contaminated_tlb_per_hour > clean_tlb_per_hour,
        "TLB must be higher per run hour on the contaminated arm: clean={clean_tlb_per_hour} \
         contaminated={contaminated_tlb_per_hour}"
    );
    assert!(
        contaminated_res_per_hour > clean_res_per_hour,
        "RES must be higher per run hour on the contaminated arm: clean={clean_res_per_hour} \
         contaminated={contaminated_res_per_hour}"
    );
    assert!(
        contaminated_irq_per_hour < clean_irq_per_hour,
        "device IRQ is the one figure that inverts, lower per run hour on the contaminated \
         arm: clean={clean_irq_per_hour} contaminated={contaminated_irq_per_hour}"
    );
}
