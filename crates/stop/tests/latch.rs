//! The latch's behavioral contract: the abort handle, the hot-path poll, keeping the first
//! aborter's record, and the absence of any path back to `Running`.

use std::sync::Arc;
use std::thread;

use nr_stop::consumer::ModelledConsumer;
use nr_stop::event::Cause;
use nr_stop::latch::EmergencyStop;
use nr_stop::state::State;

#[test]
fn a_fresh_machine_is_running() {
    assert_eq!(EmergencyStop::new().state(), State::Running);
}

#[test]
fn a_fresh_machine_issues_permits() {
    let stop = EmergencyStop::new();
    assert!(stop.try_permit().is_some());
}

#[test]
fn abort_moves_to_stopping_and_returns_true() {
    let stop = EmergencyStop::new();
    assert!(stop.abort(Cause::Operator, 42));
    assert_eq!(stop.state(), State::Stopping);
}

#[test]
fn the_permit_supply_closes_at_the_abort_not_the_acknowledgement() {
    let stop = EmergencyStop::new();
    stop.abort(Cause::Operator, 42);
    // No acknowledge() has happened at all yet. D-40: the supply is already closed.
    assert!(stop.try_permit().is_none());
}

#[test]
fn a_second_abort_returns_false_and_keeps_the_first_record() {
    let stop = EmergencyStop::new();
    assert!(stop.abort(Cause::Operator, 42));
    assert!(!stop.abort(Cause::Shutdown, 99));

    let record = stop
        .abort_record()
        .expect("the winning abort must publish a record");
    assert_eq!(record.cause, Cause::Operator);
    assert_eq!(record.at_raw_ns, 42);
}

#[test]
fn acknowledge_moves_stopping_to_stopped() {
    let stop = EmergencyStop::new();
    stop.abort(Cause::Operator, 0);
    assert!(stop.acknowledge());
    assert_eq!(stop.state(), State::Stopped);
}

#[test]
fn acknowledge_on_a_running_machine_changes_nothing() {
    let stop = EmergencyStop::new();
    assert!(!stop.acknowledge());
    assert_eq!(stop.state(), State::Running);
}

#[test]
fn a_second_acknowledge_returns_false() {
    let stop = EmergencyStop::new();
    stop.abort(Cause::Operator, 0);
    assert!(stop.acknowledge());
    assert!(!stop.acknowledge());
    assert_eq!(stop.state(), State::Stopped);
}

#[test]
fn there_is_no_path_back_to_running() {
    #[derive(Clone, Copy)]
    enum Op {
        Abort,
        Ack,
    }

    fn apply(stop: &EmergencyStop, op: Op) {
        match op {
            Op::Abort => {
                stop.abort(Cause::Operator, 0);
            }
            Op::Ack => {
                stop.acknowledge();
            }
        }
    }

    // Every ordering of a length-4 sequence drawn from {abort, acknowledge}: 2^4 = 16.
    for mask in 0u8..16 {
        let sequence = [
            if mask & 1 == 0 { Op::Abort } else { Op::Ack },
            if mask & 2 == 0 { Op::Abort } else { Op::Ack },
            if mask & 4 == 0 { Op::Abort } else { Op::Ack },
            if mask & 8 == 0 { Op::Abort } else { Op::Ack },
        ];

        let from_stopping = EmergencyStop::new();
        from_stopping.abort(Cause::Operator, 0);
        assert_eq!(from_stopping.state(), State::Stopping);
        for op in sequence {
            apply(&from_stopping, op);
            assert_ne!(from_stopping.state(), State::Running, "mask={mask:#06b}");
        }

        let from_stopped = EmergencyStop::new();
        from_stopped.abort(Cause::Operator, 0);
        from_stopped.acknowledge();
        assert_eq!(from_stopped.state(), State::Stopped);
        for op in sequence {
            apply(&from_stopped, op);
            assert_ne!(from_stopped.state(), State::Running, "mask={mask:#06b}");
        }
    }
}

#[test]
fn the_abort_record_is_absent_before_an_abort() {
    let stop = EmergencyStop::new();
    assert!(stop.abort_record().is_none());
}

#[test]
fn abort_is_callable_from_another_thread() {
    let stop = Arc::new(EmergencyStop::new());
    let stop_for_thread = Arc::clone(&stop);
    let handle = thread::spawn(move || {
        stop_for_thread.abort(Cause::Operator, 123);
    });
    handle.join().expect("the aborting thread must not panic");

    assert_ne!(stop.state(), State::Running);
    assert!(stop.abort_record().is_some());
}

#[test]
fn the_modelled_consumer_cannot_emit_after_a_stop() {
    let stop = EmergencyStop::new();
    let mut consumer = ModelledConsumer::new();

    let permit = stop.try_permit().expect("a fresh machine issues permits");
    consumer.emit(permit);
    assert_eq!(consumer.emitted(), 1);

    stop.abort(Cause::Operator, 0);

    assert!(stop.try_permit().is_none());
    assert_eq!(consumer.emitted(), 1);
}
