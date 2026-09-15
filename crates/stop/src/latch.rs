//! The atomic publication layer around [`crate::state::step`].
//!
//! # What is proved and what is argued
//!
//! Nothing in this file is proved by Kani (D-50). Kani does not explore thread interleavings,
//! and this file is where the crate's concurrency lives. Plan 02-03 proves
//! [`crate::state::step`], which this file calls; the orderings below are argued in prose and
//! stay argued until CHAN-06 brings loom into CI in Phase 4. That gap is stated in
//! `docs/proofs/emergency-stop-proof-scope.md` as part of the published claim, and it is
//! tracked into Phase 4's scope rather than left to be discovered there.
//!
//! # The ordering argument
//!
//! `state` is the only atomic on the hot path and the only one that is compare-exchanged. The
//! abort's `Running -> Stopping` transition is a compare-exchange with `Ordering::AcqRel` on
//! success and `Ordering::Acquire` on failure: the release half publishes everything the winner
//! writes afterwards to any thread that later acquire-loads `state`, and the acquire half on
//! failure lets a losing caller safely observe the winner's record. The hot-path poll is
//! `load(Acquire)`, which is what must
//! synchronize-with that release. A `Relaxed` load there would be a real bug rather than
//! excess caution, because D-43 makes the cause and the timestamp values that must become
//! visible together with the state flip.
//!
//! `SeqCst` is deliberately not used anywhere: nothing here needs a total order across
//! different atomics, so it would be a cost paid for no argued benefit and an unexamined choice
//! left for Phase 4 to inherit.
//!
//! Only the compare-exchange WINNER writes the abort record, and it publishes the record with a
//! `Release` store to `record_published` last. A loser's record is discarded. Writing the record
//! before the exchange would let a loser's relaxed writes overwrite the winner's, so the
//! published cause could name one thread while the transition belonged to another.
//!
//! # Safety and the measurement are two different instants
//!
//! [`EmergencyStop::abort`] closes the permit supply (D-40). [`EmergencyStop::acknowledge`] ends
//! the interval STOP-07 measures (D-41). A node that hangs and never acknowledges makes the
//! number unobtainable and leaves safety intact. Nothing here may be described as the system
//! becoming safe at the acknowledgement.

use core::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};

use crate::event::{Cause, Event};
use crate::gate::{OutputPermit, would_issue_permit};
use crate::state::{State, step};

/// Why an abort happened and when the aborting thread observed it, as recorded by the caller
/// that won the transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AbortRecord {
    pub cause: Cause,
    /// The `CLOCK_MONOTONIC_RAW` reading the aborting thread took immediately before it latched
    /// (D-31, D-43). This is the start of the interval STOP-07 publishes.
    pub at_raw_ns: u64,
}

/// The abort latch. Shareable across threads by reference; `abort` is callable from any of them
/// (D-30). There is no reset and no way back to `Running` (D-39): running again means
/// constructing a fresh value.
#[derive(Debug)]
pub struct EmergencyStop {
    state: AtomicU8,
    cause: AtomicU8,
    aborted_at_raw_ns: AtomicU64,
    record_published: AtomicBool,
}

impl EmergencyStop {
    pub const fn new() -> Self {
        Self {
            state: AtomicU8::new(State::Running.as_raw()),
            cause: AtomicU8::new(Cause::Operator.as_raw()),
            aborted_at_raw_ns: AtomicU64::new(0),
            record_published: AtomicBool::new(false),
        }
    }

    /// The whole hot-path poll (D-32): one acquire load and a total decode, no branch beyond the
    /// decode, nothing else. Keep it that way; the branch count in this crate is what makes plan
    /// 02-04's 100 percent gate an honest property of a small module rather than an exercise.
    pub fn state(&self) -> State {
        State::from_raw(self.state.load(Ordering::Acquire))
    }

    /// Attempts the `Running -> Stopping` edge (D-40). The destination comes from the proved
    /// [`step`] rather than a second, hand-written transition table. Returns `true` for the
    /// caller that wins the compare-exchange, which is also the only caller that writes the
    /// abort record; a losing caller (the machine was already `Stopping` or `Stopped`) returns
    /// `false` and writes nothing, so the first aborter's record can never be overwritten.
    pub fn abort(&self, cause: Cause, at_raw_ns: u64) -> bool {
        let next = step(State::Running, Event::Abort { cause, at_raw_ns });
        let won = self
            .state
            .compare_exchange(
                State::Running.as_raw(),
                next.as_raw(),
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok();
        if won {
            self.cause.store(cause.as_raw(), Ordering::Relaxed);
            self.aborted_at_raw_ns.store(at_raw_ns, Ordering::Relaxed);
            self.record_published.store(true, Ordering::Release);
        }
        won
    }

    /// Attempts the `Stopping -> Stopped` edge (D-41), the end of the interval STOP-07 measures.
    /// Same shape as [`Self::abort`] against the proved [`step`]. Writes no record: the abort
    /// record belongs entirely to whichever caller won `abort`.
    pub fn acknowledge(&self) -> bool {
        let next = step(State::Stopping, Event::Acknowledged);
        self.state
            .compare_exchange(
                State::Stopping.as_raw(),
                next.as_raw(),
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
    }

    /// Issues a permit only while the loaded state is `Running` (D-40, D-42). Loads the state
    /// once and branches on the loaded value, rather than calling `self.state()` twice.
    pub fn try_permit(&self) -> Option<OutputPermit> {
        let state = State::from_raw(self.state.load(Ordering::Acquire));
        if would_issue_permit(state) {
            Some(OutputPermit {
                _issued_by_the_gate: (),
            })
        } else {
            None
        }
    }

    /// The winning aborter's record, once published. `None` before any `abort` has succeeded.
    /// The acquire load of `record_published` is what makes the two relaxed reads behind it
    /// sound: it synchronizes-with the winner's release store, so a caller that observes `true`
    /// here also observes the winner's `cause`/`aborted_at_raw_ns` writes.
    pub fn abort_record(&self) -> Option<AbortRecord> {
        if !self.record_published.load(Ordering::Acquire) {
            return None;
        }
        Some(AbortRecord {
            cause: Cause::from_raw(self.cause.load(Ordering::Relaxed)),
            at_raw_ns: self.aborted_at_raw_ns.load(Ordering::Relaxed),
        })
    }
}

impl Default for EmergencyStop {
    fn default() -> Self {
        Self::new()
    }
}
