---
status: PASS
agent: donny-executor
phase: 01-trustworthy-measurement
plan: 20
subsystem: rig-instrumentation
tags: [rtla, hwnoise, msr-smi-count, manifest, schema, firmware, rust, parser]

# Dependency graph
requires:
  - phase: 01-trustworthy-measurement
    provides: "01-22's real 60s rtla hwnoise probe and rdmsr 0x34 capture
      (docs/rig/recon-2026-09-05/probe-*.txt), described exactly in
      docs/rig/recon-2026-09-05/FINDINGS.md, which this plan's parser and
      manifest fields are written against"
provides:
  - "crates/capture/src/hwnoise.rs: a header-name-validated rtla hwnoise
    parser, tested against a fixture byte-identical to the real probe,
    returning per-CPU rows plus explicit observed/missing CPU sets"
  - "crates/capture/src/firmware.rs: hwlatdetect event parsing, event_cpus,
    and uncovered_cpus, the standing coverage-assertion primitive"
  - "crates/capture/tests/firmware_cpu_coverage.rs: the standing check,
    applied to all eight already-committed firmware captures, asserting
    none of them names an isolated core (6-11)"
  - "nr_manifest::FirmwareScreen/CpuExposure/SmiCounts: optional, defaulted
    RunManifest fields that can record a firmware instrument's exact
    invocation, requested vs observed CPUs, per-CPU exposure, MSR_SMI_COUNT
    before/after/delta, and an explicit unavailable_reason - with no field
    the instruments cannot honestly fill"
  - "ArtifactKind::RtlaHwnoise and two new CAPTURE_GLOBS entries
    (hwnoise*.txt, rtla-hwnoise*.txt) so a stray rtla hwnoise capture fails
    the D-13 stray-capture scan the same way a .hist does"
affects: [01-21, 01-23]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Header-name validation before column lookup: read the header line,
      compare its whole token sequence against a known-good constant, and
      only then build a name-to-index map for the data rows. An
      unrecognised header (added, removed, renamed or reordered column)
      is HwnoiseError::UnexpectedHeader carrying the header verbatim,
      never a positional guess. Mirrors crates/capture/src/interference.rs
      and directly avoids the defect plan 01-04 found in a different
      hand-rolled parser."
    - "Multi-word column names in a header validated as a flattened token
      sequence (EXPECTED_COLUMNS.iter().flat_map(str::split_whitespace))
      rather than split naively and zipped 1:1 with a data row's own
      single-word values, since '% CPU Aval' is one column but three
      whitespace-separated header words."
    - "A repeating-redraw capture format parsed by keeping the LAST row
      seen per CPU (BTreeMap<u32, Row>, overwritten on each re-occurrence
      of a valid header), so the same parser handles both the full
      585-line redraw this project's only real probe captured and a
      future single quiet-mode (-q) summary block without a format
      switch."
    - "A schema-generated manifest field is added to fields.rs, the
      generated schema and the committed fixture in the same commit
      (never routed around), per the 01-05 lesson this plan calls out by
      name in its own objective."

key-files:
  created:
    - crates/capture/src/hwnoise.rs
    - crates/capture/src/firmware.rs
    - crates/capture/tests/hwnoise.rs
    - crates/capture/tests/firmware_cpu_coverage.rs
    - crates/capture/tests/fixtures/rtla-hwnoise-probe.txt
  modified:
    - crates/manifest/src/fields.rs
    - crates/cli/src/cmd/run.rs
    - crates/cli/src/cmd/reconstruct.rs
    - crates/cli/src/cmd/verify.rs
    - crates/capture/src/lib.rs
    - schemas/manifest.schema.json
    - schemas/attempt.schema.json
    - crates/metrics/tests/snapshots/report__headline_report.snap

key-decisions:
  - "requirements-completed left empty for both PLAT-03 and BENCH-04, the
    plan's own frontmatter requirements. PLAT-03 needs a published,
    attributed latency figure or residual, and this plan takes no capture
    at all (its own objective says so); BENCH-04 was already Complete in
    REQUIREMENTS.md from an earlier plan and this plan's field-only work
    does not itself constitute publishing a new figure. Matches the
    PLAT-02 precedent from 01-09/01-18/01-22."
  - "Fixed two RunManifest struct-literal construction sites (run.rs,
    reconstruct.rs) outside the plan's own file list, as Rule 3 (blocking):
    adding two non-Option, non-defaulted-at-construction fields to a
    #[deny_unknown_fields] struct does not compile without them. Both use
    the honest default (no firmware screen, no SMI read) since neither
    code path drives either instrument yet - that is plan 01-21's job."
  - "Regenerated schemas/attempt.schema.json alongside manifest.schema.json,
    also outside the plan's named file list (Rule 3, blocking): AttemptRecord
    transitively references ArtifactKind via preserved: Vec<ArtifactRecord>,
    so the new RtlaHwnoise variant changed its generated schema too."
  - "Re-pinned crates/metrics/tests/snapshots/report__headline_report.snap
    (Rule 1): the snapshot's manifest blake3 line changed because the two
    new fields serialize into the hashed manifest bytes even when empty
    (firmware_screens has no skip_serializing_if, matching the plan's own
    action text). The shift is mechanical, not a logic change; verified by
    reading crates/metrics/src/report.rs's manifest_blake3 function before
    re-pinning."
  - "column_positions validates the header as one exact token sequence
    (not a set), so a hypothetical reordered-but-same-columns header would
    also be rejected as UnexpectedHeader rather than silently accepted.
    Chosen because exactly one real header format has ever been observed
    and the plan's own rule is that an unfamiliar header is an error, not
    a default; a looser reorder-tolerant parser was not asked for and
    would be unverifiable against any real capture."

requirements-completed: []

# Metrics
duration: 22min
completed: 2026-09-05
---

# Phase 1 Plan 20: Firmware parsers and manifest fields for rtla hwnoise and MSR_SMI_COUNT Summary

**A header-name-validated `rtla hwnoise` parser tested against the real 60s probe, `FirmwareScreen`/`SmiCounts` manifest fields no instrument-incapable value can fill, and a standing test that asserts all eight committed firmware captures' event CPUs, proving in code that none of them names an isolated core.**

## Performance

- **Duration:** ~22 min (started reading context immediately after the 01-22 session ended at 2026-09-05T23:05:50Z; final commit at 2026-09-05T23:26:03Z)
- **Tasks:** 3 of 3
- **Files modified:** 13 (8 in task 1, 4 in task 2, 3 in task 3, with `crates/capture/src/lib.rs` touched by both tasks 2 and 3)

## Accomplishments

- `crates/capture/src/hwnoise.rs` parses real `rtla hwnoise` output into per-CPU rows,
  looking up every column by a validated header name rather than a hardcoded position. An
  unrecognised header (a column added, removed, renamed or reordered) fails loudly with
  `HwnoiseError::UnexpectedHeader` carrying the header text verbatim.
- The parser is tested against `crates/capture/tests/fixtures/rtla-hwnoise-probe.txt`, a
  byte-identical copy of the real 60-second capture plan 01-22 committed, including its
  once-per-second repeating redraw and the embedded `rtla hwnoise --help` text; the parser
  correctly skips both and keeps the last (final, cumulative) row per CPU.
- `crates/manifest/src/fields.rs` gains `FirmwareScreen` (instrument, full argv, requested
  vs. observed CPUs, per-CPU exposure, a maximum and the population it is a maximum of) and
  `SmiCounts` (`MSR_SMI_COUNT` before/after/delta per CPU, with an explicit
  `unavailable_reason` rather than a substituted zero). Both are optional/defaulted on
  `RunManifest`, so all eight already-committed manifests and the minimal-manifest fixture
  keep validating unchanged.
- `schemas/manifest.schema.json` and `schemas/attempt.schema.json` were regenerated together
  with the Rust types (never hand-edited), following the 01-05 lesson this plan's own
  objective names explicitly: a missing manifest field routed around instead of added
  through the schema produces a wrapper type the manifest never actually carries.
- `crates/capture/src/firmware.rs` and `crates/capture/tests/firmware_cpu_coverage.rs` parse
  every committed `hwlatdetect*.txt` capture's events and assert, as a passing test rather
  than a paragraph in a document, that not one of the eight names a CPU between 6 and 11 -
  the exact fact three D-18 arms and the 2026-08-28 baseline needed and did not have.
- `ArtifactKind::RtlaHwnoise` plus two new `CAPTURE_GLOBS` entries mean a stray `rtla hwnoise`
  capture committed outside a run directory now fails `nrmeasure verify`'s stray-capture scan
  exactly as a `.hist` file would.
- `./target/release/nrmeasure verify --strict --check-index` still reports 0 problems across
  all 8 run directories (7 re-derived, 1 not re-derivable), and `git diff --stat measurements/`
  stayed empty throughout: this plan touched no published evidence.

## Task Commits

1. **Task 1: The firmware manifest fields, schema and fixture together** - `cd49f45` (feat)
2. **Task 2: The rtla hwnoise parser, written against the committed probe** - `515be12` (feat)
3. **Task 3: The standing check, applied to every firmware capture already committed** - `a5ab6cc` (test)

**Plan metadata:** (this commit, made immediately after this SUMMARY) docs(01-20): complete plan

## Files Created/Modified

- `crates/manifest/src/fields.rs` - `FirmwareScreen`, `CpuExposure`, `SmiCounts` structs;
  `firmware_screens`/`smi_counts` fields on `RunManifest`; `ArtifactKind::RtlaHwnoise`; three
  new tests (`firmware_screen_roundtrips`, `smi_counts_roundtrip`,
  `committed_manifests_parse_without_firmware_fields`)
- `crates/cli/src/cmd/verify.rs` - `CAPTURE_GLOBS` gains `hwnoise*.txt`/`rtla-hwnoise*.txt`;
  existing coverage test extended with both names
- `crates/cli/src/cmd/run.rs`, `crates/cli/src/cmd/reconstruct.rs` - the two real
  `RunManifest` struct-literal sites updated with the two new fields' honest defaults (empty
  / absent; neither path drives a firmware instrument yet)
- `schemas/manifest.schema.json`, `schemas/attempt.schema.json` - regenerated via
  `UPDATE_SCHEMAS=1 cargo test -p nr-manifest`
- `crates/metrics/tests/snapshots/report__headline_report.snap` - re-pinned; the fixture
  manifest's blake3 shifted because the new fields serialize into the hashed bytes
- `crates/capture/src/hwnoise.rs` - the `rtla hwnoise` parser: `HwnoiseRow`, `HwnoiseRun`,
  `HwnoiseError`, `parse_hwnoise`, `parse_hwnoise_file`
- `crates/capture/tests/hwnoise.rs` - the four named behavior tests
- `crates/capture/tests/fixtures/rtla-hwnoise-probe.txt` - byte-identical copy of
  `docs/rig/recon-2026-09-05/probe-rtla-hwnoise.txt`, re-checked for host identifiers after
  copying (T-1-82)
- `crates/capture/src/firmware.rs` - `HwlatEvent`, `parse_hwlatdetect_events`, `event_cpus`,
  `uncovered_cpus`, plus inline unit tests for each (mirroring
  `crates/capture/src/interference.rs`'s own inline-plus-integration test split)
- `crates/capture/tests/firmware_cpu_coverage.rs` - the four named behavior tests, reading
  all eight committed `hwlatdetect*.txt` captures by path
- `crates/capture/src/lib.rs` - `pub mod firmware;` and `pub mod hwnoise;` registered
  (alphabetical: environment, firmware, hwnoise, interference, preconditions, sources)

## Decisions Made

See `key-decisions` in the frontmatter for the full list with rationale. In brief: neither
`PLAT-03` nor `BENCH-04` is recorded as completed by this plan (no capture is taken here);
two struct-literal call sites and the sibling `attempt.schema.json` needed updating outside
the plan's own file list to compile at all; one snapshot was re-pinned for a purely
mechanical reason; and the header validator is deliberately exact-sequence rather than
reorder-tolerant, since only one real header format has ever been observed.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Two `RunManifest` struct-literal sites needed the new fields**
- **Found during:** Task 1, immediately after adding `firmware_screens`/`smi_counts` to
  `RunManifest`
- **Issue:** `crates/cli/src/cmd/run.rs` and `crates/cli/src/cmd/reconstruct.rs` each
  construct a `RunManifest { ... }` literal exhaustively. `#[serde(default)]` only affects
  deserialization; a Rust struct literal must still name every field, so the workspace did
  not compile after task 1's own struct changes alone.
- **Fix:** Added `firmware_screens: Vec::new()` and `smi_counts: None` to both literals, each
  with a one-line comment stating why: neither `nrmeasure run` nor `nrmeasure reconstruct`
  drives a firmware instrument yet (that is plan 01-21's job for the live path; a
  reconstructed run never declared one to begin with).
- **Files modified:** `crates/cli/src/cmd/run.rs`, `crates/cli/src/cmd/reconstruct.rs`
- **Verification:** `cargo build --workspace` and `cargo test --workspace` both pass;
  `cargo clippy --workspace --all-targets -- -D warnings` is clean.
- **Committed in:** `cd49f45` (task 1's own commit; this is plumbing for that task's own
  struct change, not a separate concern)

**2. [Rule 3 - Blocking] `schemas/attempt.schema.json` also went stale**
- **Found during:** Task 1, running `cargo test -p nr-manifest` after adding
  `ArtifactKind::RtlaHwnoise`
- **Issue:** `attempt_schema_up_to_date` failed. `AttemptRecord` (a separate schema root)
  carries `preserved: Vec<ArtifactRecord>`, and `ArtifactRecord.kind: ArtifactKind`, so the
  new `RtlaHwnoise` variant changed `AttemptRecord`'s generated schema too, even though
  nothing about attempts themselves changed.
- **Fix:** Ran `UPDATE_SCHEMAS=1 cargo test -p nr-manifest` (no test-name filter, so both
  `schema_up_to_date` and `attempt_schema_up_to_date` regenerate together) rather than the
  plan's own literal `schema_up_to_date`-only command.
- **Files modified:** `schemas/attempt.schema.json`
- **Verification:** `cargo test -p nr-manifest` passes with `UPDATE_SCHEMAS` unset; the diff
  is a single line (the new enum value in `ArtifactKind`'s schema definition).
- **Committed in:** `cd49f45`

**3. [Rule 1 - Bug/staleness] Re-pinned the `headline_report` snapshot**
- **Found during:** Task 1, `cargo test --workspace`
- **Issue:** `crates/metrics/tests/report.rs::headline_report_snapshot` failed: the fixture
  manifest's published `manifest blake3:` line changed, because `manifest_blake3()`
  (`crates/metrics/src/report.rs`) hashes the manifest's serialized JSON bytes, and the two
  new fields (`firmware_screens: []` in particular, which serializes even when empty) changed
  those bytes.
- **Fix:** Confirmed the cause by reading `manifest_blake3()` first, then regenerated with
  `INSTA_UPDATE=always cargo test -p nr-metrics --test report` and removed the resulting
  `.snap.new` file so only the clean `.snap` changed.
- **Files modified:** `crates/metrics/tests/snapshots/report__headline_report.snap`
- **Verification:** `cargo test -p nr-metrics --test report` passes; `git status` shows no
  stray `.snap.new` file.
- **Committed in:** `cd49f45`

**4. [Rule 1 - Lint] `clippy::excessive_precision` on a test literal**
- **Found during:** Task 3, `cargo clippy --workspace --all-targets -- -D warnings`
- **Issue:** `crates/capture/src/firmware.rs`'s own unit test compared against the literal
  `1787951679.284904730`, more significant digits than `f64` can represent exactly.
- **Fix:** Applied clippy's own suggested replacement, `1_787_951_679.284_904_7`.
- **Files modified:** `crates/capture/src/firmware.rs`
- **Verification:** `cargo clippy --workspace --all-targets -- -D warnings` is clean.
- **Committed in:** `a5ab6cc`

**Total deviations:** 4 auto-fixed (2 Rule 3 blocking-compile fixes, 1 Rule 1 mechanical
snapshot re-pin, 1 Rule 1 lint fix). **Impact on plan:** all four were necessary for the
plan's own three tasks to actually build, test and pass clippy; none expands scope beyond
wiring the new fields' honest defaults and keeping generated artifacts in sync with the Rust
types that generate them.

## Known Stubs

`crates/cli/src/cmd/run.rs` and `crates/cli/src/cmd/reconstruct.rs` now construct every
`RunManifest` with `firmware_screens: Vec::new()` and `smi_counts: None`. These are honest
empty defaults, not placeholders papering over missing functionality: this plan's own
objective states plainly that "no captures are taken here," and neither `nrmeasure run` nor
`nrmeasure reconstruct` drives `rtla hwnoise` or `rdmsr` today. Wiring either instrument into
the live harness (populating these fields for real) is explicitly plan 01-21's job, per
`docs/rig/recon-2026-09-05/FINDINGS.md`'s own "What this changes for the firmware screen"
section. This does not block this plan's goal, which is the parser and the manifest fields
existing and being honestly fillable - not yet filled.

## Issues Encountered

None beyond the four items in Deviations from Plan, all resolved inline.

## User Setup Required

None. This plan runs entirely on the macOS dev host against a committed fixture; no rig
access, no external services, no new dependencies.

## Next Phase Readiness

- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and
  `cargo test --workspace` all pass at the end of this plan.
- `./target/release/nrmeasure verify --strict --check-index` reports 0 problems across all 8
  run directories; `git diff --stat measurements/` is empty.
- Plan 01-21 (wiring both instruments into the live harness) now has a tested parser
  (`nr_capture::hwnoise::parse_hwnoise`/`parse_hwnoise_file`) and manifest fields
  (`FirmwareScreen`, `SmiCounts`) ready to populate; `FINDINGS.md`'s recommended argv
  (`rtla hwnoise -c 6-11 -H 0-5 -P f:99 -d <duration>` and `rdmsr -p <cpu> 0x34`
  before/after) is unchanged by this plan.
- Plan 01-23 (re-taking the D-18 arms) has a standing regression test
  (`no_committed_capture_covers_the_isolated_cores`) that will need deliberate updating,
  not deletion, once a new capture actually names an isolated core - the test's own comment
  says so.
- `deferred-items.md`'s "the instrument that was available all along" section still names
  01-20 as the `rtla hwnoise` parser owner; that entry should be closed in this plan's final
  metadata commit, alongside `STATE.md`.
- No blockers for wave 13 (01-21).

## Self-Check: PASSED

Verified directly:
- `[ -f crates/capture/src/hwnoise.rs ]`, `[ -f crates/capture/src/firmware.rs ]`,
  `[ -f crates/capture/tests/hwnoise.rs ]`,
  `[ -f crates/capture/tests/firmware_cpu_coverage.rs ]`,
  `[ -f crates/capture/tests/fixtures/rtla-hwnoise-probe.txt ]` - all FOUND.
- `diff crates/capture/tests/fixtures/rtla-hwnoise-probe.txt docs/rig/recon-2026-09-05/probe-rtla-hwnoise.txt`
  - empty (byte-identical), confirmed both before and after `cargo fmt`.
- `grep -q 'pub struct FirmwareScreen' crates/manifest/src/fields.rs`,
  `grep -q 'pub struct SmiCounts' crates/manifest/src/fields.rs`,
  `grep -q 'RtlaHwnoise' crates/manifest/src/fields.rs`,
  `grep -q 'hwnoise\*' crates/cli/src/cmd/verify.rs`,
  `grep -q 'pub mod hwnoise' crates/capture/src/lib.rs`,
  `grep -q 'pub mod firmware' crates/capture/src/lib.rs` - all FOUND.
- `git log --oneline --all | grep -q cd49f45` / `515be12` / `a5ab6cc` - all FOUND.
- `cargo test --workspace`, `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` - all pass at HEAD (`a5ab6cc`).
- `./target/release/nrmeasure verify --strict --check-index` - exit 0, 0 problems.

*Phase: 01-trustworthy-measurement*
*Completed: 2026-09-05*
