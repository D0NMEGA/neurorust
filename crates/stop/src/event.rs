//! The events the machine accepts, and why an abort happened.

/// Why an abort was raised (D-44). All four variants exist from the start even though only
/// `Operator` has a caller in Phase 2, because the enum is part of the state space plan 02-03
/// proves, and adding a variant later reopens the proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(kani, derive(kani::Arbitrary))]
pub enum Cause {
    /// An operator asked for the stop. The only variant with a caller in Phase 2.
    Operator,
    /// A watchdog observed a missed deadline. The detector needs the per-node WCET annotations
    /// GRAPH-01 introduces in Phase 5; the variant is named now, not then.
    WatchdogDeadline,
    /// The runtime detected its own fault.
    InternalFault,
    /// An orderly shutdown, which takes the same path as any other abort because there is no
    /// second, gentler way out of a session.
    Shutdown,
}

impl Cause {
    /// The wire form the latch stores in an atomic.
    pub const fn as_raw(self) -> u8 {
        match self {
            Cause::Operator => 0,
            Cause::WatchdogDeadline => 1,
            Cause::InternalFault => 2,
            Cause::Shutdown => 3,
        }
    }

    /// Total over every `u8`. A value this crate never writes decodes to `InternalFault`, since
    /// a cause code that cannot be accounted for is a fault by definition and saying so is more
    /// useful to a reader of an abort record than a panic would be. `unwrap` and
    /// `unreachable!` are both excluded here on purpose: plan 02-03 family 4 asserts no panic is
    /// reachable in this crate, and a total decode is how that is earned rather than asserted.
    pub const fn from_raw(raw: u8) -> Cause {
        match raw {
            0 => Cause::Operator,
            1 => Cause::WatchdogDeadline,
            3 => Cause::Shutdown,
            _ => Cause::InternalFault,
        }
    }
}

/// What the machine reacts to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(kani, derive(kani::Arbitrary))]
pub enum Event {
    /// D-43. Carries the cause and the `CLOCK_MONOTONIC_RAW` reading the aborting thread took
    /// immediately before it latched, so the machine holds one end of the interval STOP-07
    /// measures rather than that end living somewhere else and being hoped to coincide.
    Abort { cause: Cause, at_raw_ns: u64 },
    /// D-41. Raised by the polling node at the iteration where it first observes the gate
    /// closed. This is the end of the measured interval. It is NOT the moment the system
    /// becomes safe: that is the abort, per D-40.
    Acknowledged,
}
