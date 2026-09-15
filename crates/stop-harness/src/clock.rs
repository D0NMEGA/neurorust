//! `CLOCK_MONOTONIC_RAW` reads, behind a trait so the platform-independent half of the harness
//! is exercised by `cargo test` on the macOS CI leg rather than cfg'd out of existence. This is
//! the same shape `nr-capture` uses for `SystemFacts` with `LiveFacts` and `FixtureFacts`.
//!
//! Why not `std::time::Instant`: on Linux, `Instant` is implemented over `CLOCK_MONOTONIC`, not
//! `CLOCK_MONOTONIC_RAW`. D-35 requires the raw clock at both ends, which is also the project's
//! chosen clock for WIRE-03's frame header, so the two sides of the system agree on what time
//! means. That requires an explicit `clock_gettime` call.

use std::sync::atomic::{AtomicUsize, Ordering};

/// Reads a monotonic raw clock. `None` means the clock could not be read at all; every caller
/// must treat that as a refusal to produce a figure, never as a zero.
pub trait MonotonicRawClock {
    fn now_ns(&self) -> Option<u64>;
}

/// The real clock: `CLOCK_MONOTONIC_RAW` on Linux, refused everywhere else.
#[derive(Debug, Clone, Copy, Default)]
pub struct RawClock;

#[cfg(target_os = "linux")]
impl MonotonicRawClock for RawClock {
    fn now_ns(&self) -> Option<u64> {
        let reading = nix::time::clock_gettime(nix::time::ClockId::CLOCK_MONOTONIC_RAW).ok()?;
        let whole_seconds_ns = u64::try_from(reading.tv_sec())
            .ok()?
            .checked_mul(1_000_000_000)?;
        let remaining_nanos = u64::try_from(reading.tv_nsec()).ok()?;
        whole_seconds_ns.checked_add(remaining_nanos)
    }
}

#[cfg(not(target_os = "linux"))]
impl MonotonicRawClock for RawClock {
    fn now_ns(&self) -> Option<u64> {
        // CLOCK_MONOTONIC_RAW is a Linux-only clock. Refuse rather than silently substitute
        // CLOCK_MONOTONIC or std::time::Instant (which itself reads CLOCK_MONOTONIC on Linux,
        // not the raw clock D-35 requires): a figure produced from a different clock than the
        // one it claims is worse than a harness that refuses (T-2-35).
        None
    }
}

/// A programmed sequence of readings, for tests. Available on every platform, which is what
/// lets this crate's platform-independent behaviour be exercised by `cargo test` on macOS.
///
/// Backed by an atomic index, not a `Cell`, so one instance can be shared by reference across
/// the two real threads `trial::run_trials` spawns (`C: MonotonicRawClock + Sync`): the
/// abort-latency trial loop and the D-35 cross-core offset measurement are both exercised by
/// tests through this same fixture, and both are inherently multi-threaded.
#[derive(Debug)]
pub struct FixtureClock {
    readings: Vec<u64>,
    next: AtomicUsize,
}

impl FixtureClock {
    /// Builds a fixture clock over an exact, ordered sequence of readings.
    pub fn new(readings: impl Into<Vec<u64>>) -> Self {
        Self {
            readings: readings.into(),
            next: AtomicUsize::new(0),
        }
    }
}

impl MonotonicRawClock for FixtureClock {
    /// Returns the next programmed reading, or `None` once the sequence is exhausted.
    /// Exhausted rather than wrapping, so a test can never accidentally measure a wrapped
    /// interval: once every reading has been claimed, every later call also returns `None`.
    /// `fetch_add` hands out a strictly increasing, never-repeated index to concurrent callers,
    /// so two threads reading the same fixture never observe the same reading twice.
    fn now_ns(&self) -> Option<u64> {
        let index = self.next.fetch_add(1, Ordering::SeqCst);
        self.readings.get(index).copied()
    }
}
