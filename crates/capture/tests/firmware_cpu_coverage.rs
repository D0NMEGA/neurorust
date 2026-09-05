//! The standing check plan 01-11's post-mortem calls for: assert which CPUs a committed
//! firmware capture's events actually name, not only its maximum. See
//! `docs/rig/firmware-floor-rt-vs-stock.md`, "The result: neither set of figures
//! characterises the isolated cores".
//!
//! This test reads the real, already-committed `hwlatdetect*.txt` captures under
//! `measurements/` by path; it never writes to that directory (D-12: raw captures are
//! read-only evidence).

use nr_capture::firmware::{event_cpus, parse_hwlatdetect_events, uncovered_cpus};

fn read_capture(relative: &str) -> String {
    let repo_root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let full = format!("{repo_root}/{relative}");
    std::fs::read_to_string(&full).unwrap_or_else(|err| panic!("failed to read {full}: {err}"))
}

/// Every committed `hwlatdetect*.txt` capture, with its event count and CPU set counted
/// independently on 2026-09-05 (this plan's own `<interfaces>` block). These are the
/// expected values; the test must not re-derive them from the same file it is checking.
const COMMITTED_CAPTURES: &[(&str, usize, &[u32])] = &[
    (
        "measurements/2026-08-28-precision3591/hwlatdetect-stock-15m.txt",
        291,
        &[2, 4],
    ),
    (
        "measurements/2026-08-28-precision3591/hwlatdetect-tuned-15m.txt",
        0,
        &[],
    ),
    (
        "measurements/2026-08-28-precision3591/hwlatdetect-tuned-underload-15m.txt",
        26,
        &[13, 14, 15, 20, 21],
    ),
    (
        "measurements/2026-08-28-precision3591/hwlatdetect-pcore-underload-10m.txt",
        13,
        &[0],
    ),
    (
        "measurements/2026-09-03-precision3591-calibration-clean/hwlatdetect.txt",
        0,
        &[],
    ),
    (
        "measurements/2026-09-05-precision3591-screen/hwlatdetect.txt",
        12,
        &[5],
    ),
    (
        "measurements/2026-09-05-precision3591-screen-02/hwlatdetect.txt",
        5,
        &[5],
    ),
    (
        "measurements/2026-09-05-precision3591-screen-03/hwlatdetect.txt",
        15,
        &[5],
    ),
];

#[test]
fn hwlatdetect_event_cpus_are_parsed() {
    let text =
        read_capture("measurements/2026-08-28-precision3591/hwlatdetect-pcore-underload-10m.txt");
    let events = parse_hwlatdetect_events(&text);
    assert_eq!(events.len(), 13);
    assert_eq!(event_cpus(&events), vec![0]);
}

#[test]
fn committed_captures_have_their_recorded_cpu_distribution() {
    for &(path, expected_count, expected_cpus) in COMMITTED_CAPTURES {
        let text = read_capture(path);
        let events = parse_hwlatdetect_events(&text);
        assert_eq!(events.len(), expected_count, "{path}: event count");
        assert_eq!(
            event_cpus(&events).as_slice(),
            expected_cpus,
            "{path}: event CPU set"
        );
    }
}

/// The uncomfortable fact, asserted rather than left in a paragraph: not one firmware event
/// in any committed capture names a CPU this project actually isolates (6 through 11).
///
/// Expected to change the day a future plan (01-23) commits an `rtla hwnoise` capture whose
/// events, or `crate::hwnoise::HwnoiseRun` rows, name one of those CPUs. Whoever changes
/// this test at that point must do so deliberately, by adding the new capture to the
/// evidence this test checks, not by deleting or weakening the assertion because it started
/// failing.
#[test]
fn no_committed_capture_covers_the_isolated_cores() {
    let mut all_cpus = std::collections::BTreeSet::new();
    for &(path, _, _) in COMMITTED_CAPTURES {
        let text = read_capture(path);
        all_cpus.extend(event_cpus(&parse_hwlatdetect_events(&text)));
    }

    for isolated in 6..=11u32 {
        assert!(
            !all_cpus.contains(&isolated),
            "cpu{isolated} appears in a committed firmware capture's events; if a new \
             instrument capture was just committed, update this test deliberately rather \
             than deleting it"
        );
    }
}

#[test]
fn coverage_assertion_rejects_a_capture_that_misses_a_requested_cpu() {
    let requested: Vec<u32> = (6..=11).collect();
    let observed = vec![5];
    assert_eq!(
        uncovered_cpus(&requested, &observed),
        vec![6, 7, 8, 9, 10, 11]
    );
}
