//! The D-33 stand-in hot path and the D-31 abort trial loop.
//!
//! Two long-lived threads, one per trial-count loop: the hot thread polls the latch once per
//! fixed-period iteration (D-32) until it observes the gate closed; the abort thread raises the
//! abort at a uniformly random phase within one poll period (D-34). Both pin themselves and
//! request `SCHED_FIFO` once, before the first trial, not per trial.
//!
//! A `Barrier` synchronises the two threads at the start of every trial. A barrier uses a mutex
//! and a condvar internally, which the hot-path rules forbid; this one is used only between
//! trials, to align trial boundaries, and is never touched inside the measured interval itself
//! (the interval runs from the `now_ns` read immediately before [`EmergencyStop::abort`] to the
//! `now_ns` read at first observation on the hot thread, with nothing else in between on either
//! side).

use std::sync::Barrier;

use thiserror::Error;

use nr_stop::event::Cause;
use nr_stop::latch::EmergencyStop;
use nr_stop::state::State;

use crate::clock::MonotonicRawClock;
use crate::sched::{SchedError, pin_current_thread, request_fifo};

/// Seed substituted for a caller-supplied `0`: xorshift64star is degenerate at a zero state,
/// producing zero forever, so a `0` seed is treated as "no seed chosen" rather than honoured
/// literally. Arbitrary and nonzero; not a cryptographic choice.
const FALLBACK_SEED: u64 = 0x9E37_79B9_7F4A_7C15;

#[derive(Debug, Error)]
pub enum TrialError {
    /// `trial` names which trial the clock stopped answering during, so a caller can tell a
    /// genuine refusal from a short run rather than seeing an anonymous failure.
    #[error("the clock stopped answering during trial {trial}")]
    ClockUnavailable { trial: u32 },
    #[error(
        "trial {trial} recorded an observed reading ({observed_raw_ns}) before its abort reading ({abort_raw_ns})"
    )]
    NonCausalOrdering {
        trial: u32,
        abort_raw_ns: u64,
        observed_raw_ns: u64,
    },
    #[error("failed to pin or schedule a trial thread: {0}")]
    Sched(#[from] SchedError),
    #[error("a trial thread panicked")]
    ThreadPanicked,
}

/// One run's configuration. Preserved verbatim in the published capture so a run is
/// reproducible from its own row of a headline table, not just from its raw samples.
#[derive(Debug, Clone)]
pub struct TrialConfig {
    /// The node iteration period. The latch is polled once per iteration at a fixed point in
    /// the loop (D-32), so this is one of the two terms D-34 reports separately.
    pub period_ns: u64,
    pub trials: usize,
    pub hot_cpu: usize,
    pub abort_cpu: usize,
    pub priority: u8,
    /// Seed for the abort-phase sequence, recorded so a run is reproducible.
    pub seed: u64,
    /// When `true` (a real run), a failure to pin a thread or to obtain `SCHED_FIFO` is fatal:
    /// a placement- or scheduling-sensitive figure taken without either guarantee is not
    /// defensible under this project's own rig discipline (D-36). Tests set this to `false` so
    /// the same trial loop is exercised without asserting a guarantee the test host cannot
    /// give.
    ///
    /// Both failure modes are gated by this one flag, not two: `core_affinity::set_for_current`
    /// was measured on the macOS dev host (Apple Silicon) and returns `false` unconditionally,
    /// for every cpu id, single-threaded or not, contradicting the graceful "requests highest
    /// performance instead of failing" fallback the phase's own research described as an
    /// unverified, medium-confidence claim. `thread_priority`'s `SCHED_FIFO` request fails
    /// there too, exactly as expected. Neither limitation reflects anything about the reference
    /// rig, which runs Linux, where both wrappers call the real `sched_setaffinity`/
    /// `pthread_setschedparam` and this flag is always `true`.
    pub require_realtime_scheduling: bool,
}

/// One row per trial: the phase within the period at which the abort was raised, the two raw
/// clock readings, and their difference. Every column is recorded so a reader can re-derive the
/// latency rather than trust it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrialRow {
    pub trial: u32,
    pub phase_ns: u64,
    pub abort_raw_ns: u64,
    pub observed_raw_ns: u64,
    pub latency_ns: u64,
}

#[derive(Debug, Clone)]
pub struct TrialOutcome {
    pub rows: Vec<TrialRow>,
}

/// One `(phase_ns, abort_raw_ns)` pair per trial, in trial order, from the abort thread.
type AbortSideReadings = Vec<(u64, u64)>;

/// Runs `config.trials` abort trials and returns one [`TrialRow`] per trial.
///
/// Generic over the clock rather than a trait object, so the real clock's read inlines on the
/// rig and the same loop is exercised on macOS through [`crate::clock::FixtureClock`].
pub fn run_trials<C: MonotonicRawClock + Sync>(
    clock: &C,
    config: &TrialConfig,
) -> Result<TrialOutcome, TrialError> {
    // Preallocated before the first trial, per D-33's hot-path discipline: allocating between
    // trials would put the allocator inside the thing being measured.
    let stops: Vec<EmergencyStop> = (0..config.trials).map(|_| EmergencyStop::new()).collect();
    let barrier = Barrier::new(2);

    let (abort_results, hot_results) = std::thread::scope(
        |scope| -> Result<(AbortSideReadings, Vec<u64>), TrialError> {
            let hot_handle = scope.spawn(|| -> Result<Vec<u64>, TrialError> {
                setup_realtime_scheduling(config.hot_cpu, config)?;

                let mut observed = Vec::with_capacity(config.trials);
                for (trial, stop) in stops.iter().enumerate() {
                    let reading =
                        hot_side_trial(clock, stop, &barrier, config.period_ns, trial as u32)?;
                    observed.push(reading);
                }
                Ok(observed)
            });

            setup_realtime_scheduling(config.abort_cpu, config)?;

            let mut rng = if config.seed == 0 {
                FALLBACK_SEED
            } else {
                config.seed
            };
            let mut abort_results = Vec::with_capacity(config.trials);
            for (trial, stop) in stops.iter().enumerate() {
                let phase = next_phase(&mut rng) % config.period_ns.max(1);
                let abort_raw_ns = abort_side_trial(clock, stop, &barrier, phase, trial as u32)?;
                abort_results.push((phase, abort_raw_ns));
            }

            let hot_results = hot_handle
                .join()
                .map_err(|_| TrialError::ThreadPanicked)??;

            Ok((abort_results, hot_results))
        },
    )?;

    let mut rows = Vec::with_capacity(config.trials);
    for (i, ((phase_ns, abort_raw_ns), observed_raw_ns)) in
        abort_results.into_iter().zip(hot_results).enumerate()
    {
        let trial = i as u32;
        let latency_ns =
            observed_raw_ns
                .checked_sub(abort_raw_ns)
                .ok_or(TrialError::NonCausalOrdering {
                    trial,
                    abort_raw_ns,
                    observed_raw_ns,
                })?;
        rows.push(TrialRow {
            trial,
            phase_ns,
            abort_raw_ns,
            observed_raw_ns,
            latency_ns,
        });
    }

    Ok(TrialOutcome { rows })
}

/// Pins the calling thread to `cpu` and requests `SCHED_FIFO`, honouring
/// [`TrialConfig::require_realtime_scheduling`]: both failures are fatal when `true`, both
/// tolerated when `false`. Never silently disabled on a real run; the caller (here,
/// [`run_trials`]) is always the one that chose to allow the bypass, never this function
/// deciding on its own.
fn setup_realtime_scheduling(cpu: usize, config: &TrialConfig) -> Result<(), TrialError> {
    if config.require_realtime_scheduling {
        pin_current_thread(cpu)?;
        request_fifo(config.priority)?;
    } else {
        let _ = pin_current_thread(cpu);
        let _ = request_fifo(config.priority);
    }
    Ok(())
}

/// The hot thread's work for one trial (D-32): waits on `barrier`, then polls `stop` once per
/// fixed-period iteration until it observes the gate closed, at which point it takes the
/// timestamp that ends the measured interval (D-31), acknowledges, and returns that reading.
///
/// Exposed so tests can exercise it directly against a caller-owned [`EmergencyStop`], since
/// [`TrialOutcome`] only ever returns [`TrialRow`]s, not the latch instances a multi-trial run
/// constructs internally.
pub fn hot_side_trial<C: MonotonicRawClock>(
    clock: &C,
    stop: &EmergencyStop,
    barrier: &Barrier,
    period_ns: u64,
    trial: u32,
) -> Result<u64, TrialError> {
    barrier.wait();

    let period_ns = period_ns.max(1);
    // Anchored to a reading from the same clock the deadline is later compared against, never to
    // zero. CLOCK_MONOTONIC_RAW counts nanoseconds since boot, so on any machine that has been up
    // for more than an instant every reading dwarfs a poll period measured in microseconds. A
    // deadline of `period_ns` alone is therefore already in the past at the first comparison: the
    // busy-wait below returns without waiting, `next_deadline += period_ns` never catches up, and
    // this loop degenerates into an unpaced spin that polls the latch as fast as it can read the
    // clock. `busy_wait_ns` in this same file always anchored correctly; this loop did not.
    //
    // It survived to the rig because every fixture clock in the test suite starts at 0, which is
    // the one origin where the broken arithmetic behaves. The 2026-09-16 33 us capture is what it
    // cost: a 222 ns median for a 33,000 ns poll period, a figure with no poll period in it.
    let mut next_deadline = clock
        .now_ns()
        .ok_or(TrialError::ClockUnavailable { trial })?
        .saturating_add(period_ns);
    loop {
        // The fixed point in the iteration where the latch is polled (D-32).
        if stop.state() != State::Running {
            let observed = clock
                .now_ns()
                .ok_or(TrialError::ClockUnavailable { trial })?;
            stop.acknowledge();
            return Ok(observed);
        }

        // Busy-wait to the next period boundary. No sleep: a blocking syscall here would
        // measure the scheduler's wakeup rather than the latch's propagation.
        loop {
            let now = clock
                .now_ns()
                .ok_or(TrialError::ClockUnavailable { trial })?;
            if now >= next_deadline {
                break;
            }
            std::hint::spin_loop();
        }
        next_deadline += period_ns;
    }
}

/// The abort thread's work for one trial: waits on `barrier`, busy-waits `phase_ns` (the D-34
/// uniformly random abort phase, already reduced modulo the period by the caller), then reads
/// the clock and raises the abort immediately after. Nothing else sits between the `now_ns`
/// call and [`EmergencyStop::abort`]: a log line, a bounds check, a debug assertion, would
/// silently inflate the published figure.
///
/// Exposed for the same reason as [`hot_side_trial`].
pub fn abort_side_trial<C: MonotonicRawClock>(
    clock: &C,
    stop: &EmergencyStop,
    barrier: &Barrier,
    phase_ns: u64,
    trial: u32,
) -> Result<u64, TrialError> {
    barrier.wait();
    busy_wait_ns(clock, phase_ns, trial)?;
    let at = clock
        .now_ns()
        .ok_or(TrialError::ClockUnavailable { trial })?;
    stop.abort(Cause::Operator, at);
    Ok(at)
}

/// Busy-waits until at least `duration_ns` has elapsed on `clock`, measured from a reading
/// taken at entry. No sleep, for the same reason as the hot thread's period wait.
fn busy_wait_ns<C: MonotonicRawClock>(
    clock: &C,
    duration_ns: u64,
    trial: u32,
) -> Result<(), TrialError> {
    if duration_ns == 0 {
        return Ok(());
    }
    let start = clock
        .now_ns()
        .ok_or(TrialError::ClockUnavailable { trial })?;
    let deadline = start.saturating_add(duration_ns);
    loop {
        let now = clock
            .now_ns()
            .ok_or(TrialError::ClockUnavailable { trial })?;
        if now >= deadline {
            return Ok(());
        }
        std::hint::spin_loop();
    }
}

/// A xorshift64star pseudo-random generator. Not a cryptographic choice and does not need to be
/// one: the requirement is a uniform phase within one poll period, this is eight lines, and
/// adding a dependency with a transitive tree for it would be the opposite of the project's
/// don't-hand-roll rule, which is about not reimplementing hard things, not about never writing
/// eight lines.
fn next_phase(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    *state = x;
    x.wrapping_mul(0x2545_F491_4F6C_DD1D)
}
