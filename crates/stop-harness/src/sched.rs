//! Thread affinity and `SCHED_FIFO`, through safe wrapper crates only (D-36): `core_affinity`
//! for pinning, `thread-priority` for the realtime policy. Both expose fully checked APIs to the
//! caller, so none of this crate's own code reaches for anything unchecked. Neither function
//! here panics or calls `unwrap`/`expect` on its own return values: a caller that cannot get
//! what it asked for gets an error to decide what to do with, never a crash.

use thiserror::Error;
use thread_priority::{
    RealtimeThreadSchedulePolicy, ThreadPriority, ThreadPriorityValue, ThreadSchedulePolicy,
};

#[derive(Debug, Error)]
pub enum SchedError {
    #[error("the platform refused to pin the current thread to cpu {cpu}")]
    PinRefused { cpu: usize },
    #[error("priority {priority} is not a valid thread priority: {reason}")]
    InvalidPriority { priority: u8, reason: String },
    #[error("failed to set SCHED_FIFO: {0}")]
    SetPolicy(#[from] thread_priority::Error),
}

/// Pins the current thread to `cpu`. Returns an error rather than panicking when the platform
/// refuses the pin; the caller decides what a refusal means (D-36: a real abort-latency run
/// refuses on it, task 3).
///
/// This asks the kernel to make the change and reports the kernel's answer. It deliberately does
/// NOT first look `cpu` up in `core_affinity::get_core_ids()`, which is what it used to do.
/// That call reports the calling process's inherited affinity mask (`sched_getaffinity`), and
/// `isolcpus` exists precisely to take cores out of that mask. On the reference rig, booted
/// `isolcpus=6-11`, every process starts with the mask `0-5,12-21` while all 22 cpus are online,
/// so the lookup rejected each of 6-11: the only cores this harness is ever asked to pin to.
///
/// That is not a theoretical concern. The 2026-09-16 characterise run died on
/// "cpu 7 is not in the set of cores this platform reports as available", on a machine whose own
/// `IsolcpusCoversTargetCpus` precondition had passed seconds earlier reporting `6-11`, and where
/// `taskset -c 7` pinned to cpu 7 and ran there without complaint. An inherited mask is a
/// statement about where this process has been placed, never about which cores exist, and a
/// measurement harness must not confuse the two.
pub fn pin_current_thread(cpu: usize) -> Result<(), SchedError> {
    if core_affinity::set_for_current(core_affinity::CoreId { id: cpu }) {
        Ok(())
    } else {
        Err(SchedError::PinRefused { cpu })
    }
}

/// Requests `SCHED_FIFO` at `priority` (0-99) for the current thread. Fails on macOS, which has
/// no `SCHED_FIFO`; that is expected and correct (D-36), and the caller decides what to do about
/// it rather than this function silently downgrading to a best-effort policy.
pub fn request_fifo(priority: u8) -> Result<(), SchedError> {
    let value = ThreadPriorityValue::try_from(priority)
        .map_err(|reason| SchedError::InvalidPriority { priority, reason })?;
    thread_priority::set_thread_priority_and_policy(
        thread_priority::thread_native_id(),
        ThreadPriority::Crossplatform(value),
        ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::Fifo),
    )?;
    Ok(())
}
