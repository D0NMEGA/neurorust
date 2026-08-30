//! Run manifest schema and provenance for neurorust measurement captures.
//!
//! Owns the D-14 environment snapshot field set, the D-12 sidecar manifest, per-file
//! blake3 checksums, and the D-16 provenance tier. Platform independent by design so the
//! macOS CI leg exercises it for real.

#[cfg(test)]
mod tests {
    #[test]
    fn crate_builds() {}
}
