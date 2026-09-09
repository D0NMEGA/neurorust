use nr_manifest::{ContaminationVerdict, InstrumentClass, RunClass};
use nr_metrics::baseline::{Baseline, BaselineEntry, RegressedMetric, RegressionVerdict, compare};
use nr_metrics::series::StageMetrics;
use time::macros::datetime;

fn base_entry() -> StageMetrics {
    StageMetrics {
        run_id: "2026-08-31-precision3591-weekly-002".to_string(),
        run_class: RunClass::Weekly,
        instrument_class: InstrumentClass::HeadlineSeries,
        utc_start: datetime!(2026-08-31 06:00:00 UTC),
        iso_week: "2026-W35".to_string(),
        rig_slug: "precision3591".to_string(),
        tool: "cyclictest".to_string(),
        stage: "cyclictest.wakeup_latency".to_string(),
        sample_count: 18_000_000,
        overflow_count: 888,
        p50_us: Some(2),
        p95_us: Some(6),
        p99_us: Some(9),
        p999_us: Some(12),
        max_us: 60,
        population: "scheduling wakeups, binned samples plus overflows".to_string(),
        contamination_verdict: ContaminationVerdict::Clean,
        excluded_from_series: false,
        exclusion_reason: None,
        manifest_blake3: "b".repeat(64),
    }
}

/// Loads the thresholds from the real, committed `metrics/baseline.json`, so this suite proves
/// the regression gate against the actual reviewable configuration (D-11) rather than a value
/// duplicated in a test.
fn real_thresholds() -> nr_metrics::baseline::Thresholds {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../metrics/baseline.json");
    Baseline::load(std::path::Path::new(path))
        .expect("the committed metrics/baseline.json parses")
        .thresholds
}

fn baseline_entry_for(run: &StageMetrics, p99_us: u64, max_us: u64) -> BaselineEntry {
    BaselineEntry {
        rig_slug: run.rig_slug.clone(),
        run_class: run.run_class.clone(),
        stage: run.stage.clone(),
        p99_us,
        max_us,
    }
}

fn baseline_with_entries(entries: Vec<BaselineEntry>) -> Baseline {
    Baseline {
        schema_version: 1,
        thresholds: real_thresholds(),
        note: "test fixture".to_string(),
        entries,
    }
}

/// A run whose p99 is 12 us against a baseline p99 of 9 us fails, because 12 exceeds
/// max(9 * 1.20, 9 + 2) = 11. A run whose p99 is 11 us passes, at the boundary. A run whose max
/// is 200 us against a baseline max of 60 us fails, because 200 exceeds
/// max(60 * 1.50, 60 + 100) = 160.
#[test]
fn regression_gate() {
    let run = base_entry();
    let baseline = baseline_with_entries(vec![baseline_entry_for(&run, 9, 60)]);

    let failing_p99 = StageMetrics {
        p99_us: Some(12),
        ..run.clone()
    };
    match compare(&failing_p99, &baseline) {
        RegressionVerdict::Fail { failures } => {
            assert!(
                failures
                    .iter()
                    .any(|f| f.metric == RegressedMetric::P99 && f.threshold_us == 11),
                "expected a p99 failure at threshold 11, got {failures:?}"
            );
        }
        other => panic!("expected Fail for p99=12 against baseline p99=9, got {other:?}"),
    }

    let boundary_p99 = StageMetrics {
        p99_us: Some(11),
        ..run.clone()
    };
    match compare(&boundary_p99, &baseline) {
        RegressionVerdict::Pass { .. } => {}
        other => panic!("expected Pass at the p99=11 boundary, got {other:?}"),
    }

    let failing_max = StageMetrics { max_us: 200, ..run };
    match compare(&failing_max, &baseline) {
        RegressionVerdict::Fail { failures } => {
            assert!(
                failures
                    .iter()
                    .any(|f| f.metric == RegressedMetric::Max && f.threshold_us == 160),
                "expected a max failure at threshold 160, got {failures:?}"
            );
        }
        other => panic!("expected Fail for max=200 against baseline max=60, got {other:?}"),
    }
}

/// A run whose p99 is 3 us against a baseline p99 of 2 us passes, because the absolute floor of
/// 2 us dominates the 20 percent relative threshold at these magnitudes.
#[test]
fn regression_gate_absolute_floor() {
    let run = StageMetrics {
        p99_us: Some(3),
        ..base_entry()
    };
    let baseline = baseline_with_entries(vec![baseline_entry_for(&run, 2, 60)]);

    match compare(&run, &baseline) {
        RegressionVerdict::Pass { .. } => {}
        other => panic!("expected the 2us absolute floor to dominate and pass, got {other:?}"),
    }
}

/// A weekly-class run is compared only against a weekly-class baseline entry for the same stage
/// and rig; a soak-class run against a weekly baseline returns NoComparableBaseline.
#[test]
fn regression_compares_like_with_like() {
    let baseline = baseline_with_entries(vec![BaselineEntry {
        rig_slug: "precision3591".to_string(),
        run_class: RunClass::Weekly,
        stage: "cyclictest.wakeup_latency".to_string(),
        p99_us: 9,
        max_us: 60,
    }]);

    let soak_run = StageMetrics {
        run_class: RunClass::Soak,
        ..base_entry()
    };

    match compare(&soak_run, &baseline) {
        RegressionVerdict::NoComparableBaseline { run_class, .. } => {
            assert_eq!(run_class, RunClass::Soak);
        }
        other => panic!(
            "expected NoComparableBaseline for a soak run against a weekly baseline, got {other:?}"
        ),
    }
}

/// A run whose verdict is Contaminated returns RegressionVerdict::SkippedContaminated with its
/// reason, and the caller records a coverage gap rather than a pass.
#[test]
fn contaminated_run_is_reported_not_compared() {
    let run = StageMetrics {
        contamination_verdict: ContaminationVerdict::Contaminated,
        excluded_from_series: true,
        exclusion_reason: Some("CAL IPI count exceeded threshold on cpu6".to_string()),
        ..base_entry()
    };
    let baseline = baseline_with_entries(vec![baseline_entry_for(&run, 9, 60)]);

    match compare(&run, &baseline) {
        RegressionVerdict::SkippedContaminated { reason } => {
            assert!(reason.contains("CAL IPI"), "reason was {reason:?}");
        }
        other => panic!("expected SkippedContaminated, got {other:?}"),
    }
}

/// A run whose verdict is Uncalibrated returns SkippedUncalibrated, never Pass.
#[test]
fn uncalibrated_run_is_not_a_pass() {
    let run = StageMetrics {
        contamination_verdict: ContaminationVerdict::Uncalibrated,
        ..base_entry()
    };
    let baseline = baseline_with_entries(vec![baseline_entry_for(&run, 9, 60)]);

    match compare(&run, &baseline) {
        RegressionVerdict::SkippedUncalibrated { .. } => {}
        RegressionVerdict::Pass { .. } => {
            panic!("an uncalibrated run must never be reported as a Pass")
        }
        other => panic!("expected SkippedUncalibrated, got {other:?}"),
    }
}

/// Comparing a stage absent from the baseline returns NoComparableBaseline rather than
/// defaulting to a pass.
#[test]
fn baseline_missing_stage_is_an_error() {
    let run = base_entry();
    let baseline = baseline_with_entries(Vec::new());

    match compare(&run, &baseline) {
        RegressionVerdict::NoComparableBaseline { stage, .. } => {
            assert_eq!(stage, run.stage);
        }
        other => panic!("expected NoComparableBaseline for a missing stage, got {other:?}"),
    }
}
