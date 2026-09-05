---
status: PASS
agent: donny-executor
phase: 01-trustworthy-measurement
plan: 16
subsystem: provenance
tags: [rust, blake3, cargo-build-scripts, json-schema, serde, tdd, provenance]

# Dependency graph
requires:
  - phase: 01-trustworthy-measurement
    provides: "01-11's D-24 tail-based contamination detector and the D-17 calibration pair (measurements/2026-09-01-*-calibration-clean, measurements/2026-09-02-*-calibration-contaminated) this plan's test re-derives figures from"
provides:
  - "Build-time-embedded harness identity (git_sha, git_dirty, git_sha_source) via crates/cli/build.rs, replacing a run-time git shell-out that depended on the process's working directory"
  - "HarnessInfo.executable_blake3/executable_bytes: an independently-verifiable hash of the exact binary that wrote a manifest, with no absolute executable path recorded"
  - "ToolInvocation.argv recorded byte-for-byte as executed, paired with a new ArtifactPathMapping/artifact_paths field naming which committed artifact each output path became"
  - "redact_home_prefix: a visible [redacted] token replacing a home-directory prefix in any argv element or executed_path"
  - "crates/capture/tests/published_figures.rs: a re-derivation guard that recomputes the eight published per-run-hour interference-counter figures from the two committed calibration manifests"
  - "deferred-items.md brought current: three already-closed entries marked closed with their closing commits"
affects: [01-17, 01-18, 01-19, 01-22]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Compile-time identity via a crate build.rs setting cargo:rustc-env, read back with env! at the exact call sites that used to shell out at run time"
    - "Argv redaction and artifact-path mapping both split a --flag=value token on its first '=' before touching the value, mirroring KernelInfo::redact_cmdline's own convention"
    - "A dedicated re-derivation test (published_figures.rs) that recomputes a quoted prose figure from the committed artifact it was quoted from, rather than trusting the prose"

key-files:
  created:
    - crates/cli/build.rs
    - crates/capture/tests/published_figures.rs
  modified:
    - crates/manifest/src/fields.rs
    - crates/cli/src/cmd/run.rs
    - crates/cli/src/cmd/reconstruct.rs
    - crates/cli/src/tools.rs
    - crates/cli/tests/run_pipeline.rs
    - crates/cli/tests/fixtures/fake-cyclictest.sh
    - crates/capture/src/lib.rs
    - crates/manifest/tests/fixtures/minimal-manifest.json
    - schemas/manifest.schema.json
    - crates/metrics/tests/snapshots/report__headline_report.snap
    - .planning/phases/01-trustworthy-measurement/deferred-items.md
  deleted:
    - crates/capture/src/argv.rs

key-decisions:
  - "Treated a Rust compile failure (referencing HarnessInfo/ToolInvocation fields that do not exist yet) as the valid TDD RED state for both tdd=true tasks, since the capability genuinely does not exist yet and a runtime-only RED is not achievable for new, statically-typed API surface"
  - "Extended reconstruct.rs's own harness_info() (a documented standalone mirror of run.rs's) with the full build-time-identity fix rather than a minimal compiling stub, so the same finding-6 defect is not left alive in a second, less-visible code path"
  - "redact_home_prefix splits a token on its first '=' before matching, after discovering a whole-string prefix check never fires for a --histfile=<path>-shaped argv element"
  - "harness_records_executable_hash uses assert_cmd::cargo::cargo_bin(\"nrmeasure\") plus an independently-computed nr_manifest::blake3_file, rather than trusting the harness's own internal computation"
  - "argv_redacts_a_home_directory_prefix overrides both $HOME and $TMPDIR for the child process so the scratch tempdir genuinely lands under a controlled path, rather than fabricating a synthetic argv element by hand"

patterns-established:
  - "New Option fields on an existing #[serde(deny_unknown_fields)] struct use #[serde(default, skip_serializing_if = \"Option::is_none\")] so committed historical manifests keep validating"
  - "A schema-changing field with no skip_serializing_if (ToolInvocation.artifact_paths) always serializes, including as an empty array; any test that pins a manifest's own content-hash must be re-derived, not hand-edited, when the schema changes"

requirements-completed: [BENCH-04, BENCH-05]

# Metrics
duration: 32min
completed: 2026-09-05
---

# Phase 01 Plan 16: Trustworthy Measurement Summary

**Harness identity moved from a run-time git shell-out to a crate build.rs, argv is now recorded byte-for-byte with a named artifact mapping and home-directory redaction, and a new test re-derives the eight published per-run-hour interference figures from the two committed calibration manifests.**

## Performance

- **Duration:** ~32 min
- **Started:** 2026-09-05T16:03:23Z (phase execution start recorded in STATE.md)
- **Completed:** 2026-09-05T16:35:35Z
- **Tasks:** 3 completed
- **Files:** 2 created, 11 modified, 1 deleted

## Accomplishments

- A manifest written by a run with no git checkout as its working directory (the systemd-run shape that produced seven "unknown" manifests) now records the real 40-character commit the binary was built from, embedded at compile time by `crates/cli/build.rs`.
- `HarnessInfo` gained `git_sha_source` (an explicit `build-time`/`unavailable` distinction, so an unreadable git state is never silently reported as a clean tree), `executable_blake3`, `executable_bytes`, and `invoked_from_git_sha` (the one field that still reads the working directory, kept separate so a stale executable inside a newer checkout is visible).
- `nr_capture::argv::relativize_argv` is retired (`crates/capture/src/argv.rs` deleted): it relativized argv against the run directory while the tools actually wrote into a scratch tempdir under `/tmp`, so it matched nothing. `ToolInvocation.argv` is now recorded exactly as executed, and a new `ArtifactPathMapping`/`artifact_paths` field names which committed artifact each output path became.
- `redact_home_prefix` replaces a home-directory prefix in any argv element or `artifact_paths.executed_path` with the literal `[redacted]` token, matching `KernelInfo::redact_cmdline`'s existing convention.
- `crates/capture/tests/published_figures.rs` re-derives the eight published per-run-hour CAL/TLB/RES/device-IRQ figures directly from `measurements/2026-09-01-precision3591-calibration-clean` and `measurements/2026-09-02-precision3591-calibration-contaminated`, and asserts the two directional claims the prose makes (CAL/TLB/RES higher on the contaminated arm; only device IRQ inverts).
- `deferred-items.md` now matches the tree: three entries describing already-completed work are marked closed with the commit that closed each; the two that remain open name plan 01-18 (finding 5's exemption-scoping half) and plan 01-22 (re-installing the rig's outdated `nr-run-measurement`) as owners.

## Task Commits

Each task was committed atomically, with an explicit RED/GREEN split for the two `tdd="true"` tasks:

1. **Task 1: Harness identity describes the executable, not the working directory**
   - `0c043e4` (test) - three tests against `HarnessInfo` fields that did not exist yet; confirmed to fail to compile
   - `e605a89` (feat) - `crates/cli/build.rs`, the extended `HarnessInfo`/`GitShaSource`, and the rewritten `run.rs`/`reconstruct.rs` `harness_info()` functions
2. **Task 2: Preserve the executed argv and map each path to the artifact it became**
   - `efac688` (test) - three tests against `ToolInvocation.artifact_paths`, which did not exist yet; confirmed to fail to compile
   - `3c68dc9` (feat) - `ArtifactPathMapping`, the argv/mapping finalization in `run.rs`, `redact_home_prefix`, retirement of `nr_capture::argv`, and the mechanical `tools.rs`/report-snapshot follow-ons
3. **Task 3: A test that re-derives a published figure from the artifact it is quoted from**
   - `02786ed` (test) - `crates/capture/tests/published_figures.rs` and the `deferred-items.md` closures

_Note: TDD tasks produced test -> feat pairs (2 commits each); no separate refactor commit was needed for either._

## Files Created/Modified

- `crates/cli/build.rs` - new: embeds the build commit, dirty flag, and source at compile time
- `crates/capture/tests/published_figures.rs` - new: the re-derivation guard for the eight published per-run-hour figures
- `crates/manifest/src/fields.rs` - `GitShaSource` enum, `HarnessInfo`'s four new Option fields, `ArtifactPathMapping`, `ToolInvocation.artifact_paths`
- `crates/cli/src/cmd/run.rs` - rewired `harness_info()`, added `redact_home_prefix`/`redact_home_prefix_in_path`, replaced the `relativize_argv` map with mapping construction
- `crates/cli/src/cmd/reconstruct.rs` - its own standalone `harness_info()` updated to match (a Rule 3/2 deviation; see below)
- `crates/cli/src/tools.rs` - `ToolInvocation` construction updated for the new `artifact_paths` field (Rule 3 deviation)
- `crates/cli/tests/run_pipeline.rs` - six new integration tests (three per TDD task)
- `crates/cli/tests/fixtures/fake-cyclictest.sh` - `FAKE_CYCLICTEST_ARGV_FILE` seam (Rule 3 deviation)
- `crates/capture/src/lib.rs` - removed `pub mod argv;`
- `crates/capture/src/argv.rs` - deleted
- `crates/manifest/tests/fixtures/minimal-manifest.json` - `tools[0].argv`/`artifact_paths` updated to the scratch-path convention
- `schemas/manifest.schema.json` - regenerated (twice, once per task)
- `crates/metrics/tests/snapshots/report__headline_report.snap` - updated content-hash line (Rule 1 deviation)
- `.planning/phases/01-trustworthy-measurement/deferred-items.md` - three entries marked closed

## Decisions Made

- Treated a Rust compile failure as the valid TDD RED state for both `tdd="true"` tasks: the referenced fields/types genuinely did not exist, and a runtime-only failure is not achievable when adding new, statically-typed struct fields.
- Fixed `reconstruct.rs`'s own `harness_info()` with the full build-time-identity logic rather than a minimal stub, since its doc comment already claims to mirror `run.rs`'s function and leaving it half-fixed would keep finding 6's exact defect alive in a second call site.
- `redact_home_prefix` splits on the first `=` before matching (mirroring `KernelInfo::redact_cmdline`), after a self-caught bug (see Issues Encountered) showed a whole-string check never fires for `--histfile=<path>` tokens.
- `harness_records_executable_hash` computes its expected hash via `assert_cmd::cargo::cargo_bin("nrmeasure")` + `nr_manifest::blake3_file`, an independent computation rather than trusting the harness's own value.
- `argv_redacts_a_home_directory_prefix` forces the scenario for real by overriding both `$HOME` and `$TMPDIR` on the child process, so the scratch tempdir genuinely lands under a controlled path rather than fabricating an argv element.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Extended `reconstruct.rs`'s own `harness_info()` to the new `HarnessInfo` shape**
- **Found during:** Task 1
- **Issue:** `crates/cli/src/cmd/reconstruct.rs` has its own standalone copy of `harness_info()` (not in this plan's file list) that constructs the same struct. `HarnessInfo`'s new fields made its old construction fail to compile, and reconstruct.rs's own doc comment already states it "mirrors `cmd::run::harness_info`".
- **Fix:** Applied the identical compiled-in-identity logic (same `env!` constants, same executable hash) rather than stubbing new fields to `None`, keeping the "mirrors" claim true and closing the same finding-6 defect in this second code path.
- **Files modified:** crates/cli/src/cmd/reconstruct.rs
- **Verification:** `cargo build --workspace`, `cargo test --workspace`
- **Committed in:** e605a89

**2. [Rule 3 - Blocking] `tools.rs`'s `ToolInvocation` construction updated for `artifact_paths`**
- **Found during:** Task 2
- **Issue:** `crates/cli/src/tools.rs::run_tool` (not in this plan's file list) builds a `ToolInvocation` literal that needed the new non-`Option` `artifact_paths` field to compile.
- **Fix:** Populated with an empty `Vec` at construction; `run.rs::execute` fills in the real mapping afterward once the placed artifacts are known.
- **Files modified:** crates/cli/src/tools.rs
- **Verification:** `cargo build --workspace`
- **Committed in:** 3c68dc9

**3. [Rule 3 - Blocking] Extended the `fake-cyclictest.sh` test fixture**
- **Found during:** Task 2
- **Issue:** The `argv_is_recorded_as_executed` behavior spec requires a byte-for-byte comparison against what the subprocess actually received; nothing in the existing fixture captured that independently of the manifest under test.
- **Fix:** Added an opt-in `FAKE_CYCLICTEST_ARGV_FILE` env var that makes the fake tool log its own received argv verbatim, so the test compares two independently obtained values.
- **Files modified:** crates/cli/tests/fixtures/fake-cyclictest.sh
- **Verification:** `cargo test -p nr-cli --test run_pipeline argv_is_recorded_as_executed`
- **Committed in:** efac688 (added), exercised from 3c68dc9

**4. [Rule 1 - Bug] Updated `crates/metrics/tests/report.rs`'s pinned snapshot**
- **Found during:** Task 2 (`cargo test --workspace`)
- **Issue:** `headline_report_snapshot` pins the exact value of a content-hash header computed by re-serializing a hand-built `RunManifest`. `ToolInvocation.artifact_paths` has no `skip_serializing_if`, so it always serializes (even as `[]`), changing every manifest's canonical JSON and therefore its hash.
- **Fix:** Updated the one changed line in the committed `.snap` file to the newly, correctly computed hash. No test logic or `nr-metrics` behavior changed.
- **Files modified:** crates/metrics/tests/snapshots/report__headline_report.snap
- **Verification:** `cargo test -p nr-metrics --test report`
- **Committed in:** 3c68dc9

**Total deviations:** 4 auto-fixed (3 blocking, 1 bug). **Impact on plan:** all four are direct, mechanical consequences of the two schema/type changes Tasks 1 and 2 explicitly mandate; none expand scope beyond finding 6.

## Issues Encountered

- The first draft of `redact_home_prefix` checked whether the *whole* argv element string started with the home directory. Real argv elements are `--flag=value` tokens (e.g. `--histfile=/home/.../x`), so the check silently never matched. Caught by `argv_redacts_a_home_directory_prefix` itself (the test failed with the real home path visible in the output), fixed by splitting on the first `=` first, matching `KernelInfo::redact_cmdline`'s existing convention. No published manifest was ever affected; this was caught before the GREEN commit.
- Reverting a temporarily-edited test file via `mv backup original` (to verify `published_figures.rs` actually fails when a constant is wrong, per the task's own acceptance criterion) left the file with an mtime earlier than cargo's cached build, so the first re-run of `cargo test` silently used the stale, still-broken compiled test binary. `touch`ing the file forced a real recompile, confirming the revert was correct. Not a code issue; noted here since it could otherwise look like a false pass.
- During the state-update step, hand-editing STATE.md's body `Status:` line to `01-16 complete (PASS)` tripped `state.cjs`'s status-normalization heuristic (any `Status:` line containing the substring "complete" or "done" collapses the top-level frontmatter `status:` to `completed`), which briefly and incorrectly marked the whole 23-plan phase completed after only 12 plans. Caught by re-reading STATE.md immediately after the edit; fixed by rewording the body line to `Executing Phase 01` (which the same heuristic correctly maps to `executing`) and correcting the frontmatter directly, then confirmed stable across a subsequent `state update-progress` call. Logged as a decision (STATE.md Accumulated Context) so a future executor does not phrase a `Status:` line the same way.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- All eight committed run directories (including the two D-17 calibration manifests and the reconstructed 2026-08-28 capture) still pass `nrmeasure verify --strict --check-index`.
- `measurements/` is untouched (`git diff --stat measurements/` is empty), per D-12.
- Finding 6 of `01-EXTERNAL-AUDIT.md` is now split cleanly: this plan closes the harness-identity and argv/artifact-mapping halves; the remaining sub-items (`verify.rs`'s `(0, 0)` substitution on a histogram parse failure, `REPORT.md`/`hist.tsv` regeneration-and-compare in strict verification, and JSON/`.hist` sample-count reconciliation) are explicitly plan 01-19's, per STATE.md's finding-to-plan assignment.
- Plan 01-17 (measured elapsed time) will change `published_figures.rs`'s duration denominator from `utc_end - utc_start` to the measured instrument window once it lands; the test's own doc comment already flags this.
- No blockers for the next wave.

## Self-Check: PASSED

All 13 created/modified files confirmed present on disk; `crates/capture/src/argv.rs` confirmed deleted; all 5 commit hashes (`0c043e4`, `e605a89`, `efac688`, `3c68dc9`, `02786ed`) confirmed present in `git log`.

*Phase: 01-trustworthy-measurement*
*Completed: 2026-09-05*
