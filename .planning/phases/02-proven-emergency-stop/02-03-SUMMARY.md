---
status: PASS
agent: donny-executor
phase: 02-proven-emergency-stop
plan: 03

subsystem: runtime
tags: [kani, formal-verification, emergency-stop, nr-stop, proof-report]

# Dependency graph
requires:
  - phase: 02-proven-emergency-stop
    plan: 01
    provides: Kani 0.67.0 installed and confirmed to prove and falsify a statement in this exact
      workspace, with the exact per-harness and verdict output strings recorded for reuse
  - phase: 02-proven-emergency-stop
    plan: 02
    provides: the full nr-stop public API (State/step, Cause/Event, OutputPermit/
      would_issue_permit, ModelledConsumer, EmergencyStop/AbortRecord) exactly matching this
      plan's own interfaces block, plus workspace.lints.rust.unexpected_cfgs already declaring
      cfg(kani)
provides:
  - crates/stop/src/proofs.rs, eleven cfg(kani) harnesses across D-46's four families, all
    verifying, none touching an atomic, a compare-exchange or an Ordering value
  - docs/proofs/, a new directory (D-48) holding the D-54 committed proof report
  - scripts/nr-proof-report.sh, the committed generator that renders the report from a real
    cargo kani run and exits non-zero (report still written) if the run fails
  - STOP-01 through STOP-04 as mechanically established: totality, no path back to Running, no
    permit outside Running, and panic/overflow freedom over the enumerated space
affects: ["02-04", "02-05"]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "A hand-maintained harness-to-(family, requirement) mapping lives inside the report
       generator rather than being inferred from harness names at render time, so a harness
       added without a matching row renders as an obviously incomplete row instead of a guess"
    - "A report generator that still writes its output and marks the failing rows on a bad run,
       then propagates the real tool's exit code, rather than skipping the write on failure"
    - "sh outer script plus a python3 heredoc for structured text parsing, matching
       scripts/nr-coverage.sh's own established shape in this repository"

key-files:
  created:
    - crates/stop/src/proofs.rs
    - scripts/nr-proof-report.sh
    - docs/proofs/emergency-stop-proof-report.md
  modified:
    - crates/stop/src/lib.rs

key-decisions:
  - "Wrote eleven #[kani::proof] harnesses, not ten: the plan's own action text spells out ten
     named harnesses across families 1 through 3 plus a separately-named eleventh
     (consumer_emit_never_overflows) for family 4, and its own acceptance criteria explicitly
     pre-authorizes this exact outcome ('the eleventh... brings the count to eleven if written
     separately; if the executor lands a different final count... the SUMMARY must state the
     exact final list'). The literal first acceptance bullet ('outputs 10') is satisfied in
     spirit, not in the number: this is that pre-authorized documentation, not a deviation."
  - "requirements-completed now includes STOP-01 through STOP-04, closing the gap 02-02 left
     open on its own stated reasoning: D-46 says the Kani harnesses are what establishes
     STOP-01/02/03 mechanically, and STOP-04 needed the panic/overflow-freedom harness family
     that did not exist until this plan. All four now have their proof behind them."

requirements-completed: [STOP-01, STOP-02, STOP-03, STOP-04]

# Metrics
duration: 21min
completed: 2026-09-15
---

# Phase 2 Plan 3: The four Kani harness families and the committed proof report Summary

**Eleven Kani harnesses over crates/stop's whole enumerated state space (all verifying, 132/132 checks, none touching an atomic) plus a generated, committed proof report naming the Kani version, the source commit, every harness and its outcome.**

## Performance

- **Duration:** 21 min
- **Started:** 2026-09-15T06:17:00Z
- **Completed:** 2026-09-15T06:32:46Z
- **Tasks:** 2
- **Files modified:** 4 (3 created, 1 modified)

## Accomplishments

- `crates/stop/src/proofs.rs`: eleven `#[kani::proof]` harnesses across D-46's four families,
  every one written over `kani::any()` inputs (three narrowed with `kani::assume`, never over a
  chosen constant). `cargo kani -p nr-stop` reports `VERIFICATION:- SUCCESSFUL` once per harness,
  zero `VERIFICATION:- FAILED`, `Complete - 11 successfully verified harnesses, 0 failures, 11
  total.`, 132 of 132 individual checks succeeded, in 1.4 to 1.5 seconds wall clock end to end
  (cold or warm; see Tool facts below).
- No harness touches `Atomic`, `compare_exchange` or `Ordering::` (checked by grep, zero
  matches). `mod proofs` is gated `#[cfg(kani)]` in `lib.rs`, so the eleven harnesses are invisible
  to `cargo build`, `cargo test` and `cargo clippy`; `cargo test -p nr-stop` still reports exactly
  the same 27 tests as before this plan (0 lib, 5 gate, 12 latch, 10 state).
- Family 4's `consumer_emit_never_overflows` constructs an `OutputPermit` directly via its
  `pub(crate)` field, from inside the crate, rather than adding a public constructor that would
  have reopened T-2-09 from plan 02-02's threat model; `gate.rs` still has no `pub fn new` (grep
  confirms zero matches).
- `docs/proofs/` created (D-48), holding `emergency-stop-proof-report.md`: the Kani version
  verbatim, the generating commit and its clean/dirty state, a table of all eleven harnesses
  against the D-46 family and requirement each serves (an explicit mapping inside the generator,
  never inferred from the harness name), the check totals, the final verdict line verbatim, the
  wall clock, and a pointer to the not-yet-written `emergency-stop-proof-scope.md` stating the
  proof covers no thread interleaving.
- `scripts/nr-proof-report.sh` (0755): runs the solver, renders the report even on a failing run
  (marking the failing rows, propagating the real exit code), and is not wired into CI, matching
  D-54's "refreshed deliberately, reviewed like any other published claim" design.
- The report attributes undefined-behaviour exclusion to the type system, never to Kani,
  matching D-47's correction; verified directly, not just by grep, by reading the rendered
  sentence.

## Task Commits

1. **Task 1: The four Kani harness families** - `2de5ccd` (feat)
2. **Task 2: Generate and commit the proof report** - `b494c6d` (feat)

## Files Created/Modified

- `crates/stop/src/proofs.rs` - eleven Kani proof harnesses across D-46's four families (155
  lines)
- `crates/stop/src/lib.rs` - added `#[cfg(kani)] mod proofs;`, alphabetically between `latch` and
  `state`
- `scripts/nr-proof-report.sh` - the committed, executable (0755) D-54 report generator
- `docs/proofs/emergency-stop-proof-report.md` - the generated, committed proof report

## Harnesses

The exact final list, one per line, as recorded in `crates/stop/src/proofs.rs` and required by
plan 02-04's CI guard:

```
step_is_total
state_raw_round_trips
state_from_raw_is_total_and_fails_closed
cause_raw_round_trips_and_is_total
abort_never_returns_to_running
running_is_never_reachable_from_stopping_or_stopped
stopped_is_terminal
no_permit_when_not_running
permit_supply_closes_on_the_abort_edge
modelled_consumer_never_emits_after_a_stop
consumer_emit_never_overflows
```

## Tool facts for later plans

- **`cargo kani -p nr-stop` wall clock, three separate invocations against the final committed
  `proofs.rs`:** 1.437s (warm rerun), 1.529s (cold, after `rm -rf target/kani`), 1.466s (final
  confirmation run after the doc-comment fix below). D-52's "a three-state FSM solves in seconds"
  argument for an unconditional blocking gate has a real number behind it: comfortably
  sub-two-seconds even cold.
- **Per-harness solver time (Kani's own `Verification Time`, from the final confirmation run),
  fastest to slowest:** `consumer_emit_never_overflows` 0.0073s, `no_permit_when_not_running`
  0.0079s, `state_from_raw_is_total_and_fails_closed` 0.0079s, `state_raw_round_trips` 0.0082s,
  `stopped_is_terminal` 0.0108s, `step_is_total` 0.0118s, `cause_raw_round_trips_and_is_total`
  0.0121s, `permit_supply_closes_on_the_abort_edge` 0.0099s, `abort_never_returns_to_running`
  0.0143s, `modelled_consumer_never_emits_after_a_stop` 0.0149s,
  `running_is_never_reachable_from_stopping_or_stopped` 0.0160s. Sum of all eleven: about 0.121s;
  the remaining 1.3 to 1.4s of wall clock is `cargo build`/`kani-driver` overhead, not solving.
- **Per-harness check counts** (Kani's own `** X of Y failed` line, all `X=0`): 19, 9, 2, 13, 14,
  14, 15, 15, 15, 13, 3, summing to 132 total checks, all 132 succeeded. Most of each harness's
  own check count is Kani's own automatic `pointer_dereference` and `PartialEq::eq` internal
  checks around the derived `Arbitrary`/`PartialEq` impls, not hand-written assertions; this is
  normal and expected, not a sign of hidden complexity in the harnesses themselves.
- **Kani's per-harness section format, confirmed against this crate's own multi-harness run (not
  just the single-harness spike 02-01 recorded):** each section starts with `Checking harness
  proofs::<name>...`, ends with its own `SUMMARY:` / `** X of Y failed` / `VERIFICATION:-
  SUCCESSFUL|FAILED` block, and the whole run ends with one final `Complete - N successfully
  verified harnesses, M failures, T total.` line. No separate aggregate `VERIFICATION:-` line
  exists beyond the per-harness ones; `grep -c 'VERIFICATION:- SUCCESSFUL'` equals the harness
  count exactly, confirmed both here (11) and in 02-01's single-harness spike (1).
- **`[package.metadata.kani]`:** still not needed; `crates/stop/Cargo.toml` was not touched by
  this plan and `cargo kani -p nr-stop` ran all eleven harnesses correctly with none present.

## Decisions Made

See `key-decisions` in the frontmatter for the eleven-versus-ten harness count and the
`requirements-completed` reasoning. Both are documented in full there rather than repeated here.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] The plan's own literal module-doc text collides with the plan's own acceptance grep**

- **Found during:** Task 1, writing `proofs.rs`'s module doc. The plan's own `<action>` text
  reads, verbatim: "...no harness in this file may touch an atomic: research surfaced a report of
  a Kani build modelling a failed compare_exchange as successful..." and directs the module doc
  to state that reasoning.
- **Issue:** Incorporating that sentence put the literal substring `compare_exchange` into
  `crates/stop/src/proofs.rs`, which collides with this same task's own acceptance criterion
  (`grep -qE 'Atomic|compare_exchange|Ordering::' crates/stop/src/proofs.rs` expecting no match)
  and the plan's own top-level `<verification>` block's identical grep. Caught by running the
  grep myself rather than trusting the doc comment read correctly by eye.
- **Fix:** Reworded the sentence to "a compare-exchange or an Ordering value" and "a failed
  compare-exchange as successful" (hyphenated, spelled out), preserving identical technical
  content and the same cited GitHub issue, without the colliding literal token. Matches this
  project's own established precedent for the same class of self-collision: 02-02-SUMMARY's
  deviation 3 (the `compare_exchange(AcqRel, Acquire)` prose collision in `latch.rs`'s module
  doc) and STATE.md's 01-14 entry.
- **Files modified:** `crates/stop/src/proofs.rs`
- **Verification:** `grep -nE 'Atomic|compare_exchange|Ordering::' crates/stop/src/proofs.rs`
  returns no match; re-ran `cargo kani -p nr-stop` after the wording change and it still reports
  11/11 successful (a doc-comment change cannot affect compiled behavior, confirmed rather than
  assumed).
- **Committed in:** `2de5ccd` (Task 1 commit; the fix was made before the task's own commit, so
  the collision never reached a commit)

**Total deviations:** 1 auto-fixed (1 bug, self-caught before commit).
**Impact on plan:** The fix is a wording-only change to a doc comment; no harness's assertions,
names or behavior changed. No scope creep.

## Issues Encountered

- Task 1's own acceptance criteria contain an internal tension the plan text names and resolves
  itself: the first bullet reads `grep -c '#\[kani::proof\]' crates/stop/src/proofs.rs` outputs
  `10`, while the very next bullet says the eleventh harness "brings the count to eleven if
  written separately" and instructs the SUMMARY to "state the exact final list" if the count
  differs. The plan's own `<action>` text writes out ten named harnesses for families 1 through 3
  plus a separately-boxed eleventh code sample for family 4 (`consumer_emit_never_overflows`),
  which is what was implemented: eleven harnesses, `grep -c` reads `11`. This is the
  pre-authorized outcome the plan's own second bullet describes, not a defect; the exact final
  list is recorded above under "Harnesses" per that same instruction.

## User Setup Required

None. No external service, credential, or manual dashboard step is needed; Kani was already
installed and confirmed working by plan 02-01, and this plan touched nothing under
`measurements/` or the rig.

## Next Phase Readiness

- The eleven harness names are fixed and recorded above; plan 02-04's CI guard can grep for each
  one by name directly from this list.
- `docs/proofs/` exists; plan 02-05 adds `emergency-stop-proof-scope.md` (already referenced by
  both this plan's report and `lib.rs`'s own doc comment since plan 02-02) and the Phase 6 output
  gate contract doc alongside it.
- `scripts/nr-proof-report.sh` is committed but intentionally not wired into any CI workflow; that
  stays true unless a future plan explicitly decides otherwise (D-54 is deliberate about this).
- STOP-01 through STOP-04 are now marked complete in `REQUIREMENTS.md`. STOP-05 (the blocking CI
  gate) and STOP-06 (branch coverage in CI) remain for plan 02-04; STOP-07 (the rig measurement)
  remains for plans 02-06 through 02-09.
- No blockers carried forward from this plan. Phase 1 remains open on its own track (01-15 task 3)
  and this plan touched nothing under `measurements/` or the Phase 1 planning directory.

*Phase: 02-proven-emergency-stop*
*Completed: 2026-09-15*

## Self-Check: PASSED

- FOUND: crates/stop/src/proofs.rs
- FOUND: crates/stop/src/lib.rs
- FOUND (executable, mode 0755): scripts/nr-proof-report.sh
- FOUND: docs/proofs/emergency-stop-proof-report.md
- FOUND commit: 2de5ccd (feat(02-03): the four Kani harness families)
- FOUND commit: b494c6d (feat(02-03): generate and commit the D-54 proof report)
