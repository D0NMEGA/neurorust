use nr_histogram::hist::{HistError, parse_hist};
use nr_histogram::json::{CyclictestSummary, reconcile};

const PROBE_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/probe-cyclictest-60s.json"
));
const PROBE_H: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/probe-cyclictest-h-60s.hist"
));

/// The real, committed D-17 calibration-clean pair, reconciled as-is (never edited): the second
/// named test in this file proves `reconcile` accepts real harness output, not only synthetic
/// fixtures.
const CALIBRATION_CLEAN_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../measurements/2026-09-01-precision3591-calibration-clean/cyclictest.json"
));
const CALIBRATION_CLEAN_HIST: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../measurements/2026-09-01-precision3591-calibration-clean/cyclictest.hist"
));

#[test]
fn parses_probe_json() {
    let summary: CyclictestSummary = serde_json::from_str(PROBE_JSON).unwrap();
    assert_eq!(summary.num_threads, 6);
    assert_eq!(summary.thread.len(), 6);
    assert_eq!(summary.rt_test_version, "2.80");
}

#[test]
fn json_thread_count_matches() {
    let summary: CyclictestSummary = serde_json::from_str(PROBE_JSON).unwrap();
    let run = parse_hist(PROBE_H, None).unwrap();
    assert_eq!(summary.num_threads as usize, run.threads);
}

#[test]
fn json_and_hist_maxima_agree() {
    let summary: CyclictestSummary = serde_json::from_str(PROBE_JSON).unwrap();
    let run = parse_hist(PROBE_H, None).unwrap();

    for (i, &hist_max) in run.max_us.iter().enumerate() {
        let json_max = summary.thread.get(&i.to_string()).unwrap().max;
        assert_eq!(json_max, hist_max, "thread {i} maximum disagrees");
    }

    assert!(reconcile(&summary, &run).is_ok());
}

#[test]
fn json_missing_field_is_an_error() {
    // A minimal, otherwise-complete capture with the required per-thread "min" field removed.
    let missing_min = r#"{
        "file_version": 1,
        "cmdline:": "cyclictest --histogram=400",
        "rt_test_version:": "2.80",
        "start_time": "Mon, 31 Aug 2026 00:23:41 -0500",
        "end_time": "Mon, 31 Aug 2026 00:24:41 -0500",
        "return_code": 0,
        "sysinfo": {
            "sysname": "Linux",
            "nodename": "[redacted]",
            "release": "7.0.0-30-realtime",
            "version": "test",
            "machine": "x86_64",
            "realtime": 1
        },
        "num_threads": 1,
        "resolution_in_ns": 0,
        "thread": {
            "0": {
                "histogram": {"2": 1},
                "cycles": 1,
                "max": 2,
                "avg": 2.0,
                "cpu": 0,
                "node": 0
            }
        }
    }"#;

    let result: Result<CyclictestSummary, _> = serde_json::from_str(missing_min);
    assert!(
        result.is_err(),
        "removing a required field must fail to deserialize, not default"
    );
}

#[test]
fn reconcile_disagreement_is_an_error() {
    let mut summary: CyclictestSummary = serde_json::from_str(PROBE_JSON).unwrap();
    let run = parse_hist(PROBE_H, None).unwrap();

    assert!(reconcile(&summary, &run).is_ok());

    // Corrupt one thread's reported maximum so the two files no longer describe the same run.
    summary.thread.get_mut("0").unwrap().max += 1;
    assert!(reconcile(&summary, &run).is_err());
}

/// Two files can agree on thread count and on every per-thread maximum while disagreeing on
/// how many samples were taken; finding 6 of `01-EXTERNAL-AUDIT.md`. This test corrupts only
/// `cycles`, leaving `max` untouched, so it fails only the sample-count check.
#[test]
fn reconcile_rejects_sample_count_disagreement() {
    let mut summary: CyclictestSummary = serde_json::from_str(PROBE_JSON).unwrap();
    let run = parse_hist(PROBE_H, None).unwrap();

    assert!(reconcile(&summary, &run).is_ok());

    summary.thread.get_mut("0").unwrap().cycles += 1;
    match reconcile(&summary, &run) {
        Err(HistError::SummaryDisagreement { field, .. }) => {
            assert_eq!(field, "sample count");
        }
        other => panic!("expected a sample count SummaryDisagreement, got {other:?}"),
    }
}

/// `reconcile` accepts the real, unedited D-17 calibration-clean pair: a positive control
/// proving the sample-count check does not false-positive on genuine harness output.
#[test]
fn reconcile_accepts_the_real_committed_pair() {
    let summary: CyclictestSummary = serde_json::from_str(CALIBRATION_CLEAN_JSON).unwrap();
    let run = parse_hist(CALIBRATION_CLEAN_HIST, None).unwrap();
    assert!(reconcile(&summary, &run).is_ok());
}
