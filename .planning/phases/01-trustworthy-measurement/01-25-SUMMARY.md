---
status: PASS
agent: donny-executor
phase: 01-trustworthy-measurement
plan: 25

subsystem: benchmark-methodology
tags: [rust, preconditions, tracing, rtla, osnoise, timerlat, fixtures, shell]

# Dependency graph
requires:
  - phase: 01-trustworthy-measurement
    provides: "TracersQuiescent widened to all four top-level tracing controls (plan
      01-18), PreconditionSpec carrying target_cpus, SystemFacts::list_dir and
      running_processes_matching, FixtureFacts::with_dir (plans 01-05 through 01-18)"
provides:
  - "check_tracers_quiescent also lists /sys/kernel/tracing/instances/ and checks the
    same four controls inside every instance found, and scans /proc/*/comm for an
    osnoise/<cpu> or timerlat/<cpu> kthread on each target CPU, closing finding A2"
  - "SAMPLER_KTHREAD_PREFIXES, TRACING_INSTANCES_DIR, sampler_kthreads_on,
    live_tracing_instances: the two new signals, each independently testable through
    FixtureFacts"
  - "scripts/nr-measure-mode quiesces every tracing instance on 'on' and warns if a
    sampler kthread survives; 'status' reports both signals. The guard that used to
    live only in ~/nr-arm.sh on the rig is now version controlled."
  - "docs/measurement-protocol.md and README.md describe all six TracersQuiescent
    signals while every existing statement of the fifteen-precondition count stays
    true"
affects: [01-26, 01-27, any future plan taking a headline/weekly/soak run through
  nrmeasure run]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Extend an existing named check rather than add a new one, when the check's own
      name already covers the new signal: keeps every published statement of a
      precondition count true without a second check a reader has to remember to
      read alongside the first (same pattern plan 01-18 set for this identical
      check)"
    - "Capture a pipeline's output into a variable before branching on emptiness,
      rather than `pipeline || fallback`: the exit status of A | B is B's, not A's,
      so a `||` after a pipe cannot see whether the first, failing-prone command
      (ls, pgrep) actually failed or just produced no output"

key-files:
  created: []
  modified:
    - crates/capture/src/preconditions.rs
    - crates/capture/tests/preconditions.rs
    - crates/cli/tests/snapshots/run_pipeline__full_run_report_matches_snapshot.snap
    - scripts/nr-measure-mode
    - docs/measurement-protocol.md
    - README.md
    - .planning/phases/01-trustworthy-measurement/deferred-items.md

key-decisions:
  - "requirements-completed left empty. PLAT-02 ('a clean measurement protocol is
    defined and followed') is not complete until a human follows it on the rig; this
    plan runs entirely on the macOS dev host against fixtures, per its own objective
    and unbroken precedent (01-09, 01-18, 01-20 through 01-23)."
  - "The plan's own acceptance criterion 'grep -c \"15 precondition\"
    docs/measurement-protocol.md reports 2, unchanged' does not match this
    repository: the real count was already 3 before this plan touched the file (a
    third occurrence, 'the 15 preconditions themselves', was added by 01-24's D-28
    replan on 2026-09-07, after this plan was drafted). Verified the count is
    unchanged BY this plan (still 3, all three still true) rather than forcing the
    literal number 2 by deleting an unrelated, correct sentence from a different
    plan's work."
  - "Fixed the plan's own suggested shell for the two new 'status' lines rather than
    copying it verbatim: `pgrep ... | tr ... || echo none` cannot see pgrep's exit
    status once piped into tr (tr succeeds on empty input regardless), so the
    fallback never fires and a genuinely empty result prints as nothing, not 'none'.
    Verified empirically against three cases (unreadable, empty, populated) before
    and after the fix."
  - "Closed the 01-23 deferred-items.md entry this plan exists to fix, with both
    commit hashes, rather than leaving it to a future pass: the entry named the exact
    fix location and both new signals, so closing it in place keeps the file's own
    convention (every entry says CLOSED, commit, once its fix lands)."

# Metrics
duration: 23min
completed: 2026-09-07
---

# Phase 01 Plan 25: TracersQuiescent reads rtla's own tracing instance Summary

**`TracersQuiescent` now fails a headline run when an osnoise or timerlat sampling thread is orphaned on a target CPU or a live tracing instance exists, closing the exact gap that starved `measurements/2026-09-06-precision3591-screen-02`, and the operator-side guard that used to live only on the rig is now version controlled in `scripts/nr-measure-mode`.**

## Performance

- **Duration:** 23 min (estimated; commits span 03:05:31 to 03:10:06 -0500)
- **Started:** approx. 2026-09-07T07:50:00Z
- **Completed:** 2026-09-07T08:13:17Z
- **Tasks:** 2
- **Files modified:** 7

## Accomplishments

- `check_tracers_quiescent` now takes the whole `PreconditionSpec` (not just
  `InstrumentClass`) and, for a `headline-series` run, also lists
  `/sys/kernel/tracing/instances/` (checking the same four controls inside every
  instance found) and scans `/proc/*/comm` for an `osnoise/<cpu>` or `timerlat/<cpu>`
  kthread on each of the run's own target CPUs. A machine with an orphaned osnoise
  thread on a measured core can no longer pass the check, and the refusal names the
  thread or the instance by name.
- An unreadable instances directory is recorded as `instances=unavailable` and is
  never treated as a violation, matching the same rule the four top-level controls
  already followed for an absent control file.
- An `investigation` run keeps status `NotApplicable` (tracing is the point of that
  run class) but now also records both new signals, so a capture that was itself
  starved by an orphaned sampler leaves the evidence in its own manifest.
- Eight new tests in `crates/capture/tests/preconditions.rs`, one named directly for
  the 2026-09-06 capture it exists because of, carrying that capture's real cycle
  counts and `runtime_us`/`period_us` values in its own comment rather than a rounder,
  invented pair of numbers. `run_all` still returns exactly fifteen results
  (`all_fifteen_preconditions_are_still_evaluated`), and `crates/manifest/` is
  untouched.
- `docs/measurement-protocol.md`'s `TracersQuiescent` row now names all six signals
  and the two commands that clear the new ones; a new "Orphaned sampling threads"
  subsection states the 2026-09-06 case with its real numbers. `README.md`'s one-line
  precondition summary now names the instance and kthread signals instead of "all
  four control files". The published count of fifteen preconditions is unchanged and
  still true everywhere it is stated.
- `scripts/nr-measure-mode`'s `on` arm now quiesces every tracing instance
  (`current_tracer=nop`, `tracing_on=0`, then removes the instance directory) and
  warns to stderr if a sampler kthread survives; `status` reports both the live
  instances and any sampler kthreads. This moves the guard that was living only in
  the operator's `~/nr-arm.sh` on the rig into version control; it reaches the rig
  once plan 01-27 reinstalls the script.

## Task Commits

Each task was committed atomically:

1. **Task 1: TracersQuiescent reads the instances and the sampling threads** -
   `b04c229` (feat), plus `30e49f5` (fix: re-pin the one changed report row, a direct
   consequence of task 1's own change)
2. **Task 2: The protocol, the count, and the operator guard moved into the
   repository** - `0b1fa65` (docs)

**Plan metadata:** (this commit, made immediately after this SUMMARY) docs(01-25): complete plan

## Files Created/Modified

- `crates/capture/src/preconditions.rs` - `SAMPLER_KTHREAD_PREFIXES`,
  `TRACING_INSTANCES_DIR`, `sampler_kthreads_on`, `live_tracing_instances`;
  `check_tracers_quiescent` takes `&PreconditionSpec` and evaluates both new signals
  for `HeadlineSeries`, records them for `Investigation`
- `crates/capture/tests/preconditions.rs` - 8 new named tests; updated the two
  existing `TracersQuiescent` assertions (`tracers_quiescent_refuses_headline_run`,
  `tracers_quiescent_allows_investigation_run`) for the two new observed-string
  segments
- `crates/cli/tests/snapshots/run_pipeline__full_run_report_matches_snapshot.snap` -
  re-pinned the `tracers-quiescent` row's observed/expected cells
- `scripts/nr-measure-mode` - `on` arm quiesces every tracing instance and warns on a
  surviving sampler kthread; `status` arm reports both signals
- `docs/measurement-protocol.md` - `TracersQuiescent` row rewritten for six signals;
  new "Orphaned sampling threads" subsection
- `README.md` - the one-line precondition summary names the instance and kthread
  signals
- `.planning/phases/01-trustworthy-measurement/deferred-items.md` - closed the
  2026-09-06 entry this plan fixes, with both commit hashes

## Decisions Made

See `key-decisions` in the frontmatter for full rationale. In short: `requirements-completed`
stays empty (PLAT-02 needs a real rig run, unbroken precedent since 01-09); the plan's
own literal "count reports 2" acceptance criterion does not match this repository
(a true, pre-existing third occurrence already existed, added by 01-24 the same day),
verified unchanged rather than forced to a stale number; the plan's own suggested
shell for the two new `status` lines was fixed rather than copied, because it cannot
actually distinguish "nothing found" from "unreadable" once piped through `tr`
(verified empirically before writing the fix); and the deferred-items.md entry this
plan closes was marked closed in place, matching the file's own established
convention.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Re-pinned `run_pipeline__full_run_report_matches_snapshot.snap`'s `tracers-quiescent` row**
- **Found during:** Task 1, `cargo test --workspace`
- **Issue:** The committed full-pipeline snapshot pinned the pre-widening
  `tracers-quiescent` observed/expected strings (four signals). Task 1 widens both to
  six, so the snapshot assertion failed with exactly the predicted diff.
- **Fix:** Updated the one changed row to the new, correctly-computed strings
  (`instances=unavailable samplers=none` appended to observed; the two new clauses
  appended to expected). No test logic changed.
- **Files modified:** `crates/cli/tests/snapshots/run_pipeline__full_run_report_matches_snapshot.snap`
- **Verification:** `cargo test -p nr-cli --test run_pipeline full_run_report_matches_snapshot`
- **Committed in:** `30e49f5` (separate commit immediately after `b04c229`, since the
  core commit had already been made when `cargo test --workspace` surfaced this)

**2. [Rule 1 - Bug] Fixed the plan's own suggested `status`-arm shell, which cannot print `none`**
- **Found during:** Task 2, while implementing the two new `status` lines the plan's
  own action text specifies verbatim
  (`` printf '...' "$(ls -1 ... | tr '\n' ' ' | sed 's/ $//' || echo 'unreadable ...')" ``
  and `` printf '...' "$(pgrep -a '...' | tr '\n' ';' || echo none)" ``)
- **Issue:** In `A | B || C`, the exit status `||` sees is `B`'s (here `tr`/`sed`),
  not `A`'s (`ls`/`pgrep`). `tr` and `sed` succeed on empty input regardless of
  whether `ls`/`pgrep` themselves failed or simply found nothing, so `|| echo none` /
  `|| echo 'unreadable ...'` never fires. Verified directly in a shell before
  writing the fix: a guaranteed-no-match `pgrep` piped exactly as the plan specifies
  printed an empty string, not `none`. This directly contradicts the plan's own
  acceptance criterion ("prints `none` rather than an empty field when nothing is
  found").
- **Fix:** Capture the listing into a variable first (so the `if`/`:-` can see the
  real command's own exit status and emptiness separately), then apply
  `${var:-none}` / an explicit unreadable branch. Verified against five cases
  directly in a shell (unreadable, empty-readable, populated instances; no match and
  a match for the process scan) before and after.
- **Files modified:** `scripts/nr-measure-mode`
- **Verification:** `bash -n`, `shellcheck` (clean), and the five-case manual
  verification above
- **Committed in:** `0b1fa65` (Task 2's own commit)

**Total deviations:** 2 auto-fixed (both Rule 1 bugs, both direct and predictable
consequences of the two tasks' own mandated changes). **Impact on plan:** neither
expands scope beyond the two tasks' own files; the second one changes the plan's own
suggested implementation to actually satisfy the plan's own stated acceptance
criterion, verified empirically rather than assumed.

## Issues Encountered

`cargo test --workspace`, run mid-task with uncommitted changes present, twice showed
`harness_git_sha_source_is_explicit` failing with `git_dirty` reported `false` against
a genuinely dirty tree. Root cause, confirmed by reading `crates/cli/build.rs`: that
test compares the live `git status --porcelain` state against a value
`nr-cli`'s own build script bakes in at compile time, and the build script's
`cargo:rerun-if-changed` only tracks `.git/HEAD` and `.git/index`, not working-tree
content, by design (it exists to close finding 6 of `01-EXTERNAL-AUDIT.md`, not to
rebuild on every unstaged edit). This is inherent to running the full suite against
an actively-edited, not-yet-committed tree, not a defect in this plan's own files; it
resolved on its own immediately after each task's commit changed `.git/HEAD` and
forced a fresh, correct build-script run. `cargo test --workspace` is green on the
final, fully committed tree (confirmed directly, see below).

## User Setup Required

None - no external service configuration required. The updated `scripts/nr-measure-mode`
is not live on the rig; it reaches the rig once plan 01-27 reinstalls it via
`deploy/sudoers/install.sh`, as the plan itself states.

## Next Phase Readiness

- Finding A2 of `01-REVIEW-2026-09-06.md` is closed: `TracersQuiescent` establishes
  what its name claims against `rtla`'s own tracing instance, not only the four
  top-level controls plan 01-18 widened it to.
- Plan 01-26 (wave 17, C1/B4/B5) and plan 01-27 (wave 18, D-29 and C2, which also
  reinstalls the rig's scripts) are next per STATE.md's own sequencing; neither is
  blocked by anything this plan touched.
- No new blockers. The pre-existing blockers (the unexplained 3.8 ms global stall,
  the untuned-boot governor race, the D-18 firmware-floor gap already closed by
  01-20 through 01-23) are unchanged and out of this plan's scope.

## Final verification (clean, fully committed tree)

```
cargo fmt --check                                          clean
cargo clippy --workspace --all-targets -- -D warnings       clean
cargo test --workspace                                      31 suites, 0 failed
cargo build -p nr-cli --release                              built
./target/release/nrmeasure verify --strict --check-index    13 run directories, 11 re-derived,
                                                              1 not re-derivable, 0 problems
git diff --exit-code measurements/                          untouched
```

*Phase: 01-trustworthy-measurement*
*Completed: 2026-09-07*

## Self-Check: PASSED

All 7 claimed modified files found on disk, plus this SUMMARY.md itself. All 3
claimed commit hashes (`b04c229`, `30e49f5`, `0b1fa65`) found in `git log --oneline --all`.
