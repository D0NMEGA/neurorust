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
  **CLOSED, commit `1c5051d` (plan 01-22 task 1).** Both scripts were pulled
  back from the rig into `scripts/nr-recon` and `scripts/nr-probe` and are now
  version controlled. They already read the correct
  `/sys/devices/system/cpu/intel_pstate/no_turbo` path when pulled: someone
  fixed the path directly on the rig, out of band, sometime after this entry
  was filed, so this plan's own instruction to edit the path was a no-op by
  the time it ran. Confirmed by `grep -n no_turbo scripts/nr-recon
  scripts/nr-probe`, which shows only the correct `cpu/` path in both files
  and zero occurrences of the wrong one. `nr-capture`'s own precondition code
  was unaffected either way, since it already used the correct path directly
  per this entry's original instruction.

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
  **CLOSED, commit `8db689a` (fix(01-07): redact the wall-clock date from the
  run-report snapshot).** `redact_report()` now calls `redact_leading_date()`
  on the `run id:` line, which strips exactly a leading `YYYY-MM-DD` and
  leaves the rest of the id intact. Verified 2026-09-05 (plan 01-16):
  `cargo test -p nr-cli --test run_pipeline` passes, `full_run_report_matches_
  snapshot` included, independent of the current wall-clock date.

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

  **Reporting half CLOSED, commit `364aa29` (plan 01-21 task 3).** `render_run_report` gains
  `render_firmware_screens`, printing three statements written separately for `hwlatdetect`
  and `rtla-hwnoise` (never one string shared between them; asserted to differ by
  `firmware_caveats_differ_by_instrument`), plus the exact `MSR_SMI_COUNT` delta when the
  manifest carries one. `render_plat03_verdict`'s `FirmwareObservation` (generalised from
  `HwlatObservation`) names whichever instrument produced a paired figure. The coverage half
  stays open: every statement is now correctly worded, but no committed capture has yet named
  an isolated core; that is plan 01-23's job, using the `--with-hwnoise` harness plan 01-21
  task 1 (commit `d621383`) also wired up.

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
  **Documentation half CLOSED, commit `137c3c1`** (fix(01-11): correct the recorded reason
  the interference counters were abandoned): `docs/measurement-protocol.md`'s
  `ThermalHeadroomAtStart` row now reads `package temp <= 70 C`, matching the code's actual
  70 C ceiling; `grep -c '60 C' docs/measurement-protocol.md` reports 0. **The other half
  remains open**: the exemption is still keyed on `RunClass::Screen` alone rather than a
  declared hot-screen profile, and is owned by plan 01-18.

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
  **Repository half CLOSED, commit `2b7a581`** (fix(01-11): let hwlatdetect reach the
  isolated cores, and stop misreading its exit code): `scripts/nr-run-measurement` in this
  repository now passes `--property=TimeoutStartSec="$RUNTIME_MAX"`, not `RuntimeMaxSec`;
  `grep -c TimeoutStartSec scripts/nr-run-measurement` reports 3 (the property plus two
  explanatory comment lines added by the same commit).
  **Fully CLOSED, commit `1c5051d` (plan 01-22 task 1).** `sudo
  ./deploy/sudoers/install.sh` replaced all four scripts on the rig, confirmed by `stat`
  (`/usr/local/sbin/nr-run-measurement` is `755 root:root`) and `diff` (all four installed
  copies byte-identical to `scripts/`). The runaway guard was then confirmed live: a launch
  through the entry point printed the exec banner (sha256
  `a4f8d9cd622cdcc861ee83569a0aeb7409cd91f0767ae68bd899c5469e7da7d8`, mode 755) and started
  `nr-measurement.service` with a 630 s bound; the journal for that launch window carries
  zero `RuntimeMaxSec` warnings. Four such warnings remain in the journal from before the
  fix, which is why the check was scoped to the launch's own timestamp window rather than
  the whole journal, a method note worth keeping for future verifications of the same
  guard.

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
  **Parser half CLOSED, commits `515be12`/`cd49f45` (plan 01-20).** Plan 01-22 took a real
  60 s `rtla hwnoise -c 6-11 -H 0-5 -P f:99` probe and committed it as
  `docs/rig/recon-2026-09-05/probe-rtla-hwnoise.txt`, described in full in the FINDINGS.md
  beside it. Two format details a parser needs and this entry did not anticipate: the default
  live redraw repeats a full header once per second and is preceded by a terminal reset
  artifact, and the per-CPU `Runtime` column, not the wall-clock `duration` header, is the
  real per-CPU exposure figure. `crates/capture/src/hwnoise.rs` (`parse_hwnoise`,
  `parse_hwnoise_file`) parses both correctly, tested against a byte-identical copy of that
  probe; `nr_manifest::FirmwareScreen`/`SmiCounts` can now record the result honestly.
  **Wiring half CLOSED, commits `d621383`/`4a27ee1`/`364aa29` (plan 01-21).** `nrmeasure run
  --with-hwnoise` runs `rtla hwnoise` through the harness (its own `InstrumentWindow`, a
  checksummed `rtla-hwnoise.txt` artifact, a parsed `FirmwareScreen`), and every run brackets
  an `rdmsr -p <cpu> 0x34` read before and after regardless of which firmware instrument, if
  any, also ran. REPORT.md now states the three finding-3 statements per instrument. **Re-take
  half still OPEN, owner: plan 01-23** (re-taking D-18 with both instruments live on the rig).
  No new firmware capture exists yet beyond the two probes plan 01-22 took; every event either
  instrument reports will need to name an isolated core before this item is fully closed.

- **Install `msr-tools` and read `MSR_SMI_COUNT` (0x34) around every firmware screen.**
  `rdmsr -p <cpu> 0x34` is an exact per-CPU SMI counter. Sampled before and after a run it
  answers "did SMIs reach CPUs 6-11" directly, with no sampling, no thresholds and no
  inference from timing gaps, and it costs one `apt install` plus two reads. The `msr` module
  is already loaded on the rig. This would have settled in seconds what three 15-minute arms
  could not.
  It also gives the harness a cheap, exact provenance field: SMI count per isolated CPU over
  the run, recordable in the manifest beside the interference counters.
  **CLOSED, commit `1c5051d` (plan 01-22 task 2).** `msr-tools` 1.3+git20220805.7d78c80-1build1
  is installed and the `msr` module is loaded (`stress-ng` was already installed from the
  01-11 session-2 D-18 arms, so this task only needed `msr-tools`). `rdmsr -p <cpu> 0x34` read
  `0xfa6` (4006 decimal) uniformly on CPUs 0, 5, 6, 7, 8, 9, 10 and 11, committed as
  `docs/rig/recon-2026-09-05/probe-rdmsr-smi-count.txt` and described in the FINDINGS.md
  beside it: the counter is non-zero on every isolated core, which is the fact three
  `hwlatdetect` arms could not establish.

## From 01-18 (finding 8: `TracersQuiescent` now requires four controls, not one)

- **`scripts/nr-measure-mode` sets only `current_tracer=nop` and is now insufficient to
  satisfy `TracersQuiescent`.** This plan widened the check to require `events/enable=0`,
  `set_event=` (empty) and `tracing_on=0` in addition to `current_tracer=nop`, since any of
  the three can arm tracing independently of `current_tracer` (finding 8,
  `01-EXTERNAL-AUDIT.md`). This is the intended, stricter outcome, not a regression to work
  around. The rig's own `nr-measure-mode` script (referenced in
  `docs/measurement-protocol.md`'s "The governor operating point" workflow) needs the three
  additional writes:
  ```
  echo 0 | sudo tee /sys/kernel/tracing/events/enable
  echo   | sudo tee /sys/kernel/tracing/set_event
  echo 0 | sudo tee /sys/kernel/tracing/tracing_on
  ```
  Owner: plan 01-22, which re-installs the rig's scripts (per STATE.md's "Rig root access"
  blocker). Until then, a real run against the unpatched script will correctly fail
  `TracersQuiescent` if event tracing, `set_event`, or `tracing_on` happen to be armed from
  an earlier investigation session; the operator can still satisfy the check by hand with
  the four `echo`/`tee` commands in `docs/measurement-protocol.md`'s "Required system state"
  table.

## From 01-22 (task 1: the rig has no git, so "git pull" as written does not work)

- **`git` is not installed on the rig, and `~/neurorust` there is a plain directory, not a
  git clone.** Confirmed directly: `dpkg -l git` reports `un` (never installed), `which git`
  and `bash -lc 'which git'` both report not found, and `~/neurorust/.git` does not exist.
  The directory is a working tree kept in sync by `rsync` alone (matching 01-11-SUMMARY.md's
  own "rsynced and rebuilt there" phrasing for the binary; this plan confirms the same is
  true of the source tree, not just the build).
  This plan's own task 1 step 5 instructs `cd ~/neurorust && git pull` as the first line of
  the rig-side session. That command fails outright (`git: command not found`) as written.
  Worked around here, before the checkpoint was returned: pushed the six new/changed files
  (`scripts/nr-recon`, `scripts/nr-probe`, `scripts/nr-measure-mode`,
  `scripts/nr-run-measurement`, `deploy/sudoers/nr-measurement`, `deploy/sudoers/install.sh`)
  directly from the dev host with a plain, unprivileged `rsync -av <paths>
  precision3591-rig:~/neurorust/<paths>` (verified byte-identical after transfer via
  `sha256sum` on both ends), so the operator's rig-side session can run
  `sudo ./deploy/sudoers/install.sh` immediately with no pull step.
  Whoever next writes a rig-facing plan should say `rsync` from the dev host, not `git pull`
  on the rig, unless a future plan deliberately installs `git` there first (itself a `sudo
  apt install`, i.e. another password-gated step, for a machine that has managed fine without
  it so far).

  **CLOSED, plan 01-27 task 1.** `scripts/nr-push-to-rig.sh` is now the one supported,
  version-controlled way source reaches the rig: an explicit include list, no removal flag,
  and a post-transfer sha256 check on `.git-sha`, which the rig-side build reads and records
  as `git_sha_source: pushed-stamp` whenever it has no `.git` checkout to read directly (D-29).

## From 01-23 (a killed rtla leaves osnoise kthreads that D-06 cannot see, 2026-09-06)

- **`TracersQuiescent` cannot observe `rtla`'s own tracing instance, so orphaned osnoise
  kthreads silently starve the next run's cyclictest.** Arm 1 of the D-18 re-take was taken
  twice on 2026-09-06. The first attempt was terminated by systemd at exactly 660s
  (the `--hwnoise-duration` timeout-bound defect, fixed in `b29e819`), which SIGTERMed
  `rtla` mid-session. The second attempt then produced a run
  (`measurements/2026-09-06-precision3591-screen-02`) whose cyclictest reads:

  ```
  p50=2us p95=4us p99=9us max=750021us samples=450314 overflow=360
  contamination verdict: Contaminated   tail excursion ratio=83335.7
  thread 0: cpu=6  cycles=75055 avg=602.03 max=750016
  thread 5: cpu=11 cycles=75051 avg=602.21 max=750020
  ```

  All six threads lost the same ~75% of their cycles (75,051 of an expected 300,000) and
  all six stalled at ~750,000us. That number is not a coincidence: `/sys/kernel/tracing/
  osnoise/runtime_us` reads `750000` with `period_us=1000000` and `cpus=6-11`, which rtla
  wrote and left behind. Osnoise sampling threads run at the requested SCHED_FIFO priority
  and spin for `runtime_us` of every `period_us`, so orphans left on 6-11 take 75% of each
  period from anything sharing those cores at the same priority.

  The harness is not at fault: `crates/cli/src/cmd/run.rs` runs cyclictest and the firmware
  screen strictly sequentially ("never concurrently", line 7), confirmed by artifact mtimes
  (cyclictest 21:25:02-21:26:02, hwnoise 21:26:02-21:41:02). The orphans predated the run.

  The gap is in the assertion list. Plan 01-18 widened `TracersQuiescent` to require all four
  top-level controls, but `rtla` drives osnoise through **its own tracing instance**, so the
  top-level `current_tracer` still reads `nop` and all four checks pass while osnoise kthreads
  are spinning on the measured CPUs. The D-06 list therefore certifies a machine that is not
  quiet.

  Suggested fix, for whichever plan next touches `crates/capture/src/preconditions.rs`: add a
  check that no `osnoise/` or `timerlat/` kthread exists on any CPU in `--cpus`, and/or that
  `/sys/kernel/tracing/instances/` contains no live tracer. Both are readable without root.
  A stopgap guard now lives in the operator-side `~/nr-arm.sh` on the rig, which refuses to
  launch an arm when such kthreads are present, but that is a runbook aid, not the mechanism.

  **CLOSED, commit `b04c229` (plan 01-25 task 1).** `check_tracers_quiescent` now also lists
  `/sys/kernel/tracing/instances/` and checks the same four controls inside every instance
  found, and scans `/proc/*/comm` for an `osnoise/<cpu>` or `timerlat/<cpu>` kthread on each
  target CPU. Both are readable without root, an unreadable instances directory is recorded
  rather than treated as a violation, and the check is still named `TracersQuiescent`
  (extended, not a sixteenth precondition), so `run_all` still returns exactly fifteen
  results. Commit `0b1fa65` (task 2) moves the stopgap guard out of `~/nr-arm.sh` on the rig
  and into `scripts/nr-measure-mode`, version controlled; it reaches the rig once plan 01-27
  reinstalls it.

  Note also that the contaminated run is retained rather than deleted, per this plan's own
  rule that every arm attempted appears under `measurements/`, including failures. Its
  hwnoise half is good (rows for all of 6-11, max single event 1us on five cores and 7us on
  cpu 7, NMI 0, SMI delta 0 on every isolated core); only its cyclictest half is starved.

## From 01-23 (second independent review, 2026-09-07: four items found and left alone)

Four items `01-REVIEW-2026-09-06.md` found while checking the corrected firmware record
(its own Disposition section names these "cheap and independent" of plan 01-23's B1/B2/B3
fixes). None are fixed here; each is out of scope for a documentation-correction pass and is
recorded with an owner.

- **Finding B4. REPORT.md publishes RES rescheduling-interrupt counts under the label
  "context switches".** `crates/capture/src/interference.rs` documents the substitution directly
  (`context_switches` counter is populated from the RES row; see the module's own comment
  above `TRACKED_ROWS` and the `InterferenceSnapshot::context_switches` doc comment): plan
  01-05 chose RES because `InterferenceSnapshot` had no better field, since `/proc/stat`'s
  `ctxt` counter is machine-wide with no per-CPU breakdown. The label a reader sees in a
  published REPORT.md is still "context switches," which is not what RES counts. Owner:
  whichever plan next touches `nr_manifest::InterferenceSnapshot`'s schema (`crates/manifest/`)
  or the report renderer that prints the label (`crates/metrics/src/report.rs` /
  `crates/cli/src/cmd/run.rs`) should rename the published label to match the field it
  actually reports, or add a real per-CPU context-switch source and keep RES separate.

  **CLOSED, commit `bf850e7` (plan 01-26 task 2).** `InterferenceSnapshot`/`InterferenceDelta`'s
  `context_switches` field is renamed to `rescheduling_ipis` with
  `#[serde(alias = "context_switches")]`, so every committed manifest still deserialises
  unchanged. The published counter table header now reads `res ipis`, with a line underneath
  naming what RES is and why no per-CPU context-switch count exists.
  `config/contamination-thresholds.json`'s `context_switch_delta_max` is deliberately
  untouched: it names a genuinely different, still-unimplemented quantity.

- **Finding C1. `verify --strict` does not re-derive firmware-screen figures from the raw
  capture.**
  Confirmed directly: `crates/cli/src/cmd/verify.rs` only knows `hwlatdetect*.txt` and
  `rtla-hwnoise*.txt` as filename globs for checksum and stray-file detection (lines 74 and
  80); nothing in that file parses the raw capture and compares the result against
  `manifest.json`'s `firmware_screens[].max_us`/`observed_cpus`. A hand-edited firmware
  maximum in a committed `REPORT.md` or `manifest.json` would pass `verify --strict` today.
  Owner: whichever plan extends `verify.rs`'s strict mode to re-run `nr_capture::hwnoise::
  parse_hwnoise_file` (and the equivalent `hwlatdetect` parser) against the checksummed raw
  capture and reconcile the result against the manifest, the same reconciliation D-15/D-19
  already do for the cyclictest histogram.

  **CLOSED, commit `c4b2b4c` (plan 01-26 task 1).** `check_firmware_figures` re-parses each
  run's `rtla-hwnoise.txt` with the existing `nr_capture::hwnoise` parser (no second one
  written) and compares the re-derived `observed_cpus`, `max_us`, `events_recorded` and
  per-cpu exposure against both `manifest.json`'s `firmware_screens` and `REPORT.md`'s
  `## Firmware screen` block. `hwlatdetect` is out of scope: the four committed firmware
  screens are all `rtla-hwnoise`, and a screen from any other instrument is recorded not
  re-derivable by name rather than silently skipped.

- **Finding C2. All three new D-18 captures record `harness.git_sha:
  "unavailable-at-build-time"`.**
  Confirmed in all three manifests
  (`measurements/2026-09-06-precision3591-screen-03/manifest.json`,
  `measurements/2026-09-06-precision3591-screen-04/manifest.json`,
  `measurements/2026-09-07-precision3591-screen/manifest.json`, all line 12). This is the
  honest value, not a bug: it is the same rig-has-no-git condition the 01-22 entry above
  already recorded ("`git` is not installed on the rig, and `~/neurorust` there is a plain
  directory, not a git clone"), now showing up in provenance data for the first time since
  that entry was filed. The field correctly refuses to fabricate a commit hash, but it
  identifies which bytes ran, not which source revision produced them. Owner: whichever plan
  gives the rig a real git identity (installing `git` and cloning properly, closing the 01-22
  entry) or adds a secondary, content-addressed identity (for example hashing the built
  binary's own embedded source manifest) that does not depend on the rig having git at all.

- **Finding B5. A CPU absent from `rtla hwnoise` output is reported as "sampled and reported
  nothing," which missing output cannot establish.** The exact phrase is in
  `crates/metrics/src/report.rs:419`. Idle CPU 7 in the 2026-09-06 idle arm is the
  counter-example that shows the claim can be wrong in the other direction too: a CPU that
  genuinely was sampled and saw nothing still produces an explicit row (`Noise 0, Max Single
  0, HW 0`, full `674250000` us of `Runtime`). A CPU missing from the output entirely has no
  such row to point to, so "sampled and reported nothing" and "never sampled" are
  indistinguishable from the rendered text alone; only checking `observed_cpus` against
  `requested_cpus` (which the coverage test already does) actually distinguishes them. Owner:
  whichever plan next touches `render_firmware_screens`/`render_plat03_verdict`
  (`crates/metrics/src/report.rs`) should reword the missing-CPU case to say plainly that the
  CPU produced no row and coverage cannot be confirmed for it, rather than asserting sampling
  occurred.

  **CLOSED, commits `c4b2b4c`/`bf850e7` (plan 01-26 tasks 1 and 2).** A requested CPU that
  produced no row is now a `verify --strict` problem worded as a coverage failure ("coverage
  cannot be confirmed for it"), never a claim about whether it was sampled (task 1). The
  rendered `rtla-hwnoise` caveat no longer says "sampled and reported nothing"; it states that
  a sampled CPU always emits its own row, citing cpu 7's committed row in
  `2026-09-06-precision3591-screen-03` as the proof, so a missing CPU produced no row at all
  (task 2).

- **The published `manifest blake3:` line is not the blake3 of `manifest.json`, and it changed
  on eleven reports without any evidence changing.** Found 2026-09-07 while independently
  re-checking plan 01-26's report rewrite. `render_header`
  (`crates/metrics/src/report.rs:193`) prints `manifest_blake3(manifest)`, which is
  `blake3(serde_json::to_vec(manifest))`: a hash over the manifest re-serialised by the
  running build, not over the committed file. So for
  `2026-09-06-precision3591-screen-03` the report states
  `98aaa38b...` while `b3sum manifest.json` gives `3d02815a...`, and the pre-rewrite report
  stated `0bd5cc00...` against the same unchanged file. The field has never equalled the
  file digest; this is not a regression from 01-26.

  Two things are wrong with it. The label reads as "the blake3 of the manifest" and is not,
  which is the same defect shape as finding B4 that plan 01-26 just fixed: the code is
  honest (the `manifest_blake3` doc comment says "computed by the renderer itself rather than
  read from a field") and the published label is not. A reader who checks the stated digest
  against the file gets a mismatch and would reasonably read it as tampering, in a repository
  whose entire claim is that published evidence can be checked.

  And the guarantee its own doc comment asserts is weaker than stated. It says "the header
  line changes whenever the manifest's content changes, so the two cannot silently drift
  apart." Plan 01-26 demonstrated the converse: renaming the `context_switches` field to
  `rescheduling_ipis` behind a serde alias changed the rendered digest on eleven reports while
  no manifest content changed at all. The fingerprint tracks serialisation shape, not content,
  so it cannot detect drift between a report and the manifest file beside it.

  Owner: whichever plan next touches `render_header`. The likely fix is to publish
  `blake3_file(manifest.json)` (`crates/manifest/src/checksum.rs:31`), which is the quantity
  the label already promises and the one a reader can reproduce, and to correct the
  `manifest_blake3` doc comment. Note this makes the eleven reports change again, so it
  belongs with a rewrite pass rather than on its own.
