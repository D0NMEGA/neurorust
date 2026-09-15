//! D-35's second deliverable, and it is not a footnote on the first: the clock's own read
//! overhead, the cross-core offset between two threads, and the running clocksource, each
//! measured by its own code path rather than assumed away. Plan 02-08 publishes these beside the
//! abort-latency figure rather than folding them into it.

use std::path::Path;
use std::sync::Barrier;
use std::sync::atomic::{AtomicU64, Ordering};

use thiserror::Error;

use crate::clock::MonotonicRawClock;
use crate::sched::{SchedError, pin_current_thread};

#[derive(Debug, Error)]
pub enum CharacteriseError {
    /// `reads` is how many readings were successfully taken before the clock stopped
    /// answering, so a caller can tell a genuine refusal from a short run.
    #[error("the clock stopped answering after {reads} reads")]
    ClockUnavailable { reads: usize },
    #[error("consecutive clock readings went backwards: {earlier} then {later}")]
    NonMonotonic { earlier: u64, later: u64 },
    #[error("clock reading {value} does not fit in a signed 64-bit nanosecond count")]
    TimestampOutOfRange { value: u64 },
    #[error("failed to pin a characterisation thread: {0}")]
    Sched(#[from] SchedError),
    #[error("a characterisation thread panicked")]
    ThreadPanicked,
}

/// Takes `iterations + 1` readings in a tight loop with nothing between them, and returns the
/// `iterations` consecutive deltas. Each delta is one clock read plus one loop iteration, which
/// is what a caller of this clock actually pays; that is deliberately not called "the syscall
/// cost", because `CLOCK_MONOTONIC_RAW` has been vDSO-accelerated since Linux 5.3 and the rig is
/// far past that, so the expectation is tens to low hundreds of nanoseconds, and the point of
/// this function is that the number is measured rather than assumed.
pub fn read_overhead_ns<C: MonotonicRawClock>(
    clock: &C,
    iterations: usize,
) -> Result<Vec<u64>, CharacteriseError> {
    let mut readings = Vec::with_capacity(iterations + 1);
    for reads in 0..=iterations {
        let reading = clock
            .now_ns()
            .ok_or(CharacteriseError::ClockUnavailable { reads })?;
        readings.push(reading);
    }

    readings
        .windows(2)
        .map(|pair| {
            pair[1]
                .checked_sub(pair[0])
                .ok_or(CharacteriseError::NonMonotonic {
                    earlier: pair[0],
                    later: pair[1],
                })
        })
        .collect()
}

/// The Cristian's-algorithm round-trip offset estimate for one ping-pong round: `T1` is the
/// sender's send time, `T2` is the receiver's own clock reading on receipt, `T3` is the
/// sender's clock reading on the reply. This is a well-known general technique, applied here by
/// analogy to same-machine, invariant-TSC, cross-core `CLOCK_MONOTONIC_RAW` skew; no source
/// specific to this exact scenario was found, so the application is argued, not quoted. It
/// assumes symmetric propagation delay in both directions of the exchange, an assumption stated
/// here rather than buried in the published figure that uses it.
///
/// Every reading is converted with `i64::try_from` before any subtraction, so a timestamp large
/// enough to overflow a signed 64-bit nanosecond count is refused rather than silently wrapped.
pub fn offset_estimate_ns(t1: u64, t2: u64, t3: u64) -> Result<i64, CharacteriseError> {
    let signed = |value: u64| {
        i64::try_from(value).map_err(|_| CharacteriseError::TimestampOutOfRange { value })
    };
    let (t1, t2, t3) = (signed(t1)?, signed(t2)?, signed(t3)?);
    Ok(((t2 - t1) + (t2 - t3)) / 2)
}

/// Runs `rounds` ping-pong exchanges between a thread pinned to `cpu_a` and a thread pinned to
/// `cpu_b`, and returns one [`offset_estimate_ns`] per round.
///
/// `require_pinning` governs whether a pin failure is fatal, matching
/// `TrialConfig::require_realtime_scheduling`'s own split (`crate::trial`): `true` on a real run,
/// where a cross-core skew measurement that silently ran on the wrong cores would look measured
/// and would not be (the same concern D-36 names for the abort-latency run), and `false` under
/// this harness's fixture mode, where `core_affinity::set_for_current` returns `false`
/// unconditionally on the macOS dev host regardless of which cpu id is requested. Without this
/// bypass, `run_characterise`'s fixture path could never complete on macOS at all, which is
/// exactly the integration gap plan 02-07's own dry run exists to catch.
///
/// `rounds` and `cpu_a`/`cpu_b` are not validated against the rig's isolated set here; the
/// caller (plan 02-08's rig invocation) is responsible for passing the isolated cores.
pub fn cross_core_offset_ns<C: MonotonicRawClock + Sync>(
    clock: &C,
    cpu_a: usize,
    cpu_b: usize,
    rounds: usize,
    require_pinning: bool,
) -> Result<Vec<i64>, CharacteriseError> {
    let ping = AtomicU64::new(0);
    let pong = AtomicU64::new(0);
    // Both threads pin and request readiness before either one reads a clock for the first
    // exchange, so a slow-to-pin thread cannot inflate round 1's estimate.
    let ready = Barrier::new(2);

    std::thread::scope(|scope| -> Result<Vec<i64>, CharacteriseError> {
        let b_thread = scope.spawn(|| -> Result<Vec<u64>, CharacteriseError> {
            pin_for_characterisation(cpu_b, require_pinning)?;
            ready.wait();
            let mut readings = Vec::with_capacity(rounds);
            for round in 1..=rounds as u64 {
                while ping.load(Ordering::Acquire) != round {
                    std::hint::spin_loop();
                }
                let t2 = clock.now_ns().ok_or(CharacteriseError::ClockUnavailable {
                    reads: readings.len(),
                })?;
                readings.push(t2);
                pong.store(round, Ordering::Release);
            }
            Ok(readings)
        });

        pin_for_characterisation(cpu_a, require_pinning)?;
        ready.wait();
        let mut a_readings = Vec::with_capacity(rounds);
        for round in 1..=rounds as u64 {
            let t1 = clock.now_ns().ok_or(CharacteriseError::ClockUnavailable {
                reads: a_readings.len(),
            })?;
            ping.store(round, Ordering::Release);
            while pong.load(Ordering::Acquire) != round {
                std::hint::spin_loop();
            }
            let t3 = clock.now_ns().ok_or(CharacteriseError::ClockUnavailable {
                reads: a_readings.len(),
            })?;
            a_readings.push((t1, t3));
        }

        let b_readings = b_thread
            .join()
            .map_err(|_| CharacteriseError::ThreadPanicked)??;

        a_readings
            .into_iter()
            .zip(b_readings)
            .map(|((t1, t3), t2)| offset_estimate_ns(t1, t2, t3))
            .collect()
    })
}

/// Pins the calling thread to `cpu`, honouring `require_pinning`: fatal when `true`, tolerated
/// when `false`. Mirrors `crate::trial::setup_realtime_scheduling`'s own split, scoped to pinning
/// only, since this function requests no scheduling policy.
fn pin_for_characterisation(cpu: usize, require_pinning: bool) -> Result<(), CharacteriseError> {
    if require_pinning {
        pin_current_thread(cpu)?;
    } else {
        let _ = pin_current_thread(cpu);
    }
    Ok(())
}

/// Reads `<sys_root>/devices/system/clocksource/clocksource0/current_clocksource`, trimmed.
/// `sys_root` is a parameter rather than a hardcoded `/sys` so a test can point it at a
/// temporary directory, which is how this gets covered on macOS. Returns `None` when the file
/// is absent or empty, never a guess: an unreadable clocksource is a fact worth publishing
/// plainly, not worth papering over with an assumed value.
pub fn current_clocksource(sys_root: &Path) -> Option<String> {
    let path = sys_root.join("devices/system/clocksource/clocksource0/current_clocksource");
    let text = std::fs::read_to_string(path).ok()?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
