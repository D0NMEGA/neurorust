---
status: PASS
agent: donny-executor
phase: 01-trustworthy-measurement
plan: 26

subsystem: provenance
tags: [rust, verification, blake3, rtla-hwnoise, serde, rescheduling-ipis, report-rendering]

# Dependency graph
requires:
  - phase: 01-trustworthy-measurement
    provides: "nr_capture::hwnoise::parse_hwnoise_file and HwnoiseRun/HwnoiseRow (plan
      01-20), check_derived_figures's DerivedFiguresReport shape and re-derivation
      pattern (plan 01-19), InterferenceSnapshot/InterferenceDelta and the RES-as-
      context_switches substitution (plan 01-05), the Series admission section and
      contamination-verdict sentence render_contamination gained (plan 01-24), the
      tail-metrics block render_contamination gained (mid-01-11)"
provides:
  - "check_firmware_figures: verify --strict re-derives every firmware screen's
    manifest.json and REPORT.md figures (observed_cpus, max_us, events_recorded,
    per-cpu exposure) from rtla-hwnoise.txt via the existing hwnoise parser, closing
    C1"
  - "the B5 coverage rule: a requested CPU that produced no row is a strict problem
    worded as a coverage failure, never a claim about whether it was sampled"
  - "InterferenceSnapshot/InterferenceDelta.rescheduling_ipis, renamed from
    context_switches with #[serde(alias = \"context_switches\")], closing B4"
  - "the res ipis published counter label and its explanatory line in REPORT.md"
  - "the corrected rtla-hwnoise coverage caveat text, citing the committed cpu 7
    counterexample by path"
  - "nrmeasure verify --rewrite-reports: regenerates a run's REPORT.md from its own
    manifest and raw capture, writing only when the bytes differ, refusing to combine
    with --strict"
  - "eleven re-published REPORT.md files carrying the corrected rendering"
affects: [01-27, any future plan reading InterferenceSnapshot/InterferenceDelta, a
  firmware screen's manifest or report figures, or render_run_report's output shape]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "A field rename lands as #[serde(rename = \"new\", alias = \"old\")] rather than
      a schema-version bump, when the schema pins its own version number and every
      already-committed document would fail a blocking gate under a bump"
    - "A rewrite command (--rewrite-reports) as the correction mechanism for a
      generated artifact, rather than a hand edit, extending the nrmeasure attempt
      precedent from plan 01-23 to a second class of derived evidence"
    - "A scripted numeric-survival check over git diff -U0, run before staging a bulk
      rewrite of published evidence, that fails on any digit lost between a removed
      and an added line rather than trusting an eyeballed diff across eleven files"

key-files:
  created: []
  modified:
    - crates/cli/src/cmd/verify.rs
    - crates/cli/tests/verify.rs
    - crates/manifest/src/fields.rs
    - crates/manifest/tests/fixtures/minimal-manifest.json
    - schemas/manifest.schema.json
    - crates/capture/src/interference.rs
    - crates/capture/tests/interference.rs
    - crates/capture/tests/published_figures.rs
    - crates/cli/src/cmd/run.rs
    - crates/cli/src/cmd/reconstruct.rs
    - crates/metrics/src/report.rs
    - crates/metrics/tests/report.rs
    - crates/metrics/tests/snapshots/report__headline_report.snap
    - crates/cli/tests/snapshots/run_pipeline__full_run_report_matches_snapshot.snap
    - docs/measurement-protocol.md
    - README.md
    - measurements/2026-09-01-precision3591-calibration-clean/REPORT.md
    - measurements/2026-09-02-precision3591-calibration-contaminated/REPORT.md
    - measurements/2026-09-03-precision3591-calibration-clean/REPORT.md
    - measurements/2026-09-03-precision3591-calibration-contaminated/REPORT.md
    - measurements/2026-09-05-precision3591-screen/REPORT.md
    - measurements/2026-09-05-precision3591-screen-02/REPORT.md
    - measurements/2026-09-05-precision3591-screen-03/REPORT.md
    - measurements/2026-09-06-precision3591-screen-02/REPORT.md
    - measurements/2026-09-06-precision3591-screen-03/REPORT.md
    - measurements/2026-09-06-precision3591-screen-04/REPORT.md
    - measurements/2026-09-07-precision3591-screen/REPORT.md
    - .planning/phases/01-trustworthy-measurement/deferred-items.md

key-decisions:
  - "Task 2's own acceptance criterion 'grep -c context_switches schemas/manifest.schema.json reports 0' cannot be satisfied while also following the task's own mandated field doc comment verbatim: schemars propagates a field's Rust doc comment into the JSON schema's description string, and the plan's own specified comment text explains 'it was called context_switches until 2026-09-07'. Verified precisely: the JSON-key-quoted form \"context_switches\" (what a live schema property or a required-array entry would look like) has zero occurrences; the only two matches are backtick-quoted prose inside a description field. Kept the mandated doc comment (valuable, explicit, explains the rename) and treated the literal grep as unsatisfiable in combination with it, rather than weakening documentation to make a blunt substring check pass."
  - "The plan's own top-level <verification> hand-edit test snippet (cp -R a run directly into $d, then --root \"$d\" with --measurements defaulting to $d/measurements) has an authoring bug: the copied run never lands under a measurements/ subdirectory, so check_run_directories finds zero run directories and the reported problems are 'capture-shaped file outside measurements/' rather than the intended tampered-maximum disagreement. Confirmed by rerunning with mkdir -p \"$d/measurements\" before the copy: verify --strict then correctly reports 'published firmware maximum disagrees: published 999 us, re-derived 1 us' and exits non-zero, matching task 1's own intent. The six Rust integration tests in crates/cli/tests/verify.rs (which use the correct measurements/<run>/ layout throughout) are the tests of record for this behavior."
  - "The numeric-survival check (git diff -U0 measurements/ piped through the plan's own Python script) does report lost digits after the real rewrite, entirely from the 'manifest blake3:' header line: render_run_report's manifest_blake3() rehashes serde_json::to_vec(&manifest) at render time, and task 2's rescheduling_ipis rename means that reserialisation now always emits the new key regardless of which key the committed (untouched) manifest.json carries on disk, so the recomputed hash necessarily differs from what an older build published. Not a measurement changing: verified by rerunning the same check with '^[+-]manifest blake3: ' lines excluded, which passes clean ('no published number changed or dropped'), and independently by verify --strict --check-index reporting 0 problems against the rewritten tree (it re-derives every actual published figure, and does not touch the blake3 line at all)."
  - "Every one of the 122 added / 37 blank-line lines across the eleven rewritten reports was categorised by a small script before staging anything, leaving zero unaccounted-for lines: the res ipis header (this plan's own task 2), the Series admission section and a one-sentence contamination-verdict addition (plan 01-24), and a tail-metrics block that only the two oldest reports predate (a mid-01-11 commit, bc1993a, identified by git log -S and its position in 01-11's own commit sequence)."
  - "requirements-completed left empty. BENCH-04 and BENCH-05 were already marked Complete by earlier plans and this plan's changes don't newly complete either; PLAT-03 needs a real headline capture under the gate or a named-cause attribution of the residual, and this plan explicitly takes no captures (macOS dev host only, per its own objective)."

# Metrics
duration: ~40min (commits span 03:32:04 to 03:49:42 -0500)
completed: 2026-09-07
---

# Phase 01 Plan 26: The firmware figure joins the strict gate, and two published claims become true Summary

**`verify --strict` now re-derives every firmware screen's manifest and REPORT.md figures from `rtla-hwnoise.txt` and fails a missing CPU as an unconfirmed-coverage problem rather than explaining it away; the published counter column and its manifest field are renamed from `context switches` to `res ipis`/`rescheduling_ipis` (alias-preserving every committed manifest); and a new `verify --rewrite-reports` command re-published all eleven affected `REPORT.md` files with the corrected rendering, verified line by line rather than by eye.**

## Performance

- **Duration:** ~40 min (estimated; the three task commits span 03:32:04 to 03:49:42 -0500)
- **Started:** approx. 2026-09-07T08:16:00Z (immediately following 01-25's completion)
- **Completed:** 2026-09-07T08:49:42Z
- **Tasks:** 3
- **Files modified:** 27 (16 code/doc/test files, 11 rewritten `REPORT.md` files)

## Accomplishments

- `check_firmware_figures`, a sibling of plan 01-19's `check_derived_figures`, re-parses each harness-generated run's `rtla-hwnoise.txt` with the existing `nr_capture::hwnoise::parse_hwnoise_file` (no second parser written) and compares the re-derived `observed_cpus`, `max_us`, `events_recorded` and per-CPU exposure against both `manifest.json`'s `firmware_screens` and `REPORT.md`'s `## Firmware screen` block, pushing one problem per disagreement. Against the real tree: `4 firmware re-derived, 0 firmware not re-derivable, 0 problems`. A screen from an instrument this project has no parser for yet is recorded not re-derivable by name.
- B5: any CPU in `requested_cpus` that produced no row in the re-derived parse is a strict problem reading `cpu N was requested but produced no row in rtla-hwnoise.txt; coverage cannot be confirmed for it`, never a claim that the CPU was or was not sampled.
- `InterferenceSnapshot`/`InterferenceDelta`'s `context_switches` field is renamed to `rescheduling_ipis` with `#[serde(rename = "rescheduling_ipis", alias = "context_switches")]`, so all twelve manifests committed under the old key still deserialise and validate unchanged, with no schema-version bump. The published counter table header now reads `| cpu | cal ipis | tlb ipis | res ipis | irqs |`, with a line underneath naming what RES is and why no true per-CPU context-switch count exists. `config/contamination-thresholds.json`'s `context_switch_delta_max` is untouched, with its doc comment now cross-referencing the renamed field so the two can't be confused later.
- The `rtla-hwnoise` coverage caveat no longer states that a CPU absent from the observed list "was sampled and reported nothing"; it states that a sampled CPU always emits its own row, cites cpu 7's committed row in `2026-09-06-precision3591-screen-03/rtla-hwnoise.txt` (full exposure, zero events, an explicit row) as the proof, and says a missing CPU produced no row at all with coverage unconfirmed.
- `nrmeasure verify --rewrite-reports` regenerates a run's `REPORT.md` from its own manifest and raw capture, writing only when the rendered bytes differ, skipping a run it cannot re-derive (reconstructed, no recorded `--histogram` bound, no `CyclictestHist` artifact) with the reason printed, and refusing to combine with `--strict`. Run against the real tree: 11 written, 1 skipped (`2026-08-28-precision3591`, reconstructed), 0 errors; a second pass writes 0 and produces a byte-for-byte identical diff to the first.
- Every one of the 27 changed hunks across the eleven rewritten reports was categorised before staging (see Deviations below); `verify --strict --check-index` reports 0 problems against the rewritten tree and `git diff --stat measurements/` lists only the eleven `REPORT.md` paths.

## Task Commits

Each task was committed atomically:

1. **Task 1: Strict verification re-derives every firmware figure, and absence fails coverage** - `c4b2b4c` (feat)
2. **Task 2: The published quantity is named after what it holds, and the coverage caveat is true** - `bf850e7` (fix)
3. **Task 3: A command that re-publishes a corrected rendering, applied to the eleven committed reports** - `472fe44` (feat)

_Note: none of this plan's tasks were marked `tdd="true"` in a way requiring a separate RED commit; tasks 1 and 2 were TDD-designated but, following plan 01-19's precedent for this shape of change, each was committed as a single, fully-verified unit (a missing function is itself a valid RED state for a compiled language)._

## Files Created/Modified

- `crates/cli/src/cmd/verify.rs` - `check_firmware_figures` and its helpers (`compare_firmware_manifest`, `compare_firmware_report`, `parse_report_firmware`, `firmware_screen_section`, `take_firmware_block`, `format_firmware_max`, `parse_cpu_list`); the `--rewrite-reports` flag, `rewrite_reports`, `run_rewrite_reports`, `RewriteReport`; the mutual-exclusion check; module doc updated
- `crates/cli/tests/verify.rs` - 6 new firmware-re-derivation tests, 4 new `--rewrite-reports` tests
- `crates/manifest/src/fields.rs` - `context_switches` renamed to `rescheduling_ipis` (alias-preserving) on `InterferenceSnapshot` and `InterferenceDelta`, with the full rationale doc comment
- `crates/manifest/tests/fixtures/minimal-manifest.json` - test fixture updated to the new key (not published evidence)
- `schemas/manifest.schema.json` - regenerated via `UPDATE_SCHEMAS=1 cargo test -p nr-manifest schema_up_to_date`
- `crates/capture/src/interference.rs` - `build_snapshot`/`compute_delta`/`counter_breach` updated to the new field name; `CalibratedThresholds::context_switch_delta_max`'s doc comment cross-references the rename without renaming the field itself
- `crates/capture/tests/interference.rs` - 2 new serde round-trip tests (old key still deserialises, new value serialises the new key)
- `crates/capture/tests/published_figures.rs`, `crates/cli/src/cmd/run.rs`, `crates/cli/src/cmd/reconstruct.rs` - read/construction sites updated to the new field name
- `crates/metrics/src/report.rs` - counter table header/separator and explanatory line; the corrected `rtla-hwnoise` coverage caveat
- `crates/metrics/tests/report.rs` - 2 new tests (`the_report_names_res_not_context_switches`, `the_coverage_caveat_does_not_claim_sampling`); fixture JSON updated to the new key
- `crates/metrics/tests/snapshots/report__headline_report.snap`, `crates/cli/tests/snapshots/run_pipeline__full_run_report_matches_snapshot.snap` - re-pinned; diffs contain only the header/explanatory-line change (and, for the metrics snapshot, the manifest blake3 line the fixture's own key rename moves)
- `docs/measurement-protocol.md`, `README.md` - `--rewrite-reports` documented: what it regenerates, what it never touches
- Eleven `measurements/*/REPORT.md` files - re-rendered by `--rewrite-reports`; see Deviations for the full hunk taxonomy
- `.planning/phases/01-trustworthy-measurement/deferred-items.md` - findings B4, C1, B5 marked CLOSED with commit hashes

## Decisions Made

See `key-decisions` in the frontmatter for full text. In short: task 2's literal schema grep criterion conflicts with its own mandated doc-comment text (schemars puts doc comments into schema descriptions), resolved by verifying the schema's actual JSON-key surface is clean and keeping the documentation; the plan's own hand-edit verification snippet has a directory-layout bug, resolved by re-running it correctly and confirming the intended behavior (the six Rust tests are the tests of record); the numeric-survival check's non-zero exit traces entirely to the self-referential manifest-blake3 fingerprint recomputing under the new field name, confirmed by an exclusion rerun and independently by the strict gate; and every rewritten hunk was categorised and traced to a specific plan (or this plan) before anything was staged.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] rustfmt's own line wrap broke a literal acceptance-criterion grep**
- **Found during:** Task 1, running `grep -q 'coverage cannot be confirmed' crates/cli/src/cmd/verify.rs` after `cargo fmt`
- **Issue:** The B5 problem message's string literal was 90 characters of content at 20-space indent (112 columns unwrapped), so rustfmt inserted a `\` continuation splitting the phrase "coverage cannot" from "be confirmed" across two source lines, exactly where the acceptance grep expected one contiguous line.
- **Fix:** Rewrapped the string manually so both resulting lines fit under 100 columns without needing rustfmt to choose the split point, moving the break to before "coverage" instead of inside the phrase. Idempotent under `cargo fmt --check`.
- **Files modified:** `crates/cli/src/cmd/verify.rs`
- **Verification:** `cargo fmt --check`; `grep -q 'coverage cannot be confirmed' crates/cli/src/cmd/verify.rs` exits 0
- **Committed in:** `c4b2b4c` (task 1's own commit)

**2. [Rule 1 - Bug] clippy's `trim_split_whitespace` lint on a new test helper**
- **Found during:** Task 1, `cargo clippy --workspace --all-targets -- -D warnings`
- **Issue:** `a_requested_cpu_with_no_row_fails_coverage`'s row filter wrote `line.trim().split_whitespace()`; `split_whitespace()` already ignores leading/trailing whitespace, so the `.trim()` was redundant and clippy denies it under this workspace's `-D warnings`.
- **Fix:** Removed the redundant `.trim()` call. No behavior change (the fixture's rows have no meaningful content difference with or without the leading trim).
- **Files modified:** `crates/cli/tests/verify.rs`
- **Verification:** `cargo clippy --workspace --all-targets -- -D warnings` clean; the test still passes
- **Committed in:** `c4b2b4c` (task 1's own commit)

**3. [Rule 1 - Bug] Re-pinned two snapshots for the header/caveat rendering change**
- **Found during:** Task 2, `cargo test -p nr-metrics --test report` and `cargo test -p nr-cli --test run_pipeline`
- **Issue:** Both committed snapshots pinned the pre-rename counter table header (`context switches`) and lacked the new explanatory line; `crates/metrics/tests/snapshots/report__headline_report.snap` additionally pinned a manifest blake3 that moved because its own fixture JSON (`SAMPLE_MANIFEST_JSON`) uses the renamed key.
- **Fix:** `INSTA_UPDATE=always cargo test` for both suites, then read every line of both diffs by hand: each contains exactly the header/separator change, the new explanatory line, and (for the metrics one) the moved blake3 line. No unrelated content changed.
- **Files modified:** `crates/metrics/tests/snapshots/report__headline_report.snap`, `crates/cli/tests/snapshots/run_pipeline__full_run_report_matches_snapshot.snap`
- **Verification:** `cargo test -p nr-metrics --test report`, `cargo test -p nr-cli --test run_pipeline` (both green on the committed tree)
- **Committed in:** `bf850e7` (task 2's own commit)

**Total deviations:** 3 auto-fixed (2 Rule 1 formatting/lint, 1 Rule 1 snapshot re-pin). **Impact on plan:** all three are direct, mechanical, or newly-exposed consequences of the plan's own two mandated changes (task 1's problem message, task 2's header/caveat rewording and field rename); none expand scope beyond C1/B4/B5.

## Issues Encountered

- My own first draft of `rewrite_reports_regenerates_a_stale_report` (task 3) asserted the freshly-rewritten report matched the pre-tamper committed text byte for byte. It failed immediately, correctly: `measurements/2026-09-01-precision3591-calibration-clean/REPORT.md` was itself stale relative to the current renderer (predating both the Series admission section and this plan's own header rename), so "matches the original" was never the right invariant. Rewrote the test to assert the tampered placeholder is gone, the current header is present, and a second pass converges to zero further writes, which is what the property actually guarantees. This was a useful early, isolated preview of exactly the kind of renderer drift the real tree's own rewrite (step 2) went on to show.
- See `key-decisions` for two further findings that are not defects in this plan's own work: task 2's schema-grep criterion cannot hold simultaneously with its own mandated doc comment, and the plan's top-level verification snippet's hand-edit test has a directory-layout bug. Both were investigated to a specific, verified root cause rather than worked around silently.

## User Setup Required

None - no external service configuration required. Runs entirely on the macOS dev host, per the plan's own objective; no rig access, no new captures.

## Next Phase Readiness

- C1, B4 and B5 of `01-REVIEW-2026-09-06.md` are closed (see `deferred-items.md`). The phase's headline firmware result is now the best-checked number in the repository rather than the least-checked one.
- Plan 01-27 (wave 18, D-29 and C2, which reinstalls the rig's scripts) is next per STATE.md's own sequencing. Plans 01-12 through 01-15 remain at waves 19-22, held until 01-27 lands.
- No new blockers. The pre-existing blockers (the unexplained 3.8 ms global stall, the untuned-boot governor race, PLAT-03's still-pending headline capture) are unchanged and out of this plan's scope.

*Phase: 01-trustworthy-measurement*
*Completed: 2026-09-07*

## Self-Check: PASSED

All 28 claimed files (16 code/doc/test files, 11 rewritten `REPORT.md` files, and this
SUMMARY.md itself) confirmed present on disk. All 3 claimed commit hashes (`c4b2b4c`,
`bf850e7`, `472fe44`) confirmed present in `git log --oneline --all`.
