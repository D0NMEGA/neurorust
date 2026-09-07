//! The standing check plan 01-11's post-mortem calls for: assert which CPUs a committed
//! firmware capture's events actually name, not only its maximum. See
//! `docs/rig/firmware-floor-rt-vs-stock.md`, "The result: neither set of figures
//! characterises the isolated cores".
//!
//! This test reads the real, already-committed `hwlatdetect*.txt` captures (by path) and
//! `manifest.json` files (by their already-parsed `firmware_screens`) under `measurements/`;
//! it never writes to that directory (D-12: raw captures are read-only evidence).

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

/// The uncomfortable fact `hwlatdetect` never overcame on this rig, asserted rather than
/// left in a paragraph: not one `hwlatdetect` capture's events name a CPU this project
/// actually isolates (6 through 11). This is permanent, not a placeholder: the hwlat
/// tracer's `mode` is `none` on this kernel and `hwlatdetect` exposes no way to change that
/// (`docs/rig/firmware-floor-rt-vs-stock.md`, "Setting the mode was tried, and hwlatdetect
/// prevents it"), so no future `hwlatdetect` capture will cover these cores either.
///
/// Renamed from `no_committed_capture_covers_the_isolated_cores` when plan 01-23 committed
/// the first captures that DO cover these cores: three `rtla hwnoise` arms, asserted by
/// [`hwnoise_captures_cover_the_isolated_cores`] below, which reads those manifests rather
/// than re-deriving from `hwlatdetect`'s (unrelated) output format.
#[test]
fn hwlatdetect_captures_still_cover_no_isolated_core() {
    let mut all_cpus = std::collections::BTreeSet::new();
    for &(path, _, _) in COMMITTED_CAPTURES {
        let text = read_capture(path);
        all_cpus.extend(event_cpus(&parse_hwlatdetect_events(&text)));
    }

    for isolated in 6..=11u32 {
        assert!(
            !all_cpus.contains(&isolated),
            "cpu{isolated} appears in a committed hwlatdetect capture's events, which the \
             tracer's own mode should make impossible on this rig; investigate before \
             assuming this is progress"
        );
    }
}

/// The new coverage plan 01-23 committed: every manifest whose `firmware_screens` names
/// `rtla-hwnoise` must report `observed_cpus` covering all six isolated CPUs, because that
/// instrument runs one osnoise sampling thread per requested CPU rather than one migrating
/// thread (`docs/rig/firmware-floor-rt-vs-stock.md`, "The re-take with rtla hwnoise"). Reads
/// each already-parsed `manifest.json` under `measurements/` rather than re-deriving from
/// `rtla-hwnoise.txt`: the parser itself (`nr_capture::hwnoise::parse_hwnoise`) has its own
/// tests, and this one is the standing coverage check the 01-11 post-mortem asked for,
/// applied to the instrument that actually answers it.
///
/// If a future arm does not cover all six, this must fail naming the gap, not be weakened to
/// pass: a coverage regression is exactly what this test exists to catch.
#[test]
fn hwnoise_captures_cover_the_isolated_cores() {
    let repo_root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let measurements_dir = format!("{repo_root}/measurements");
    let requested: Vec<u32> = (6..=11).collect();

    let mut checked = 0;
    for entry in std::fs::read_dir(&measurements_dir)
        .unwrap_or_else(|err| panic!("failed to read {measurements_dir}: {err}"))
    {
        let entry = entry.expect("readable measurements/ directory entry");
        let manifest_path = entry.path().join("manifest.json");
        let Ok(text) = std::fs::read_to_string(&manifest_path) else {
            continue;
        };
        let Ok(manifest) = serde_json::from_str::<nr_manifest::RunManifest>(&text) else {
            continue;
        };

        for screen in &manifest.firmware_screens {
            if screen.instrument != "rtla-hwnoise" {
                continue;
            }
            checked += 1;
            let missing = uncovered_cpus(&requested, &screen.observed_cpus);
            assert!(
                missing.is_empty(),
                "{}: rtla-hwnoise screen does not cover cpu(s) {missing:?} (observed {:?})",
                manifest_path.display(),
                screen.observed_cpus
            );
        }
    }

    assert!(
        checked >= 3,
        "expected at least the three D-18 re-take arms (plan 01-23) to carry an rtla-hwnoise \
         firmware screen; found {checked}. A count of 0 means the re-take captures are \
         missing from measurements/, not that this test's floor is wrong."
    );
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
