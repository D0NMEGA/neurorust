---
status: PASS
agent: donny-executor
phase: 01-trustworthy-measurement
plan: 06
subsystem: measurement-metrics
tags: [rust, schemars, blake3, serde, time, insta, regression-testing, iso-week, bench-08, plat-03]

# Dependency graph
requires:
  - phase: 01-trustworthy-measurement (plan 03)
    provides: nr_manifest's RunManifest, RunClass, InstrumentClass, ProvenanceTier, ContaminationVerdict, PreconditionResult, InterferenceSnapshotPair and the generated-schema pattern to copy
  - phase: 01-trustworthy-measurement (plan 04)
    provides: nr_histogram's CyclictestRun, Percentiles, parse_hist_file, and the overflow-inclusive percentile path
provides:
  - The BENCH-08 metrics series (MetricsSeries/StageMetrics), stage-name open by design (D-04), with a generated and drift-checked JSON Schema
  - append(), the only mutation the series offers: rejects a duplicate run_id, keeps entries sorted by utc_start, never filters on contamination_verdict or excluded_from_series (BENCH-06)
  - The D-08 coverage record (Coverage/WeekRecord/GapReason) and record_gaps/record_gap, which can only ever write a Gap entry, never a synthesised metric
  - The D-11 regression comparison (baseline::compare), with thresholds loaded exclusively from the committed metrics/baseline.json, and Contaminated/Uncalibrated/NoComparableBaseline kept distinct from Pass
  - Per-run REPORT.md rendering (report::render_run_report) and the D-22 PLAT-03 decomposition (report::render_plat03_verdict), which refuses to render without a firmware floor and its source run id
  - The BENCH-06 index renderer (index::render_index), which emits a row for every run summary unconditionally
affects: [01-07-nr-cli, 01-08-nr-cli, 01-14-nr-cli, phase-02-STOP-07, phase-03-SUBS-06]

# Tech tracking
tech-stack:
  added: [blake3 (new direct nr-metrics dependency, for the report header's manifest fingerprint)]
  patterns:
    - "Schema-generated-not-authored, reused from nr-manifest: schemars::schema_for!(MetricsSeries) compared byte-for-byte against the committed schemas/metrics.schema.json in metrics_schema_up_to_date, with the same UPDATE_SCHEMAS=1 regeneration escape hatch"
    - "Config-not-code for judgement thresholds: baseline.rs contains no threshold literal at all; compare() loads relative_pct/absolute_us exclusively from a parsed metrics/baseline.json, so D-11's regression judgement is always a reviewable config diff"
    - "Distinct-verdict-never-Pass: RegressionVerdict and (already, from plan 01-05) ContaminationVerdict both keep a losing or uncomparable outcome as its own variant, so a gate can never silently default to a pass for what it cannot evaluate"
    - "Generated report ties itself to its source: render_run_report computes a blake3 fingerprint of the manifest it was given and prints it in the header, so the human-readable artifact and the manifest it was derived from can never quietly disagree (D-12)"
    - "Shared kebab() helper (lib.rs, pub(crate)) renders any rename_all=kebab-case enum the same way its JSON form does, used by both report.rs and index.rs so a report and the index it feeds agree with the manifest's own vocabulary"

key-files:
  created:
    - crates/metrics/src/series.rs
    - crates/metrics/src/coverage.rs
    - crates/metrics/src/schema.rs
    - crates/metrics/src/baseline.rs
    - crates/metrics/src/report.rs
    - crates/metrics/src/index.rs
    - crates/metrics/tests/series.rs
    - crates/metrics/tests/regression.rs
    - crates/metrics/tests/report.rs
    - crates/metrics/tests/snapshots/report__headline_report.snap
    - schemas/metrics.schema.json
    - metrics/baseline.json
  modified:
    - crates/metrics/src/lib.rs
    - crates/metrics/Cargo.toml
    - Cargo.lock

key-decisions:
  - "record_gap(coverage, iso_week, reason) added as a generic single-week gap writer alongside the plan's named record_gaps(coverage, up_to), so GapReason::RefusedOnPrecondition and RunFailed have a real producer; without it, D-08's 'a refused run produces the same recorded gap' and the refused_run_records_gap test would have no function to call (Rule 2)"
  - "series.rs's module doc comment reworded to avoid the literal substring percentiles_excluding_overflows, which tripped this plan's own grep -c acceptance criterion (no doc-comment exception, unlike the parallel Task 2 threshold-literal check); caught and fixed before the Task 1 commit"
  - "blake3 added as a direct nr-metrics dependency (Rule 3, blocking): render_run_report's header must name the manifest blake3 it was generated from, and nr_manifest's public API only exposes a file-path hasher (blake3_file), not a bytes hasher, so computing a fingerprint of an in-memory RunManifest requires hashing its serialized bytes directly in this crate"
  - "The Task 3 'hand-built manifest' is authored as an embedded JSON string (adapted from crates/manifest/tests/fixtures/minimal-manifest.json) parsed via serde_json::from_str, rather than ~20 nested Rust struct literals; RunManifest already round-trips through serde, and the project's own convention already treats JSON as the canonical worked-example form"
  - "ISO week arithmetic (coverage.rs) uses time::Date::from_iso_week_date/to_iso_week_date rather than a hand-rolled '52 weeks per year' assumption, so a 53-ISO-week year rolls over correctly; verified against the time crate's documented API via Context7 before writing it"
  - "No over-gate sample count (samples_above/samples_at_or_above) appears anywhere in this plan's code: the PLAT-03 verdict compares two scalars (observed_max_us vs gate_us and vs firmware_floor_us), not a population count, so the 2026-08-31 convention change (samples_at_or_above, overflow-inclusive denominator) had no code surface to apply to here. Confirmed by grep after implementation."

requirements-completed: []

# Metrics
duration: ~11min across three task commits (context-loading and research preceded PLAN_START_TIME, which was not captured at session start; see Performance)
completed: 2026-08-31
---

# Phase 01 Plan 06: nr-metrics - BENCH-08 series, D-11 baseline, D-08 coverage, D-22 PLAT-03 decomposition Summary

**nr-metrics crate: an open-stage BENCH-08 series with a generated schema, a D-11 regression gate whose thresholds live entirely in `metrics/baseline.json`, a D-08 coverage record that can only write a gap (never backfill), and a generated REPORT.md/INDEX.md pair where the PLAT-03 verdict refuses to render without its firmware-floor attribution.**

## Performance

- **Duration:** approximately 11 minutes of active implementation, measured commit-to-commit (`f59ac2d` at 10:11:30 to `ce021be` at 10:21:30, local time on 2026-08-31). This session did not run the `record_start_time` step before beginning context-loading, so the preceding research phase (reading `01-06-PLAN.md`, `01-VALIDATION.md`, three prior `SUMMARY.md` files, `FINDINGS.md`, `RIG.txt`, the manifest/histogram source, and two Context7 lookups confirming the `time` crate's ISO-week-date API) is not included in this figure and was not separately timestamped.
- **Started:** 2026-08-31T10:11:30-05:00 (Task 1 commit; see note above)
- **Completed:** 2026-08-31T10:21:30-05:00 (Task 3 commit)
- **Tasks:** 3 (all complete)
- **Files changed:** 15 (12 created, 3 modified)

## Accomplishments

- `MetricsSeries`/`StageMetrics` (BENCH-08): `p50_us`, `p95_us`, `p99_us`, `p999_us` and `max_us` per stage, with `stage` typed as an open `String` (D-04) rather than a closed enum, demonstrated in `stage_names_are_open` with a non-cyclictest stage (`"emergency_stop.abort_latency"`, tool `"nr-stop-harness"`)
- `append()` is the only mutation the series offers: rejects a duplicate `run_id`, keeps `entries` sorted by `utc_start`, and inspects nothing about `contamination_verdict` or `excluded_from_series` to decide whether to keep an entry, so a contaminated run is retained exactly like a clean one (`contaminated_run_retained`)
- `schemas/metrics.schema.json` generated via `schemars` from `MetricsSeries` and drift-checked (`metrics_schema_up_to_date`), matching nr-manifest's established pattern exactly
- The D-08 coverage record (`Coverage`/`WeekRecord`/`GapReason`): `record_gaps` fills every ISO week strictly between the last recorded week and a new entry's week with `GapReason::NoRunRecorded`, correctly handling ISO week-numbering year rollover via `time::Date::from_iso_week_date`/`to_iso_week_date` rather than assuming 52 weeks a year; `WeekRecord::Gap` has no field that could hold a metric value, so backfilling is not representable, not just discouraged (`gaps_are_never_backfilled`)
- The D-11 regression gate (`baseline::compare`) contains no threshold literal: every number comes from a parsed `metrics/baseline.json`, so a change to the judgement is a reviewable config diff. `Contaminated`, `Uncalibrated` and `NoComparableBaseline` are distinct `RegressionVerdict` variants and none of them can ever be constructed as `Pass`
- `metrics/baseline.json` ships with the stated thresholds (p99: 20% or 2us; max: 50% or 100us, whichever is larger) and an empty `entries` array, exactly as the plan specifies; the empty baseline is the honest state until plan 01-14 seeds the first entry from the PLAT-03 headline run
- `report::render_run_report` generates the full per-run `REPORT.md`: header (with a blake3 fingerprint of the manifest computed by the renderer itself), rig and tuning in `RIG.txt`'s own field order, all 14 preconditions always, the D-15 contamination verdict with per-CPU counter deltas, a percentile/sample/overflow/max results section (via the overflow-inclusive `CyclictestRun::percentiles`, never the excluding path), a fixed-width ASCII log-scale histogram, the overflow convention paragraph, and an artifacts table
- `report::render_plat03_verdict` implements D-22 mechanically: `Plat03Input.firmware_floor_us`/`firmware_floor_source_run_id` are not `Option`, and rendering with an empty source run id returns `Err(ReportError::FirmwareFloorRequired)` rather than an undecomposed total; a miss reads "documented limitation", never "fail" alone
- `index::render_index` (BENCH-06) emits one row per `RunSummary` unconditionally, including excluded ones, with `no` in the "in series" column and the exclusion reason in the reason column (`losing_config_rendered`, `index_never_omits`)
- A snapshot test (`report__headline_report.snap`) renders a full headline report from a hand-built manifest plus the real, committed 2026-08-28 fixture (`crates/histogram/tests/fixtures/cyclictest-rt-isolated-idle-10m.hist`: 6 threads, 17,995,844 total samples including 888 overflows, 3,806us max), ASCII-clean, with all 14 precondition rows present
- 21 new tests (8 series, 6 regression, 7 report/index), all passing; full workspace suite green with zero failures across all crates

## Task Commits

Each task was committed atomically:

1. **Task 1: The BENCH-08 metrics series, the coverage record, and the generated schema** - `f59ac2d` (feat)
2. **Task 2: The D-11 baseline comparison with concrete thresholds** - `95c8863` (feat)
3. **Task 3: Report rendering, the D-22 PLAT-03 decomposition, and the BENCH-06 index** - `ce021be` (feat)

**Plan metadata:** commit hash recorded after this summary is written (see final commit).

## Files Created/Modified

- `crates/metrics/src/series.rs` - `MetricsSeries`, `StageMetrics`, `SeriesError`, `append()`
- `crates/metrics/src/coverage.rs` - `Coverage`, `WeekRecord`, `GapReason`, `record_gap()`, `record_gaps()`, and the private ISO-week arithmetic helpers
- `crates/metrics/src/schema.rs` - `metrics_schema_json()`, the schemars-driven generator for `MetricsSeries`
- `crates/metrics/src/baseline.rs` - `Threshold`, `Thresholds`, `BaselineEntry`, `Baseline` (with `Baseline::load`), `RegressedMetric`, `MetricRegression`, `RegressionVerdict`, `compare()`
- `crates/metrics/src/report.rs` - `ReportError`, `Plat03Input`, `render_plat03_verdict()`, `render_run_report()` and its section helpers
- `crates/metrics/src/index.rs` - `RunSummary`, `render_index()`
- `crates/metrics/src/lib.rs` - registers all five new modules; adds the shared `pub(crate) fn kebab()` helper
- `crates/metrics/Cargo.toml` - adds `blake3` as a direct dependency
- `Cargo.lock` - updated for the new dependency edge
- `crates/metrics/tests/series.rs` - the 8 Task 1 behavior tests
- `crates/metrics/tests/regression.rs` - the 6 Task 2 behavior tests, loading real thresholds from the committed `metrics/baseline.json`
- `crates/metrics/tests/report.rs` - the 6 named Task 3 behavior tests plus the snapshot test, and the hand-built manifest fixture JSON
- `crates/metrics/tests/snapshots/report__headline_report.snap` - the committed, reviewable snapshot of a full rendered report
- `schemas/metrics.schema.json` - generated from `MetricsSeries`
- `metrics/baseline.json` - the committed D-11 thresholds and empty baseline entries array

## Decisions Made

See the frontmatter `key-decisions` block for the full list. The two with the widest downstream effect:

**`record_gap` added alongside the plan's named `record_gaps`.** D-08 requires "a run refused on a precondition violation produces a Gap entry with reason naming the failed check," and `GapReason::RefusedOnPrecondition`/`RunFailed` are named variants in the plan's own action text, but only `record_gaps` (plural, for missed weeks) was specified. Without a producer, those two variants would be unreachable by any real caller and the `refused_run_records_gap` behavior test (required by `01-VALIDATION.md`) would have nothing to call. Added `record_gap(coverage, iso_week, reason)` as a small, generic, single-week writer that `record_gaps` itself uses internally for `NoRunRecorded`, and that a future caller (nr-cli, catching `nr-capture`'s `RefusalError`) calls directly for the other two reasons.

**`blake3` added as a direct nr-metrics dependency.** `render_run_report`'s header must name "the manifest blake3 it was generated from" (a literal, tested acceptance criterion). `nr_manifest`'s public surface exposes only `blake3_file(path)`, a file hasher, not a bytes hasher, and `render_run_report` receives an in-memory `&RunManifest`, not a path. Rather than touch `nr_manifest` (outside this plan's file list) to add a generic hasher, `report.rs` serializes the manifest to bytes and hashes them directly with `blake3` (already a pinned workspace dependency used identically by `nr_manifest`).

**BENCH-06/BENCH-08/PLAT-03 traceability.** This plan carries these three jointly with plans 01-10, 01-13, 01-14 and 01-15. Per explicit instruction, `requirements mark-complete` was not run and `.planning/REQUIREMENTS.md` was left completely untouched; the end-of-phase verifier owns marking these complete once every contributing plan has landed.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Added `record_gap` as a generic single-week gap writer**
- **Found during:** Task 1, implementing `coverage.rs`'s behavior list (`refused_run_records_gap`)
- **Issue:** The plan's action text names `GapReason::RefusedOnPrecondition { check }` and `RunFailed { detail }` as variants and states D-08's rule that a refusal "records the same gap" a missed week does, but only specifies `record_gaps(coverage, up_to)` (for filling missed weeks between two dates). No function in the plan's text can construct a `RefusedOnPrecondition` or `RunFailed` gap directly, so those variants would be unreachable and the `refused_run_records_gap` test (required by `01-VALIDATION.md`, a row this plan explicitly owns) could not be implemented against real crate behavior.
- **Fix:** Added `pub fn record_gap(coverage: &mut Coverage, iso_week: impl Into<String>, reason: GapReason)`, a small, generic building block. `record_gaps` (plural) now calls it internally in its loop for `NoRunRecorded`; the test calls it directly for `RefusedOnPrecondition`.
- **Files modified:** `crates/metrics/src/coverage.rs`
- **Verification:** `refused_run_records_gap` and all 8 `tests/series.rs` tests pass; `grep -q 'RefusedOnPrecondition' crates/metrics/src/coverage.rs` exits 0.
- **Committed in:** `f59ac2d` (Task 1 commit)

**2. [Rule 1 - Bug] Reworded a doc comment that tripped this plan's own acceptance-criteria grep**
- **Found during:** Task 1, running the full acceptance-criteria sweep after implementing `series.rs`
- **Issue:** `series.rs`'s module doc comment originally read "...never calls `percentiles_excluding_overflows`...", which is itself a literal match for the plan's own acceptance criterion `grep -c 'percentiles_excluding_overflows' crates/metrics/src/` reporting 0 (this criterion, unlike the parallel Task 2 threshold-literal check, has no "outside a doc comment" exception).
- **Fix:** Reworded to "...never reaches for the sibling, overflow-excluding computation...", preserving the same meaning without the literal identifier.
- **Files modified:** `crates/metrics/src/series.rs`
- **Verification:** `grep -rc 'percentiles_excluding_overflows' crates/metrics/src/` now reports 0; `cargo test -p nr-metrics` unaffected.
- **Committed in:** `f59ac2d` (Task 1 commit)

**3. [Rule 3 - Blocking] Added `blake3` as a direct `nr-metrics` dependency**
- **Found during:** Task 3, implementing `render_run_report`'s header section
- **Issue:** The header must name "the manifest blake3 it was generated from" (acceptance criterion: a `manifest blake3:` line in the rendered output). `render_run_report` takes only `&RunManifest` and `&CyclictestRun` per the plan's own interfaces block, so the fingerprint must be computed by the renderer itself. `nr_manifest`'s public API (`blake3_file`) only hashes a file at a path, not an in-memory struct's bytes, and adding a bytes-hasher to `nr_manifest` would touch a crate outside this plan's file list.
- **Fix:** Added `blake3 = { workspace = true }` (already a pinned, approved workspace dependency used identically by `nr_manifest`) to `crates/metrics/Cargo.toml`, and serialize-then-hash the manifest directly in `report.rs`.
- **Files modified:** `crates/metrics/Cargo.toml`, `Cargo.lock`, `crates/metrics/src/report.rs`
- **Verification:** `cargo deny check` reports `advisories ok, bans ok, licenses ok, sources ok` with no new issues; `report_is_generated_not_authored` passes.
- **Committed in:** `ce021be` (Task 3 commit)

---

**Total deviations:** 3 auto-fixed (1 missing-critical closure of a gap in the plan's own text, 1 self-caught bug against this plan's own acceptance criterion, 1 blocking dependency addition).
**Impact on plan:** All three were necessary for this plan's own literal acceptance criteria and `01-VALIDATION.md` rows to pass. No scope creep: every change stayed inside `crates/metrics/`, and `nr_manifest`/`nr_histogram` were left completely untouched throughout.

## Issues Encountered

Two self-caught, pre-commit mechanical errors during Task 3 iteration, neither of which reached a commit:

- The Task 3 raw string literal for the hand-built manifest fixture (`r#"..."#`) terminated early, because the JSON content itself contains `"#30-Ubuntu` (a value starting with a literal `#`), which matches the `r#"..."#` closing delimiter. Fixed by switching to `r##"..."##`.
- `insta::assert_snapshot!` auto-prefixes the given name with the test binary's own target name (`report`), so passing the full name `"report__headline_report"` produced `report__report__headline_report.snap`. Fixed by passing just `"headline_report"`, letting insta's own prefixing produce the exact required filename.

Neither affected the crate's behavior; both were caught and corrected before the Task 3 commit.

## Known Stubs

- **`metrics/baseline.json`'s `entries` array is intentionally empty.** This is not a shortcut: the plan's own action text specifies exactly this ("Ship `metrics/baseline.json` with an empty `entries` array... An empty baseline yields `NoComparableBaseline` for every stage, which is the honest state until then"). Every stage compared against this baseline today returns `RegressionVerdict::NoComparableBaseline`, never a silent pass. Plan 01-14 populates the first entry from the PLAT-03 headline run, which is the first run measured under the finished protocol and therefore the first defensible baseline.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `nr-metrics` is complete for this plan's scope: the BENCH-08 series, the D-11 regression gate, the D-08 coverage record, per-run report rendering with the D-22 PLAT-03 decomposition, and the BENCH-06 index renderer. `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` are all green (97 tests passing, 0 failed).
- Plan 01-07 (`nr-cli` `run`) can call `report::render_run_report(&manifest, &run)` directly after a capture, exactly as this plan's `<interfaces>` block specifies.
- Plan 01-08 (`nr-cli` `verify`) can call `index::render_index(&[RunSummary])` for `measurements/INDEX.md`.
- Plan 01-14 (`nr-cli` `series`) can call `series::append`, `coverage::record_gaps`/`record_gap`, and `baseline::compare` directly, and is responsible for constructing `StageMetrics` values from a completed run (this plan does not build that conversion, since no task or test in `01-06-PLAN.md` required it) and for seeding `metrics/baseline.json`'s first entry from the PLAT-03 headline run.
- Phase 2's STOP-07 and Phase 3's SUBS-06 can append a `StageMetrics` with a new `stage` string (e.g. `"emergency_stop.abort_latency"`) into the same `MetricsSeries`/`schemas/metrics.schema.json` contract without any change to this crate, demonstrated here with `stage_names_are_open`.
- `BENCH-06`, `BENCH-08` and `PLAT-03` remain open at the requirements level (shared with 01-10/01-13/01-14/01-15 as detailed under Decisions); no action taken here, flagged for the end-of-phase verifier.
- No blockers.

---
*Phase: 01-trustworthy-measurement*
*Completed: 2026-08-31*

## Self-Check: PASSED

- All 12 created files and 3 modified files confirmed present on disk with `[ -f ]`.
- All 3 commits (`f59ac2d`, `95c8863`, `ce021be`) confirmed present in `git log --oneline --all`.
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` (97 tests, 0 failed) all re-ran clean immediately before this SUMMARY was finalized.
- The plan's own `<verification>` block re-run directly and passed: `UPDATE_SCHEMAS=1 cargo test -p nr-metrics metrics_schema_up_to_date` followed by `git diff --exit-code schemas/` (clean), the BENCH-08 field-name check (`p50_us`/`p95_us`/`p99_us` all `True`), and the ASCII-only check on `report__headline_report.snap` (clean).
- All six `01-VALIDATION.md` rows this plan owns re-ran individually and passed: `contaminated_run_retained`, `losing_config_rendered`, `schema_roundtrip`, `regression_gate`, `missed_week_records_gap`, `plat03_report_decomposition`.
- `cargo deny check` re-run after adding the new `blake3` dependency: `advisories ok, bans ok, licenses ok, sources ok`.
- Confirmed via grep: zero occurrences of `samples_above`/`samples_at_or_above` anywhere in `crates/metrics/` (no over-gate figure is rendered by this plan) and zero non-ASCII bytes in any new source or snapshot file.
