---
status: PASS
agent: donny-executor
phase: 01-trustworthy-measurement
plan: 19
subsystem: provenance
tags: [rust, verification, blake3, cyclictest, provenance]

# Dependency graph
requires:
  - phase: 01-trustworthy-measurement
    provides: "01-16's build-time-embedded harness identity, verbatim argv recording, and artifact_paths mapping that this plan's re-derivation check reads the recorded --histogram bound through"
provides:
  - "nrmeasure verify --strict re-derivation (check_derived_figures): hist.tsv and REPORT.md's ## Results figures are recomputed from each harness-generated run's own raw capture and compared, using the --histogram bound recorded in that run's own argv, never an assumed default"
  - "RunSummary.p99_us/max_us as Option<u64>, rendered as the literal unavailable in measurements/INDEX.md on a histogram parse failure, never a substituted zero, and failing the --strict gate"
  - "nr_histogram::json::reconcile checks per-thread sample count, not only thread count and maxima"
  - "measurements/2026-08-28-precision3591/README.md's 1us-threshold detector-validity claim relabelled as an unverifiable note with no capture behind it, owned by plan 01-23"
affects: [01-20, 01-21, 01-22, 01-23]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "A shared format_hist_tsv function (crates/cli/src/cmd/run.rs) used by both the writer and the strict-mode re-deriver, so the on-disk hist.tsv format and the verifier's regenerated form can never drift apart"
    - "Plain line/pipe-cell matching (table_row_cells, parse_report_results) to parse a generated REPORT.md's own Results block back out, rather than a markdown parser, since the shape is fully controlled by this project's own renderer"
    - "A distinct 'not re-derivable' outcome, never counted as a --strict problem, for a run with genuinely insufficient recorded information (no --histogram bound, or a reconstructed run with no generated report) as opposed to one that fails a real re-derivation check"

key-files:
  created:
    - crates/metrics/tests/index.rs
  modified:
    - crates/cli/src/cmd/verify.rs
    - crates/cli/src/cmd/run.rs
    - crates/cli/tests/verify.rs
    - crates/metrics/src/index.rs
    - crates/metrics/tests/report.rs
    - crates/histogram/src/json.rs
    - crates/histogram/tests/json_parser.rs
    - crates/cli/tests/fixtures/fake-cyclictest.sh
    - crates/cli/tests/snapshots/run_pipeline__full_run_report_matches_snapshot.snap
    - measurements/2026-08-28-precision3591/README.md

key-decisions:
  - "Split tasks 1 and 2 into genuinely independent, self-contained commits (not merely file-scoped) by temporarily stripping task 2's additions back out of the shared verify.rs, verifying task 1 alone compiles and passes its own acceptance criteria, committing, then restoring task 2's code, verifying, and committing it separately: git's hunk-based staging could not cleanly separate two tasks intermixed in the same function"
  - "recorded_histogram_bound matches only the literal --histogram= prefix, not --histofall=, since crates/cli/src/cmd/run.rs never emits the latter and the plan's own text names only the former"
  - "Sample-count reconciliation derives each thread's total from CyclictestRun's existing bins+overflows fields rather than adding a new field to hist.rs (outside this task's file list); the computation was verified in Python against the real D-17 calibration-clean pair before writing the Rust"
  - "Fixed a real, previously invisible defect the new sample-count check exposed: fake-cyclictest.sh's synthetic --json output claimed a flat 3000000 cycles per thread that never matched its accompanying real .hist capture's true per-thread totals (~2999300); corrected to the real computed values and re-pinned the one dependent snapshot"

requirements-completed: [BENCH-04, BENCH-05]

# Metrics
duration: 38min
completed: 2026-09-05
---

# Phase 01 Plan 19: Trustworthy Measurement Summary

**Strict verification now re-derives every published percentile table and histogram from its own raw capture rather than trusting the harness that wrote it, a computation that cannot be performed renders as `unavailable` instead of a zero, and the one published claim with no saved capture behind it says so in the document that makes the claim.**

## Performance

- **Duration:** ~38 min
- **Started:** 2026-09-05T16:39:31Z (STATE.md, immediately following 01-16's completion)
- **Completed:** 2026-09-05T17:17:03Z
- **Tasks:** 3 completed
- **Files:** 1 created, 10 modified

## Accomplishments

- `nr_metrics::index::RunSummary.p99_us`/`max_us` are now `Option<u64>`, rendered as the literal `unavailable` in `measurements/INDEX.md` when a run's histogram cannot be parsed. The old `compute_percentiles(...).unwrap_or((0, 0))` (finding 6's third bullet) is gone; a malformed-but-correctly-checksummed capture now fails `verify --strict` by name instead of silently publishing a zero. Rendering the eight real committed manifests still reproduces `measurements/INDEX.md` byte for byte, proving the optional columns changed no published value.
- `nrmeasure verify --strict` gained a fourth check, `check_derived_figures`: for every harness-generated run it re-parses the run's own `cyclictest.hist` under the `--histogram` bound recorded in that run's own argv, regenerates `hist.tsv` (via a function now shared with the writer so the two can never drift), and re-derives every `## Results` figure in `REPORT.md` (p50/p95/p99/p99.9, sample count, overflow count, maximum), comparing all of it against what is committed. Against the real tree: `verify: 8 run directories, 7 re-derived, 1 not re-derivable, 0 problems (strict)`.
- The verifier never assumes a histogram bound: a run whose recorded argv carries no `--histogram=` is reported as `not re-derivable: no recorded --histogram bound` rather than guessed at. The 2026-08-28 reconstructed run (no generated report to check) is reported as `not re-derivable: reconstructed run has no generated report` by name, not silently skipped.
- `nr_histogram::json::reconcile` now also checks per-thread sample count (binned counts plus that thread's overflow count) against the `--json` summary's `cycles` field, not only thread count and maxima. This immediately caught a real, previously invisible defect: `fake-cyclictest.sh`'s synthetic `--json` output claimed a flat 3000000 cycles per thread against its accompanying real `.hist` capture's true per-thread totals of roughly 2999300, undetected because nothing checked sample count before this plan.
- `measurements/2026-08-28-precision3591/README.md`'s Caveats section no longer states the `--threshold=1` detector-validity observation as confirmed; it is relabelled as an unverifiable note with no saved capture in this repository, and plan 01-23 is named as the owner of retaking it.

## Task Commits

1. **Task 1: A value that could not be computed is unavailable, not zero**
   - `a4ce61c` (fix) - `RunSummary.p99_us`/`max_us` become `Option<u64>`; `compute_percentiles` returns `Result<(u64, u64), String>`; `render_index` renders `unavailable`; `crates/metrics/tests/index.rs` proves the committed index is byte-unchanged
2. **Task 2: Strict verification re-derives the published numbers from the raw capture**
   - `a10279d` (feat) - `check_derived_figures`, `recorded_histogram_bound`, `parse_report_results`/`table_row_cells`, `compare_derived_results`, and the shared `format_hist_tsv`
3. **Task 3: Reconcile sample counts, and label the claim with no capture behind it**
   - `c9e7926` (fix) - `reconcile`'s new sample-count check, the `fake-cyclictest.sh` fixture correction it exposed, and the README.md relabelling

_Note: none of this plan's three tasks were marked `tdd="true"` in a way requiring a separate RED commit; each was committed as a single, fully-verified unit._

## Files Created/Modified

- `crates/metrics/tests/index.rs` - new: proves `render_index` over the eight real committed manifests reproduces `measurements/INDEX.md` byte for byte
- `crates/cli/src/cmd/verify.rs` - `Option<u64>` percentile fields, `NO_HISTOGRAM_ARTIFACT` shared constant, `check_derived_figures` and its helpers (check 4)
- `crates/cli/src/cmd/run.rs` - `write_hist_tsv` split into a shared `format_hist_tsv` plus a thin disk-write wrapper
- `crates/cli/tests/verify.rs` - 7 new tests: 3 for the unavailable/strict-failure behavior, 4 for strict re-derivation
- `crates/metrics/src/index.rs` - `RunSummary.p99_us`/`max_us: Option<u64>`, `render_optional_us`
- `crates/metrics/tests/report.rs` - 3 `RunSummary` literals updated to wrap `p99_us`/`max_us` in `Some(...)` (Rule 3 deviation)
- `crates/histogram/src/json.rs` - `reconcile`'s new per-thread sample-count check, `samples_per_thread`
- `crates/histogram/tests/json_parser.rs` - 2 new tests: rejects a sample-count disagreement, accepts the real committed pair
- `crates/cli/tests/fixtures/fake-cyclictest.sh` - corrected per-thread `cycles` values (Rule 1 deviation)
- `crates/cli/tests/snapshots/run_pipeline__full_run_report_matches_snapshot.snap` - re-pinned `cyclictest.json` blake3 line (Rule 1 deviation)
- `measurements/2026-08-28-precision3591/README.md` - Caveats section relabelled; only file touched under this run directory (D-12)

## Decisions Made

- Split tasks 1 and 2 into genuinely independent commits rather than one combined commit, even though both are concentrated in `crates/cli/src/cmd/verify.rs`'s `run()`/`build_summaries`/`compute_percentiles` region: temporarily removed task 2's `check_derived_figures` section, its imports, its `run()` wiring, and its four tests, confirmed the resulting task-1-only state independently compiles, passes `cargo clippy`/`cargo fmt`, passes all of task 1's own named tests, and reproduces task 1's own real-tree verification line, then committed; restored task 2's code from a scratch backup, re-verified, and committed separately.
- `recorded_histogram_bound` matches only `--histogram=`, not `--histofall=`: `run.rs` never emits the latter, and matching only the one real form kept the "no hardcoded default bound" acceptance check (no literal `400` outside a test) satisfiable without weakening the check.
- Sample-count reconciliation computes each thread's total from `CyclictestRun`'s existing `bins` (bin_us -> per-thread counts) and `overflows` (per-thread overflow counts) fields rather than adding a new field to `hist.rs`, which is outside task 3's declared file list. Verified in Python against the real `measurements/2026-09-01-precision3591-calibration-clean` pair before writing the Rust: per-thread totals matched the `--json` summary's `cycles` exactly.
- The malformed-capture fixture for task 1 (`write_run_with_malformed_hist`) follows the existing `write_valid_run` pattern in `crates/cli/tests/verify.rs` (a fixture manifest plus a garbage capture file with a matching blake3) rather than copying a whole real run directory, since only one artifact and its checksum needed to be internally consistent.
- The four task-2 re-derivation tests copy a real committed run directory (`copy_real_run`/`copy_dir_recursive`) into a tempdir and edit exactly one value, per the plan's own instruction, rather than reusing the minimal fixture manifest (which uses a `--histofall=400` argv convention that predates this plan and is unrelated to the real runs' `--histogram=400` argv).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Updated `crates/metrics/tests/report.rs`'s `RunSummary` literals for the `Option<u64>` field change**
- **Found during:** Task 1 (`cargo clippy --workspace --all-targets`)
- **Issue:** Three `RunSummary { p99_us: 9, max_us: 27, ... }`-shaped literals (not in task 1's declared file list) failed to compile once `p99_us`/`max_us` became `Option<u64>`.
- **Fix:** Wrapped each literal integer in `Some(...)`. No test logic changed.
- **Files modified:** crates/metrics/tests/report.rs
- **Verification:** `cargo test -p nr-metrics`
- **Committed in:** a4ce61c

**2. [Rule 3 - Blocking] Extracted `format_hist_tsv` from `crates/cli/src/cmd/run.rs`**
- **Found during:** Task 2
- **Issue:** `run.rs` is not in task 2's declared `<files>` list, but the task's own action text explicitly requires extracting the `hist.tsv` formatting into a function the verifier can call, "prefer sharing" over duplicating.
- **Fix:** Split `write_hist_tsv` into `pub(crate) fn format_hist_tsv(&CyclictestRun) -> String` plus a thin `std::fs::write` wrapper; `check_derived_figures` calls `crate::cmd::run::format_hist_tsv` directly, so the writer and the verifier share one formatting implementation.
- **Files modified:** crates/cli/src/cmd/run.rs
- **Verification:** `cargo test -p nr-cli --test verify strict_rederives_hist_tsv`
- **Committed in:** a10279d

**3. [Rule 1 - Bug] Corrected `fake-cyclictest.sh`'s per-thread `cycles` values**
- **Found during:** Task 3 (`cargo test --workspace`, run against the real tree after staging)
- **Issue:** The test fixture's synthetic `--json` output reported a flat `3000000` cycles for every thread. Its accompanying real `.hist` capture (a byte-identical copy of the committed 2026-08-28 capture) has true per-thread totals of 2999329/2999297/2999302/2999303/2999313/2999300. Nothing checked sample count before this plan, so the mismatch was invisible; task 3's new `reconcile` check immediately refused the pairing, failing 11 `run_pipeline.rs` integration tests that exercise the full pipeline.
- **Fix:** Replaced the flat value with the real computed per-thread totals (verified independently in Python from the committed `.hist` file's own bins and overflow footer).
- **Files modified:** crates/cli/tests/fixtures/fake-cyclictest.sh
- **Verification:** `cargo test --workspace` (all 11 previously-failing tests pass)
- **Committed in:** c9e7926

**4. [Rule 1 - Bug] Re-pinned `run_pipeline__full_run_report_matches_snapshot.snap`'s `cyclictest.json` blake3 line**
- **Found during:** Task 3, immediately after fix #3
- **Issue:** Correcting the fixture's JSON content (fix #3) changed that file's bytes and therefore its blake3 digest, which one pinned snapshot asserts as part of a rendered `## Artifacts` table.
- **Fix:** Updated the one changed hex digest in the committed `.snap` file to the newly, correctly computed value. No test logic or `nr-metrics`/`nr-cli` behavior changed; the file's byte count (1363) was unaffected since the replacement values are the same digit length as the original.
- **Files modified:** crates/cli/tests/snapshots/run_pipeline__full_run_report_matches_snapshot.snap
- **Verification:** `cargo test -p nr-cli --test run_pipeline full_run_report_matches_snapshot`
- **Committed in:** c9e7926

**Total deviations:** 4 auto-fixed (2 blocking, 2 bug). **Impact on plan:** all four are direct, mechanical, or newly-exposed consequences of the plan's own two mandated changes (the `Option<u64>` field type in task 1, sample-count reconciliation in task 3) and the plan's own explicit sharing instruction (task 2); none expand scope beyond finding 6.

## Issues Encountered

- `crates/cli/build.rs` (from plan 01-16) embeds `git_dirty` at build time and only reruns on `.git/HEAD`/`.git/index` changes, so `cargo test --workspace` run mid-edit (before staging a task's files) can embed a stale `git_dirty` that disagrees with `crates/cli/tests/run_pipeline.rs::harness_git_sha_source_is_explicit`'s own live `git status --porcelain` check, failing that one test with no code defect involved. Not a bug in either file (both are outside this plan's scope and correct on their own terms); staging a task's files before running the full workspace suite (which the per-task commit protocol does anyway) gives build.rs a fresh `.git/index` to react to and the test passes. Recorded here so a future executor does not mistake this for a real regression if it recurs.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- All eight committed run directories (the D-17 calibration pair, the 2026-09-03 pair, the three 2026-09-05 screens, and the 2026-08-28 reconstruction) pass `nrmeasure verify --strict --check-index` with 0 problems; seven re-derive, one is recorded as not re-derivable by name.
- `measurements/` is otherwise untouched: `git diff --name-only measurements/2026-08-28-precision3591/` (against the pre-plan tree) lists only `README.md`, per D-12.
- Finding 6 of `01-EXTERNAL-AUDIT.md` is fully closed: plan 01-16 closed the harness-identity and argv/artifact-mapping halves; this plan closes the `(0, 0)` substitution, the `hist.tsv`/`REPORT.md` re-derivation gap, the JSON/`.hist` sample-count reconciliation gap, and the unsaved 1us detector-validity claim.
- Plan 01-23 (re-taking the D-18 firmware screens on the isolated cores) now has an explicit second obligation named in the 2026-08-28 README: retake the `--threshold=1` detector-validity run with a saved capture.
- No blockers for the next wave.

## Self-Check: PASSED

All 10 modified files and the 1 created file confirmed present on disk with the expected content; all 3 commit hashes (`a4ce61c`, `a10279d`, `c9e7926`) confirmed present in `git log`.

*Phase: 01-trustworthy-measurement*
*Completed: 2026-09-05*
