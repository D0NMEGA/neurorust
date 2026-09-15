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
    #[error("cpu {cpu} is not in the set of cores this platform reports as available")]
    CpuNotAvailable { cpu: usize },
    #[error("the platform refused to pin the current thread to cpu {cpu}")]
    PinRefused { cpu: usize },
    #[error("priority {priority} is not a valid thread priority: {reason}")]
    InvalidPriority { priority: u8, reason: String },
    #[error("failed to set SCHED_FIFO: {0}")]
    SetPolicy(#[from] thread_priority::Error),
}

/// Pins the current thread to `cpu`. Returns an error rather than panicking when `cpu` is not
/// among the cores this platform reports, or when the platform refuses the pin outright; the
/// caller decides what a refusal means (D-36: a real abort-latency run refuses on it, task 3).
pub fn pin_current_thread(cpu: usize) -> Result<(), SchedError> {
    let core_ids = core_affinity::get_core_ids().unwrap_or_default();
    let target = core_ids
        .into_iter()
        .find(|core| core.id == cpu)
        .ok_or(SchedError::CpuNotAvailable { cpu })?;

    if core_affinity::set_for_current(target) {
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
