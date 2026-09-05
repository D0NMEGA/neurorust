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

## From 01-11 (external audit, 2026-09-04; see 01-EXTERNAL-AUDIT.md)

These are the audit findings NOT closed in this plan. Each names the finding number so the
audit record and the fix can be read together.

- **Finding 3, partially open: hwlatdetect exposure is not wall-clock duration, and the
  report must say what the instrument actually measures.** Three statements have to appear
  wherever an hwlatdetect figure is published, and none of them do yet. First, hwlatdetect
  detects execution gaps; those are not uniquely identified SMIs, and NMI accounting and
  other hardware effects contribute. Second, its output counts threshold-exceeding sampling
  records, not a census of SMI invocations, and several gaps can land in one record. Third,
  under round-robin sampling across a CPU list, wall-clock duration is not per-CPU exposure:
  the 2026-08-28 baseline used a 500 ms sampling width in a 1 s window, so 600 s across 12
  logical CPUs is roughly 300 aggregate CPU-seconds, about 25 per CPU, not 600 per CPU.
  Verify the actual sampling mode and coverage rather than assuming rotation.
  Related: all 13 events in `hwlatdetect-pcore-underload-10m.txt` name CPU 0. That does not
  establish an observed 22 us maximum on the runtime's own isolated CPUs 6-11, which is
  what plan 01-13 wanted to consume.

- **Finding 6, open: provenance does not yet support "every claim is reproducible".**
  `run.rs` derives harness identity from `git rev-parse HEAD` in the working directory, so a
  stale executable can inherit a newer checkout's identity, and a failure records `unknown`
  with `git_dirty: false`. The committed calibration manifests already carry that unknown
  identity. Recorded argv retains `/tmp/.tmp*` paths that no longer exist, because
  relativization happens against the final run directory while the tools wrote elsewhere.
  `verify.rs` substitutes `(0, 0)` when histogram metrics cannot be computed, turning a parse
  failure into a published zero. Strict verification does not regenerate `REPORT.md` or
  `hist.tsv` and compare them, and the JSON reconciliation checks thread count and maxima but
  not sample-count agreement.
  Also: the 2026-08-28 README cites six events and a 7 us maximum at a 1 us threshold, but no
  1 us raw capture exists in the repository. Recover it or mark the observation as lacking a
  published raw capture.

- **Finding 7, open: a failed attempt can vanish, and the interference window is mismatched.**
  Raw output lives in a temporary directory until the run directory is created, so a capture
  that fails parsing or reconciliation loses its evidence entirely. Create a durable attempt
  record before launching instruments and preserve partial output, stderr and exit status.
  Separately, the interference snapshots bracket cyclictest plus hwlatdetect plus parsing,
  but the delta is normalised by the requested cyclictest duration alone: a 3600 s cyclictest
  followed by a 900 s hwlatdetect divides roughly 4500 s of activity by 3600.

- **Finding 8, open: two gates are narrower than their names.** `DeepCstatesDisabled` reads
  CPU 0 only and cannot establish the state of target CPUs 6-11 after per-CPU tuning.
  `TracersQuiescent` treats `current_tracer == nop` as quiescence, but event tracing has
  separate enable controls. Also, the facts fixture is forbidden for publishable classes
  while the interrupts fixture is not, and reusing it for both snapshots yields zero deltas;
  it must be forbidden too, or force explicit non-publication.

- **Finding 5, partially open: scope the Screen thermal exemption to a declared profile.**
  The exemption currently applies to any `Screen` run and is decided after observing that the
  temperature exceeded the ceiling, so an unintentionally hot idle screen is exempted too.
  Declare a hot-screen measurement profile up front and key the exemption on that instead.

- **Findings 9 and 10, open, both in unexecuted plans.** Plan 01-12 assigns the ~3.8 ms
  maximum to the isolated events rather than the sustained burst, an association the captures
  cannot support; says `--tracemark` "arms ftrace" when the flags mark and stop tracing at a
  threshold without enabling a diagnostic event set; and claims rtla runs "alongside" when the
  foreground pipeline completes first. Plan 01-14 instructs unavailable hwlat percentiles to be
  filled with the maximum, and its regression guard uses `merge-base origin/main HEAD`, which
  on a main-branch push makes the diff empty and lets through exactly the simultaneous
  baseline change it exists to prohibit.

## From 01-11 (surfaced by the D-18 arm 2 capture itself, 2026-09-05)

- **`RuntimeMaxSec` is silently ignored on a `Type=oneshot` unit, so the runaway guard
  added in 178c7e8 does nothing.** systemd said so directly in the journal:
  `nr-measurement.service: RuntimeMaxSec= has no effect in combination with Type=oneshot.
  Ignoring.` The whole point of that commit was to stop a hung capture from wedging the rig
  the way the `--duration 0` cyclictest did, and it does not. For a oneshot unit the entire
  run is the start phase, so the equivalent bound is `TimeoutStartSec=`. Change
  `scripts/nr-run-measurement` to pass `--property=TimeoutStartSec=` instead, and verify the
  journal no longer prints the "has no effect" line, because that line is the only reason
  this was caught. Requires one interactive sudo session on the rig to re-install the script,
  so it is queued rather than fixed in place. Until then a hung capture still needs the
  operator's password to clear.

- **`hwlatdetect` exits 1 when it finds latency above the hard limit, and the harness reports
  that as a tool failure.** `warn_on_tool_failure` printed `warning: hwlatdetect exited with
  code 1` for arm 2, which reads as a broken capture. It was not: the tool ran to completion
  and produced `Max Latency: 15us, Samples recorded: 12, Samples exceeding threshold: 12`.
  `/usr/sbin/hwlatdetect` line 549 is `sys.exit(maxlatency > hardlimit)`, and line 458
  defaults `hardlimit` to the threshold when `--hardlimit` is not passed, so any run that
  observes anything above 10 us exits 1 by design. For a firmware screen that is the expected
  outcome, not an error. Teach the harness the difference, or the D-18 arms will always look
  like failures and a real failure will be indistinguishable from a finding.

- **Open question, and the most consequential thing arm 2 produced: all 12 events landed on
  CPU 5.** Not one on the isolated cores 6-11 that the runtime actually uses. This is the same
  shape as the 2026-08-28 baseline, where all 13 P-core-arm events named CPU 0. With no
  `--cpu-list`, `hwlatdetect` leaves `tracing_cpumask` at the system default and the hwlat
  tracer round-robins under it (`/usr/sbin/hwlatdetect` only writes the mask when `--cpu-list`
  is given, lines 491-502). Two candidate explanations, not yet distinguished: either the
  tracer genuinely rotated and only CPU 5 exhibited gaps above threshold, or the tracer kthread
  never sampled 6-11 at all under `isolcpus=6-11`/`nohz_full=6-11`. These have opposite
  meanings, and the second would mean an unrestricted `hwlatdetect` run on this rig says
  nothing whatsoever about the isolated cores. Arm 3, which passes `--hwlatdetect-cpu-list
  0-11` and therefore writes the mask explicitly, is the experiment that separates them.

## From 01-11 (the hwlatdetect coverage defect, 2026-09-05)

- **`hwlatdetect` cannot sample more than one CPU on this kernel, and no option it exposes
  changes that.** The hwlat tracer's `mode` governs whether its kernel thread migrates.
  `none`, the value on this rig, means it does not: it samples whichever CPU the scheduler
  has it on, and `isolcpus=6-11` guarantees that is never one of the isolated cores. Setting
  the mode beforehand does not survive: measured on 2026-09-05, `round-robin` immediately
  before a run and `none` twice during it. The kernel accepts a mode write only while
  `current_tracer` is not `hwlat`, and `hwlatdetect`'s startup clears the tracer, resetting
  the mode before it selects `hwlat`. Writing the mode while the tracer is selected is
  silently rejected and leaves `none`, with or without `tracing_on=0`.
  Consequence: three D-18 arms, 32 events between them, every one on CPU 5. Nothing in this
  repository characterises firmware latency on CPUs 6-11, including the 2026-08-28 baseline,
  whose P-core arm put all 13 of its events on CPU 0.
  The fix is a new instrument: drive the hwlat tracer directly (write `current_tracer`,
  `hwlat_detector/mode`, `window`, `width`, `tracing_thresh`, enable `tracing_on`, read the
  trace buffer) instead of shelling out to `hwlatdetect`. `per-cpu` mode would additionally
  give each CPU its own sampling thread and therefore real per-CPU exposure rather than one
  thread's polling time divided across the set. This is new capability work and belongs in
  its own plan, not in 01-11.

- **Standing check to add wherever an hwlatdetect figure is consumed: assert the CPU
  distribution of the events, not just the maximum.** The information that would have caught
  this was present in the 2026-08-28 raw capture from the day it was taken. A test that reads
  the committed `hwlatdetect.txt` files and asserts which CPUs appear would have failed on
  the baseline immediately.

## From 01-11 (the instrument that was available all along, 2026-09-05)

- **Re-take D-18 with `rtla hwnoise`, not a new tracer driver.** The D-18 document proposes
  driving the hwlat tracer directly because `hwlatdetect` cannot be made to sample more than
  one CPU. That is true but the conclusion is wrong: `rtla hwnoise` already does exactly
  this. It is installed (rtla 7.0.12, from `linux-tools-common`), takes `-c/--cpus` to run
  one osnoise thread per CPU in the list, and `-H/--house-keeping` to keep its own control
  threads off the measured cores. `rtla hwnoise -c 6-11 -H 0-5 -P f:99 -d 900s` is the
  measurement D-18 wanted.
  STATE.md has recorded since plan 01-02 that rtla ships in `linux-tools-common` and needs no
  build, but filed it as settling plan 01-12's instrument only. Nobody connected that
  `hwnoise` is the per-CPU replacement for `hwlatdetect`, and three arms were spent
  discovering the limitation the hard way.

- **Install `msr-tools` and read `MSR_SMI_COUNT` (0x34) around every firmware screen.**
  `rdmsr -p <cpu> 0x34` is an exact per-CPU SMI counter. Sampled before and after a run it
  answers "did SMIs reach CPUs 6-11" directly, with no sampling, no thresholds and no
  inference from timing gaps, and it costs one `apt install` plus two reads. The `msr` module
  is already loaded on the rig. This would have settled in seconds what three 15-minute arms
  could not.
  It also gives the harness a cheap, exact provenance field: SMI count per isolated CPU over
  the run, recordable in the manifest beside the interference counters.
