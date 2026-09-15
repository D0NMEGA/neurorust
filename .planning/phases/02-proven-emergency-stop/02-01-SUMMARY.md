---
status: PASS
agent: donny-executor
phase: 02-proven-emergency-stop
plan: 01

subsystem: infra
tags: [kani, cargo-llvm-cov, formal-verification, branch-coverage, nr-stop]

# Dependency graph
requires:
  - phase: 01-trustworthy-measurement
    provides: the Cargo workspace layout, the workspace lint table, deny.toml's license/bans
      configuration, and the per-crate manifest shape (crates/histogram/Cargo.toml) this plan's
      new crate copies
provides:
  - crates/stop (package nr-stop), the sixth workspace member, zero dependencies,
    #![forbid(unsafe_code)] restated at its root (D-49)
  - kani-verifier 0.67.0 installed and confirmed to prove and falsify a statement inside this
    exact workspace (edition 2024, resolver 3), with the exact tool output strings recorded below
  - a pinned, dated nightly (nightly-2026-08-01) confirmed to run cargo-llvm-cov --branch on this
    workspace, with the full JSON totals shape recorded below
  - scripts/nr-coverage.sh, the committed, non-vacuous STOP-06 gate command
affects: [02-02, 02-03, 02-04, 02-05]

# Tech tracking
tech-stack:
  added:
    - "kani-verifier 0.67.0 (cargo install --locked; bundles nightly-2025-11-21 into ~/.kani/,
       independent of rustup)"
    - "cargo-llvm-cov 0.9.1 (cargo install --locked, on stable)"
    - "nightly-2026-08-01 (rustup toolchain, coverage job only, rust-toolchain.toml unchanged)"
  patterns:
    - "Confirmatory Wave-0 spike: written, exercised (pass and fail), then fully deleted inside
       the same task, with the evidence moved into the plan SUMMARY rather than left in the tree"
    - "External tool version pins recorded as exact, quoted output strings rather than assumed"
    - "The STOP-06 gate is enforced by reading the coverage JSON's totals, never by a
       --fail-under-* flag"

key-files:
  created:
    - crates/stop/Cargo.toml
    - crates/stop/src/lib.rs
    - scripts/nr-coverage.sh
  modified:
    - Cargo.toml
    - Cargo.lock

key-decisions:
  - "Kani pinned at 0.67.0 exactly as researched; cargo install --locked succeeded on the first attempt, no substitution needed"
  - "Coverage nightly pinned at nightly-2026-08-01, the first candidate tried; every check passed without needing to step forward a week"
  - "cargo-llvm-cov 0.9.1 has no --fail-under-branches flag at all (only --fail-under-functions/-lines/-file-lines/-regions), which independently reinforces the plan's own JSON-read enforcement design"
  - "Excluded lib.rs by name from nr-coverage.sh's missing-file check (Rule 1 fix): a function-free file can never appear in an LLVM source-coverage report, so requiring its presence made the gate permanently red against a fully covered crate"
  - "requirements-completed left empty for STOP-04/05/06 despite appearing in this plan's own frontmatter, matching this project's established precedent (01-08, 01-09, 01-18, 01-20 through 01-25): this plan confirms the tools work on a throwaway spike, it does not implement the FSM (STOP-04), the CI gate (STOP-05), or coverage of a real module (STOP-06)"

patterns-established:
  - "nr-coverage.sh is the single, committed definition of the STOP-06 gate: cargo-llvm-cov --branch under a pinned dated nightly, threshold enforced by parsing the JSON export, not a CLI flag"
  - "Wave 0 spikes are created, exercised to both a passing and a falsified result, and fully deleted within the task that needed them"

requirements-completed: []

# Metrics
duration: 14min
completed: 2026-09-15
---

# Phase 2 Plan 1: Crate scaffold and tool confirmation Summary

**`nr-stop` scaffolded as the sixth workspace member; Kani 0.67.0 and a pinned nightly's `cargo-llvm-cov --branch` both confirmed working end to end in this exact workspace, with the gate command committed as `scripts/nr-coverage.sh`.**

## Performance

- **Duration:** 14 min
- **Started:** 2026-09-15T05:30:27Z
- **Completed:** 2026-09-15T05:44:38Z
- **Tasks:** 3 (Task 2 produced no commit; see below)
- **Files modified:** 5 (2 created source files, 1 created script, 2 modified manifest files)

## Accomplishments

- `crates/stop` (package `nr-stop`) exists as the sixth workspace member: zero dependencies,
  `#![forbid(unsafe_code)]` restated on line 1 of `src/lib.rs` ahead of the crate doc comment
  (D-49)
- Kani 0.67.0 installed (`cargo install --locked kani-verifier@0.67.0` plus `cargo kani setup`)
  and confirmed, via a deliberately created and deleted spike, to both verify a true statement
  and falsify a false one inside this workspace's actual `edition = "2024"` / `resolver = "3"`
  configuration
- `nightly-2026-08-01` confirmed (first candidate tried, no rejections) to build this workspace
  and to run `cargo-llvm-cov --branch` correctly: a one-branch spike read 50 percent covered with
  one test and 100 percent with both, and `--branch` produced no error on `aarch64-apple-darwin`
- `scripts/nr-coverage.sh` committed: the single, executable (0755) definition of the STOP-06
  gate, verified to exit 0 on a fully-covered spike and exit 1 (naming the file) when coverage
  regresses
- `rust-toolchain.toml` confirmed byte-identical throughout; both new toolchains live outside
  rustup's default selection

## Task Commits

1. **Task 1: Create the nr-stop package and add it to the workspace** - `e42b391` (feat)
2. **Task 2: Install Kani at a pinned version and confirm it proves something in this workspace** - no commit (the spike is created and deleted within the task by design; `crates/stop/src/spike.rs` and the `mod spike;` line in `lib.rs` both left zero net diff, so there was nothing to commit. All evidence is recorded in this SUMMARY.)
3. **Task 3: Pin a nightly, confirm branch coverage works, and commit the gate command** - `5717b50` (chore)

_Note: Task 2's absence of a commit is not a gap — its own `<files>` field in the plan states the spike files are "created and deleted within this task," and its acceptance criteria are file-existence and version checks, not a diff to commit._

## Files Created/Modified

- `crates/stop/Cargo.toml` - the `nr-stop` package manifest, no `[dependencies]` section
- `crates/stop/src/lib.rs` - crate root; `#![forbid(unsafe_code)]` then the crate doc comment
- `scripts/nr-coverage.sh` - the committed STOP-06 gate command, `nightly-2026-08-01` baked in
- `Cargo.toml` - `crates/stop` added to the workspace `members` list
- `Cargo.lock` - regenerated for the new workspace member

## Tool facts for later plans

### Kani

- **Version, verbatim (`cargo kani --version`):**
  ```
  cargo-kani 0.67.0
  ```
- **Per-harness start line, verbatim, copied character for character from the spike run:**
  ```
  Checking harness spike::spike_double_never_overflows...
  ```
- **Passing verdict, verbatim (final line and the per-check result):**
  ```
  Check 2: spike::spike_double_never_overflows.assertion.1
  	 - Status: SUCCESS
  	 - Description: "assertion failed: doubled <= 510"
  	 - Location: crates/stop/src/spike.rs:14:5 in function spike::spike_double_never_overflows

  SUMMARY:
   ** 0 of 2 failed

  VERIFICATION:- SUCCESSFUL
  Verification Time: 0.34261858s

  Manual Harness Summary:
  Complete - 1 successfully verified harnesses, 0 failures, 1 total.
  ```
- **Failing verdict, verbatim, produced by deliberately changing the spike's assertion to
  `<= 509` (a false bound) and re-running the identical harness:**
  ```
  Check 2: spike::spike_double_never_overflows.assertion.1
  	 - Status: FAILURE
  	 - Description: "assertion failed: doubled <= 509"
  	 - Location: crates/stop/src/spike.rs:14:5 in function spike::spike_double_never_overflows

  SUMMARY:
   ** 1 of 2 failed
  Failed Checks: assertion failed: doubled <= 509
   File: "crates/stop/src/spike.rs", line 14, in spike::spike_double_never_overflows

  VERIFICATION:- FAILED
  Verification Time: 0.007553167s

  Manual Harness Summary:
  Verification failed for - spike::spike_double_never_overflows
  Complete - 0 successfully verified harnesses, 1 failures, 1 total.
  ```
  Plan 02-04's CI guard can therefore key on `VERIFICATION:- SUCCESSFUL` /
  `VERIFICATION:- FAILED` (both confirmed to appear verbatim in both directions) and, if a
  per-harness signal is wanted, the `Complete - N successfully verified harnesses, M failures,
  T total.` line, which differs by wording (`successfully verified` vs `failed for`) as well as
  by count.
- **`--output-format`:** exists (`cargo kani --help`). Default `regular`; other values `terse`
  and `old`. All three are text presentation styles; none is a structured (JSON or similar)
  format. Plan 02-03 should plan on parsing the regular text output (as shown above), not a
  machine-readable export.
- **`[package.metadata.kani]` table:** NOT needed. `crates/stop/Cargo.toml` carries no such
  table (see the file as committed in `e42b391`) and `cargo kani -p nr-stop` ran the spike
  harness correctly with none present. Plan 02-03 should not add one speculatively.
- **Wall clock:** the full `cargo kani -p nr-stop` invocation (cold, including the 0.73s
  `cargo build` compile step) took 3 seconds end to end on this machine. Kani's own internally
  reported `Verification Time` (the solver phase alone) was 0.343s on the passing run and
  0.0076s on the failing run.
- **Install commands used, exactly:**
  ```sh
  cargo install --locked kani-verifier@0.67.0
  cargo kani setup
  ```
  `cargo kani setup` output confirmed it installs `nightly-2025-11-21-aarch64-apple-darwin` into
  `~/.kani/kani-0.67.0` via its own bundled rustup-driven step; `rust-toolchain.toml` was
  confirmed byte-identical (`git diff --exit-code rust-toolchain.toml`) both immediately after
  install and again at the end of the plan.

### Coverage (cargo-llvm-cov)

- **Nightly candidates tried:** one. `nightly-2026-08-01` was the starting candidate per the
  plan's own instruction and it passed every check on the first attempt (workspace build,
  `--branch` acceptance, correct 50 percent -> 100 percent discrimination). No candidate was
  rejected and no stepping-forward was needed.
- **`cargo-llvm-cov` version pinned:** 0.9.1, `cargo install --locked cargo-llvm-cov@0.9.1` (run
  once, on stable, per the plan; used from the pinned nightly via `cargo +nightly-2026-08-01
  llvm-cov`).
- **`--fail-under-*` flags that exist in this version (`cargo +nightly-2026-08-01 llvm-cov
  --help`):** `--fail-under-functions`, `--fail-under-lines`, `--fail-under-file-lines`,
  `--fail-under-regions`. **There is no `--fail-under-branches` flag at all in cargo-llvm-cov
  0.9.1.** This is a stronger fact than the plan's own script comment assumed (which reads as
  though `--fail-under-branches` exists but is simply not used); it does not exist to use. This
  independently confirms the JSON-read design is not just preferred but necessary.
- **`--branch` on `aarch64-apple-darwin`:** accepted, no error. Only a `warning: --branch option
  is unstable` diagnostic, exactly as documented.
- **`data[0].totals`, verbatim, from the one-test spike (only `even_is_even` present):**
  ```json
  {
    "branches": { "count": 2, "covered": 1, "notcovered": 1, "percent": 50 },
    "functions": { "count": 1, "covered": 1, "percent": 100 },
    "instantiations": { "count": 2, "covered": 1, "percent": 50 },
    "lines": { "count": 3, "covered": 3, "percent": 100 },
    "mcdc": { "count": 0, "covered": 0, "notcovered": 0, "percent": 0 },
    "regions": { "count": 5, "covered": 4, "notcovered": 1, "percent": 80 }
  }
  ```
- **`data[0].totals`, verbatim, from the two-test spike (`even_is_even` plus `odd_is_not_even`):**
  ```json
  {
    "branches": { "count": 2, "covered": 2, "notcovered": 0, "percent": 100 },
    "functions": { "count": 1, "covered": 1, "percent": 100 },
    "instantiations": { "count": 2, "covered": 1, "percent": 50 },
    "lines": { "count": 3, "covered": 3, "percent": 100 },
    "mcdc": { "count": 0, "covered": 0, "notcovered": 0, "percent": 0 },
    "regions": { "count": 5, "covered": 5, "notcovered": 0, "percent": 100 }
  }
  ```
  Note the object carries a `notcovered` key alongside `count`/`covered`/`percent` on `branches`
  and `regions` (not just the three keys the plan's own text named); plan 02-04's gate can use
  `notcovered` directly rather than subtracting.
- **`data[N].files[].filename`:** an **absolute path**
  (e.g. `/Users/d0nmega/Developer/neurorust/crates/stop/src/spike.rs`), not repo-relative.
  `scripts/nr-coverage.sh`'s use of `pathlib.Path(f["filename"]).name` (basename only) is
  unaffected by this either way; recorded so plan 02-04 does not assume a relative path if it
  reads the same field for a different purpose.
- **The covered/uncovered discrimination the plan asked to confirm:** with only `even_is_even`,
  `branches.percent` read 50 (not 100). Adding `odd_is_not_even` brought it to 100. The gate can
  tell a covered branch from an uncovered one.
- **Gate script behaviour, confirmed by direct execution:**
  - Two tests present: exit 0, stdout `branches 2/2  regions 5/5  lines 3/3`
  - One test present (the other deleted): exit 1, stderr
    `nr-coverage.sh: 1 of 2 branches uncovered` plus
    `nr-coverage.sh:   /Users/d0nmega/Developer/neurorust/crates/stop/src/spike.rs: 1 of 2`,
    stdout `branches 1/2  regions 4/5  lines 3/3`

## Decisions Made

See `key-decisions` in the frontmatter. The one substantive engineering decision (excluding
`lib.rs` from the coverage gate's missing-file check) is documented in full under Deviations
below, since it was a fix to the plan's own prescribed script text, not a free-standing choice.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `nr-coverage.sh`'s missing-file check flagged a legitimately code-free `lib.rs` as "not instrumented," making the gate permanently red**

- **Found during:** Task 3, on the very first run of the script against the fully-covered
  two-test spike (which the plan's own text expects to exit 0)
- **Issue:** The plan's script glob-checks every `crates/stop/src/*.rs` file against the
  coverage report's file list. `crates/stop/src/lib.rs` is a doc comment plus a `pub mod`
  declaration with no function of its own (the same shape already established by
  `crates/histogram/src/lib.rs`). LLVM's source-based coverage instrumentation only emits a
  report entry for a file that contains at least one coverable region, which requires at least
  one function; a file with none can never appear in `data[].files`, regardless of how much of
  the crate is actually covered. Run verbatim, the plan's script exited 1 on a spike that was, in
  fact, 100 percent branch-covered, citing `lib.rs` as "not instrumented."
- **Fix:** Excluded `lib.rs` by name from the required-files list in the Python check, with a
  comment stating why (no function means no possible branch, confirmed empirically before the
  comment was written) and flagging explicitly that the exclusion needs revisiting if `lib.rs`
  ever gains real logic of its own.
- **Files modified:** `scripts/nr-coverage.sh`
- **Verification:** Re-ran the two-test spike (exit 0, `branches 2/2`) and the one-test spike
  (exit 1, correctly naming `spike.rs`, not `lib.rs`, as the uncovered file)
- **Committed in:** `5717b50` (Task 3 commit)

**Total deviations:** 1 auto-fixed (1 bug).
**Impact on plan:** Necessary for the gate to be usable at all against this crate's own
established `lib.rs` convention; without it, plan 02-04's blocking CI gate would be permanently
red regardless of how well `crates/stop`'s real modules are covered. No scope creep: the fix is
four lines inside the one file the plan already asked this task to create.

## Issues Encountered

- One of Task 3's own literal acceptance criteria (`grep -q 'fail-under' scripts/nr-coverage.sh`
  outputs nothing) is satisfied in substance but not in the strictest literal sense: the actual
  `cargo llvm-cov` invocation in the committed script never passes any `--fail-under-*` flag
  (confirmed by inspecting the invocation line directly), but the word `fail-under` does appear
  in an explanatory comment, both in the plan's own prescribed script text and in this session's
  necessary addition documenting that `--fail-under-branches` does not exist in this version.
  `grep -q` alone never prints to stdout regardless of match; the intended check (no `--fail-under`
  flag is actually invoked) was verified directly rather than by the literal grep. Not treated as
  a defect to fix, since removing the explanatory comment would reduce clarity for a future
  reader for no behavioural gain.

## User Setup Required

None - no external service configuration required. Tool installs (`kani-verifier`,
`cargo-llvm-cov`, the `nightly-2026-08-01` toolchain) were performed directly by this agent under
this machine's standing tool-install authority; nothing requires a human account, API key, or
manual dashboard step.

## Next Phase Readiness

- `crates/stop` exists and is ready for plan 02-02 to add the actual state machine, event types,
  latch, and gate.
- Kani and the coverage nightly are both installed and confirmed against this exact workspace;
  plans 02-03 and 02-04 can write real harnesses and the CI gate directly from the tool facts
  recorded above without re-deriving them.
- `docs/proofs/` does not exist yet; it is plan 02-05's responsibility (D-48), not this plan's.
- No blockers. Phase 1 remains open on its own track (01-15 task 3) and this plan touched
  nothing under `measurements/` or the Phase 1 planning directory.

*Phase: 02-proven-emergency-stop*
*Completed: 2026-09-15*

## Self-Check: PASSED

- FOUND: crates/stop/Cargo.toml
- FOUND: crates/stop/src/lib.rs
- FOUND: scripts/nr-coverage.sh (executable)
- CONFIRMED ABSENT: crates/stop/src/spike.rs
- FOUND commit: e42b391 (feat(02-01): add nr-stop as the sixth workspace member)
- FOUND commit: 5717b50 (chore(02-01): commit the STOP-06 branch coverage gate command)
