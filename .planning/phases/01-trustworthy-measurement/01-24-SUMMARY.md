---
status: PASS
agent: donny-executor
phase: 01-trustworthy-measurement
plan: 24

subsystem: measurement-harness
tags: [run-admission, contamination-detection, manifest-schema, report-rendering, d-28]

# Dependency graph
requires:
  - phase: 01-trustworthy-measurement
    provides: RunManifest, nr_manifest::validate, interference::verdict, cmd::run::execute, render_run_report (plans 01-01 through 01-23)
provides:
  - SeriesAdmission/AdmissionEvidence/AdmissionEvidenceSource/AdmissionDisposition manifest types
  - determine_admission, the D-28 admission gate that cannot see the contamination verdict or tail metrics
  - interference::counter_breach, the public per-run-hour counter comparison shared by the gate and the D-15 verdict
  - the validate() rule tying excluded_from_series/exclusion_reason to series_admission
  - the "Series admission" REPORT.md section, rendered before the contamination verdict
  - every published description of the removed determine_exclusion behaviour, corrected
affects: [01-13, 01-14, 01-15, any future plan that reads excluded_from_series or interference.verdict]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Two-part admission record: an admission verdict decided from evidence upstream of a measurement, kept structurally separate (by function signature, not by convention) from a shape-based classifier of the measurement itself"

key-files:
  created: []
  modified:
    - crates/manifest/src/fields.rs
    - crates/manifest/src/validate.rs
    - crates/manifest/tests/checksum.rs
    - schemas/manifest.schema.json
    - crates/cli/src/cmd/reconstruct.rs
    - crates/cli/src/cmd/run.rs
    - crates/capture/src/interference.rs
    - crates/metrics/src/report.rs
    - crates/metrics/tests/report.rs
    - crates/metrics/tests/snapshots/report__headline_report.snap
    - crates/cli/tests/snapshots/run_pipeline__full_run_report_matches_snapshot.snap
    - config/contamination-thresholds.json
    - docs/measurement-protocol.md
    - docs/publication-layout.md
    - README.md

key-decisions:
  - "Fixed both RunManifest struct-literal compile sites (reconstruct.rs, run.rs) inside task 1's own commit rather than task 2's, following plan 01-20's own precedent for the same class of problem: reconstruct.rs gets a permanent None (a reconstructed run never went through the live gate), run.rs gets a compile-time placeholder None that task 2 replaces with the real value in the next commit."
  - "The 01-13 case (a bad-looking shape alone does not exclude) is a unit test inside crates/cli/src/cmd/run.rs, not a crates/cli/tests/run_pipeline.rs subprocess test: live_facts() unconditionally refuses on macOS, so every subprocess-level nrmeasure run in this suite structurally requires the facts fixture, and fixture use is itself an excluding evidence source. A true no-fixtures scenario cannot be exercised through a real subprocess on this dev host. Used the real, committed 2026-08-28 capture (the same file fake-cyclictest.sh serves by default) and the real shipped config/contamination-thresholds.json so the claim is checked against production data, not a hand-picked pair of numbers."
  - "requirements-completed left empty. BENCH-04 and BENCH-06 were already marked complete by earlier plans; this plan's own changes don't newly complete either. BENCH-08 needs a real committed weekly JSON metrics file, which is plan 01-14's job (still unexecuted per STATE.md); this plan only removes the blocker that made 01-14 unable to complete as written."
  - "kebab() (this file's own established enum-rendering helper) is reused for the Series admission table's evidence-source and disposition columns, giving hyphenated labels like tool-exit-codes rather than the plan's illustrative space-separated example. Consistent with every other enum column already in REPORT.md; not treated as a literal formatting requirement."
  - "Restored a corrupted, uncommitted STATE.md to HEAD before starting any task work. The working tree already carried an incoherent, unstaged edit (Current Position read 'Plan: 1 of 27' with orphaned prose describing 01-23) contradicting the coherent, committed version naming plan 01-24 as next. Matches this project's own documented donny-tools STATE.md corruption pattern (01-02, 01-16 through 01-19, 01-23); not part of this plan's task list, so it was reverted rather than hand-corrected."

# Metrics
duration: 42min
completed: 2026-09-07
---

# Phase 01 Plan 24: Run admission decided from evidence upstream of the measurement Summary

**Split run admission from the D-24 contamination verdict: `determine_admission` decides `excluded_from_series` from seven evidence sources causally upstream of the measured latency, and is never handed the verdict or the tail metrics that would let a genuine platform regression hide behind the same signature as contamination.**

## Performance

- **Duration:** 42 min (estimated; commits span 02:19:40 to 02:41:24 -0500)
- **Started:** approx. 2026-09-07T07:00:00Z
- **Completed:** 2026-09-07T07:41:24Z
- **Tasks:** 3
- **Files modified:** 15

## Accomplishments

- `SeriesAdmission`, `AdmissionEvidence`, `AdmissionEvidenceSource` and `AdmissionDisposition` added to `nr_manifest`, additive and schema-generated, so all twelve committed manifests keep validating untouched
- `nr_manifest::validate` refuses a manifest whose `excluded_from_series`/`exclusion_reason` disagree with `series_admission`, or whose admitted run still carries exclusions
- `determine_exclusion` (which forced `excluded_from_series = true` unconditionally whenever the contamination thresholds were provisional, excluding every clean D-18 re-take arm) is gone, replaced by `determine_admission`, whose input struct structurally cannot see `ContaminationVerdict`, `TailMetrics` or `InterferenceSnapshotPair`
- `interference::counter_breach` is a public helper shared by the D-15 verdict and the new gate, so the per-run-hour comparison exists in exactly one place
- `REPORT.md` renders a `## Series admission` section (admitted verdict, seven-row evidence table, exclusions list) before `## Contamination verdict`, and the contamination verdict itself now states plainly that it does not by itself remove a run from the series
- Every published description of the removed behaviour is corrected: the thresholds-file calibration note, `Thresholds::Provisional`'s doc comment, `thresholds_provisional`'s doc comment (schema regenerated to match), a dangling intra-doc link, `docs/measurement-protocol.md`'s "what invalidates a run" section, `docs/publication-layout.md`, and `README.md`'s phase-1 status paragraph
- A test asserts the 01-13 case directly against production data: the real 2026-08-28 capture scores Contaminated under the real shipped provisional thresholds, and `determine_admission` admits it anyway

## Task Commits

Each task was committed atomically:

1. **Task 1: The admission record, as a manifest field with its own validation rule** - `6d5447a` (feat)
2. **Task 2: The admission gate, which is never handed the number** - `899ad7f` (feat)
3. **Task 3: Render both verdicts, and correct every description of the old behaviour** - `8e121fc` (feat)

_No separate plan-metadata commit exists yet; this SUMMARY, STATE.md and ROADMAP.md are committed together as the final step below._

## Files Created/Modified

- `crates/manifest/src/fields.rs` - `SeriesAdmission`/`AdmissionEvidence`/`AdmissionEvidenceSource`/`AdmissionDisposition`, the `series_admission` field, corrected `excluded_from_series`/`exclusion_reason`/`thresholds_provisional` doc comments
- `crates/manifest/src/validate.rs` - `AdmissionDisagreesWithExclusion`/`AdmittedRunCarriesExclusions` and their checks
- `crates/manifest/tests/checksum.rs` - the two new `validate()` tests
- `schemas/manifest.schema.json` - regenerated (task 1 and task 3's doc-comment fix)
- `crates/cli/src/cmd/reconstruct.rs` - `series_admission: None` (a reconstructed run never went through the live gate)
- `crates/cli/src/cmd/run.rs` - `determine_admission`, `AdmissionInputs`, `format_counter_deltas`; the decision site now calls the gate instead of an if/else-if chain; nine unit tests (two adapted, seven new)
- `crates/capture/src/interference.rs` - `counter_breach` lifted out of `evaluate` as a public fn; `Thresholds::Provisional`'s doc comment corrected
- `crates/metrics/src/report.rs` - `render_series_admission`; the "does not by itself remove the run" sentence in `render_contamination`
- `crates/metrics/tests/report.rs` - two new tests for the present/absent admission-record cases
- `crates/metrics/tests/snapshots/report__headline_report.snap`, `crates/cli/tests/snapshots/run_pipeline__full_run_report_matches_snapshot.snap` - re-pinned; diffs contain only the new section and sentence
- `config/contamination-thresholds.json` - the one stale clause in `calibration.note` corrected; CAL/TLB/RES/device-IRQ figures and `derived_from` byte-unchanged
- `docs/measurement-protocol.md` - the provisional-verdict sentence and the "what invalidates a run" third bullet corrected, a new bullet listing the seven evidence sources added
- `docs/publication-layout.md` - one sentence naming `series_admission` added
- `README.md` - the phase-1 status paragraph corrected

## Decisions Made

See `key-decisions` in the frontmatter above for the full rationale on each. In short: the two struct-literal compile fixes were folded into task 1's own commit (precedented by plan 01-20); the 01-13 case is proven as a unit test against real production data rather than a subprocess test that structurally cannot avoid the facts fixture; `requirements-completed` is left empty because none of BENCH-04/06/08 is newly, substantively completed by this plan; and a corrupted, uncommitted `STATE.md` was restored to HEAD before any task work began.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Two `RunManifest` struct-literal sites needed the new field**
- **Found during:** Task 1, immediately after adding `series_admission` to `RunManifest`
- **Issue:** `crates/cli/src/cmd/run.rs` and `crates/cli/src/cmd/reconstruct.rs` each construct a `RunManifest { ... }` literal exhaustively. `#[serde(default)]` only affects deserialization; a Rust struct literal must still name every field, so `cargo build -p nr-cli` (mandated by task 1's own `<verify>` block) did not compile after task 1's own struct change alone, even though neither file is in task 1's declared `<files>` list.
- **Fix:** Added `series_admission: None` to both literals, each with a comment: `reconstruct.rs`'s is permanent (a reconstructed run never went through the live gate); `run.rs`'s is a stated compile-time placeholder, replaced with `Some(admission)` in task 2's own commit once `determine_admission` existed.
- **Files modified:** `crates/cli/src/cmd/reconstruct.rs`, `crates/cli/src/cmd/run.rs`
- **Verification:** `cargo test --workspace`, `cargo build -p nr-cli --release`, `./target/release/nrmeasure verify --strict --check-index` (0 problems), `git diff --exit-code measurements/` all pass after the fix
- **Committed in:** `6d5447a` (task 1's own commit; this is plumbing for that task's own struct change)

**Total deviations:** 1 auto-fixed (1 Rule 3 blocking-compile fix, precedented by plan 01-20's own identical resolution for `firmware_screens`/`smi_counts`).
**Impact on plan:** Necessary for task 1's own mandated verify block to run at all. No scope creep: the fix is two one-line struct-literal additions with explanatory comments, nothing else in either file was touched.

## Issues Encountered

None beyond the one item in Deviations from Plan, resolved inline.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Plan 01-13 (the headline capture) and plan 01-14 (the weekly series) can now complete as written: a clean run with a bad-looking shape is admitted, and the manifest carries both verdicts separately.
- Plans 01-25 through 01-27 (the remaining second-review findings) and plans 01-12 through 01-15 (moved to waves 19-22) are unblocked in the order STATE.md already records.
- No blockers introduced by this plan. The pre-existing blockers (the unexplained 3.8 ms global stall, the untuned-boot governor race, the D-18 firmware-floor gap) are unchanged and out of this plan's scope.

*Phase: 01-trustworthy-measurement*
*Completed: 2026-09-07*

## Self-Check: PASSED

All 15 claimed files found on disk, plus this SUMMARY.md itself. All three claimed commit
hashes (`6d5447a`, `899ad7f`, `8e121fc`) found in `git log --oneline --all`.
