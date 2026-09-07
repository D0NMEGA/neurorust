---
status: PASS
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
  - "Flagged rather than resolved a conflict between this plan's own DO-NOT-TOUCH instruction and a fix request targeting a sentence inside the protected section; the human operator confirmed the instruction was self-contradictory and authorized a narrow, precisely-scoped exception at the checkpoint"
  - "Logged findings B4, B5, C1, C2 from 01-REVIEW-2026-09-06.md to deferred-items.md with named owners rather than fixing them, matching the review's own disposition that they are independent of this plan's B1/B2/B3"
  - "Corrected the premise but not the conclusion of the one authorized sentence: there is still no firmware_floor_us value, now because an observed noise ceiling over three 900s arms is not a floor, rather than because no observation exists at all"
  - "Did not mark PLAT-03 complete despite it appearing in this plan's own frontmatter requirements field: PLAT-03's gate needs either a clean headline cyclictest capture under 30us (plan 01-13, unexecuted) or the residual attributed to a named platform cause (plan 01-12's job, and PLAT-01, the 3.8ms stall's root cause, is itself still Pending). This plan rules firmware/SMI out as a contributor on the isolated cores; it does not attribute the residual to anything, and the only committed capture on these cores is the explicitly do-not-publish contaminated run. Ran requirements mark-complete once, caught the mismatch by reading STATE.md's own 'Unexplained' 3.8ms blocker and REQUIREMENTS.md's PLAT-01 status before committing, and reverted before this was ever staged, matching the precedent 01-09/01-18/01-20/01-21/01-22 already set of leaving a listed requirement incomplete when the substance is not there yet"

patterns-established:
  - "When a document states an instrument's exposure bracket, cite what the code comment actually says it brackets, not what the surrounding prose's most recent number suggests"
  - "When a do-not-touch instruction and a fix-this instruction genuinely conflict, escalate the conflict at the checkpoint rather than guessing which one yields"
  - "A plan's frontmatter requirements field names what a requirement needs, not a guarantee that this plan delivers all of it; check the requirement's own text and this project's own recorded blockers before running requirements mark-complete, the same way five prior plans in this phase already did"

requirements-completed: []

# Metrics
duration: ~25min across two checkpoint rounds (second-review corrections plus the authorized line-182 fix and plan completion; task 1's captures and task 2's document corrections were an earlier session, see Task Commits)
completed: 2026-09-07
---

# Phase 1 Plan 23: Re-take D-18 with rtla hwnoise Summary

**Three rtla hwnoise firmware screens name CPUs 6-11 with an exact zero MSR_SMI_COUNT delta each; a second independent review's three overstatements and one operator-authorized correction are now fixed, and the human operator has approved publication.**

## Performance

- **Duration across both checkpoint rounds:** ~25 min (round 1: read the second review, verified against raw captures and source, corrected findings B1/B2/B3, ran the full verification suite; round 2: made the operator-authorized line-182 fix, re-verified, completed the plan). Task 1's captures and Task 2's document corrections were an earlier session; see Task Commits for that session's own work.
- **Tasks:** 3 of 3 complete. Task 3 (`checkpoint:human-verify`, blocking) approved by the human operator, with one authorized narrow exception to its own do-not-touch instruction.
- **Files modified across both rounds:** 4 (docs/rig/firmware-floor-rt-vs-stock.md, measurements/2026-08-28-precision3591/README.md, deferred-items.md, this SUMMARY)

## Accomplishments

- Task 1 (earlier session): three `rtla hwnoise -c 6-11 -H 0-5 -P f:99 -d 900s` screens taken on the installed PREEMPT_RT rig -- idle, all 22 logical CPUs loaded, and housekeeping-only loaded -- every arm naming all six isolated CPUs and recording an exact `MSR_SMI_COUNT` delta of zero on each. Arm 1 (idle) took three attempts: the first (`measurements/2026-09-06-precision3591-screen`) was SIGTERM'd by systemd at 660s on a `--hwnoise-duration` timeout-bound defect (fixed in `b29e819`) and is retained as a `failed` attempt; the second (`measurements/2026-09-06-precision3591-screen-02`) completed but is `excluded_from_series` as `Contaminated`, because the first attempt's killed `rtla` left orphaned osnoise kthreads that starved its cyclictest to 25% of its expected cycles (its hwnoise half is still good: rows for all of 6-11, max 7us on CPU 7, SMI delta 0 throughout); the third (`measurements/2026-09-06-precision3591-screen-03`) is the clean arm 1 used throughout this record. Both earlier attempts are retained rather than deleted, per this plan's own rule that every arm attempted appears under `measurements/`, including failures. A new `nrmeasure attempt <run-dir> --reason "..."` subcommand was added to close the first attempt's orphaned in-progress `ATTEMPT.json` honestly (status `failed`, with the SIGTERM cause named in `failure.message`) rather than hand-editing the evidence file, matching this project's own provenance discipline.
- Task 2 (earlier session): `docs/rig/firmware-floor-rt-vs-stock.md` and `measurements/2026-08-28-precision3591/README.md` corrected in place for the coverage defect the retake fixes; the firmware coverage test split into `hwlatdetect_captures_still_cover_no_isolated_core` (still true) and `hwnoise_captures_cover_the_isolated_cores` (now true).
- Task 3, round 1: a second independent review (`01-REVIEW-2026-09-06.md`, findings B1, B2, B3) found the Task 2 write-up itself overstated what the new captures support. All three corrected against the raw captures and the harness source, not against the review's prose alone (see Deviations below for the exact evidence checked for each).
- Task 3, round 2: the human operator approved publication and authorized a narrow, precisely-scoped exception to fix one further stale sentence inside the previously-protected "What PLAT-03 should report" section, after confirming the do-not-touch instruction and the fix-this instruction had genuinely conflicted. Applied exactly as scoped; plan marked complete.

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

Task 3, round 1 (second review's corrections):

11. `8442cb2` docs(01-23): correct three overstatements the second review found
12. `8c02ffc` docs(01-23): record the second review's corrections at the Task 3 checkpoint (interim SUMMARY + STATE.md, superseded by this final version)

Task 3, round 2 (operator-approved, authorized exception, plan completion):

13. `9066bdf` docs(01-23): correct the premise in What PLAT-03 should report

**Plan metadata:** this commit (docs: complete the 01-23 plan)

## Files Created/Modified

- `docs/rig/firmware-floor-rt-vs-stock.md` - reworded the 1us-on-every-CPU claim to a pooled per-arm maximum (CPU 7 in the idle arm recorded 0, not 1); reworded the MSR_SMI_COUNT bracket in three places to name cyclictest+hwnoise+snapshot rather than the 900s hwnoise window alone; corrected the premise (not the conclusion) of one sentence in "What PLAT-03 should report," under explicit operator authorization
- `measurements/2026-08-28-precision3591/README.md` - marked the withdrawn 7us/1us-threshold figure unverifiable in the results table; led the Findings improvement claim with the verifiable "at least 12x" rather than the unverifiable "18x"; marked the SMI kernel-independence carry-over claim as the untested assumption it was, without altering the quoted sentence itself
- `.planning/phases/01-trustworthy-measurement/deferred-items.md` - logged findings B4, B5, C1, C2 from the second review with named owners

## Decisions Made

See `key-decisions` in the frontmatter. The one most worth restating in prose: Task 3's own task instructions contained a "FIX 3" (a stale sentence "around line 182" of `firmware-floor-rt-vs-stock.md`, inside the "What PLAT-03 should report" section) that conflicted with an explicit, doubly-sourced instruction not to touch that section (both this plan's own Task 2 action text and the same task prompt). The official review (`01-REVIEW-2026-09-06.md`) did not list this sentence as a finding at all -- its B1/B2/B3 mapped exactly to the three other fixes, with no fourth item. Rather than guess which instruction should yield, this was flagged at the checkpoint. The human operator confirmed the instruction had been self-contradictory (their error, not the executor's), and authorized a narrow, precisely-scoped exception: correct the stated reason, keep the conclusion word for word, change nothing else in the section. Applied exactly as scoped and confirmed via `git diff`.

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

### Human-authorized correction (not a Rule 1-3 auto-fix)

**4. Corrected the premise, not the conclusion, of one sentence in "What PLAT-03 should report"**
- **Found during:** Task 3 preparation (this plan's own "FIX 3" instruction, in conflict with a DO-NOT-TOUCH instruction covering the same section)
- **Issue:** "There is no `firmware_floor_us` value to hand plan 01-13, because no capture in this repository establishes a firmware observation on CPUs 6-11" gave a correct conclusion for a reason the three 2026-09-06 arms now make false. This sentence sits inside a section this plan's own Task 2 action text, and this session's own task instructions, both required to stay byte-identical -- a genuine conflict, not resolved unilaterally. Flagged at the Task 3 checkpoint instead.
- **Resolution:** The human operator confirmed the instruction had been self-contradictory (operator error, not executor error) and authorized a narrow, precisely-scoped exception: fix only the two-line premise, preserve the conclusion ("no firmware_floor_us value") word for word, preserve the distinction that an observed noise ceiling over three 900s arms is not a floor, and change nothing else in the section.
- **Fix:** Replaced "because no capture in this repository establishes a firmware observation on CPUs 6-11" with "The three 2026-09-06 arms do now establish firmware observations on CPUs 6-11 (see the re-take section above), but an observed noise ceiling over three 900 second arms is not a floor: it bounds what those arms saw, under those conditions, not what the machine can do." Everything else in the section, including the independent-invalidity argument and the `render_plat03_verdict` sentence, is untouched.
- **Files modified:** docs/rig/firmware-floor-rt-vs-stock.md
- **Verification:** `git diff` shows exactly one hunk (4 insertions, 2 deletions) at the target sentence; the rest of the section and the 3.8ms attribution paragraph confirmed byte-identical.
- **Committed in:** 9066bdf

**Total deviations:** 3 auto-fixed (Rule 1, factual corrections to already-published prose) plus 1 human-authorized correction (an explicit exception to a do-not-touch instruction, granted at the checkpoint after the conflict was flagged rather than resolved unilaterally), plus 4 items logged to deferred-items.md rather than fixed (review findings B4, B5, C1, C2 -- named by the review itself as independent of this plan's scope).

**Impact on plan:** All four fixes are corrections to prose only; no code changed, no test behavior changed. `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (31 test binaries, all passing), and `./target/release/nrmeasure verify --strict --check-index` (0 problems) all pass after every round of edits, confirming the correction passes did not regress anything Task 1 or Task 2 established.

## Issues Encountered

**Conflict flagged at the Task 3 checkpoint, then resolved by the human operator.** This plan's own "FIX 3" task instruction identified a statement "around line 182" of `docs/rig/firmware-floor-rt-vs-stock.md` claiming no committed capture establishes a firmware observation on CPUs 6-11. That statement was real, at lines 182-183, and stale in the same direction as findings B1/B2/B3: the three new rtla hwnoise arms do establish firmware observations (zero SMI delta, a 1us noise ceiling) on CPUs 6-11, even though they still do not establish a *floor* (finding 1's point, which the surrounding paragraph is making). The sentence sat inside the "## What PLAT-03 should report" section, which:

- this plan's own Task 2 action text required kept "exactly as it is" because "it does not change because a better firmware number now exists,"
- the same task prompt separately listed by name as a DO-NOT-TOUCH zone, requiring explicit confirmation that it was unchanged, and
- `01-REVIEW-2026-09-06.md` (the actual second review, findings B1 through C2) did not list at all -- its only findings assigned to plan 01-23 were B1, B2 and B3, addressed in full by the other three fixes.

Rather than guess which of two explicit, conflicting instructions should yield, this was flagged at the Task 3 checkpoint with the exact conflicting text quoted for the human to decide. The human operator's response: "APPROVED... with one authorized exception to the do-not-touch rule. You were right to escalate rather than resolve it yourself: the instruction was self-contradictory, and that was my error, not yours." The operator then specified the exact, narrow replacement (preserving the conclusion and the noise-ceiling-is-not-a-floor distinction, correcting only the stated reason), which was applied verbatim in scope and confirmed via `git diff` to touch only that one sentence (commit `9066bdf`).

**Confirmed unchanged throughout, as required:**
- The "3.8 ms attribution paragraph" in `measurements/2026-08-28-precision3591/README.md` (the paragraphs around "the roughly 3.8 ms maximum appeared on all six threads"): untouched across every commit this plan made.
- The rest of the "## What PLAT-03 should report" section in `docs/rig/firmware-floor-rt-vs-stock.md`, outside the two-line authorized exception: untouched, confirmed via `git diff` showing exactly one hunk.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Plan 01-23 is complete. Task 3 (`checkpoint:human-verify`, blocking) was approved by the human operator, who worked through the plan's `<how-to-verify>` checks and additionally resolved the one flagged conflict by authorizing a narrow, precisely-scoped exception rather than accepting either an unfixed staleness or a unilateral rewrite. The isolated cores now have a real firmware observation with exact SMI counts, and the published record states what that observation does and does not support.

**Neither requirement in this plan's own frontmatter (`requirements: [PLAT-03, BENCH-06]`) is marked complete.** BENCH-06 was already complete before this plan (unrelated to it). PLAT-03 ("worst-case scheduling latency on isolated cores is either brought under 30us, or the residual is attributed to a named platform cause and published as a documented limitation") is not satisfied by this plan's work: the only committed cyclictest capture on the isolated cores is the 2026-08-28 run, explicitly contaminated and marked do-not-publish; PLAT-01 (naming the 3.8ms stall's cause, the residual PLAT-03's second disjunct would need to attribute) is itself still Pending; and the clean headline capture PLAT-03's first disjunct would need is plan 01-13's job, still unexecuted. This plan rules firmware/SMI out as a contributor on CPUs 6-11, which is real, useful input to whichever plan eventually attributes the 3.8ms residual, but it is not itself that attribution. `requirements mark-complete PLAT-03 BENCH-06` was run once, its result (PLAT-03 flipped to Complete) was checked against STATE.md's own recorded blocker and REQUIREMENTS.md's PLAT-01 status before anything was committed, found to be premature, and reverted.

Phase 1 is not ready to close, independent of this plan. `01-REVIEW-2026-09-06.md` names two items that block closure and are explicitly not this plan's responsibility, per the operator's own instruction to leave them to a replan rather than start new work here:
- **A1** (largest, least specified): provisional contamination thresholds (`config/contamination-thresholds.json`, `"status": "provisional"`) mark every run `excluded_from_series`, including all three of this plan's `Clean`-verdict arms, so plan 01-13's headline capture cannot complete as specified and 01-14's weekly series would be empty. The obvious fix (flip the status) is wrong, because the D-24 classifier decides admissibility from the measured latency's own shape, so a genuine regression would be indistinguishable from contamination. Needs a new plan.
- **A2**: orphaned `rtla` osnoise kthreads defeat `TracersQuiescent` because `rtla` drives its own tracing instance, invisible to the four top-level controls plan 01-18 widened. Already logged in `deferred-items.md` under this plan's "killed rtla" entry; needs an extension of 01-18 or a new plan.

Also open from the review, independent of 01-23: C1 (verify --strict does not re-derive firmware figures, logged here with an owner), and D1/D2 (defects in the as-yet-unexecuted plans 01-14 and 01-12, need editing before those plans execute).

*Phase: 01-trustworthy-measurement*
*Completed: 2026-09-07*

## Self-Check: PASSED

All files referenced above exist (docs/rig/firmware-floor-rt-vs-stock.md, the README, deferred-items.md, the three new run directories' manifest.json and ATTEMPT.json, the two retained failed/contaminated attempt directories, and this SUMMARY itself). All thirteen referenced commit hashes (9926cf6, d4931e7, c969ab8, fbbafe9, 6cf5613, 03a8f6e, ea33773, 078e476, ac67117, 60eb3d6, 8442cb2, 8c02ffc, 9066bdf) resolve in `git cat-file -e`. Full verification suite re-run after the final edit: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, and `./target/release/nrmeasure verify --strict --check-index` all exit 0 (see Deviations, item 4, for the exact output).
