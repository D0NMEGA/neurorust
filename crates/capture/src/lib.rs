//! Precondition checks and environment capture for neurorust measurement runs.
//!
//! Owns the D-06 precondition assertions (assert-and-record, never enforce), the D-14
//! environment snapshot readers (Linux-gated via procfs), and the D-15 interference diff
//! and contamination verdict.

pub mod preconditions;
pub mod sources;
