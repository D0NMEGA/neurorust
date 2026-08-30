# Deferred items

Out-of-scope discoveries logged during plan execution. Not fixed as part of the
originating plan; noted here for a future plan or maintenance pass to pick up.

## From 01-01 (cargo workspace, licence, CI, fixture)

- **`rust-version = "1.85"` understates the real MSRV.** `cargo clippy`/`cargo test`
  resolution reports `hdrhistogram v7.6.0` and `time v0.3.55` (and its `time-macros`/
  `time-core` companions) require Rust 1.88, not 1.85. This is metadata-only: the dev
  host runs rustc 1.96.0 and CI installs `stable` via `dtolnay/rust-toolchain`, so
  nothing is broken today. Non-blocking (no acceptance criterion checks the
  `rust-version` value), left as the plan specified it verbatim. If the workspace
  ever needs to defend a literal MSRV claim, bump `workspace.package.rust-version`
  to `"1.88"` to match reality.
