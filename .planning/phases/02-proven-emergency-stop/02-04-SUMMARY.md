---
status: PASS
agent: donny-executor
phase: 02-proven-emergency-stop
plan: 04

subsystem: infra
tags: [kani, cargo-llvm-cov, ci, github-actions, nr-stop, branch-coverage]

# Dependency graph
requires:
  - phase: 02-proven-emergency-stop
    plan: 01
    provides: Kani 0.67.0 and cargo-llvm-cov 0.9.1 pinned and confirmed working in this exact
      workspace, the exact per-harness/verdict output strings, the pinned nightly-2026-08-01,
      and scripts/nr-coverage.sh's original STOP-06 gate design
  - phase: 02-proven-emergency-stop
    plan: 03
    provides: the eleven Kani harness names crates/stop/src/proofs.rs carries, which
      scripts/nr-proofs.sh's EXPECTED list is copied from verbatim
provides:
  - scripts/nr-proofs.sh, the committed STOP-05 gate with a non-vacuity guard proven to catch
    a lost harness while cargo kani itself still exits 0
  - a blocking, unconditional kani job in ci.yml, confirmed green on ubuntu-latest twice
    (39s cold cache, 27s warm cache)
  - crates/stop confirmed at 100 percent branch coverage (6/6), with the false-positive
    proofs.rs missing-file check fixed in scripts/nr-coverage.sh
  - a blocking, unconditional coverage job in ci.yml, confirmed green on ubuntu-latest, with
    a self-check that refuses a nightly-date mismatch between itself and the script
affects: ["02-05"]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "A CI job installs its exact pinned tool version itself rather than via a vendor-supplied
       action, cached explicitly by a version-keyed cache path, matching D-51's precedent a
       second time in this phase (kani, now cargo-llvm-cov)"
    - "A gate script that owns a second copy of a cross-file fact (the nightly date) checks the
       other copy against itself and fails naming both, rather than trusting a human to keep a
       shell variable and a YAML line in sync by eye"
    - "A non-vacuity guard is proven, not just written: the exact regression it exists to catch
       is manufactured once, observed to produce the intended failure, then reverted, with both
       outputs recorded verbatim"

key-files:
  created:
    - scripts/nr-proofs.sh
  modified:
    - .github/workflows/ci.yml
    - scripts/nr-coverage.sh

key-decisions:
  - "crates/stop needed zero new tests for STOP-06: 02-02/02-03's existing suite already covered
     all 6 real branches (all in latch.rs; every match-based file compiles to a jump table with
     0 LLVM branch regions, confirmed from the raw JSON per file)"
  - "Fixed nr-coverage.sh's missing-file check to also exclude proofs.rs (Rule 1 bug), mirroring
     02-01's lib.rs fix: proofs.rs is cfg(kani)-gated and a normal llvm-cov build, even on the
     pinned nightly, never compiles it, so it can never appear in the report regardless of test
     coverage"
  - "Task 2's tdd=true flag does not map cleanly onto RED-fails/GREEN-passes: the production code
     already existed and was already correct, so there was no missing implementation to write
     against a failing test. Executed as one task commit per this project's established
     one-commit-per-task convention (02-01 through 02-03), substituting the plan's own prescribed
     empirical check (watch the branch count move) for a failing unit test"
  - "Left EmergencyStop::default() untested (94.1 percent function coverage, not 100): it has
     zero branches of its own, D-53 scopes the gate to branches only, and the plan's own text
     forbids writing tests speculatively beyond the branches a coverage run actually names"

patterns-established:
  - "Pin-and-cache-by-version is the CI pattern for external verification tools in this repo,
     now used twice: kani-verifier@0.67.0 keyed 'kani-0.67.0-${{ runner.os }}', cargo-llvm-cov@0.9.1
     keyed 'cargo-llvm-cov-0.9.1-${{ runner.os }}'"

# REQUIRED - copy ALL requirement IDs from this plan's `requirements` frontmatter field.
requirements-completed: [STOP-05, STOP-06]

# Metrics
duration: 17min
completed: 2026-09-15
---

# Phase 2 Plan 4: The blocking Kani and coverage CI gates Summary

**Blocking `kani` and `coverage` GitHub Actions jobs, each carrying a non-vacuity guard that was deliberately broken and observed to fail before being trusted, with `crates/stop` confirmed at 100 percent branch coverage from the existing test suite alone.**

## Performance

- **Duration:** 17 min
- **Started:** 2026-09-15T06:35:17Z (immediately following 02-03's completion)
- **Completed:** 2026-09-15T06:52:35Z
- **Tasks:** 2
- **Files modified:** 3 (1 created, 2 modified)

## Accomplishments

- `scripts/nr-proofs.sh` (mode 0755) committed as the single definition of the STOP-05 gate.
  Its `EXPECTED` harness list matches `crates/stop/src/proofs.rs`'s eleven `#[kani::proof]`
  functions exactly, checked in both directions with a shell loop, not by eye.
- The guard was proven, not assumed: `step_is_total` was renamed to `step_covers_every_state`,
  `cargo kani` itself still reported `Complete - 11 successfully verified harnesses, 0 failures,
  11 total.` (exit 0), and the script's own guard caught the missing name and exited 1. The
  rename was then reverted; `git diff --stat crates/stop/src/proofs.rs` showed zero net change.
- A blocking `kani` job was added to `ci.yml` (no `if:` at job level, no `paths:` filter
  anywhere in the file). Confirmed green on `ubuntu-latest` twice: 39s on a cold cache (first
  push, run 34938167522) and 27s on a warm cache with both install steps skipped (second push,
  run 34938678761). No `RUSTFLAGS` override was needed; the job passed cleanly under the
  workspace's `-D warnings`.
- Running `scripts/nr-coverage.sh` against the crate as 02-02/02-03 left it surfaced a real bug
  (Rule 1): `crates/stop/src/proofs.rs` is `#[cfg(kani)]`-gated, so a normal (non-Kani)
  `cargo llvm-cov` build never compiles it and it can never appear in the report, independent of
  test coverage. The script named it "not instrumented" even though branches already read 6/6.
  Fixed by extending the existing `lib.rs` exclusion (02-01) to also exclude `proofs.rs` by
  name, with an inline comment explaining the `cfg(kani)` reasoning.
- Once that false positive was fixed, `crates/stop` already reported 100 percent branch coverage
  (6/6) from the test suite plans 02-02 and 02-03 had already written. Zero new tests were
  needed.
- Added the nightly-date self-check the plan calls for: `scripts/nr-coverage.sh` now reads its
  own `NIGHTLY` value and greps `ci.yml`'s coverage job for `toolchain: nightly-...`, failing
  and naming both if they differ. Proven by deliberately editing `ci.yml`'s date to
  `nightly-2026-08-08`: the gate exited 1 with `nr-coverage.sh: nightly date mismatch: this
  script pins nightly-2026-08-01, .github/workflows/ci.yml pins nightly-2026-08-08`. Restored
  and reconfirmed exit 0.
- Proved the coverage gate itself fails on a real regression. Since no new tests were added,
  used the one existing test that is the sole exerciser of one branch outcome
  (`the_abort_record_is_absent_before_an_abort` in `tests/latch.rs`, the only test that calls
  `abort_record()` before any abort has happened, exercising the "not yet published" arm of
  `abort_record`'s guard). Commented it out: the gate exited 1 with `nr-coverage.sh: 1 of 6
  branches uncovered` / `nr-coverage.sh:   .../crates/stop/src/latch.rs: 5 of 6`, and printed
  `branches 5/6  regions 133/137  lines 107/111`. Restored; `git diff --stat` showed zero net
  change; the gate exited 0 again.
- A blocking `coverage` job was added to `ci.yml` (no `if:` at job level, no `paths:` filter).
  Confirmed green on `ubuntu-latest` at 1m38s on its first-ever run (run 34938678761); its CI
  log printed the identical `branches 6/6  regions 134/137  lines 108/111` line the dev host
  produced.
- Both pushes (`a5003f5`, `c107b86`) left every job in `ci.yml` green: `fmt`, `clippy`,
  `test` (ubuntu-latest and macos-latest), `deny`, `kani`, `coverage`.

## Task Commits

Each task was committed atomically:

1. **Task 1: The blocking Kani job, and the guard that proves the harnesses ran** - `a5003f5` (feat)
2. **Task 2: Reach 100 percent branch coverage on crates/stop and make it a blocking job** - `c107b86` (feat)

## Files Created/Modified

- `scripts/nr-proofs.sh` - the committed STOP-05 gate: runs `cargo kani -p nr-stop`, refuses to
  pass unless every expected harness name appears and the successful-verification count matches
  exactly
- `.github/workflows/ci.yml` - added the `kani` and `coverage` jobs, both unconditional at job
  level, both running the committed scripts rather than inline commands
- `scripts/nr-coverage.sh` - excluded `proofs.rs` from the missing-file check (Rule 1 fix,
  mirroring 02-01's `lib.rs` fix); added the nightly-date self-check against `ci.yml`

## Decisions Made

See `key-decisions` in the frontmatter for the full list. The two most consequential:

**Zero new tests needed.** `crates/stop` already reported 100 percent branch coverage before
this plan wrote a single test. The raw JSON shows why: `consumer.rs`, `event.rs`, `gate.rs` and
`state.rs` all report `0/0` branches (their logic is exhaustive `match` statements over densely
packed enum discriminants, which LLVM compiles to jump tables rather than conditional branches);
`latch.rs` is the only file with real branches (3 `if` statements, 6 branch outcomes), and all 6
were already exercised by plan 02-02/02-03's test suite. This matches the objective's own stated
hope that D-32's small branch-count design would make STOP-06 "a matter of a small number of
missing tests rather than an exercise in writing tests for coverage's sake" - in this case, zero.

**`tdd="true"` on Task 2 was interpreted as empirical-verification discipline, not a literal
RED/GREEN commit pair.** The task's `<behavior>` block describes writing a test per uncovered
branch and watching the branch count move, but the underlying production code already existed
and was already correct (this is coverage backfill, not new-feature TDD), so no test could
genuinely fail for want of a missing implementation. Executed per the task's own detailed
`<action>` text as a single task commit, matching this project's established one-commit-per-task
convention (02-01 Task 3, 02-02's three tasks, 02-03's two tasks), while still honoring the
literal instruction to watch coverage move and to prove the gate itself can fail.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `nr-coverage.sh`'s missing-file check flagged `proofs.rs` as "not
instrumented," making the gate red despite the crate already being at 100 percent branch
coverage**

- **Found during:** Task 2, the first run of `scripts/nr-coverage.sh` against the crate as
  plans 02-02 and 02-03 left it
- **Issue:** `crates/stop/src/proofs.rs` is declared `#[cfg(kani)] mod proofs;` in `lib.rs`
  (plan 02-03). That `cfg` is set only by `cargo kani` itself, never by a normal `cargo build`,
  so a normal `cargo llvm-cov` run (even under the pinned nightly) never compiles the module and
  it can never appear in the coverage report, independent of how well the rest of the crate is
  tested. Run verbatim, the script named it "not instrumented" even though the actual branch
  metric already read 6/6.
- **Fix:** Extended the existing `lib.rs` exclusion (from 02-01) to also exclude `proofs.rs` by
  name, with an inline comment stating the `cfg(kani)` reasoning and flagging that the exclusion
  needs revisiting if `proofs.rs` ever gains code reachable outside `cfg(kani)`. The
  `--ignore-filename-regex` CLI flag itself was left untouched, still naming only
  `crates/stop/tests/`, per the plan's own acceptance criterion.
- **Files modified:** `scripts/nr-coverage.sh`
- **Verification:** Re-ran the gate: exit 0, `branches 6/6  regions 134/137  lines 108/111`.
  Confirmed the identical output on CI (run 34938678761, job `coverage`).
- **Committed in:** `c107b86` (Task 2 commit)

**Total deviations:** 1 auto-fixed (1 bug).
**Impact on plan:** Necessary for the gate to be usable at all against this crate's own
established `cfg(kani)` convention from plan 02-03; without it, the blocking coverage gate would
be permanently red regardless of how well `crates/stop`'s real modules are covered, exactly
mirroring 02-01's `lib.rs` finding for a structurally identical reason. No scope creep: five
lines inside the one file the plan already asked this task to modify.

## Issues Encountered

None blocking. One informational finding, recorded rather than rounded away: `functions`,
`regions` and `lines` do not read 100 percent (94.1 percent / 97.8 percent / 97.3 percent). The
entire gap is one never-called function, `impl Default for EmergencyStop { fn default() -> Self
{ Self::new() } }`, which has zero branches of its own. Since it contributes no branches, it does
not affect the metric this gate enforces, and the plan's own text explicitly forbids writing
tests speculatively against branches a coverage run did not name (`cargo-llvm-cov` 0.9.1 also has
no `--fail-under-lines`/`--fail-under-regions` gate this plan calls for; STOP-06's text and D-53
both say "branch coverage," not lines or functions). Left untested; stated here plainly instead
of being silently absorbed into a rounded "100 percent" claim.

## User Setup Required

None - no external service configuration required. Both new CI jobs use only actions the
workflow already trusted (`actions/checkout`, `actions/cache`, `dtolnay/rust-toolchain`,
`Swatinem/rust-cache`) plus `cargo install --locked` at a pinned version; no new secret, token,
or account setup was needed.

## Next Phase Readiness

- STOP-05 and STOP-06 both close here: both gates exist, are unconditional (no `if:` at job
  level, no `paths:` filter anywhere in `ci.yml`), have each been observed failing on a real,
  deliberately manufactured regression, and are confirmed green on `ubuntu-latest` in a real CI
  run (verified via `gh run list` and `gh run view --log`, not assumed from a local pass alone).
- `rust-toolchain.toml` is unchanged (`git diff --exit-code rust-toolchain.toml` exits 0 at every
  checkpoint in this plan); every other job (`fmt`, `clippy`, `test` on both `ubuntu-latest` and
  `macos-latest`, `deny`) stayed on stable and stayed green across both pushes.
- Plan 02-05 is next: STOP-04's published claim (the `docs/proofs/emergency-stop-proof-scope.md`
  scoping note D-48 asks for) is the one thing this phase still owes before STOP-04 itself can
  close; STOP-05 and STOP-06 no longer block anything downstream.
- No blockers carried forward. Phase 1 remains open on its own track (01-15 task 3) and this plan
  touched nothing under `measurements/` or the Phase 1 planning directory. This plan ran entirely
  on the macOS dev host plus two real CI runs on `ubuntu-latest`; no rig was touched.

*Phase: 02-proven-emergency-stop*
*Completed: 2026-09-15*

## Self-Check: PASSED

- FOUND: scripts/nr-proofs.sh (executable, mode 0755)
- FOUND: .github/workflows/ci.yml (contains `kani:` and `coverage:` job blocks)
- FOUND: scripts/nr-coverage.sh (contains the `proofs.rs` exclusion and the nightly self-check)
- FOUND commit: a5003f5 (feat(02-04): the blocking Kani job and the non-vacuity guard)
- FOUND commit: c107b86 (feat(02-04): the blocking coverage job and its two non-vacuity guards)
- CONFIRMED: both `gh run list` CI runs (34938167522, 34938678761) show every job, including
  `kani` and `coverage`, concluded success on `ubuntu-latest`
