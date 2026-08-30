//! Cyclictest histogram parsing and percentile computation for neurorust.
//!
//! Owns parsing of the cyclictest `.hist` and `--json` output formats, hdrhistogram-backed
//! percentile computation, and overflow accounting (BENCH-05). Platform independent by
//! design so the macOS CI leg exercises it for real.

#[cfg(test)]
mod tests {
    #[test]
    fn crate_builds() {}
}
