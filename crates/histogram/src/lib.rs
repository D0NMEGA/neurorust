//! Cyclictest histogram parsing and percentile computation for neurorust, plus percentile
//! computation over raw sample slices with no cyclictest structure at all.
//!
//! Owns parsing of the cyclictest `.hist` and `--json` output formats, hdrhistogram-backed
//! percentile computation, overflow accounting (BENCH-05), and hdrhistogram-backed statistics
//! over a flat slice of raw samples for callers with no per-thread or overflow structure to
//! parse. Platform independent by design so the macOS CI leg exercises it for real.

pub mod hist;
pub mod json;
pub mod percentiles;
pub mod samples;
