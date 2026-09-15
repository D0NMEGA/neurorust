# Emergency-stop proof report

## What this report is

The committed result of a Kani solver run over crates/stop (package nr-stop), and the evidence behind STOP-04. The crate contains no unsafe code, so most classes of undefined behaviour are already excluded by the type system before Kani runs. What this report adds is a totality proof over the enumerated state space, the latch property, the gate invariant, and panic and overflow freedom, harness by harness below.

## Provenance

- Kani version: cargo-kani 0.67.0
- Generated from commit: 2de5ccdf7c8faa050bb254e7489c8dd478521696 (dirty)
- Wall clock: 1s

## Harnesses

One row per harness: which D-46 family it belongs to, which requirement it
serves, and its outcome, in Kani's own verdict vocabulary.

| Harness | Family | Requirement | Outcome |
|---|---|---|---|
| step_is_total | 1 | STOP-01 | VERIFICATION:- SUCCESSFUL |
| state_raw_round_trips | 1 | STOP-01 | VERIFICATION:- SUCCESSFUL |
| state_from_raw_is_total_and_fails_closed | 1 | STOP-01 | VERIFICATION:- SUCCESSFUL |
| cause_raw_round_trips_and_is_total | 1 | STOP-01 | VERIFICATION:- SUCCESSFUL |
| abort_never_returns_to_running | 2 | STOP-02 | VERIFICATION:- SUCCESSFUL |
| running_is_never_reachable_from_stopping_or_stopped | 2 | STOP-02 | VERIFICATION:- SUCCESSFUL |
| stopped_is_terminal | 2 | STOP-02 | VERIFICATION:- SUCCESSFUL |
| no_permit_when_not_running | 3 | STOP-03 | VERIFICATION:- SUCCESSFUL |
| permit_supply_closes_on_the_abort_edge | 3 | STOP-03 | VERIFICATION:- SUCCESSFUL |
| modelled_consumer_never_emits_after_a_stop | 3 | STOP-03 | VERIFICATION:- SUCCESSFUL |
| consumer_emit_never_overflows | 4 | STOP-04 | VERIFICATION:- SUCCESSFUL |

## Checks

132 checks reported across every harness above, 132 succeeded.

## Verdict

The final line of the run, verbatim:

    Complete - 11 successfully verified harnesses, 0 failures, 11 total.

## What this report does not say

This report covers the sequential transition function and the gate predicate. It says nothing about thread interleavings: crate::latch's atomic publication layer is unverified here and stays unverified until CHAN-06 brings loom into CI in Phase 4. See docs/proofs/emergency-stop-proof-scope.md for the full statement of what is proved, what the type system already excludes, and what nothing in this phase covers.

## Regenerating this report

    sh scripts/nr-proof-report.sh

This command re-runs the solver and rewrites this file. It does not run in CI and is not a gate; it is refreshed deliberately and reviewed like any other published claim, per D-07.
