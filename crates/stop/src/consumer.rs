//! A consumer that models what Phase 6 will do with a permit, and emits nothing.

use crate::gate::OutputPermit;

/// The modelled consumer D-45 requires. It takes permits and produces nothing real, so STOP-03
/// can be proved against something concrete at FSM level while the decoder and sink that will
/// actually hold the permit do not arrive until Phase 6.
///
/// Two obligations are encoded here in types rather than prose, and
/// `docs/proofs/phase-6-output-gate-contract.md` states them for the real consumer:
///   - [`Self::emit`] takes the permit BY VALUE, so a permit cannot be cached across iterations.
///   - there is no method that emits without one.
#[derive(Debug, Default)]
pub struct ModelledConsumer {
    emitted: u64,
}

impl ModelledConsumer {
    pub const fn new() -> Self {
        Self { emitted: 0 }
    }

    /// A consumer that has already emitted `emitted` times. Present so a test and a Kani harness
    /// can reach the counter's upper boundary without calling `emit` 2^64 times.
    pub const fn with_emitted(emitted: u64) -> Self {
        Self { emitted }
    }

    /// Consumes the permit and records one emission. Emits nothing.
    ///
    /// `let _ = permit;`, not `drop(permit)`: `OutputPermit` has no `Drop` impl of its own, and
    /// clippy's `drop_non_drop` (part of the workspace's denied `clippy::all`) flags an explicit
    /// `drop()` call on a type that drops trivially anyway. The discard pattern still makes the
    /// consumption visible in the diff and still takes the permit by value, so it cannot be used
    /// twice; only the spelling changed; see 02-02-SUMMARY.md.
    ///
    /// `saturating_add`, not `+= 1`. A plain `+= 1` panics on overflow in a debug build, which
    /// would make a panic reachable in this crate and fail plan 02-03's family 4 harness. The
    /// saturation is not a substituted number that could be mistaken for a measurement: this
    /// counter exists so a test can assert it never moves after a stop, and a saturated counter
    /// still satisfies that assertion.
    pub fn emit(&mut self, permit: OutputPermit) {
        let _ = permit;
        self.emitted = self.emitted.saturating_add(1);
    }

    pub const fn emitted(&self) -> u64 {
        self.emitted
    }
}
