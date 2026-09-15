#![forbid(unsafe_code)]
//! The neurorust emergency_stop state machine and output gate.
//!
//! This is the first runtime crate in the repository and it depends on nothing. The workspace
//! lint table already sets `unsafe_code = "forbid"`; the restatement on the first line of this
//! file is deliberate and is part of the published claim (D-49). It makes the guarantee local,
//! it survives any future change to the workspace table, and it puts the property inside the
//! crate whose whole value is that property.
//!
//! What this crate proves, what the compiler proves, and what nothing in Phase 2 proves are
//! stated in `docs/proofs/emergency-stop-proof-scope.md`. Read that before quoting anything
//! here as verified.
