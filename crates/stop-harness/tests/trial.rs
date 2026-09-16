use std::sync::Barrier;
use std::sync::atomic::{AtomicUsize, Ordering};

use nr_stop::event::Cause;
use nr_stop::latch::EmergencyStop;
use nr_stop::state::State;

use nr_stop_harness::clock::{FixtureClock, MonotonicRawClock};
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

/// A clock that closes the latch part-way through the readings it hands out.
///
/// The hot loop only paces while the latch is still `Running`, so a single-threaded test of its
/// pacing needs something to close the latch mid-flight. Doing it from the clock keeps the whole
/// test deterministic: no second thread, no real time, no dependence on how the two are scheduled.
struct LatchClosingClock<'a> {
    stop: &'a EmergencyStop,
    base_ns: u64,
    step_ns: u64,
    close_after_reads: usize,
    reads: AtomicUsize,
}

impl MonotonicRawClock for LatchClosingClock<'_> {
    fn now_ns(&self) -> Option<u64> {
        let index = self.reads.fetch_add(1, Ordering::SeqCst);
        let reading = self.base_ns + (index as u64) * self.step_ns;
        if index == self.close_after_reads {
            self.stop.abort(Cause::Operator, reading);
        }
        Some(reading)
    }
}

#[test]
fn the_hot_loop_polls_once_per_period_against_a_boot_relative_clock() {
    // `CLOCK_MONOTONIC_RAW` counts nanoseconds since boot, so on any machine that has been up for
    // more than an instant every reading dwarfs a poll period measured in microseconds. A busy-wait
    // deadline anchored at 0 instead of at a clock reading is therefore already in the past at the
    // first comparison: the wait returns immediately, the deadline never catches up, and the loop
    // degenerates into an unpaced spin that polls the latch as fast as it can read the clock.
    //
    // Every other fixture in this file starts at 0, which is the one origin where that arithmetic
    // behaves, which is exactly why the defect reached the rig. On 2026-09-16 a 33,000 ns poll
    // period produced a 222 ns median end to end: bare cross-core propagation, with no poll period
    // in it at all. This fixture starts where a real clock starts.
    const BASE_NS: u64 = 432_000_000_000_000; // about five days of uptime
    const STEP_NS: u64 = 1;
    const PERIOD_NS: u64 = 1_000;
    const CLOSE_AFTER_READS: usize = 5;

    let stop = EmergencyStop::new();
    let clock = LatchClosingClock {
        stop: &stop,
        base_ns: BASE_NS,
        step_ns: STEP_NS,
        close_after_reads: CLOSE_AFTER_READS,
        reads: AtomicUsize::new(0),
    };
    // One party, so `wait` returns immediately and this stays single-threaded.
    let barrier = Barrier::new(1);

    let observed =
        hot_side_trial(&clock, &stop, &barrier, PERIOD_NS, 0).expect("this clock never runs out");

    // The latch closed five readings in, well inside the first period. D-32 says the hot path
    // checks the latch once per iteration, so the loop must not observe that close until its first
    // period boundary. Unpaced, it observes within a handful of readings of the close and this
    // lands far below the boundary.
    assert!(
        observed >= BASE_NS + PERIOD_NS,
        "observed {observed} is inside the first poll period (boundary {}), so the loop polled the \
         latch without pacing",
        BASE_NS + PERIOD_NS
    );
}
