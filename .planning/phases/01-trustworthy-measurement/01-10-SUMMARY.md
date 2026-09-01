---
status: PARTIAL
agent: donny-executor
phase: 01-trustworthy-measurement
plan: 10
subsystem: measurement-provenance
tags: [provenance, cyclictest, histogram-percentiles, reconstruction, publication-correction, nrmeasure]

# Dependency graph
requires:
  - phase: 01-08
    provides: "nrmeasure verify --strict --check-index and nrmeasure reconstruct, the provenance gate and reconstruction command this plan drives"
  - phase: 01-09
    provides: "the measurement protocol and publication layout conventions this plan's corrected README and generated INDEX follow"
provides:
  - "measurements/2026-08-28-precision3591/manifest.json reconstructed, contaminated, excluded_from_series, every unrecorded field explicitly absent with a reason"
  - "measurements/INDEX.md, generated, publishing the founding measurement as a losing (contaminated) configuration rather than omitting it"
  - "measurements/2026-08-28-precision3591/README.md corrected in place: the bimodality claim withdrawn, the over-gate count and percentile table regenerated with the 888 histogram overflows counted"
  - "a green nrmeasure verify --strict --check-index over the whole repository, for the first time"
affects: [PLAT-01 (two-phenomena finding), any future plan reading measurements/INDEX.md or citing this README's percentile convention]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Overflow-inclusive percentile convention: histogram overflows recorded at the bin bound, included in both the percentile numerator and the population denominator"
    - "Corrected-in-place publication: a dated correction note names what changed and why; raw captures and their checksums stay untouched rather than the artifact being silently edited"

key-files:
  created: []
  modified:
    - measurements/2026-08-28-precision3591/manifest.json
    - measurements/2026-08-28-precision3591/README.md
    - measurements/INDEX.md
    - crates/cli/src/cmd/reconstruct.rs
    - crates/cli/tests/reconstruct.rs
    - .planning/phases/01-trustworthy-measurement/deferred-items.md

key-decisions:
  - "The 2026-08-28 capture set is reconstructed and published as contaminated/excluded rather than exempted, closing D-16's gate on the project's own founding measurement"
  - "The bimodality claim is withdrawn outright as false, not merely imprecise: bins 14-99 hold 410 samples against a claimed zero, and the largest true empty run above 13us is 6 bins"
  - "Over-gate boundary is >= 30 us, not > 30 us, and the denominator includes the 888 overflow samples: 2,093 of 17,995,844 (0.0116%), per the 2026-08-31 operator decision"
  - "p99.99 (12 -> 108 us) and max (~3800 -> 3806 us) are corrected because the previous figures were computed from histogram bins alone and omitted 888 overflow samples that are 0.0049% of the run and therefore dominate the top 0.01 percent"
  - "A human explicitly confirmed both percentile changes and the published exclusion-reason wording before publication (Task 3 blocking checkpoint); the orchestrator independently re-derived every figure before approving"

patterns-established:
  - "Reconstruction of a pre-harness directory: every field the source artifacts cannot supply is named in absent_fields with a specific reason, never inferred from the rig's current state"
  - "A corrected published artifact carries a dated note naming exactly what changed, rather than a silent edit"

requirements-completed: []  # Not marked per explicit instruction: the end-of-phase verifier owns REQUIREMENTS.md. Plan frontmatter names BENCH-04/05/06.

# Metrics
duration: 23min
completed: 2026-09-01
---

# Phase 01 Plan 10: Reconstruct and Correct the 2026-08-28 Measurement Summary

**The 2026-08-28 rig-selection capture is reconstructed inside the provenance contract as a contaminated, excluded run, and its README's bimodality claim and over-gate count are corrected against the raw histogram (p99.99 12->108us, max ~3800->3806us, over-gate 1,201->2,093 samples).**

## Performance

- **Duration:** 8 min (Tasks 1-2, commits `0e4502d`..`012df67`) + ~15 min (Task 3 finalization, this continuation session) = ~23 min active execution. The multi-hour gap between the two sessions was the human checkpoint review, not execution time.
- **Started:** 2026-08-31T19:56:28Z
- **Checkpoint reached:** 2026-08-31T20:04:10Z (commit `012df67`)
- **Approved and finalized:** 2026-09-01T04:08:53Z
- **Tasks:** 3 (plus 3 pre-task deviation commits inside Task 1's own scope)
- **Files modified:** 6

## Accomplishments

- Reconstructed `measurements/2026-08-28-precision3591/manifest.json`: `provenance_tier: reconstructed`, `contaminated`, `excluded_from_series: true`, with every field the source artifacts cannot supply (preconditions, interference before/after/delta, `tuning.per_cpu_governor`, `energy_performance_preference`, `rt_tuning_service`, `power.thermal_zones`, `utc_start`/`utc_end` precision, and the two-kernel tension in `kernel.release`) named in `absent_fields` with a specific reason.
- Generated `measurements/INDEX.md`; `nrmeasure verify --strict --check-index` now exits 0 over the whole repository for the first time, with no allowlist added.
- Withdrew the false bimodality claim and replaced it with the real distribution shape (230 samples in bins 31-99, 971 in bins 100-399, longest empty run above 13us is 6 bins).
- Corrected the over-gate count to 2,093 of 17,995,844 (0.0116%, `>= 30 us` boundary) and regenerated the percentile table with the 888 overflows counted (p99.99 12 -> 108 us, max ~3800 -> 3806 us).
- Human confirmed both percentile changes and the published exclusion-reason wording (Task 3 checkpoint); the orchestrator independently re-derived every figure before approving.
- I independently re-derived every one of those figures again while finalizing Task 3 (the plan's own `awk` commands, plus a from-scratch Python re-implementation of the bin/overflow parser, cross-checked against `crates/histogram/tests/fixtures/README.md`'s recorded expected values) - all match exactly. Detail in "Verified Figures" below.

## Task Commits

Each task was committed atomically:

1. **Task 1 (dev): mark `energy_performance_preference` explicitly absent in reconstruct** - `0e4502d` (fix)
2. **Task 1 (dev): record `utc_start`/`utc_end` as absent-precision in reconstruct** - `09bbdc5` (fix)
3. **Task 1 (dev): exclude the real manifest.json from the reconstruct test fixture copy** - `a6b6b26` (test)
4. **Task 1: reconstruct the 2026-08-28 manifest and generate the published index** - `10c1b4e` (feat)
5. **Task 2: correct the bimodality claim and the over-gate count in the README** - `012df67` (fix)
6. **Task 3: log the 01-07 date-flake test as a deferred item** - `573dbd1` (docs)

**Plan metadata:** committed together with this SUMMARY, STATE.md and ROADMAP.md (see final commit below).

## Files Created/Modified

- `measurements/2026-08-28-precision3591/manifest.json` - reconstructed manifest: reconstructed/contaminated/excluded, full absent-fields list, 7 artifact checksums
- `measurements/2026-08-28-precision3591/README.md` - bimodality claim withdrawn; over-gate count and percentile table corrected; two-phenomena finding and dated correction note added
- `measurements/INDEX.md` - generated; one row for the 2026-08-28 run, contaminated and not in series
- `crates/cli/src/cmd/reconstruct.rs` - `build_tuning` now emits `AbsentField` entries for `energy_performance_preference` and for `utc_start`/`utc_end` precision
- `crates/cli/tests/reconstruct.rs` - asserts the new absent-field entries; fixture copy excludes the real directory's own `manifest.json`
- `.planning/phases/01-trustworthy-measurement/deferred-items.md` - logs the out-of-scope `run_pipeline.rs` date-flake discovery (see Issues Encountered)

## Decisions Made

- **Reconstruct rather than exempt.** The 2026-08-28 directory is brought inside the provenance contract exactly as D-16 requires, published as `contaminated`/`excluded_from_series` rather than given a special-cased pass. The project's own founding measurement now appears in `measurements/INDEX.md` as a losing configuration - BENCH-06 working as designed.
- **Withdraw, don't soften, the bimodality claim.** The raw histogram has 410 samples in bins 14-99, not zero, and no empty run longer than 6 bins anywhere above 13us. The original claim was false, not imprecise, so it is withdrawn outright rather than hedged, and the structural inference that rested on it ("a specific recurring event") is withdrawn with it.
- **Boundary and denominator for the over-gate count.** `>= 30 us` (not `> 30 us`), because PLAT-03 requires latency "brought under 30 us" and a sample landing exactly on the gate has not met it (bin 30 holds 4 samples). The denominator includes the 888 overflow samples the old, excluded denominator dropped: 2,093 of 17,995,844 (0.0116%).
- **Percentile table regenerated wholesale, not patched.** Because it inherited the same overflow omission as the over-gate count, both p99.99 (12 -> 108 us) and max (~3800 -> 3806 us) change. This is arithmetically forced: 888 overflow samples are 0.0049% of the run, so the top 0.01 percent contains every overflow plus roughly 300 more, making a bins-only p99.99 of 12us impossible once overflows are counted.
- **Human sign-off before publication, not after.** Task 3 was a blocking checkpoint specifically because the overflow-omission fix has a consequence (the percentile change) that CONTEXT.md's original D-23 discussion did not enumerate. The human reviewed the re-derivation, the corrected section, and the published exclusion-reason string, and approved publishing exactly what was already in the working tree with no further edits.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Reconstruct silently omitted an absent-fields entry for `energy_performance_preference`**
- **Found during:** Task 1
- **Issue:** `crates/cli/src/cmd/reconstruct.rs` predates commit `d73cc27` (plan 01-09), which added `CpuGovernor::energy_performance_preference` to the manifest schema. `build_tuning` never referenced the new field, so a reconstructed manifest would silently omit an `absent_fields` entry for a field the schema now carries, understating what the reconstruction actually covers.
- **Fix:** `build_tuning` now emits an explicit `AbsentField` entry naming the field, explaining the 2026-08-28 captures cannot supply it under any circumstance (a live-USB stock-kernel state, different from the installed system the field now describes).
- **Files modified:** `crates/cli/src/cmd/reconstruct.rs`, `crates/cli/tests/reconstruct.rs`
- **Verification:** `reconstruct_records_absent_fields` test extended and passing
- **Committed in:** `0e4502d`

**2. [Rule 1 - Bug] `utc_start`/`utc_end` precision was never recorded as absent by reconstruct**
- **Found during:** Task 1
- **Issue:** No pre-harness capture records a live run-start/end timestamp, so `--utc-start`/`--utc-end` are always the operator's best defensible reconstruction from indirect evidence, never a value observed by a tool that ran. This is true of every reconstruction by construction, not specific to this one run, so the plan's own task 1 requires it recorded as a structural absence.
- **Fix:** `reconstruct.rs` now records `utc_start` and `utc_end` as `AbsentField` entries alongside the other structural absences (`preconditions`, `interference.*`, `tools`).
- **Files modified:** `crates/cli/src/cmd/reconstruct.rs`
- **Verification:** `cargo test -p nr-cli` passing
- **Committed in:** `09bbdc5`

**3. [Rule 3 - Blocking] The real directory's own new manifest.json broke the reconstruct test fixture copy**
- **Found during:** Task 1
- **Issue:** `copy_measurement_dir()` copies every file from the real `measurements/2026-08-28-precision3591/` into a temp directory for each test. Once Task 1 gave that directory its own reconstructed `manifest.json`, every test that reconstructs without `--force` broke, because those tests exercise reconstruction starting from a pre-harness directory that has no manifest yet - a state the real directory itself will never be in again.
- **Fix:** the fixture copy excludes `manifest.json` specifically.
- **Files modified:** `crates/cli/tests/reconstruct.rs`
- **Verification:** `cargo test -p nr-cli --test reconstruct` passing (8/8)
- **Committed in:** `a6b6b26`

Task 3 itself required no implementation: the human approved every figure exactly as written, so no README, manifest, or index edit was made in this session.

---

**Total deviations:** 3 auto-fixed (2 bug, 1 blocking), all inside Task 1's own scope and necessary for its own acceptance criteria (a complete `absent_fields` set and a passing pre-existing test suite).
**Impact on plan:** No scope creep. All three were required for Task 1's own acceptance criteria, not optional extensions.

## Issues Encountered

While re-running the workspace verification suite to finalize this checkpoint, `cargo test --workspace` reported 148/149 tests passing with one failure: `crates/cli/tests/run_pipeline.rs::full_run_report_matches_snapshot`. This is a pre-existing, out-of-scope issue, and it is fully root-caused rather than merely observed:

- The wall clock crossed midnight UTC (2026-08-31 -> 2026-09-01) between when that snapshot was captured (plan 01-07, commit `8271264`) and this verification pass.
- `crates/cli/src/cmd/run.rs:238` sets `utc_start = OffsetDateTime::now_utc()` for a live run (correct production behavior). `rundir.rs` derives the generated `run_id`'s date component from that timestamp. The integration test exercises this real path end to end with fake tool binaries and no injected clock, so its `run_id` always carries the actual wall-clock date. The committed snapshot pins a literal calendar-date string (`2026-08-31-precision3591-recon`) and therefore fails exactly one day after it was captured, with zero code change involved.
- Confirmed as the only difference in the entire 429-line report: `run id: 2026-08-31-precision3591-recon` (snapshot) vs `run id: 2026-09-01-precision3591-recon` (actual output). Every other line, including all rig, tuning, precondition, percentile, and distribution fields, matches byte for byte.
- Out of scope for plan 01-10: it touches `crates/cli/tests/run_pipeline.rs` and its snapshot, which belong to plan 01-07 (already summarized and committed), not `measurements/2026-08-28-precision3591/` or `measurements/INDEX.md`. Not fixed here per the scope boundary. Logged to `.planning/phases/01-trustworthy-measurement/deferred-items.md` with the exact one-line fix location (the test's own `redact_report()` helper at `run_pipeline.rs:172` already redacts `utc_start`/`utc_end` and should redact the `run id:` line the same way) - commit `573dbd1`.
- **This is why this SUMMARY's status is PARTIAL rather than PASS.** Every one of plan 01-10's own tasks, acceptance criteria, and success criteria pass cleanly (`nrmeasure verify --strict --check-index` exits 0, `cargo fmt --check` exits 0, `cargo clippy --workspace --all-targets -- -D warnings` exits 0 with zero warnings, and every figure in the corrected README independently re-derives). The sole red item is this unrelated, already-diagnosed, already-deferred test flake in a different, already-completed plan, surfaced only because the calendar rolled over during this session. Reporting PASS anyway would have hidden a real (if narrow) red result behind a plan boundary, which is exactly the kind of quiet omission this phase exists to prevent.

## Verified Figures (independently re-derived, not merely re-stated)

Per the human's explicit instruction, these are recorded as verified rather than claimed. I re-ran the plan's own `awk` one-liners against the raw `.hist` file and additionally wrote a from-scratch Python re-implementation of the bin/overflow parser (independent of both the harness binary and the `awk` commands), then cross-checked both against `crates/histogram/tests/fixtures/README.md`'s recorded expected-value contract. Every figure matches exactly, on top of the orchestrator's own independent re-derivation noted in the approval:

| Figure | Published 2026-08-28 | Corrected (published now) | Independently reconfirmed this session |
|---|---|---|---|
| p99.99 | 12 us | 108 us | yes - 12us (bins-only) and 108us (overflow-inclusive) both reproduced from raw bytes via a from-scratch percentile-rank calculation (rank 17,994,044 of 17,995,844 falls in bin 108, cumulative 17,994,052) |
| max | "approximately 3800 us" | 3806 us | yes - footer `# Max Latencies: 03785 03679 03787 03693 03806 03726` |
| over-gate (`>= 30 us`) | 1,201 of 17,994,956 (0.0067%) | 2,093 of 17,995,844 (0.0116%) | yes - bins `>= 30` sum to 1,205; 1,205 + 888 overflows = 2,093; 17,994,956 + 888 = 17,995,844 |
| bimodality | "zero samples between 13 and 100 us" | withdrawn - bins 14-99 hold 410 samples | yes - 410 confirmed; also confirmed bins 31-99 = 230, bins 100-399 = 971, longest empty run above 13us = 6 bins (278-283) |
| raw captures | - | unchanged | yes - `git diff --stat` from before plan 01-10 (`d73cc27..HEAD`) touches only `README.md` and `manifest.json`; independently recomputed `b3sum` for all 7 artifacts in the directory matches every checksum recorded in `manifest.json` |

Also worth recording, because it is this phase's own thesis in miniature: the 888 overflow samples are 0.0049% of the run (888 / 17,995,844), so the top 0.01 percent of samples is mostly overflow. Omitting them, as the original README did, did not shave the tail - it deleted it. That is the same class of error as the over-gate undercount, and it is why the raw capture travels with the figure rather than the figure standing alone.

The exclusion reason in `measurements/2026-08-28-precision3591/manifest.json` and `measurements/INDEX.md` (naming `ps -L`, `tmux capture-pane`, `scp`, and the active GNOME session) was left exactly as written, per explicit instruction; no change was made to it.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `measurements/2026-08-28-precision3591/` is fully inside the provenance contract: reconstructed, contaminated, excluded, every unrecorded field named with a reason. `measurements/INDEX.md` is generated and `nrmeasure verify --strict --check-index` is green over the whole repository.
- PLAT-01 has its "two phenomena, not one" finding on record in the README: a sustained ~140-overflow-per-thread burst (cycles 1,820,584-1,826,674) is a different phenomenon from the isolated cross-thread spikes near cycles 2,259,348 / 2,349,428 / 2,709,344 that the ~3.8 ms maximum belongs to. These should be investigated separately, not as one event.
- Carried forward, not this plan's to fix: `crates/cli/tests/run_pipeline.rs::full_run_report_matches_snapshot` will fail again on any day after 2026-09-01 until the `run id:` line is added to that test's own redaction helper. See `deferred-items.md`, "From 01-10", for the exact fix location.
- No blockers to this phase's next plan.

## Self-Check: PASSED

All 7 claimed files confirmed present on disk (`manifest.json`, `README.md`, `measurements/INDEX.md`, `crates/cli/src/cmd/reconstruct.rs`, `crates/cli/tests/reconstruct.rs`, `deferred-items.md`, this SUMMARY). All 6 claimed commits confirmed present in git history (`0e4502d`, `09bbdc5`, `a6b6b26`, `10c1b4e`, `012df67`, `573dbd1`).

---
*Phase: 01-trustworthy-measurement*
*Completed: 2026-09-01*
