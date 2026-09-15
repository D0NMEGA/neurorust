//! Tests for the output gate capability and the modelled consumer.
//!
//! `OutputPermit` has no public constructor by design (D-42, T-2-09). The two permit-consuming
//! tests below obtain real permits through `EmergencyStop::try_permit`, added by task 3, rather
//! than this file inventing a test-only constructor.

use nr_stop::consumer::ModelledConsumer;
use nr_stop::event::{Cause, Event};
use nr_stop::gate::would_issue_permit;
use nr_stop::latch::EmergencyStop;
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

#[test]
fn consumer_counts_one_emission_per_permit() {
    let stop = EmergencyStop::new();
    let mut consumer = ModelledConsumer::new();

    let first = stop.try_permit().expect("a fresh machine issues permits");
    consumer.emit(first);
    let second = stop.try_permit().expect("still running after one emission");
    consumer.emit(second);

    assert_eq!(consumer.emitted(), 2);
}

#[test]
fn consumer_saturates_rather_than_overflowing() {
    let stop = EmergencyStop::new();
    let mut consumer = ModelledConsumer::with_emitted(u64::MAX);

    let permit = stop.try_permit().expect("a fresh machine issues permits");
    consumer.emit(permit);

    assert_eq!(consumer.emitted(), u64::MAX);
}
