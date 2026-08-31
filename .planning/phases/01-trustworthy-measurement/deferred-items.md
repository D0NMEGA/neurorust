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

## From 01-03 (D-14 manifest field set, checksums, schema)

- **`nr-capture` (plan 01-05) must relativize `tools[].argv` output-file paths to
  the run directory.** Task 3's human review (Decision B) approved recording
  `ToolInvocation.argv` with output-file paths relative to the run directory
  rather than absolute (`$HOME/...`) paths, so the argv is a command a third
  party can actually paste and run. `crates/manifest/tests/fixtures/
  minimal-manifest.json` already shows the intended form
  (`--histfile=cyclictest-rt-isolated-idle-10m.hist`). The rewrite itself
  belongs in `nr-capture`, where argv is captured, which does not exist yet;
  `nr-manifest` only documents the convention on the `argv` field's doc
  comment (and therefore the generated schema description) for plan 01-05 to
  implement as a contract, not re-derive.
