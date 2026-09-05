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

## From 01-10 (discovered while re-verifying `cargo test --workspace` before publishing)

- **`crates/cli/tests/run_pipeline.rs::full_run_report_matches_snapshot` fails
  once a day passes, independent of any code change.** Root cause, fully
  traced: `crates/cli/src/cmd/run.rs:238` sets `utc_start = OffsetDateTime::
  now_utc()` for a live run (correct production behaviour). `rundir.rs`
  derives the run directory name, and therefore `run_id`, from that
  timestamp's calendar date (`<YYYY-MM-DD>-<rig-slug>-<run-class>`). The
  integration test exercises this real path end to end with fake tool
  binaries (no injected clock), so its generated `run_id` always carries the
  actual wall-clock UTC date. The committed snapshot
  (`crates/cli/tests/snapshots/run_pipeline__full_run_report_matches_snapshot.snap`)
  pins a literal date string (`2026-08-31-precision3591-recon`, the day the
  snapshot was captured), so the test fails every day thereafter with a
  one-line diff on the `run id:` field alone (confirmed here: it failed as
  `2026-09-01-precision3591-recon` vs the pinned `2026-08-31-...`, both dates
  otherwise identical in every other field). All 148 other workspace tests
  pass; this is the only failure and it is not caused by, and does not
  touch, any file in plan 01-10's scope
  (`measurements/2026-08-28-precision3591/`, `measurements/INDEX.md`).
  Not fixed here: `run_pipeline.rs` and its snapshot belong to plan 01-07
  (already summarized, already committed), not to this plan's file list.
  Suggested fix for whoever next touches `nr-cli`'s test suite: the test's
  own `redact_report()` helper (`crates/cli/tests/run_pipeline.rs:172`)
  already redacts `utc_start`/`utc_end` to `[redacted]` before comparing to
  the snapshot; extend the same redaction to the `run id:` line (or just its
  date prefix) so the snapshot asserts the parts of the report that are
  actually deterministic. Re-run `cargo test -p nr-cli --test run_pipeline`
  after the fix and `cargo insta accept` (or hand-edit) the one changed line.

## From 01-11 (discovered while taking the D-18 arms through the new root entry point)

- **`scripts/nr-run-measurement` cannot launch a capture interactively as
  written.** It calls `systemd-run` with no delay, so the transient unit starts
  within milliseconds and `nrmeasure run` evaluates its preconditions while the
  launching SSH connection is still established. `NoActiveSshSessions` expects
  `0` and observes at least `1`, so the run is refused every time. Confirmed
  directly: a dry-run launched from an open SSH session reported
  `NoActiveSshSessions: observed "2", expected "0" (Fail)` with the other 14
  preconditions passing. This is not a precondition bug; the gate is correct,
  because SSH traffic broadcasts TLB shootdown IPIs to the isolated cores and is
  what made the 2026-08-28 baseline unpublishable. The script simply has no way
  to get out of its own way.
  Worked around in this plan by wrapping each launch in a detached
  `setsid` script that sleeps 30 seconds before invoking the entry point, so the
  launching connection is gone by the time the check runs. That wrapper lives in
  `/tmp` on the rig and is deliberately not committed; it is scaffolding, not a
  design.
  Note this does NOT affect plan 01-14's weekly timer, which is the case the
  script was really written for: systemd fires the unit with nobody connected,
  so there is no session to wait out.
  Suggested fix for whoever next touches the script (most naturally plan 01-14,
  which owns `deploy/systemd/`): either accept an explicit `--start-delay
  <seconds>` and pass it through as `systemd-run --on-active=<n>s`, or make the
  delay unconditional with a stated default. If `--on-active` is used, the
  "already running" guard must also check `nr-measurement.timer`, not just
  `nr-measurement.service`, because a pending timer leaves the service inactive.

- **A logind session can outlive its TCP connection and linger for days.**
  Session 40 on the rig dated from 2026-08-31 and was still listed `Active=yes`
  by `loginctl` on 2026-09-04 with no established connection behind it. It does
  not affect `NoActiveSshSessions`, which counts established TCP connections via
  `ss` rather than logind sessions, and it is not counted by
  `NoActiveLoginSessions` either because that check requires `Seat=seat0` and a
  remote session carries no seat. Harmless for the gates, but it makes
  `loginctl list-sessions` misleading when diagnosing what is holding a
  connection open. Diagnose with `ss -Htn state established '( sport = :22 )'`
  and match peer addresses; `loginctl show-session <id> -p RemoteHost -p
  Timestamp` then identifies which is which.
