use std::sync::Barrier;

use nr_stop::latch::EmergencyStop;
use nr_stop::state::State;

use nr_stop_harness::clock::FixtureClock;
use nr_stop_harness::trial::{
    TrialConfig, TrialError, abort_side_trial, hot_side_trial, run_trials,
};

/// Builds a fixture clock whose consecutive values jump by a step far larger than any
/// `period_ns`/phase used in these tests. A busy-wait's deadline check depends only on which
/// pre-programmed value comes next, never on real elapsed time, so a large step bounds every
/// busy-wait in this file to a small, fixed number of reads regardless of real thread-scheduling
/// timing. The readings are an ordering device, not a measurement of real time.
///
/// There is deliberately no budget. How many readings a trial consumes depends on how many
/// times the hot path polls before it observes the abort, which depends on when the aborting
/// thread is scheduled, which varies by machine and by how many of this binary's parallel
/// `#[test]` functions are contending at that moment. Every fixed budget is therefore a bet
/// about the slowest machine that will ever run this suite, and that bet was already raised
/// once for local contention and still lost on a macOS CI runner: 2,000,000 readings ran dry at
/// trial 3244 of 10,000, so `phases_are_uniform_enough_to_state_so` never reached its
/// assertion. The assertion itself is deterministic, since phases come from a seeded PRNG and
/// not from timing, so the test was machine-independent and only its budget was not.
///
/// `FixtureClock::generated` removes the budget instead of resizing it. Exhaustion is still
/// tested, deliberately, by the tests that construct a short exact sequence with
/// `FixtureClock::new`.
fn generous_clock() -> FixtureClock {
    const STEP: u64 = 10_000_000;
    FixtureClock::generated(STEP)
}

fn safe_config(trials: usize, period_ns: u64, seed: u64) -> TrialConfig {
    TrialConfig {
        period_ns,
        trials,
        // 0 and 1 rather than the rig's isolated 7/8: this must also pass on a minimal CI
        // runner, which may report far fewer cores than the reference rig.
        hot_cpu: 0,
        abort_cpu: 1,
        priority: 10,
        seed,
        require_realtime_scheduling: false,
    }
}

#[test]
fn a_trial_records_the_interval_the_decision_defines() {
    let clock = generous_clock();
    let config = safe_config(1, 1_000, 42);

    let outcome = run_trials(&clock, &config).expect("fixture provides ample headroom");
    assert_eq!(outcome.rows.len(), 1);

    let row = &outcome.rows[0];
    assert_eq!(row.latency_ns, row.observed_raw_ns - row.abort_raw_ns);
}

#[test]
fn the_hot_thread_acknowledges_exactly_once_per_trial() {
    let stop = EmergencyStop::new();
    let clock = generous_clock();
    let barrier = Barrier::new(2);

    std::thread::scope(|scope| {
        let hot = scope.spawn(|| hot_side_trial(&clock, &stop, &barrier, 1_000, 0));
        abort_side_trial(&clock, &stop, &barrier, 0, 0).expect("fixture provides ample headroom");
        hot.join()
            .expect("hot thread must not panic")
            .expect("fixture provides ample headroom");
    });

    assert_eq!(stop.state(), State::Stopped);
    assert!(!stop.acknowledge());
}

#[test]
fn the_abort_record_holds_the_starting_timestamp() {
    let stop = EmergencyStop::new();
    let clock = generous_clock();
    let barrier = Barrier::new(2);

    let abort_raw_ns = std::thread::scope(|scope| {
        let hot = scope.spawn(|| hot_side_trial(&clock, &stop, &barrier, 1_000, 0));
        let abort_raw_ns = abort_side_trial(&clock, &stop, &barrier, 0, 0)
            .expect("fixture provides ample headroom");
        hot.join()
            .expect("hot thread must not panic")
            .expect("fixture provides ample headroom");
        abort_raw_ns
    });

    let record = stop
        .abort_record()
        .expect("the winning abort publishes a record");
    assert_eq!(record.at_raw_ns, abort_raw_ns);
}

#[test]
fn phases_are_recorded_per_trial() {
    let clock = generous_clock();
    let config = safe_config(1_000, 1_000, 42);

    let outcome = run_trials(&clock, &config).expect("fixture provides ample headroom");
    assert_eq!(outcome.rows.len(), 1_000);
    for row in &outcome.rows {
        assert!(
            row.phase_ns < config.period_ns,
            "phase {} must be strictly less than the poll period {}",
            row.phase_ns,
            config.period_ns
        );
    }
}

#[test]
fn phases_are_uniform_enough_to_state_so() {
    let clock = generous_clock();
    let config = safe_config(10_000, 1_000, 12345);

    let outcome = run_trials(&clock, &config).expect("fixture provides ample headroom");
    assert_eq!(outcome.rows.len(), 10_000);

    let bucket_width = config.period_ns / 10;
    let mut buckets = [0u32; 10];
    for row in &outcome.rows {
        let bucket = ((row.phase_ns / bucket_width) as usize).min(9);
        buckets[bucket] += 1;
    }

    let total = outcome.rows.len() as f64;
    for (bucket, &count) in buckets.iter().enumerate() {
        let fraction = f64::from(count) / total;
        assert!(
            (0.08..=0.12).contains(&fraction),
            "bucket {bucket} held {fraction:.4} of trials, outside [0.08, 0.12]"
        );
    }
}

#[test]
fn a_clock_failure_refuses_rather_than_records_zero() {
    // One reading is nowhere near enough for even one full trial: the abort thread alone needs
    // a start read plus a deadline-check read plus its own `at` read.
    let clock = FixtureClock::new([100]);
    let config = safe_config(1, 1_000, 42);

    let result = run_trials(&clock, &config);
    assert!(
        matches!(result, Err(TrialError::ClockUnavailable { .. })),
        "expected ClockUnavailable, got {result:?}"
    );
}

#[test]
fn the_sample_vector_is_preallocated() {
    let clock = generous_clock();
    let config = safe_config(7, 1_000, 7);

    let outcome = run_trials(&clock, &config).expect("fixture provides ample headroom");
    assert_eq!(outcome.rows.len(), 7);
    assert_eq!(outcome.rows.capacity(), 7);
}
