use nr_histogram::hist::{HistError, OverflowBoundSource, parse_hist};

const REAL_CAPTURE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/cyclictest-rt-isolated-idle-10m.hist"
));
const PROBE_H: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/probe-cyclictest-h-60s.hist"
));
const PROBE_HISTOFALL: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/probe-cyclictest-histofall-60s.hist"
));

#[test]
fn parses_thread_count() {
    let run = parse_hist(REAL_CAPTURE, None).unwrap();
    assert_eq!(run.threads, 6);
}

#[test]
fn parses_bin_totals() {
    let run = parse_hist(REAL_CAPTURE, None).unwrap();
    let mut totals = vec![0u64; run.threads];
    for counts in run.bins.values() {
        for (total, &count) in totals.iter_mut().zip(counts.iter()) {
            *total += count;
        }
    }
    assert_eq!(
        totals,
        vec![2999187, 2999145, 2999156, 2999151, 2999168, 2999149]
    );
}

#[test]
fn parses_total_binned() {
    let run = parse_hist(REAL_CAPTURE, None).unwrap();
    let total: u64 = run.bins.values().flat_map(|counts| counts.iter()).sum();
    assert_eq!(total, 17994956);
}

#[test]
fn parses_overflows() {
    let run = parse_hist(REAL_CAPTURE, None).unwrap();
    assert_eq!(run.overflows, vec![142, 152, 146, 152, 145, 151]);
    assert_eq!(run.overflows.iter().sum::<u64>(), 888);
}

#[test]
fn parses_maxima() {
    let run = parse_hist(REAL_CAPTURE, None).unwrap();
    assert_eq!(run.max_us, vec![3785, 3679, 3787, 3693, 3806, 3726]);
    assert_eq!(run.max_us.iter().copied().max(), Some(3806));
}

#[test]
fn parses_minima() {
    let run = parse_hist(REAL_CAPTURE, None).unwrap();
    assert_eq!(run.min_us, vec![1, 1, 1, 2, 1, 1]);
}

#[test]
fn parses_bin_range() {
    let run = parse_hist(REAL_CAPTURE, None).unwrap();
    assert_eq!(run.bins.keys().next().copied(), Some(1));
    assert_eq!(run.bins.keys().next_back().copied(), Some(399));
}

#[test]
fn derives_overflow_bound() {
    let run = parse_hist(REAL_CAPTURE, None).unwrap();
    assert_eq!(run.overflow_bound_us, 400);
    assert_eq!(run.overflow_bound_source, OverflowBoundSource::Derived);
}

#[test]
fn declared_bound_is_recorded_when_given() {
    let run = parse_hist(REAL_CAPTURE, Some(500)).unwrap();
    assert_eq!(run.overflow_bound_us, 500);
    assert_eq!(run.overflow_bound_source, OverflowBoundSource::Declared);
}

#[test]
fn parses_overflow_cycles() {
    let run = parse_hist(REAL_CAPTURE, None).unwrap();
    assert_eq!(run.overflow_cycles[0].len(), 142);
    assert_eq!(run.overflow_cycles[0][0], 51);
}

#[test]
fn histofall_summary_column_not_double_counted() {
    let h_run = parse_hist(PROBE_H, None).unwrap();
    let histofall_run = parse_hist(PROBE_HISTOFALL, None).unwrap();

    assert_eq!(h_run.threads, 6);
    assert_eq!(histofall_run.threads, 6);
    assert!(!h_run.summary_column_present);
    assert!(histofall_run.summary_column_present);

    for counts in h_run.bins.values() {
        assert_eq!(counts.len(), 6);
    }
    for counts in histofall_run.bins.values() {
        assert_eq!(
            counts.len(),
            6,
            "the -H summary column must be dropped, not counted as a 7th thread"
        );
    }
}

#[test]
fn rejects_ragged_line() {
    let mut lines: Vec<String> = REAL_CAPTURE.lines().map(String::from).collect();
    // Line 1 is "# Histogram", line 2 establishes the real column width (6 threads).
    // Corrupt line 3 (the second data line) to fewer columns than that established width.
    lines[2] = "000003\t539242".to_string();
    let input = lines.join("\n");

    let result = parse_hist(&input, None);
    assert!(matches!(result, Err(HistError::RaggedColumns { line: 3 })));
}
