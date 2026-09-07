---
status: PARTIAL
agent: donny-executor
phase: 01-trustworthy-measurement
plan: 23
subsystem: measurement
tags: [rtla-hwnoise, msr-smi-count, firmware-floor, provenance, D-18, D-27]

# Dependency graph
requires:
  - phase: 01-trustworthy-measurement
    provides: "nrmeasure run --with-hwnoise (01-21), the rtla-hwnoise parser and manifest fields (01-20), the passwordless rig entry point (01-22)"
provides:
  - "Three rtla hwnoise firmware screens on the installed PREEMPT_RT system naming CPUs 6-11 (idle, all-CPUs-loaded, housekeeping-loaded), each with an exact per-CPU MSR_SMI_COUNT delta"
  - "A corrected docs/rig/firmware-floor-rt-vs-stock.md: the re-take table, and three overstatements a second independent review found, reworded to what the captures support"
  - "A corrected measurements/2026-08-28-precision3591/README.md: the withdrawn 7us/18x figure marked unverifiable everywhere it is used, and the kernel-independence carry-over claim marked as the untested assumption it was"
affects: [PLAT-03, BENCH-06, any later plan citing a firmware floor for CPUs 6-11]

# Tech tracking
tech-stack:
  added: []
  patterns: ["cite the manifest's own utc_start/utc_end when describing an instrument's true measurement bracket, rather than the narrowest named sub-window"]

key-files:
  created:
    - measurements/2026-09-06-precision3591-screen-03/ (arm 1, idle)
    - measurements/2026-09-06-precision3591-screen-04/ (arm 2, 22 CPUs loaded)
    - measurements/2026-09-07-precision3591-screen/ (arm 3, housekeeping loaded)
  modified:
    - docs/rig/firmware-floor-rt-vs-stock.md
    - measurements/2026-08-28-precision3591/README.md
    - crates/capture/tests/firmware_cpu_coverage.rs
    - .planning/phases/01-trustworthy-measurement/deferred-items.md

key-decisions:
  - "Reworded the 1us-on-every-CPU claim to a pooled per-arm maximum after CPU 7's idle-arm row showed Max Single 0, not 1"
  - "Reworded the MSR_SMI_COUNT bracket in three places to name cyclictest+hwnoise+snapshot (about 961s per manifest), not the 900s hwnoise window alone"
  - "Led the tuned-idle improvement claim with the verifiable at-least-12x figure; kept the unsaved-capture 18x figure visible but explicitly marked as dependent on unverifiable data"
  - "Marked the kernel-independence carry-over claim as untested rather than editing the sentence itself, since docs/rig/firmware-floor-rt-vs-stock.md quotes it verbatim as the historical claim under examination"
  - "Left the 'What PLAT-03 should report' section and the 3.8ms attribution paragraph byte-identical, per this plan's own task text and the executor's explicit instruction, even though one sentence inside the former is now stale in the same way as the fixed claims; flagged at the checkpoint rather than resolved unilaterally"
  - "Logged findings B4, B5, C1, C2 from 01-REVIEW-2026-09-06.md to deferred-items.md with named owners rather than fixing them, matching the review's own disposition that they are independent of this plan's B1/B2/B3"

patterns-established:
  - "When a document states an instrument's exposure bracket, cite what the code comment actually says it brackets, not what the surrounding prose's most recent number suggests"

requirements-completed: []

# Metrics
duration: ~15min (this session's corrections only; task 1's captures and task 2's document corrections were an earlier session, see Task Commits)
completed: 2026-09-07
---

# Phase 1 Plan 23: Re-take D-18 with rtla hwnoise Summary

**Three rtla hwnoise firmware screens now name CPUs 6-11 with an exact zero MSR_SMI_COUNT delta each; a second independent review then found three overstatements in the resulting write-up, now corrected, with Task 3's human-verify checkpoint still open.**

## Performance

- **This session's duration:** ~15 min (reading the review, verifying against raw captures and source, editing, running the full verification suite, committing)
- **Tasks:** 2 of 3 complete (Task 1 and Task 2 committed in an earlier session); Task 3 is a blocking `checkpoint:human-verify`, not yet approved
- **Files modified this session:** 3 (docs/rig/firmware-floor-rt-vs-stock.md, measurements/2026-08-28-precision3591/README.md, deferred-items.md)

## Accomplishments

- Task 1 (earlier session): three `rtla hwnoise -c 6-11 -H 0-5 -P f:99 -d 900s` screens taken on the installed PREEMPT_RT rig -- idle, all 22 logical CPUs loaded, and housekeeping-only loaded -- every arm naming all six isolated CPUs and recording an exact `MSR_SMI_COUNT` delta of zero on each.
- Task 2 (earlier session): `docs/rig/firmware-floor-rt-vs-stock.md` and `measurements/2026-08-28-precision3591/README.md` corrected in place for the coverage defect the retake fixes; the firmware coverage test split into `hwlatdetect_captures_still_cover_no_isolated_core` (still true) and `hwnoise_captures_cover_the_isolated_cores` (now true).
- This session: a second independent review (`01-REVIEW-2026-09-06.md`, findings B1, B2, B3) found the Task 2 write-up itself overstated what the new captures support. All three corrected against the raw captures and the harness source, not against the review's prose alone (see Deviations below for the exact evidence checked for each).

## Task Commits

Earlier session (Tasks 1-2, for reference; not committed by this invocation):

1. `9926cf6` fix(01-23): sum hwnoise duration into the run's timeout bound
2. `d4931e7` docs(01-23): record that a killed rtla starves the next run invisibly
3. `c969ab8` fix(01-23): add nrmeasure attempt to close an orphaned in-progress record
4. `fbbafe9` fix(01-23): mark the SIGTERM-killed arm 1 attempt failed, not in-progress
5. `6cf5613` fix(01-23): correct the CLI name attempt records for its own correction
6. `03a8f6e` fix(01-23): redact home-directory paths reliably under systemd-run
7. `ea33773` feat(01-23): commit the D-18 re-take: three rtla hwnoise arms plus retries (Task 1)
8. `078e476` fix(01-23): stop two workspace tests from choking on real firmware data
9. `ac67117` docs(01-23): correct the published firmware record for the isolated cores (Task 2)
10. `60eb3d6` docs(01-23): record mid-plan position at the Task 3 checkpoint (first checkpoint return)

This session (Task 3 preparation, second review's corrections):

11. `8442cb2` docs(01-23): correct three overstatements the second review found

**Plan metadata:** this commit (docs: record this session's corrections and re-present the Task 3 checkpoint)

## Files Created/Modified

- `docs/rig/firmware-floor-rt-vs-stock.md` - reworded the 1us-on-every-CPU claim to a pooled per-arm maximum (CPU 7 in the idle arm recorded 0, not 1); reworded the MSR_SMI_COUNT bracket in three places to name cyclictest+hwnoise+snapshot rather than the 900s hwnoise window alone
- `measurements/2026-08-28-precision3591/README.md` - marked the withdrawn 7us/1us-threshold figure unverifiable in the results table; led the Findings improvement claim with the verifiable "at least 12x" rather than the unverifiable "18x"; marked the SMI kernel-independence carry-over claim as the untested assumption it was, without altering the quoted sentence itself
- `.planning/phases/01-trustworthy-measurement/deferred-items.md` - logged findings B4, B5, C1, C2 from the second review with named owners

## Decisions Made

See `key-decisions` in the frontmatter. The one most worth restating in prose: this session found that FIX 3 in its own task instructions (a stale sentence "around line 182" of `firmware-floor-rt-vs-stock.md`, inside the "What PLAT-03 should report" section) conflicts with an explicit, doubly-sourced instruction not to touch that section (both this plan's own Task 2 action text and this session's task prompt). The official review (`01-REVIEW-2026-09-06.md`) does not list this sentence as a finding at all -- its B1/B2/B3 map exactly to the three fixes made here, with no fourth item. Given the conflict and the absence of the review's own backing, the section was left untouched and the staleness is flagged below for explicit human decision rather than resolved unilaterally.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Reworded the false "1us on every isolated CPU" claim (review finding B1)**
- **Found during:** Task 3 preparation (second independent review)
- **Issue:** `docs/rig/firmware-floor-rt-vs-stock.md`'s opening status paragraph stated all three arms "report a maximum single event of 1 us on every one of the six isolated CPUs." Checked directly against `measurements/2026-09-06-precision3591-screen-03/rtla-hwnoise.txt`'s final block: CPU 7 reads `Max Single 0, HW 0, Noise 0` at full `674250000`us `Runtime`. The claim is false for that CPU in that arm.
- **Fix:** Reworded to "each report a pooled maximum single event of 1 us across the six isolated CPUs -- no CPU exceeded it, and CPU 7 recorded no noise event at all in the idle arm." Checked the other two arms' final blocks directly: in both, all six CPUs independently show Max Single 1, so the false phrasing was specific to the idle arm.
- **Files modified:** docs/rig/firmware-floor-rt-vs-stock.md
- **Verification:** Hand-computed HW-event sums per arm from the raw files (13, 64, 44) match the table's existing event counts; cross-checked arm 1's REPORT.md, which independently reports "maximum: 1 us" and "events recorded: 13," matching.
- **Committed in:** 8442cb2

**2. [Rule 1 - Bug] Corrected the MSR_SMI_COUNT bracket description in three places (review finding B2)**
- **Found during:** Task 3 preparation (second independent review)
- **Issue:** The document stated three times that the SMI reads bracketed "each 900 second window" / "each run" in a way that reads as the hwnoise window alone. `crates/cli/src/cmd/run.rs` (lines 615 and 847-849) reads the register once before cyclictest starts and once after the firmware screen's window closes, so the true bracket also includes cyclictest and the harness's environment snapshots.
- **Fix:** Reworded all three occurrences to state the actual bracket (cyclictest plus the 900s hwnoise screen plus the snapshots around them), and cited the three manifests' own `utc_start`/`utc_end` (about 961 seconds each) as a concrete, checkable illustration that the true span exceeds 900s.
- **Files modified:** docs/rig/firmware-floor-rt-vs-stock.md
- **Verification:** Computed utc_end minus utc_start for all three manifests directly: 961.1s, 961.6s, 961.7s.
- **Committed in:** 8442cb2

**3. [Rule 1 - Bug] Stopped relying on a withdrawn figure in a derived claim, and marked an untested assumption as untested (review finding B3)**
- **Found during:** Task 3 preparation (second independent review)
- **Issue:** `measurements/2026-08-28-precision3591/README.md`'s own Caveats section (written by plan 01-19) says the 7us/1us-threshold observation has no raw capture in this repository and is "an unverifiable note rather than as evidence," yet the results table and the Findings section's "18x improvement" claim both still used it as if it were solid. Separately, the README still asserted SMI behaviour "carries over to a PREEMPT_RT install" with no qualification, which is exactly the assumption D-18 was opened to test.
- **Fix:** Table cell now reads "(7 us at a 1us threshold per an unsaved, unverifiable capture; see Caveats)." The Findings sentence now leads with "at least a 12x improvement" (125us to under 10us, both from committed captures) and states the 18x figure's dependency on the unsaved capture explicitly rather than presenting it as the headline number. Added a sentence immediately after the kernel-independence claim marking it untested when written and pointing to `docs/rig/firmware-floor-rt-vs-stock.md`'s Interpretation section, without editing the original sentence itself (it is quoted verbatim, as a historical claim under examination, in that same target document).
- **Files modified:** measurements/2026-08-28-precision3591/README.md
- **Verification:** `grep -c 'worst-case bound'` still 0 in both documents; `grep -q 'rtla hwnoise'` still matches; no non-ASCII characters introduced (checked both files with `LC_ALL=C grep -P '[^\x00-\x7F]'`).
- **Committed in:** 8442cb2

**Total deviations:** 3 auto-fixed (all Rule 1, factual corrections to already-published prose), plus 4 items logged to deferred-items.md rather than fixed (review findings B4, B5, C1, C2 -- named by the review itself as independent of this plan's scope), plus 1 flagged conflict left unresolved (below).

**Impact on plan:** All three fixes are corrections to prose only; no code changed, no test behavior changed. `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (139 tests across 30 binaries, all passing), and `./target/release/nrmeasure verify --strict --check-index` (0 problems) all pass after the edits, confirming the correction pass did not regress anything Task 1 or Task 2 established.

## Issues Encountered

**Unresolved conflict, flagged rather than resolved: FIX 3 in this session's own task instructions targets a sentence inside a section this plan is explicitly told not to touch.** The task's "FIX 3" identified a statement "around line 182" of `docs/rig/firmware-floor-rt-vs-stock.md` claiming no committed capture establishes a firmware observation on CPUs 6-11. That statement is real, at lines 182-183, and it is now stale in the same direction as the other three findings: the three new rtla hwnoise arms do establish firmware observations (zero SMI delta, a 1us noise ceiling) on CPUs 6-11, even though they still do not establish a *floor* (finding 1's point, which the surrounding paragraph is making). The sentence sits inside the "## What PLAT-03 should report" section (lines 178-194), which:

- this plan's own Task 2 action text requires to be kept "exactly as it is" because "it does not change because a better firmware number now exists,"
- this session's own task instructions separately list by name as a DO-NOT-TOUCH zone, requiring explicit confirmation that it is unchanged, and
- `01-REVIEW-2026-09-06.md` (the actual second review, findings B1 through C2) does not list at all -- its only findings assigned to plan 01-23 are B1, B2 and B3, which this session's other three fixes address in full.

Given two explicit, independently-sourced instructions not to edit this section, and no corroborating finding in the review that generated the other three fixes, this sentence was left untouched. `git diff` confirms the section is byte-identical to the pre-session state (see below). This is now a decision for the human reviewer at the Task 3 checkpoint: leave it as a known, minor staleness owned by whichever plan next revisits "What PLAT-03 should report" (most naturally the plan that closes finding 1 for good, per the review's own framing), or explicitly authorize a one-line qualification now.

**Confirmed unchanged, as required:**
- The "3.8 ms attribution paragraph" in `measurements/2026-08-28-precision3591/README.md` (the paragraphs around "the roughly 3.8 ms maximum appeared on all six threads," lines 132-151): `git diff` for this session's commit touches only lines 11-46, nowhere near this range.
- The "## What PLAT-03 should report" section in `docs/rig/firmware-floor-rt-vs-stock.md` (lines 178-194): `git diff` for this session's commit touches only lines 5-12 and 96-133, nowhere near this range.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Not ready to close. Task 3 (`checkpoint:human-verify`, blocking) is open and has not been self-approved. The checkpoint below restates the six checks from the plan's `<how-to-verify>` block against the now-further-corrected documents, plus the one flagged conflict above for explicit resolution. `01-REVIEW-2026-09-06.md` also names several items outside this plan's scope that block Phase 1's overall closure regardless of this plan's outcome (A1: provisional contamination thresholds exclude every run from the series; A2: orphaned `rtla` kthreads defeat `TracersQuiescent`) -- these are not this plan's responsibility and are not addressed here.

*Phase: 01-trustworthy-measurement*
*Completed: not yet -- paused at Task 3 checkpoint, 2026-09-07*

## Self-Check: PASSED

All files referenced above exist (docs/rig/firmware-floor-rt-vs-stock.md, the README, deferred-items.md, the three new run directories' manifest.json, and this SUMMARY itself). All eleven referenced commit hashes (9926cf6, d4931e7, c969ab8, fbbafe9, 6cf5613, 03a8f6e, ea33773, 078e476, ac67117, 60eb3d6, 8442cb2) resolve in `git cat-file -e`.
