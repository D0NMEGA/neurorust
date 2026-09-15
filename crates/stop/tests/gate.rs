//! Tests for the output gate capability and the modelled consumer.
//!
//! `OutputPermit` has no public constructor by design (D-42, T-2-09), so any test that needs a
//! real one obtains it through `EmergencyStop::try_permit`, which task 3 adds. Until then, only
//! the tests that need no permit live here; task 3 appends the permit-consuming tests to this
//! same file rather than this file inventing a test-only constructor.

use nr_stop::consumer::ModelledConsumer;
use nr_stop::event::{Cause, Event};
use nr_stop::gate::would_issue_permit;
use nr_stop::state::{State, step};

#[test]
fn would_issue_permit_is_true_only_for_running() {
    assert!(would_issue_permit(State::Running));
    assert!(!would_issue_permit(State::Stopping));
    assert!(!would_issue_permit(State::Stopped));
}

#[test]
fn permit_supply_closes_on_the_abort_edge() {
    for state in [State::Running, State::Stopping, State::Stopped] {
        let next = step(
            state,
            Event::Abort {
                cause: Cause::Operator,
                at_raw_ns: 0,
            },
        );
        assert!(!would_issue_permit(next), "state={state:?}");
    }
}

#[test]
fn consumer_starts_at_zero() {
    let consumer = ModelledConsumer::new();
    assert_eq!(consumer.emitted(), 0);
}
