//! The transition function's six arms, and totality of both raw encodings.
//!
//! Lives under `tests/`, not a `#[cfg(test)]` module in `src/`, because plan 02-04's coverage
//! gate runs `--ignore-filename-regex '(^|/)crates/stop/tests/'` so the 100 percent branch bar
//! measures production code only.

use nr_stop::event::{Cause, Event};
use nr_stop::state::{State, step};

#[test]
fn step_running_abort_is_stopping() {
    let next = step(
        State::Running,
        Event::Abort {
            cause: Cause::Operator,
            at_raw_ns: 0,
        },
    );
    assert_eq!(next, State::Stopping);
}

#[test]
fn step_running_acknowledged_stays_running() {
    let next = step(State::Running, Event::Acknowledged);
    assert_eq!(next, State::Running);
}

#[test]
fn step_stopping_abort_stays_stopping() {
    let next = step(
        State::Stopping,
        Event::Abort {
            cause: Cause::Shutdown,
            at_raw_ns: 7,
        },
    );
    assert_eq!(next, State::Stopping);
}

#[test]
fn step_stopping_acknowledged_is_stopped() {
    let next = step(State::Stopping, Event::Acknowledged);
    assert_eq!(next, State::Stopped);
}

#[test]
fn step_stopped_abort_stays_stopped() {
    let next = step(
        State::Stopped,
        Event::Abort {
            cause: Cause::InternalFault,
            at_raw_ns: 9,
        },
    );
    assert_eq!(next, State::Stopped);
}

#[test]
fn step_stopped_acknowledged_stays_stopped() {
    let next = step(State::Stopped, Event::Acknowledged);
    assert_eq!(next, State::Stopped);
}

#[test]
fn state_raw_round_trips() {
    for state in [State::Running, State::Stopping, State::Stopped] {
        assert_eq!(State::from_raw(state.as_raw()), state);
    }
}

#[test]
fn state_from_raw_is_total_and_fails_closed() {
    for raw in [3u8, 99, 255] {
        assert_eq!(State::from_raw(raw), State::Stopped, "raw={raw}");
    }
}

#[test]
fn cause_raw_round_trips() {
    for cause in [
        Cause::Operator,
        Cause::WatchdogDeadline,
        Cause::InternalFault,
        Cause::Shutdown,
    ] {
        assert_eq!(Cause::from_raw(cause.as_raw()), cause);
    }
}

#[test]
fn cause_from_raw_is_total_and_fails_closed() {
    for raw in [4u8, 255] {
        assert_eq!(Cause::from_raw(raw), Cause::InternalFault, "raw={raw}");
    }
}
