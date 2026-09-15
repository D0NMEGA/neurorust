//! The reachable state space and the transition function.
//!
//! `step` contains no atomics, no interior mutability and no allocation: it maps an owned state
//! and an owned event to an owned state. That is deliberate. It is the function plan 02-03's
//! Kani harnesses prove, and keeping it free of atomics keeps the proof clear of any question
//! about how a model checker models a concurrent primitive. The atomic publication layer lives
//! in `crate::latch` and is hand-argued rather than proved (D-50).

use crate::event::Event;

/// The whole reachable state space (STOP-01). Three states, no others, no reset (D-39).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(kani, derive(kani::Arbitrary))]
pub enum State {
    /// Output is permitted. The only state in which the gate issues a permit.
    Running,
    /// An abort has been latched and work already in flight is draining. New output is NOT
    /// permitted here: D-40 closes the permit supply on the edge into this state, not on the
    /// edge out of it, so safety never waits for anything to respond.
    Stopping,
    /// Terminal (D-39). There is no edge out of this state. Running again means constructing a
    /// fresh `EmergencyStop`.
    Stopped,
}

impl State {
    /// The wire form `crate::latch` stores in an atomic.
    pub const fn as_raw(self) -> u8 {
        match self {
            State::Running => 0,
            State::Stopping => 1,
            State::Stopped => 2,
        }
    }

    /// Total over every `u8`. Any value this crate never writes decodes to `Stopped`, the most
    /// restrictive state, so a corrupted or future-written encoding fails closed rather than
    /// reopening a stopped session.
    pub const fn from_raw(raw: u8) -> State {
        match raw {
            0 => State::Running,
            1 => State::Stopping,
            _ => State::Stopped,
        }
    }
}

/// The transition function. Total over the enumerated space by construction: every
/// `(State, Event)` pair has its own arm and there is no wildcard.
///
/// The exhaustive match is not stylistic. The project's Rust conventions forbid a wildcard arm
/// on a business-critical enum, and here it also means that adding a state or an event breaks
/// the build rather than silently falling into a catch-all that happens to be safe today.
pub fn step(state: State, event: Event) -> State {
    match (state, event) {
        (State::Running, Event::Abort { .. }) => State::Stopping,
        (State::Running, Event::Acknowledged) => State::Running,
        (State::Stopping, Event::Abort { .. }) => State::Stopping,
        (State::Stopping, Event::Acknowledged) => State::Stopped,
        (State::Stopped, Event::Abort { .. }) => State::Stopped,
        (State::Stopped, Event::Acknowledged) => State::Stopped,
    }
}
