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
/// Where a [`FixtureClock`]'s readings come from.
#[derive(Debug)]
enum Readings {
    /// An exact, finite sequence, which exhausts once every reading is claimed. This is what
    /// a test asserting the refusal path needs, so it stays the behaviour of
    /// [`FixtureClock::new`].
    Exact(Vec<u64>),
    /// Reading `n` computed on demand as `n * step_ns`. Never exhausts, because there is no
    /// budget to exhaust.
    Generated { step_ns: u64 },
}

#[derive(Debug)]
pub struct FixtureClock {
    readings: Readings,
    next: AtomicUsize,
}

impl FixtureClock {
    /// Builds a fixture clock over an exact, ordered sequence of readings. Exhausts.
    pub fn new(readings: impl Into<Vec<u64>>) -> Self {
        Self {
            readings: Readings::Exact(readings.into()),
            next: AtomicUsize::new(0),
        }
    }

    /// Builds a fixture clock that cannot run out: reading `n` is `n * step_ns`.
    ///
    /// Why this exists. How many readings a trial consumes depends on how many times the hot
    /// path polls before it observes the abort, which depends on when the aborting thread gets
    /// scheduled, which varies by machine. A fixed budget is therefore a bet about the slowest
    /// machine that will ever run the suite, and `phases_are_uniform_enough_to_state_so` lost
    /// that bet on a macOS CI runner: it exhausted a 2,000,000-reading budget at trial 3244 of
    /// 10,000 and never reached its assertion, while the same test passed on Linux and on the
    /// dev host. That assertion is deterministic (phases come from a seeded PRNG, not from
    /// timing), so a machine-independent test was failing for a machine-dependent reason.
    ///
    /// Use this wherever exhaustion is not the property under test. Use [`FixtureClock::new`]
    /// where it is.
    pub fn generated(step_ns: u64) -> Self {
        Self {
            readings: Readings::Generated { step_ns },
            next: AtomicUsize::new(0),
        }
    }
}

impl MonotonicRawClock for FixtureClock {
    /// Returns the next programmed reading, or `None` once an exact sequence is exhausted.
    /// Exhausted rather than wrapping, so a test can never accidentally measure a wrapped
    /// interval: once every reading has been claimed, every later call also returns `None`.
    /// A generated clock keeps the same guarantee by a different route: `checked_mul` returns
    /// `None` rather than wrapping if the index ever multiplies past `u64`.
    /// `fetch_add` hands out a strictly increasing, never-repeated index to concurrent callers,
    /// so two threads reading the same fixture never observe the same reading twice.
    fn now_ns(&self) -> Option<u64> {
        let index = self.next.fetch_add(1, Ordering::SeqCst);
        match &self.readings {
            Readings::Exact(readings) => readings.get(index).copied(),
            Readings::Generated { step_ns } => (index as u64).checked_mul(*step_ns),
        }
    }
}
