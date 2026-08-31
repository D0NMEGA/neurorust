---
status: PASS
agent: donny-executor
phase: 01-trustworthy-measurement
plan: 04
subsystem: measurement-histogram
tags: [rust, hdrhistogram, serde, cyclictest, parsing, bench-05]

# Dependency graph
requires:
  - phase: 01-trustworthy-measurement (plan 01)
    provides: cargo workspace, nr-histogram stub crate, the real 2026-08-28 cyclictest fixture and its documented expected values
  - phase: 01-trustworthy-measurement (plan 02)
    provides: real rig-captured -h/-H/--json probe fixtures and the FINDINGS.md schema/column-count ground truth
provides:
  - CyclictestRun/OverflowBoundSource (hist.rs), the typed .hist parser including the footer
    block, both -h and -H column layouts, and truncation/mismatch detection
  - Percentiles/PercentileError (percentiles.rs), hdrhistogram-backed percentile computation
    with cyclictest's separately reported overflow samples counted by default
  - CyclictestSummary/SysInfo/ThreadSummary and reconcile() (json.rs), --json parsing against
    the real rt-tests 2.9-1ubuntu1 (cyclictest V 2.80) schema, cross-checked against the paired
    .hist file
  - The D-23 regression fixed and locked down by test: 2,089 of 17,994,956 samples (0.0116%)
    over the 30us gate with overflows counted, versus the published 1,201 (0.0067%) that
    silently dropped them
affects: [01-06, 01-07]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Overflow-inclusive by default, overflow-exclusive only under an explicitly named method
      with a \"do not use for a new figure\" doc comment (percentiles()/samples_above() vs
      percentiles_excluding_overflows()/samples_above_excluding_overflows()) -- the smallest
      mechanism that stops the 2026-08-28 error recurring by accident"
    - "Thread count derived from the one footer line FINDINGS.md confirms never gains a -H
      summary column (# Min Latencies:), not from # Max Latencies: as the plan text assumed;
      the two lines that can carry a summary column (# Max Latencies:, # Histogram Overflows:)
      are truncated back to the thread count wherever they carry it"
    - "deny_unknown_fields on every --json struct (CyclictestSummary, SysInfo, ThreadSummary),
      typed against a real captured schema rather than an assumed rt-tests version, so a future
      schema change fails loudly instead of silently dropping data"
    - "reconcile() as a cheap cross-file integrity check: a --json summary and a .hist file from
      different runs disagree on thread count or per-thread maxima and are rejected rather than
      silently paired"

key-files:
  created:
    - crates/histogram/src/hist.rs
    - crates/histogram/src/percentiles.rs
    - crates/histogram/src/json.rs
    - crates/histogram/tests/hist_parser.rs
    - crates/histogram/tests/percentiles.rs
    - crates/histogram/tests/json_parser.rs
  modified:
    - crates/histogram/src/lib.rs

key-decisions:
  - "Thread count is derived from \"# Min Latencies:\", not \"# Max Latencies:\" as the plan's
    own action text asserted -- FINDINGS.md's empirically measured \"Column counts\" section
    (from plan 01-02's real rig probes) shows \"# Max Latencies:\" and \"# Histogram Overflows:\"
    both gain the -H summary column (6 to 7 fields), while \"# Min Latencies:\" and
    \"# Avg Latencies:\" stay thread-count-only in both layouts. Followed the real fixture data
    per this plan's own instruction to trust real captures over an assumed rule."
  - "Added parse_hist_file(&Path, Option<u64>) and parse_json_file(&Path) as thin file-reading
    wrappers around parse_hist/serde_json::from_str, matching the plan's <interfaces> block
    (\"Consumers, so the public surface is right first time\") ahead of plans 01-06/01-07 needing
    them, with HistError gaining an Io variant to carry read failures."
  - "HistError (defined in hist.rs) is shared by json.rs's SummaryDisagreement and
    JsonDeserialize variants, added incrementally in Task 3's commit rather than front-loaded
    into Task 1, so each task's diff matches exactly what that task's own tests exercise."

requirements-completed: [BENCH-05]

# Metrics
duration: 18min
completed: 2026-08-31
---

# Phase 01 Plan 04: Cyclictest histogram parsing, overflow accounting, and hdrhistogram percentiles Summary

**nr-histogram crate parsing both cyclictest .hist column layouts and the real --json schema into one run type, with hdrhistogram-backed percentiles that count the 888 overflow samples by default and fix the published D-23 over-gate error (2,089 of 17,994,956, not 1,201).**

## Performance

- **Duration:** 18 min
- **Started:** 2026-08-31T05:59:00Z
- **Completed:** 2026-08-31T06:17:07Z
- **Tasks:** 3
- **Files changed:** 7 (6 created, 1 modified)

## Accomplishments

- `parse_hist`/`parse_hist_file` parse the cyclictest `.hist` footer block (Min/Avg/Max
  Latencies, Histogram Overflows, per-thread overflow cycle numbers) and both the `-h` and `-H`
  column layouts against the real 2026-08-28 10-minute capture (6 threads, 17,994,956 binned
  samples, 888 overflows, 3,806 us maximum) and the real 60-second rig probes from plan 01-02
- Corrected the plan's own stated column-count rule using FINDINGS.md's empirically measured
  data: thread count comes from `# Min Latencies:`, the one footer line that never gains a `-H`
  summary column, not from `# Max Latencies:` as the plan's action text claimed
- `CyclictestRun::percentiles`/`samples_above` count the 888 overflow samples at the histogram
  bound by default; the incorrect, overflow-excluding computation that produced the published
  2026-08-28 README figure is reachable only through `percentiles_excluding_overflows`/
  `samples_above_excluding_overflows`, both documented "do not use for a new figure"
  Reproduced and locked in with a test: 2,089 of 17,994,956 (0.0116%) over the 30us gate with
  overflows counted versus the published 1,201 (0.0067%) without, and p99.99 of 108us versus 12us
- `CyclictestSummary`/`SysInfo`/`ThreadSummary` parse the real `cyclictest --json` schema
  (rt-tests 2.9-1ubuntu1, cyclictest V 2.80) captured on the rig by plan 01-02, including the
  literal trailing-colon JSON keys (`"cmdline:"`, `"rt_test_version:"`) and the sparse
  string-keyed per-thread histogram; `deny_unknown_fields` on all three structs
- `reconcile()` cross-checks a `--json` summary against its paired `.hist` file (thread count,
  per-thread maxima), catching a mismatched or truncated pairing
- 24 new tests (12 hist parsing, 7 percentiles, 5 json), all passing against real rig-captured
  fixtures with no synthetic data; full workspace suite at 43 tests, fmt and clippy clean

## Task Commits

Each task was committed atomically:

1. **Task 1: Parse the cyclictest .hist file, footer block included** - `0ef4311` (feat)
2. **Task 2: Percentiles and over-gate counts with the overflows included** - `bc64684` (feat)
3. **Task 3: Parse cyclictest --json against the real schema from the rig** - `d259763` (feat)
4. **Fix: remove an accidental "percentile" substring from hist.rs's doc comment** - `0af1ec6` (fix)

**Plan metadata:** committed separately after this SUMMARY (see final commit).

## Files Created/Modified

- `crates/histogram/src/hist.rs` - `CyclictestRun`, `OverflowBoundSource`, `HistError`,
  `parse_hist`/`parse_hist_file`; parses the `.hist` footer and both column layouts, nothing else
- `crates/histogram/src/percentiles.rs` - `Percentiles`, `PercentileError`, and the
  `impl CyclictestRun` block computing percentiles/over-gate counts via per-thread
  `hdrhistogram::Histogram<u64>` (3 sigfig) merged with `add()`
- `crates/histogram/src/json.rs` - `CyclictestSummary`/`SysInfo`/`ThreadSummary`,
  `parse_json_file`, `reconcile()`; typed against the real probe JSON, not an assumed schema
- `crates/histogram/src/lib.rs` - registers `pub mod hist; pub mod json; pub mod percentiles;`;
  removed the plan 01-01 placeholder `crate_builds` test now that real coverage exists
- `crates/histogram/tests/hist_parser.rs` - 12 tests against the real 10-minute capture and both
  `-h`/`-H` probes
- `crates/histogram/tests/percentiles.rs` - 7 tests reproducing the exact fixture-README contract
  values, including the corrected D-23 figures
- `crates/histogram/tests/json_parser.rs` - 5 tests against the real probe JSON/`.hist` pair

## Decisions Made

**Thread count derived from `# Min Latencies:`, correcting the plan's own stated rule.** Task 1's
action text asserts "`# Max Latencies:` ... always has exactly one value per thread." FINDINGS.md
"Column counts" (measured directly on the real `-h`/`-H` probe fixtures by plan 01-02) shows this
is false for `-H` captures: `# Max Latencies:` and `# Histogram Overflows:` both gain the `-H`
summary column (6 to 7 fields for a 6-thread run), while `# Min Latencies:` and `# Avg Latencies:`
stay thread-count-only in both layouts. Implemented against the real, measured rule rather than
the plan's assumption, per this plan's own explicit instruction to follow real rig data over a
guessed schema. `# Max Latencies:` and `# Histogram Overflows:` are truncated back to the thread
count wherever they carry the extra column, via a shared `drop_optional_summary_column` helper
also used by the data body's own column-count decision.

**`parse_hist_file`/`parse_json_file` added ahead of being strictly required by this plan's own
task acceptance criteria.** The plan's `<interfaces>` block states nr-cli (01-07) will call
`parse_hist_file(&Path) -> Result<CyclictestRun, HistError>` and
`parse_json_file(&Path) -> Result<CyclictestSummary, HistError>`, framed as "the public surface
right first time." Neither function is named in any task's `<action>` text or acceptance
criteria, but building them now (thin wrappers around `parse_hist`/`serde_json::from_str`, plus
an `HistError::Io` variant) avoids a known gap for plan 01-07 and costs a handful of lines.
`parse_hist_file` keeps `parse_hist`'s `declared_bound_us: Option<u64>` parameter rather than
matching the interfaces block's zero-argument signature literally, since a caller passing `None`
behaves identically to a hypothetical one-argument version while keeping the declared-bound path
available.

**BENCH-05 traceability.** This plan carries `BENCH-05` jointly with plans 01-01, 01-07, 01-10,
and 01-13. Per explicit instruction, `requirements mark-complete` was not run and
`.planning/REQUIREMENTS.md` was left untouched; the end-of-phase verifier owns marking `BENCH-05`
complete once all contributing plans have landed.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Corrected the plan's own column-count rule for deriving thread count**
- **Found during:** Task 1, reading `docs/rig/recon-2026-08-31/FINDINGS.md` section 6
  ("Column counts") per the task's own `<read_first>` instruction
- **Issue:** The plan's action text states thread count is authoritative from
  `# Max Latencies:`, "which always has exactly one value per thread." FINDINGS.md's real,
  measured data from plan 01-02's probes shows this line gains a `-H` summary column (6 to 7
  fields for a 6-thread run), same as `# Histogram Overflows:`; only `# Min Latencies:` and
  `# Avg Latencies:` are reliably thread-count-only in both layouts. Following the plan's literal
  rule would misparse a `-H` capture's `# Max Latencies:` line by one column.
- **Fix:** Derived `threads` from `# Min Latencies:` instead, validated `# Avg Latencies:`
  against the same count, and applied `drop_optional_summary_column` independently to
  `# Max Latencies:` and `# Histogram Overflows:` to strip their optional extra column.
- **Files modified:** `crates/histogram/src/hist.rs`
- **Verification:** `histofall_summary_column_not_double_counted` and all 12 `hist_parser.rs`
  tests pass against both the real `-h` and `-H` rig probes.
- **Committed in:** `0ef4311` (Task 1 commit)

**2. [Rule 3 - Blocking] Registered percentiles/json modules in lib.rs outside their tasks' declared file lists**
- **Found during:** Tasks 2 and 3, attempting to run `cargo test -p nr-histogram` against the
  newly written modules
- **Issue:** Task 2's `<files>` list is `percentiles.rs, tests/percentiles.rs`; Task 3's is
  `json.rs, tests/json_parser.rs`. Neither lists `lib.rs`. Without `pub mod percentiles;` /
  `pub mod json;` in `lib.rs`, neither module is part of the compiled crate at all, and the
  integration test binaries (separate compilation units linked against the library) cannot see
  `CyclictestRun`'s percentile methods or the JSON types -- the plan's own mandated test commands
  cannot pass without this change.
  Task 3 additionally needed `HistError::JsonDeserialize`/`HistError::SummaryDisagreement`
  (defined in `hist.rs`, not in Task 3's declared file list) since `HistError` is shared across
  both parsers by design (the `<interfaces>` block routes both `parse_hist_file` and
  `parse_json_file` through the same error type).
- **Fix:** Edited `lib.rs` in both tasks' commits to add the missing `pub mod` line, and edited
  `hist.rs` in Task 3's commit to add the two additional `HistError` variants Task 3's own action
  text requires (`reconcile()` returning `Result<(), HistError>`).
- **Files modified:** `crates/histogram/src/lib.rs` (Tasks 2 and 3), `crates/histogram/src/hist.rs`
  (Task 3)
- **Verification:** `cargo test -p nr-histogram` and `cargo test --workspace` both exit 0 after
  each task; `cargo clippy --workspace --all-targets -- -D warnings` clean.
- **Committed in:** `bc64684` (Task 2), `d259763` (Task 3)

**3. [Rule 1 - Bug] Removed a self-referential "percentile" substring from hist.rs's own doc comment**
- **Found during:** The mandatory acceptance-criteria sweep after all three tasks were
  implemented, re-running Task 1's own `grep -ci 'quantile\|percentile' crates/histogram/src/hist.rs`
  check
- **Issue:** `hist.rs`'s module doc comment read "...that lives in `crate::percentiles`", and the
  module name `percentiles` contains `percentile` as a case-insensitive substring, tripping the
  acceptance criterion that requires zero occurrences (this module parses only; it must not
  reference statistical computation even in prose).
- **Fix:** Reworded to describe the separation of concerns without naming the sibling module:
  "This module only parses cyclictest output into typed values. It performs no statistical
  computation of any kind; that is a sibling module's job."
- **Files modified:** `crates/histogram/src/hist.rs`
- **Verification:** `grep -ci 'quantile\|percentile' crates/histogram/src/hist.rs` reports 0; all
  43 workspace tests, fmt, and clippy remain green.
- **Committed in:** `0af1ec6` (standalone fix commit, not amended into `0ef4311` per the
  never-amend git policy)

---

**Total deviations:** 3 auto-fixed (1 bug correcting the plan's own stated rule against real rig
data, 1 blocking cross-file dependency the plan under-scoped, 1 bug caught by the plan's own
acceptance-criteria sweep).
**Impact on plan:** All three were necessary for the plan's own literal verification/acceptance
commands to pass against real fixture data. No scope creep: every change stayed inside
`crates/histogram/`, and the two `lib.rs`/`hist.rs` cross-task touches are the minimum needed for
each task's own mandated test commands to compile and run at all.

## Issues Encountered

None beyond the three auto-fixed deviations above. The `hdrhistogram` per-thread-record-then-merge
approach specified in Task 2's action text reproduced every expected percentile value exactly on
the first run (no bucket-rounding tolerance needed, despite the plan anticipating one might be).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `nr-histogram` is complete for this plan's scope: `.hist` parsing (both column layouts),
  overflow-inclusive percentiles backed by `hdrhistogram`, and `--json` parsing against the real
  rig schema, all cross-checked against real captures. `cargo fmt --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` (43 tests)
  are green on this host.
- Plan 01-06 (`nr-metrics`) can call `CyclictestRun::percentiles`, `samples_above`, `max_us`, and
  `to_bin_table` directly, exactly as the `<interfaces>` block specified.
- Plan 01-07 (`nr-cli`) can call `parse_hist_file`/`parse_json_file` directly; both exist now,
  ahead of being needed, per this plan's `<interfaces>` contract.
- The D-23 published error (over-gate count computed from bins alone, dropping 888 overflow
  samples) is now fixed in code and locked down by `overflow_counted`, ready for plan 01-10 to
  cite when it corrects the published README figure (2,089 of 17,994,956, 0.0116%).
- `BENCH-05` remains open at the requirements level (shared with 01-01, 01-07, 01-10, 01-13); no
  action needed here, flagged for the end-of-phase verifier.
- No blockers.

---
*Phase: 01-trustworthy-measurement*
*Completed: 2026-08-31*

## Self-Check: PASSED

All 7 key files (6 created, 1 modified) confirmed present on disk with `[ -f ]`. All 4 commits
(`0ef4311`, `bc64684`, `d259763`, `0af1ec6`) confirmed present in `git log --oneline --all`.
`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and
`cargo test --workspace` (43 tests) all re-ran clean immediately before this SUMMARY was written.
The plan's own `<verification>` block (fmt, clippy, `cargo test -p nr-histogram`,
`overflow_counted -- --nocapture`, `json_and_hist_maxima_agree`) and the reviewer-runnable `awk`
hand check (`binned: 17994956`, `overflows: 888`) were re-run and match exactly.
