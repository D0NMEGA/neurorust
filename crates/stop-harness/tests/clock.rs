use nr_stop_harness::characterise::{
    CharacteriseError, current_clocksource, offset_estimate_ns, read_overhead_ns,
};
use nr_stop_harness::clock::{FixtureClock, MonotonicRawClock};

#[test]
fn fixture_clock_returns_the_programmed_sequence() {
    let clock = FixtureClock::new([10, 20, 35]);
    assert_eq!(clock.now_ns(), Some(10));
    assert_eq!(clock.now_ns(), Some(20));
    assert_eq!(clock.now_ns(), Some(35));
}

#[test]
fn fixture_clock_is_exhausted_rather_than_wrapping() {
    let clock = FixtureClock::new([10, 20, 35]);
    for _ in 0..3 {
        clock.now_ns();
    }
    assert_eq!(
        clock.now_ns(),
        None,
        "the fourth call must not restart the sequence"
    );
    assert_eq!(clock.now_ns(), None, "exhaustion is sticky, not a one-off");
}

#[test]
fn read_overhead_uses_consecutive_deltas() {
    let clock = FixtureClock::new([100, 140, 190, 260]);
    let overhead = read_overhead_ns(&clock, 3).expect("fixture provides exactly enough readings");
    assert_eq!(overhead, vec![40, 50, 70]);
}

#[test]
fn read_overhead_rejects_a_clock_that_stops_answering() {
    // iterations=3 needs 4 readings; the fixture provides only 2.
    let clock = FixtureClock::new([100, 140]);
    let result = read_overhead_ns(&clock, 3);
    assert!(
        matches!(result, Err(CharacteriseError::ClockUnavailable { .. })),
        "expected ClockUnavailable, got {result:?}"
    );
}

#[test]
fn offset_estimate_matches_the_textbook_formula() {
    let (t1, t2, t3) = (1_000u64, 1_050u64, 1_010u64);
    let expected = ((t2 as i64 - t1 as i64) + (t2 as i64 - t3 as i64)) / 2;
    let offset = offset_estimate_ns(t1, t2, t3).expect("in-range timestamps");
    assert_eq!(offset, expected);
}

#[test]
fn clocksource_is_read_from_the_named_sysfs_path() {
    let populated = tempfile::tempdir().expect("tempdir");
    let clocksource_dir = populated
        .path()
        .join("devices/system/clocksource/clocksource0");
    std::fs::create_dir_all(&clocksource_dir).expect("mkdir -p");
    std::fs::write(clocksource_dir.join("current_clocksource"), "tsc\n")
        .expect("write fixture file");

    assert_eq!(
        current_clocksource(populated.path()),
        Some("tsc".to_string())
    );

    let empty = tempfile::tempdir().expect("tempdir");
    assert_eq!(current_clocksource(empty.path()), None);
}
