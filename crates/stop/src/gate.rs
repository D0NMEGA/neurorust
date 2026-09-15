//! The output gate. Producing output requires a capability, not a passed check.

use crate::state::State;

/// The capability to produce one unit of output.
///
/// This is D-42. The gate is a capability rather than a boolean because a boolean makes STOP-03
/// a convention every node is trusted to honour, and this crate exists precisely so that the
/// guarantee does not rest on that trust. A stopped session cannot emit because there is
/// nothing to emit with.
///
/// It has no public constructor, is not `Clone` and is not `Copy`. The only way to hold one is
/// to have been issued one by [`crate::latch::EmergencyStop::try_permit`] while the machine was
/// `Running`. Consuming it takes it by value, so it cannot be used twice.
///
/// The obligations this places on the real Phase 6 consumer are written down in
/// `docs/proofs/phase-6-output-gate-contract.md`.
#[derive(Debug)]
pub struct OutputPermit {
    /// Private and uninhabited of meaning. Its only job is to stop any code outside this module
    /// constructing an `OutputPermit` with struct-literal syntax.
    pub(crate) _issued_by_the_gate: (),
}

/// Whether a gate in `state` would issue a permit.
///
/// Pure and separate from the latch on purpose: this is the function plan 02-03's harness family
/// 3 asserts against, and it must be provable without reasoning about an atomic load.
pub const fn would_issue_permit(state: State) -> bool {
    matches!(state, State::Running)
}
