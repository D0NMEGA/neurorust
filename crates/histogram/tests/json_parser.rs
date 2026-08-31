use nr_histogram::hist::parse_hist;
use nr_histogram::json::{CyclictestSummary, reconcile};

const PROBE_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/probe-cyclictest-60s.json"
));
const PROBE_H: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/probe-cyclictest-h-60s.hist"
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
