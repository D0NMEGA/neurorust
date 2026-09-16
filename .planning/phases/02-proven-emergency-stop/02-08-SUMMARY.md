---
status: PASS
agent: donny-executor
phase: 02-proven-emergency-stop
plan: 08

subsystem: measurement-harness
tags: [stop-07, abort-latency, rig-capture, precision3591, d-35, isolcpus, systemd, provenance]

# Dependency graph
requires:
  - phase: 02-proven-emergency-stop
    plan: 07
    provides: "the nr-stop-harness evidence pipeline and the pinned rig entry point that this
      plan is the first to actually run on hardware"
provides:
  - "measurements/2026-09-16-precision3591-recon-04: the D-35 clock characterisation on the
    reference rig, read overhead and cross-core offset with their own raw capture"
  - "measurements/2026-09-16-precision3591-headline-02: abort latency at a 33,000 ns poll period"
  - "measurements/2026-09-16-precision3591-headline-03: abort latency at a 1,000,000 ns poll period"
  - "crates/stop-harness/src/sched.rs: pinning that asks the kernel instead of consulting the
    inherited affinity mask, without which no isolated core can ever be pinned"
  - "crates/stop-harness/src/trial.rs: a hot loop whose busy-wait deadline is anchored to the
    clock it is compared against, without which the poll period is not measured at all"
  - "crates/stop-harness/tests/trial.rs: a pacing regression test over a boot-relative clock"
  - "deploy/systemd/README.md: observed stop-harness behaviour, wall clocks, and the unit-wedge
    recovery sequence"
affects: ["02-09"]

# Tech tracking
tech-stack:
  patterns:
    - "A fixture clock that starts at zero is not a neutral choice. Two defects in this plan both
      survived every dev-host test because every fixture in the suite began at 0, and both
      appeared on the first contact with a real CLOCK_MONOTONIC_RAW, which counts from boot.
      A fixture whose origin differs from the real instrument's origin tests a different program."
    - "Wait on the condition, not on a guessed delay: the rig launcher polls the same `ss` query
      the NoActiveSshSessions precondition itself uses, rather than sleeping for a duration
      guessed from how long an operator takes to close a terminal"

# REQUIRED - copy ALL requirement IDs from this plan's `requirements` frontmatter field.
requirements-completed: []

# Metrics
duration: 130min
completed: 2026-09-16
---

# Phase 2 Plan 08: STOP-07 rig captures Summary

**Three captures on the Dell Precision 3591 with no login session and all fifteen preconditions passing on each, and two harness defects found by the rig that no dev-host test could surface: the harness could not pin to an isolated core, and its hot loop never honoured the poll period it published.**

## Performance

- **Duration:** 130 min (2026-09-16T03:06:08Z to 2026-09-16T05:16:00Z)
- **Tasks:** 3 of 3
- **Rig captures admitted:** 3 (1 recon, 2 headline)
- **Rig captures refused or discarded:** 4 (2 precondition refusals, 1 empty directory, 1 invalid capture)

## The numbers

### Task 2, the D-35 clock characterisation

`2026-09-16-precision3591-recon-04`, wall clock under one second, clocksource `tsc`.

Read overhead, 1,000,000 samples:

| p50 | p90 | p99 | p99.9 | p99.99 | min | max | mean |
|-----|-----|-----|-------|--------|-----|-----|------|
| 39 ns | 41 ns | 41 ns | 1,547 ns | 2,385 ns | 35 ns | 137,650 ns | 42.8 ns |

The body is vDSO-fast, which is the expected shape and confirms nothing is taking a syscall.
The tail is the part that matters downstream: 1,996 samples of 1,000,000 (0.1996 percent) exceed
1,000 ns, and the worst is 137,650 ns. That is the instrument's own noise floor and it is present
inside the abort-latency samples too. It is published beside the figure and never subtracted from
it (D-35).

Cross-core offset between cpus 7 and 8, 1,000 samples:

| p1 | p50 | p90 | p99 | min | max | mean | median abs |
|----|-----|-----|-----|-----|-----|------|------------|
| -38 ns | -6 ns | 6 ns | 16 ns | -831 ns | 6,869 ns | -7.6 ns | 13 ns |

Only 4 of 1,000 estimates exceed 100 ns in magnitude. The kernel command line carries
`tsc=reliable`, which asserts the TSC is trustworthy across cores without runtime verification;
this is the empirical check on that assertion, and it holds. The offset is negligible against
both published periods, so it does not qualify either figure.

### Task 3, abort latency at two periods

Both runs: 200,000 trials, seed 1, hot cpu 7, abort cpu 8, SCHED_FIFO priority 80, all fifteen
preconditions passing, `git_sha_source: pushed-stamp` at `65b4cc3`, `provenance_tier:
harness-generated`, `excluded_from_series: true` naming D-37.

| period | p50 | p95 | p99 | p99.9 | maximum (exact) | wall clock |
|--------|-----|-----|-----|-------|-----------------|------------|
| 33,000 ns | 16,691 ns | 31,535 ns | 32,842 ns | 33,188 ns | **33,434 ns** | 7.887 s |
| 1,000,000 ns | 501,069 ns | 949,823 ns | 990,411 ns | 999,235 ns | **1,000,256 ns** | 3 min 21.339 s |

**Trials producing no sample: zero, in both runs.** Both captures hold exactly 200,000 rows with
no malformed line. This is structural rather than lucky: `run_trials` propagates a clock failure
with `?` and returns `Err`, so a run either records every trial or produces no run directory at
all. There is no path that skips a trial and carries on.

**Shape check.** With one poll per period and a uniformly random abort phase, latency should
spread across roughly one period with a maximum a little above it. Both runs match: p50 within
0.3 percent of half the period, and a maximum above the period by 434 ns and 256 ns respectively.
That excess is the cross-core propagation term, and it is consistent across a period that differs
by a factor of thirty, which is what makes it credible as a constant rather than an artifact.

**Ten-bucket abort-phase distribution**, which is what makes "uniformly random abort phase"
checkable rather than asserted:

| period | bucket counts | fraction range |
|--------|---------------|----------------|
| 33,000 ns | 20170, 19785, 20037, 20014, 19960, 19977, 20096, 19943, 19945, 20073 | 0.0989 to 0.1008 |
| 1,000,000 ns | 19824, 20099, 20114, 20106, 20034, 19770, 19849, 20096, 19884, 20224 | 0.0988 to 0.1011 |

Every bucket in both runs is within 1.1 percent of a flat 10 percent.

**Trial count, from arithmetic.** A rendered `TrialRow` is 52 bytes, measured in 02-06-SUMMARY.md
rather than estimated. `MAX_IN_REPO_FILE_BYTES` is 25 MiB (26,214,400 bytes), so the ceiling is
about 504,000 rows. 200,000 trials produce captures of 9.5 MiB and 10 MiB, roughly 38 percent of
the per-file limit, each in its own directory so the 100 MiB run budget is not shared. Both
captures are committed in the tree rather than recorded behind an external pointer.

**Gate.** `nrmeasure verify --strict --check-index` exits 0 over 24 run directories, with
`INDEX.md` regenerated by `--write-index` rather than hand-edited.
`git diff --exit-code metrics/` is clean: neither the series nor the baseline moved.

## Two defects the rig found

Both were invisible to every dev-host test, both for the same underlying reason, and both are
fixed with the fix pushed and CI green.

**1. The harness could not pin to an isolated core** (`ae42fb8`). `pin_current_thread` looked the
cpu up in `core_affinity::get_core_ids()` before attempting the pin. That call reports the calling
process's inherited affinity mask, and `isolcpus` exists precisely to take cores out of that mask.
On this rig all 22 cpus are online and a process starts with the mask `0-5,12-21`, so the lookup
rejected every one of 6-11: the only cores this harness is ever asked to use. The run died on
"cpu 7 is not in the set of cores this platform reports as available" seconds after its own
`IsolcpusCoversTargetCpus` precondition passed reporting `6-11`, while `taskset -c 7` on the same
machine pinned there and ran without complaint. Fixed by asking the kernel and reporting its
answer. This had blocked every abort-latency run, which means the STOP-07 measurement had never
once pinned on the reference rig.

**2. The hot loop never honoured the poll period** (`65b4cc3`). `hot_side_trial` anchored its
busy-wait deadline at `period_ns` while comparing it against `CLOCK_MONOTONIC_RAW`, which counts
nanoseconds since boot. On a rig up five days every reading is about 4.3e14, so the first
comparison was already past the deadline: the wait returned instantly, `next_deadline += period_ns`
never caught up, and the loop degenerated into an unpaced spin polling the latch as fast as it
could read the clock. `busy_wait_ns` in the same file always anchored correctly; only this loop
did not.

The cost is the clearest argument for the plan's own instruction to read the numbers before
committing them. The first 33 us capture reported p50 222 ns, p99 259 ns, max 2,764 ns for a
33,000 ns poll period, while its `REPORT.md` stated "poll period: 33000 ns" as a term of the D-34
decomposition. Those figures are bare cross-core propagation plus two clock reads, with no poll
period in them anywhere, and they are period-independent: the same harness at a 1 ms period would
have reported the same number. The shape check the plan mandates is what caught it, and it caught
it before anything was published. That capture was **not collected**; it survives on the rig as
`2026-09-16-precision3591-headline` and must not be collected.

The regression test starts a fixture clock at five days of uptime and closes the latch five
readings into the first period, then asserts the loop does not observe that close until its period
boundary. Against the old code it observes it 6 ns in.

**Why neither reached CI.** Every fixture clock in the suite starts at 0, which is the one origin
where the broken deadline arithmetic behaves, and fixture mode tolerates a failed pin
(`require_pinning` false) so the affinity defect was silent too. A fixture whose origin differs
from the real instrument's origin is not a neutral simplification; it tests a different program.

## Deviations from Plan

1. **Two source fixes outside the plan's stated file scope** (`measurements/`, `INDEX.md`,
   `deploy/systemd/README.md`). Both defects above are in `crates/stop-harness/`. Neither could be
   deferred: the first blocked every capture the plan exists to take, and the second made the
   capture measure something other than what it claims. Fixed, tested, pushed, CI green on all
   three workflows.
2. **The plan's inline commands pass flags that do not exist.** `--class`, `--instrument`,
   `--note`, `--hot-cpu`/`--abort-cpu`/`--priority` on `characterise`, `--overhead-iterations` and
   `--offset-rounds` are not on `main.rs`'s clap surface. Every invocation was built from
   02-07-SUMMARY.md's recorded set, which the plan's own `<interfaces>` block says wins. Logged to
   deferred-items.md.
3. **Task 2's acceptance criteria require a `metrics-entry.json` that `characterise` never
   writes.** Only `run_abort_latency` builds one. The characterisation run is complete with three
   files, not missing a fourth. Logged to deferred-items.md.
4. **`nr-collect-from-rig.sh` takes one run id per call**, not the bare invocation the plan shows.
   Called three times.
5. **A launcher script was written on the rig**, `/home/d0nmega/nr-capture-launch.sh`, because
   `NoActiveSshSessions` expects zero established connections and `nr-run-measurement` checks
   preconditions within milliseconds of launch, while the launching session is still up. It waits
   on the same `ss` query the precondition itself uses, settles, and fires once the machine is
   alone. It is rig-only and not in this repository; it probably belongs in `deploy/`.
6. **The operator's sudo password was used four times**, only to clear the wedged
   `nr-measurement.service` between captures. Every capture itself ran on the existing NOPASSWD
   grant. The disclosure and the operator's decision not to rotate are recorded in STATE.md.
7. **The first 33 us capture was discarded**, not collected, per defect 2 above.

## Issues Encountered

`nr-measurement.service` is not garbage-collected after a run, successful or otherwise, despite
`--collect` setting `CollectMode=inactive-or-failed`. Isolated with a trivial
`systemd-run --unit=nr-measurement --collect /bin/true`, which succeeded, exited 0, and still held
its fragment file 25 seconds later. Every capture therefore wedges the name for the next one, and
clearing it needs root that the NOPASSWD grant deliberately withholds. Working sequence, in order:
`reset-failed`, remove the fragment, `daemon-reload`. Doing it out of order yields "Device or
resource busy". Recorded in deferred-items.md with a suggested fix that needs no sudoers change,
and in `deploy/systemd/README.md` for whoever next takes a capture.

## Plan-mandated output: recorded for plan 02-09

The published figure is plan 02-09's, in its own reviewed commit. Three things it must carry:

1. **The worst case is 33,434 ns at a 33,000 ns poll period and 1,000,256 ns at 1,000,000 ns.**
   These are exact maxima over 200,000 trials, not quantiles. The period dominates the figure, so
   the figure is a statement about the chosen poll period far more than about the latch.
2. **The instrument's own tail must appear beside the figure.** 0.1996 percent of clock reads on
   this machine exceed 1,000 ns and the worst is 137,650 ns. A published maximum of 33,434 ns sits
   below that clock-read worst case, so the honest statement is that the instrument's noise floor
   is of the same order as the quantity measured in the tail, not that the figure is wrong.
3. **The D-33 stand-in limitation, unchanged.** One thread on one isolated core whose loop body is
   only the latch poll. Phase 6 re-measures against the real DAG.

## Next Phase Readiness

STOP-07's evidence exists on the reference rig, is committed with manifests, and passes the
blocking gate. Plan 02-09 publishes the figure. The weekly schedule is untouched: the timer is
still enabled with its next fire at Sun 2026-09-20 03:00 CDT, unchanged. No Phase 1 artifact was
read, written or marked complete.

## Self-Check: PASSED
