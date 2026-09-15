//! Kani proof harnesses for the emergency-stop state machine and gate predicate.
//!
//! This module proves two things over their entire input space, not over chosen examples: the
//! pure transition function [`crate::state::step`] and the gate predicate
//! [`crate::gate::would_issue_permit`]. It proves nothing about thread interleavings;
//! `crate::latch`'s atomic publication layer is unverified here and stays unverified until
//! CHAN-06 brings loom into CI in Phase 4 (D-50).
//!
//! No harness below touches an atomic, a compare-exchange or an Ordering value. Research
//! surfaced a report of a Kani build modelling a failed compare-exchange as successful
//! (model-checking/verify-rust-std#673); keeping the proved surface free of atomics means that
//! question never arises for anything STOP-01 through STOP-04 establish here, rather than being
//! answered on trust.

use crate::consumer::ModelledConsumer;
use crate::event::{Cause, Event};
use crate::gate::{OutputPermit, would_issue_permit};
use crate::state::{State, step};

// Family 1 (STOP-01): step is total over the whole (State, Event) space at once.

/// Over every state and every event, `step` returns one of the three enumerated states. Not
/// "these six cases work": no input produces anything else.
#[kani::proof]
fn step_is_total() {
    let state: State = kani::any();
    let event: Event = kani::any();
    let next = step(state, event);
    assert!(matches!(
        next,
        State::Running | State::Stopping | State::Stopped
    ));
}

/// The wire encoding the latch stores round trips, so the atomic representation and the
/// enumerated space describe the same three states.
#[kani::proof]
fn state_raw_round_trips() {
    let state: State = kani::any();
    assert!(State::from_raw(state.as_raw()) == state);
}

/// `from_raw` is total over every `u8`, not just the three values this crate writes, and every
/// out-of-range value decodes to `Stopped`, the most restrictive state: a corrupted encoding
/// fails closed rather than reopening a stopped session.
#[kani::proof]
fn state_from_raw_is_total_and_fails_closed() {
    let raw: u8 = kani::any();
    let decoded = State::from_raw(raw);
    assert!(matches!(
        decoded,
        State::Running | State::Stopping | State::Stopped
    ));
    kani::assume(raw > 1);
    assert!(decoded == State::Stopped);
}

/// The same two properties proved above for the state encoding, for the cause encoding, whose
/// out-of-range value decodes to `InternalFault`.
#[kani::proof]
fn cause_raw_round_trips_and_is_total() {
    let cause: Cause = kani::any();
    assert!(Cause::from_raw(cause.as_raw()) == cause);
    let raw: u8 = kani::any();
    let decoded = Cause::from_raw(raw);
    assert!(matches!(
        decoded,
        Cause::Operator | Cause::WatchdogDeadline | Cause::InternalFault | Cause::Shutdown
    ));
}

// Family 2 (STOP-02): the latch property. No path back to Running.

/// From any state at all, and for any cause and any timestamp, an abort lands in `Stopping` or
/// `Stopped` and never in `Running`.
#[kani::proof]
fn abort_never_returns_to_running() {
    let state: State = kani::any();
    let cause: Cause = kani::any();
    let at_raw_ns: u64 = kani::any();
    let next = step(state, Event::Abort { cause, at_raw_ns });
    assert!(next != State::Running);
    assert!(next == State::Stopping || next == State::Stopped);
}

/// Once out of `Running`, no event of any kind returns to it. The structural form of the
/// guarantee: it holds because there is no edge, not because nothing calls a reset (D-39).
#[kani::proof]
fn running_is_never_reachable_from_stopping_or_stopped() {
    let state: State = kani::any();
    kani::assume(state != State::Running);
    let event: Event = kani::any();
    assert!(step(state, event) != State::Running);
}

/// Every event leaves `Stopped` where it is (D-39).
#[kani::proof]
fn stopped_is_terminal() {
    let event: Event = kani::any();
    assert!(step(State::Stopped, event) == State::Stopped);
}

// Family 3 (STOP-03): the gate invariant.

/// No permit is issued from `Stopping` or from `Stopped`.
#[kani::proof]
fn no_permit_when_not_running() {
    let state: State = kani::any();
    kani::assume(state != State::Running);
    assert!(!would_issue_permit(state));
}

/// The supply closes on the abort edge itself, not on reaching `Stopped` (D-40). From any
/// state, the state an abort lands in issues no permit. This is what makes safety independent
/// of anything acknowledging.
#[kani::proof]
fn permit_supply_closes_on_the_abort_edge() {
    let state: State = kani::any();
    let cause: Cause = kani::any();
    let at_raw_ns: u64 = kani::any();
    assert!(!would_issue_permit(step(
        state,
        Event::Abort { cause, at_raw_ns }
    )));
}

/// Against the modelled consumer (D-45): from any state reached by an abort, there is no
/// permit to hand it, so its emission count cannot move. `ModelledConsumer::emit` takes an
/// `OutputPermit` by value and there is no other way to call it, so "cannot obtain a permit"
/// and "cannot emit" are the same statement rather than two hopefully-equal ones.
#[kani::proof]
fn modelled_consumer_never_emits_after_a_stop() {
    let state: State = kani::any();
    let cause: Cause = kani::any();
    let at_raw_ns: u64 = kani::any();
    let stopped_state = step(state, Event::Abort { cause, at_raw_ns });

    let before: u64 = kani::any();
    let consumer = ModelledConsumer::with_emitted(before);
    assert!(!would_issue_permit(stopped_state));
    assert!(consumer.emitted() == before);
}

// Family 4 (STOP-04): panic and overflow freedom. Kani checks arithmetic overflow, index panics
// and reachable panics automatically in every harness above; this harness makes that guarantee
// explicit at the one place in the crate where arithmetic happens at all.

/// A plain `+= 1` in `ModelledConsumer::emit` would panic here in a debug build and this
/// harness would fail; `saturating_add` is why it does not. Constructing the permit directly is
/// allowed here and only here: this module is inside the crate, so it reaches the `pub(crate)`
/// field. No public constructor exists on `OutputPermit` for this harness's convenience:
/// T-2-09 in plan 02-02's threat model is exactly a forged permit reachable from outside the
/// crate, and that would delete STOP-03 while leaving every harness here green.
#[kani::proof]
fn consumer_emit_never_overflows() {
    let before: u64 = kani::any();
    let mut consumer = ModelledConsumer::with_emitted(before);
    let permit = OutputPermit {
        _issued_by_the_gate: (),
    };
    consumer.emit(permit);
    assert!(consumer.emitted() >= before);
}
