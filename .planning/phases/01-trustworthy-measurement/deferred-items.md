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

## From 01-02 (rig recon: tracers, tooling, isolation, tuning state)

- **The rig boots untuned: `rt-tuning.service` loses a race with
  `power-profiles-daemon` on every boot.** `rt-tuning.service` runs and exits
  successfully at boot, but GDM's own greeter session (independent of any real
  login) D-Bus-activates `power-profiles-daemon` within the same second, and
  that daemon's "performance" profile is expressed on this Meteor Lake/HWP
  backend as `scaling_governor=powersave` plus `energy_performance_preference=
  performance`, never as the legacy `scaling_governor=performance` override
  `rt-tuning.sh` writes. Net effect: every CPU is left at `powersave`
  regardless of which service ran last or how healthy either looks in
  `systemctl status`. Full root-cause chain: `docs/rig/recon-2026-08-31/
  FINDINGS.md`, "The rt-tuning.service contradiction". Not fixed here per
  explicit instruction (recon records rig state, never mutates it). Whichever
  plan owns the harness's tuning/precondition story should close this, for
  example by masking `power-profiles-daemon.service`, having `rt-tuning.service`
  run later (`After=graphical.target`) or re-assert on a timer, or accepting
  the race and relying entirely on `nr-capture`'s D-06 `governor-is-performance`
  precondition to refuse untuned runs rather than trusting the service's
  `ActiveState`.
- **`nr-recon`/`nr-probe` (the two passwordless-root scripts) query the wrong
  `no_turbo` sysfs path.** Both read `/sys/devices/system/intel_pstate/no_turbo`
  (no `cpu/` segment), which does not exist on this kernel, and so always
  report `unavailable`. The real, populated path is
  `/sys/devices/system/cpu/intel_pstate/no_turbo` (confirmed present,
  world-readable, currently `1`). Editing either script requires rig access
  this plan did not use for anything beyond read-only recon and the two
  approved probes; whoever next touches these scripts' source (outside this
  repo, on the rig) should fix the path. Meanwhile, any in-repo precondition
  code (`nr-capture`, plan 01-05) must use the correct path directly rather
  than copying the scripts' query.
