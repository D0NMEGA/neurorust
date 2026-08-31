use std::collections::BTreeMap;

use nr_histogram::hist::{CyclictestRun, OverflowBoundSource, parse_hist};
use nr_histogram::percentiles::PercentileError;

const REAL_CAPTURE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/cyclictest-rt-isolated-idle-10m.hist"
));

// Note on precision: hdrhistogram is configured with 3 significant figures (`Histogram::new(3)`),
// under which every value asserted below (all well under 1000 us) is stored exactly, with no
// bucket rounding. If a future change to this crate ever needs to raise SIGFIG or the value
// range grows past 1000 us, `value_at_quantile` may return a neighbouring integer; widen the
// affected assertion by +/-1 with a comment rather than silently changing the expected figure.

#[test]
fn percentiles_from_fixture() {
    let run = parse_hist(REAL_CAPTURE, None).unwrap();
    let excluded = run
        .percentiles_excluding_overflows(&[0.50, 0.95, 0.99, 0.999, 0.9999])
        .unwrap();

    assert_eq!(excluded.total_samples, 17994956);
    assert_eq!(excluded.binned_samples, 17994956);
    assert_eq!(excluded.overflow_samples, 0);
    assert_eq!(
        excluded.values,
        vec![(0.50, 2), (0.95, 4), (0.99, 9), (0.999, 10), (0.9999, 12)]
    );
}

#[test]
fn overflow_counted() {
    let run = parse_hist(REAL_CAPTURE, None).unwrap();
    assert_eq!(run.overflows.iter().sum::<u64>(), 888);

    let inclusive = run.percentiles(&[0.50, 0.95, 0.99, 0.999, 0.9999]).unwrap();
    assert_eq!(inclusive.total_samples, 17995844);
    assert_eq!(inclusive.overflow_samples, 888);
    assert_eq!(
        inclusive.values,
        vec![(0.50, 2), (0.95, 4), (0.99, 9), (0.999, 10), (0.9999, 108)]
    );

    // The corrected D-23 figure, per the 2026-08-31 operator decision. Two corrections over
    // the previously published number, and the figure must be self-consistent in both:
    //
    //   1. Boundary. PLAT-03 requires latency "brought under 30 us", so a sample landing
    //      exactly on 30 us has NOT met the gate. Count at-or-above, not strictly above.
    //      Bin 30 holds 4 samples, which is the whole 2,089 vs 2,093 difference.
    //   2. Denominator. 17,994,956 is the histogram BODY total and excludes the 888
    //      overflows. Counting overflows in the numerator but not the denominator mixes
    //      populations, in a correction whose entire purpose is fixing a population error.
    //      The true total is 17,995,844, which is what `total_samples` reports.
    let above_gate_inclusive = run.samples_at_or_above(30);
    assert_eq!(above_gate_inclusive, 2093);
    let total_with_overflows = inclusive.total_samples;
    assert_eq!(total_with_overflows, 17_995_844);
    let pct_inclusive = above_gate_inclusive as f64 / total_with_overflows as f64 * 100.0;
    assert!(
        (pct_inclusive - 0.0116).abs() < 0.0001,
        "expected ~0.0116%, got {pct_inclusive}"
    );

    // The strict-boundary count is retained but must not be used for a published figure.
    assert_eq!(run.samples_above(30), 2089);

    // The figure the 2026-08-28 README actually published, reproduced here so the D-23
    // correction can be checked side by side. Its own convention (strictly above 30 us,
    // overflows omitted, body-only denominator) is stated wherever it is published.
    let above_gate_exclusive = run.samples_above_excluding_overflows(30);
    assert_eq!(above_gate_exclusive, 1201);
    let pct_exclusive = above_gate_exclusive as f64 / 17_994_956.0 * 100.0;
    assert!(
        (pct_exclusive - 0.0067).abs() < 0.0001,
        "expected ~0.0067%, got {pct_exclusive}"
    );
}

#[test]
fn overflow_counted_is_the_default() {
    let run = parse_hist(REAL_CAPTURE, None).unwrap();
    let default_path = run.percentiles(&[0.9999]).unwrap();
    let exclusive_path = run.percentiles_excluding_overflows(&[0.9999]).unwrap();

    assert_eq!(default_path.total_samples, 17995844);
    assert_eq!(exclusive_path.total_samples, 17994956);
    assert_ne!(default_path.values[0].1, exclusive_path.values[0].1);
    assert_eq!(default_path.values[0].1, 108);
    assert_eq!(exclusive_path.values[0].1, 12);
}

#[test]
fn max_is_not_the_bound() {
    let run = parse_hist(REAL_CAPTURE, None).unwrap();
    assert_eq!(run.overflow_bound_source, OverflowBoundSource::Derived);
    assert_eq!(run.overflow_bound_us, 400);
    assert_eq!(run.max_us(), 3806);
    assert_ne!(run.max_us(), run.overflow_bound_us);
}

#[test]
fn per_thread_and_merged_agree() {
    let run = parse_hist(REAL_CAPTURE, None).unwrap();
    let merged = run.percentiles(&[0.50]).unwrap();

    let binned: u64 = run.bins.values().flat_map(|counts| counts.iter()).sum();
    let overflow: u64 = run.overflows.iter().sum();

    assert_eq!(merged.total_samples, binned + overflow);
    assert_eq!(merged.total_samples, 17995844);
    assert_eq!(merged.binned_samples, 17994956);
    assert_eq!(merged.overflow_samples, 888);
}

#[test]
fn empty_run_is_an_error() {
    let empty = CyclictestRun {
        threads: 1,
        summary_column_present: false,
        bins: BTreeMap::new(),
        min_us: vec![0],
        avg_us: vec![0],
        max_us: vec![0],
        overflows: vec![0],
        overflow_cycles: vec![vec![]],
        overflow_bound_us: 1,
        overflow_bound_source: OverflowBoundSource::Derived,
    };

    let result = empty.percentiles(&[0.5]);
    assert!(matches!(result, Err(PercentileError::EmptyRun)));
}

#[test]
fn to_bin_table_sums_across_threads() {
    let run = parse_hist(REAL_CAPTURE, None).unwrap();
    let table = run.to_bin_table();
    let total: u64 = table.iter().map(|&(_, count)| count).sum();
    assert_eq!(total, 17994956);
    assert!(table.iter().any(|&(bin_us, _)| bin_us == 1));
    assert!(table.iter().any(|&(bin_us, _)| bin_us == 399));
}
