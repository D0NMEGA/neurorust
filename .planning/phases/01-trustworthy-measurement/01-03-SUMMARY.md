---
status: PASS
agent: donny-executor
phase: 01-trustworthy-measurement
plan: 03
subsystem: measurement-provenance
tags: [rust, schemars, blake3, serde, json-schema, provenance, threat-modeling]

# Dependency graph
requires:
  - phase: 01-trustworthy-measurement (plan 01)
    provides: cargo workspace, dual licence, CI, and the nr-manifest stub crate
provides:
  - The full D-14 environment snapshot as typed, deny-unknown-fields Rust structs (RunManifest)
  - D-16 provenance tier (ProvenanceTier, non-Option, never defaulted) and AbsentField
  - Streaming blake3 checksums (checksum.rs) and cross-field manifest validation (validate.rs)
  - A schemars-generated JSON Schema (schemas/manifest.schema.json) with a committed drift test
  - Human-reviewed, redacted kernel.cmdline (root=/resume= -> [redacted]) via KernelInfo::redact_cmdline
  - Documented run-directory-relative convention for tools[].argv, for nr-capture to implement
affects: [01-05-nr-capture, 01-06-nr-metrics, 01-07-nr-cli, 01-08-nr-cli, 01-14-nr-cli, phase-02-STOP-07, phase-03-SUBS-06]

# Tech tracking
tech-stack:
  added: [schemars, blake3, thiserror, time (rfc3339 serde feature)]
  patterns:
    - "Schema generated, never hand-authored: schemars::schema_for! compared byte-for-byte against the committed schemas/manifest.schema.json in schema_up_to_date, so drift fails the build (RESEARCH.md pattern 3, D-04)"
    - "Required-field enforcement via #[serde(deny_unknown_fields)] plus the absence of #[serde(default)], not a runtime check: a manifest missing a required key fails to deserialize before any validation code runs (T-1-03)"
    - "validate() collects every failure into Vec<ValidationError> rather than short-circuiting, so one verify pass reports all problems"
    - "Visible redaction over silent drop: a field found late to carry a machine identifier gets a pure, unit-tested transform (KernelInfo::redact_cmdline) co-located with the struct, so a reader can tell 'redacted' from 'never recorded'"

key-files:
  created:
    - crates/manifest/src/fields.rs
    - crates/manifest/src/provenance.rs
    - crates/manifest/src/checksum.rs
    - crates/manifest/src/validate.rs
    - crates/manifest/src/schema.rs
    - crates/manifest/tests/required_fields.rs
    - crates/manifest/tests/checksum.rs
    - crates/manifest/tests/schema_up_to_date.rs
    - crates/manifest/tests/fixtures/minimal-manifest.json
    - schemas/manifest.schema.json
  modified:
    - crates/manifest/src/lib.rs
    - .planning/phases/01-trustworthy-measurement/deferred-items.md

key-decisions:
  - "D-14 field set typed exactly as RIG.txt's de facto schema, with provenance_tier required and never defaulted (D-16)"
  - "blake3 checksums documented as proving integrity, not authenticity (T-1-07); no signing story in v1"
  - "run_id and artifact paths validated by hand (no regex dependency); ASVS V12 traversal surface closed before any path is joined to a run directory"
  - "Human review (Task 3, checkpoint:human-verify) approved the field set with two changes: kernel.cmdline redacts root= and resume= to the literal [redacted], implemented now via KernelInfo::redact_cmdline; tools[].argv documents a run-directory-relative path convention, with the actual rewrite deferred to nr-capture (plan 01-05) since it does not exist yet"
  - "BENCH-04 is shared across plans 01-01/01-03/01-07/01-08/01-10; per explicit instruction this plan does not mark it complete in REQUIREMENTS.md, leaving that call to the end-of-phase verifier"

patterns-established:
  - "Schema-generated-not-authored: any future struct/enum change to RunManifest must be followed by UPDATE_SCHEMAS=1 cargo test -p nr-manifest schema_up_to_date, never a hand edit to schemas/manifest.schema.json"
  - "Redact-at-the-source convention: when a field is found to carry a machine-instance identifier, add a pure transform function next to the struct, unit test it directly (no regex needed for simple key-based redaction), and update the field's own doc comment so the generated schema description states the transform"

requirements-completed: []

# Metrics
duration: ~6min active (2 sessions; see Performance note)
completed: 2026-08-31
---

# Phase 01 Plan 03: D-14 Manifest, Checksums, and Human-Reviewed Schema Summary

**Typed D-14 environment snapshot (nr-manifest) with blake3 checksums, deny-unknown-fields validation, and a schemars-generated JSON Schema; human review redacted root=/resume= UUIDs from kernel.cmdline and documented the argv run-directory-relative convention for nr-capture.**

## Performance

- **Duration:** ~6 min of active agent execution, across two sessions
- **Session 1 (Tasks 1-2):** 2026-08-30T22:25:14Z -> 2026-08-30T22:29:38Z (~4m 24s)
- **Checkpoint pause (Task 3, `checkpoint:human-verify`):** ~6h 35m, awaiting human review of the generated schema and worked-example fixture
- **Session 2 (Task 3 continuation):** 2026-08-31T05:05:02Z -> 2026-08-31T05:05:14Z commits, verification and this summary immediately following
- **Tasks:** 3 (all complete)
- **Files modified:** 12 (10 created in tasks 1-2, `lib.rs` modified, `deferred-items.md` modified in the continuation)

## Accomplishments

- The complete D-14 field set (`RunManifest` and 20+ nested structs/enums) typed with `#[serde(deny_unknown_fields)]` and no `#[serde(default)]` anywhere, so a required field's absence is a deserialization error, not a silently-defaulted value
- `ProvenanceTier` is a required, non-`Option` field with no default: a reconstructed manifest can never be mistaken for a harness-generated one (D-16)
- Streaming blake3 checksums (64 KiB chunks) and `validate()`, which collects every failure (bad checksum, unsafe path, missing exclusion reason, unjustified reconstructed-without-absent-fields) in one pass rather than stopping at the first
- `schemas/manifest.schema.json` generated from the types via `schemars`, with a byte-for-byte drift test (`schema_up_to_date`) that fails the build if the schema and the types disagree
- Human review of the generated schema and the worked-example fixture (Task 3) approved the field set with two explicit, recorded decisions (below), both now implemented
- `KernelInfo::redact_cmdline` redacts `root=` and `resume=` values to the literal `[redacted]`, covered by a unit test over a realistic cmdline string (including a `resume_offset=` near-miss that must survive verbatim)

## Task Commits

Each task was committed atomically:

1. **Task 1: The D-14 field set as typed structs, with a non-optional provenance tier** - `5203043` (feat)
2. **Task 2: blake3 checksums, manifest validation, and the generated JSON Schema** - `9e7d120` (feat)
3. **Task 3: Review what the D-14 snapshot publishes (Decision A: redact kernel.cmdline)** - `fbfdb05` (feat)
4. **Task 3 (continued): carry Decision B's argv convention forward to plan 01-05** - `78ff941` (docs)

**Plan metadata:** commit hash recorded after this summary is written (see final commit).

## Files Created/Modified

- `crates/manifest/src/fields.rs` - `RunManifest` and the full D-14 struct/enum set; `KernelInfo::redact_cmdline`; two unit tests
- `crates/manifest/src/provenance.rs` - `ProvenanceTier`, `AbsentField` (D-16)
- `crates/manifest/src/checksum.rs` - `blake3_file`, `verify_artifact`; documents integrity-not-authenticity (T-1-07)
- `crates/manifest/src/validate.rs` - `validate()`, `ValidationError` (9 variants); run_id charset check by hand
- `crates/manifest/src/schema.rs` - `manifest_schema_json()`, the schemars-driven generator
- `crates/manifest/tests/required_fields.rs` - 7 tests for required-field enforcement and round trips
- `crates/manifest/tests/checksum.rs` - 6 tests for checksum/path/validation behavior
- `crates/manifest/tests/schema_up_to_date.rs` - the committed-schema drift test
- `crates/manifest/tests/fixtures/minimal-manifest.json` - the worked example; `kernel.cmdline` now shows the redacted form with both `root=[redacted]` and `resume=[redacted]`
- `schemas/manifest.schema.json` - generated; only the `cmdline` and `argv` property descriptions changed in the Task 3 continuation
- `crates/manifest/src/lib.rs` - re-exports the public surface
- `.planning/phases/01-trustworthy-measurement/deferred-items.md` - new "From 01-03" section carrying Decision B's convention forward to plan 01-05

## Decisions Made

**Decision A - kernel.cmdline: redact, visibly.** `/proc/cmdline`'s `root=` and `resume=` values are replaced with the literal token `[redacted]`; every other parameter (`BOOT_IMAGE`, `ro`, `quiet`, `splash`, `isolcpus`, `nohz_full`, `rcu_nocbs`, `irqaffinity`, `intel_pstate`, and anything else present) stays verbatim. Rationale: the root/swap filesystem UUIDs carry zero reproduction value and are the only machine-instance identifiers in the manifest; every reproduction-relevant parameter survives untouched. This does not touch D-12, which governs raw captures (checksummed artifacts), not manifest fields. Implemented now, in this crate, as `KernelInfo::redact_cmdline` (a pure string transform, `split_whitespace` + `split_once('=')`, no regex dependency), because unlike Decision B it needs no run-directory context and is fully testable without `nr-capture` existing. Redaction is visible by design (the literal `[redacted]` token appears) rather than a silent drop, so a reader can distinguish "redacted" from "never recorded."

**Decision B - tools[].argv: relativize, documented now, implemented in nr-capture.** Output-file paths in `argv` should be recorded relative to the run directory rather than as absolute home-directory paths, so the argv is a command a third party can actually paste and run. The fixture already demonstrated this (`--histfile=cyclictest-rt-isolated-idle-10m.hist`) and needed no change. The actual rewrite happens where argv is captured, in `nr-capture` (plan 01-05), which does not exist yet; this crate only documents the convention on the `argv` field's doc comment (and therefore the generated schema description), and carries it forward via a new entry in `deferred-items.md` so plan 01-05 inherits a clear contract rather than missing it. Rationale: the field's stated purpose is "for reproduction," and an absolute `/home` path is strictly worse at that job.

**BENCH-04 traceability.** This plan carries `BENCH-04` jointly with plans 01-01, 01-07, 01-08 and 01-10. Per explicit instruction, `requirements mark-complete` was not run and `.planning/REQUIREMENTS.md` was left untouched; the end-of-phase verifier owns marking `BENCH-04` complete once all five plans have landed.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Updated the worked-example fixture's `kernel.cmdline` to match the newly-documented redaction contract**
- **Found during:** Task 3 continuation, implementing Decision A
- **Issue:** Task 3's `<files>` list named only `schemas/manifest.schema.json` and `crates/manifest/src/fields.rs` as edit targets. After redacting `KernelInfo.cmdline`'s doc comment to state that `root=`/`resume=` are redacted, the worked-example fixture (`minimal-manifest.json`) still carried a raw, unredacted UUID (`root=UUID=00000000-...`) and no `resume=` parameter at all. Left as-is, the fixture the plan calls "a complete, valid manifest... real values in every field" would directly contradict the field's own documented contract, and would still fail the human's explicit rationale that "a real Ubuntu cmdline commonly carries both" root and resume.
- **Fix:** Updated the fixture's `cmdline` value to `"BOOT_IMAGE=/vmlinuz root=[redacted] ro quiet splash isolcpus=6-11 nohz_full=6-11 rcu_nocbs=6-11 resume=[redacted]"`, demonstrating both redactions while leaving every reproduction-relevant parameter untouched.
- **Files modified:** `crates/manifest/tests/fixtures/minimal-manifest.json`
- **Verification:** `cargo test -p nr-manifest` (all 17 tests, including `fixture_parses_as_valid_manifest`) still passes; `grep` confirms no UUID pattern remains in the fixture.
- **Committed in:** `fbfdb05` (Task 3 commit)

**2. [Rule 1 - Bug] Found and corrected a pre-existing donny-tools `state update-progress` bug while updating STATE.md**
- **Found during:** State/roadmap updates after the Task 3 commits
- **Issue:** `node donny-tools.cjs state update-progress` reported `"updated": true` with the correct percent (13) each time, and `.planning/STATE.md`'s YAML frontmatter (`progress.percent`, `progress.completed_plans`) was correctly resynced by a separate mechanism on every write. But the body's `## Current Position` `Progress: [bar] X%` line stayed stuck at `0%`. Root cause (confirmed with a direct `node -e` repro against the live file): the command's fallback regex, `/^(Progress:\s*).*/im`, is case-insensitive with no frontmatter exclusion, so it matches the YAML frontmatter's lowercase `progress:` key (which appears first in the file, before the body's capitalized `Progress:` line) instead of the intended body line; the non-global `.replace()` then edits that first match, which frontmatter resync immediately overwrites back to a correct value, masking the bug while the real body line is never touched. This is a bug in the shared, cross-project `~/.claude/donny/bin/lib/state.cjs` CLI, not in anything specific to this plan.
- **Fix:** This plan's scope is the neurorust `nr-manifest` crate, not the global donny-tools CLI, so the shared script was not patched here. Directly corrected the single stale line in `.planning/STATE.md` (`Progress: [░░░░░░░░░░] 0%` -> `Progress: [█░░░░░░░░░] 13%`) to match the now-correct frontmatter, so the artifact this plan is responsible for updating is accurate. Also fixed a related cosmetic byproduct in the same update pass: `roadmap update-plan-progress`'s `status.padEnd(11)` leaves no trailing space when the status string (`"In Progress"`) is already exactly 11 characters, producing `| In Progress|  |` in `.planning/ROADMAP.md`'s progress table; corrected to `| In Progress | - |` to match the table's own convention for not-yet-complete rows.
- **Files modified:** `.planning/STATE.md`, `.planning/ROADMAP.md`
- **Verification:** `grep -n "Progress:" .planning/STATE.md` shows `13%`; frontmatter and body now agree. Table row renders consistently with the other 7 rows.
- **Committed in:** `22fc762` (final metadata commit)

---

**Total deviations:** 2 auto-fixed (1 missing critical / consistency, 1 bug)
**Impact on plan:** Necessary to keep the worked example and the planning artifacts (STATE.md, ROADMAP.md) truthful. No scope creep: the underlying donny-tools bug was documented here for visibility rather than patched, since patching shared, cross-project tooling is outside this plan's boundary.

## Issues Encountered

None blocking. One pre-existing, out-of-scope item noted for transparency: `.planning/phases/01-trustworthy-measurement/01-LEDGER.jsonl` was untracked in git both before and after this session (a donny-tools phase-tracking artifact, timestamped before this session began). It is unrelated to plan 01-03's file list and was left untouched, per the scope boundary that only task-related files are staged.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `nr-manifest` is complete: typed D-14 snapshot, D-16 provenance tier, blake3 checksums, cross-field validation, and a human-reviewed, schema-drift-checked JSON Schema. All three plan tasks are done; `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` are green.
- Plan 01-05 (`nr-capture`) has two concrete, documented obligations inherited from this plan: call `KernelInfo::redact_cmdline` when populating `KernelInfo.cmdline` from `/proc/cmdline`, and relativize `ToolInvocation.argv` output-file paths to the run directory (tracked in `deferred-items.md`).
- Plans 01-06 (`nr-metrics`) and 01-07/01-08/01-14 (`nr-cli`) can build against the published `RunManifest` surface and `schemas/manifest.schema.json` as a stable contract per D-04.
- `BENCH-04` remains open at the requirements level (shared with 01-01, 01-07, 01-08, 01-10); no action needed here, flagged for the end-of-phase verifier.

---
*Phase: 01-trustworthy-measurement*
*Completed: 2026-08-31*

## Self-Check: PASSED

- All 12 files listed under key-files (created/modified) confirmed present on disk with `[ -f ]`.
- All 4 commits (`5203043`, `9e7d120`, `fbfdb05`, `78ff941`) confirmed present in `git log --oneline --all`.
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` re-run clean after the final commit, with a clean `git status` apart from the pre-existing, out-of-scope `01-LEDGER.jsonl`.
