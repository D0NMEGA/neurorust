---
status: PASS
agent: donny-executor
phase: 02-proven-emergency-stop
plan: 05

subsystem: proof-docs
tags: [kani, proof-scope, phase-6-handoff, coverage, checkpoint, d-07]

# Dependency graph
requires:
  - phase: 02-proven-emergency-stop
    plan: 03
    provides: the eleven Kani harnesses in crates/stop/src/proofs.rs and the D-54 generated
      proof report (docs/proofs/emergency-stop-proof-report.md) this plan's scope note names
      by harness and cross-references by filename
  - phase: 02-proven-emergency-stop
    plan: 04
    provides: the coverage figures this plan's Coverage section states in full (branches 6/6,
      functions 16/17, regions 134/137, lines 108/111) and the EmergencyStop::default() finding
      this plan's required addition records
provides:
  - docs/proofs/emergency-stop-proof-scope.md, the D-48 published scope note, human-reviewed
    and approved, naming what the proof establishes, what the compiler establishes, and what
    nothing in this phase establishes
  - docs/proofs/phase-6-output-gate-contract.md, the D-45 written contract binding the real
    Phase 6 decoder and sink to the obligations the FSM-level proof assumed
  - docs/proofs/README.md, what docs/proofs/ is, what is in it now, and why proof results stay
    outside measurements/
  - STOP-04 closed: both the checkbox and the traceability table row
affects: ["phase-4-CHAN-06", "phase-6-decoder-and-sink"]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "A published claim about what a proof does and does not establish gets a dedicated
       document and a blocking human-verify checkpoint before the commit that publishes it
       (D-07), rather than a paragraph folded into an existing README"
    - "A checkpoint approval that requires a change is implemented as its own commit, separate
       from the artifacts under review, with the before and after sentence recorded in the
       plan's SUMMARY, so a correction to a published claim stays part of the visible record"

key-files:
  created:
    - docs/proofs/emergency-stop-proof-scope.md
    - docs/proofs/phase-6-output-gate-contract.md
    - docs/proofs/README.md
  modified:
    - .planning/REQUIREMENTS.md

key-decisions:
  - "D-07 precedent, recorded here for every future checkpoint that reviews a published claim
     in this project: 'reaches main' means reaching the shared remote, not the local main
     branch. Tasks 1 and 2 were committed locally (080c099, 617c2da) before task 3's human
     review, and stay exactly as committed, unamended and unsquashed, because origin/main is
     behind by design and neither commit had reached it when review happened. Plan 01-03
     already worked this way without the question being asked explicitly (its Task 3 commit
     landed before the human read it, and a continuation session added a second commit
     afterward); this plan is where the reading is written down."
  - "The required coverage addition landed as its own commit (017a02b), separate from tasks 1
     and 2, because it originates from the checkpoint's finding rather than from either task's
     own acceptance criteria. Matches this project's one-fix-one-commit discipline."
  - "STOP-03 is not listed in requirements-completed below despite appearing in this plan's own
     `requirements` frontmatter field. Plan 02-03 already marked it Complete (STATE.md:
     'requirements-completed now covers STOP-01 through STOP-04'). This plan's contract
     document states what Phase 6 must do to keep STOP-03's guarantee; it does not newly
     complete STOP-03 itself. Matches this project's established precedent (02-02, 01-24) of
     leaving a requirement out of requirements-completed when this plan's own work is not what
     completes it."

patterns-established:
  - "Checkpoint review against a named, enumerated test list (the plan's own <how-to-verify>
     block) rather than an open-ended re-read: each of the four tests either passes silently or
     returns a finding naming document, sentence and defect, matching this plan's own
     acceptance criteria for task 3."

# REQUIRED - copy ALL requirement IDs from this plan's `requirements` frontmatter field.
requirements-completed: [STOP-04]

# Metrics
duration: ~15min active (2 sessions; ~8h53m checkpoint pause between them; see Performance note)
completed: 2026-09-15
---

# Phase 2 Plan 5: The published proof scope note and the Phase 6 output gate contract Summary

**The D-48 proof-scope note, the D-45 Phase 6 output gate contract, and docs/proofs/README.md, human-reviewed and approved with one required addition (function/region/line coverage stated beside the 100 percent branch figure); STOP-04 closes.**

## Performance

- **Duration:** ~15 min of active agent execution across two sessions, separated by an
  approximately 8h53m checkpoint pause
- **Session 1 (Tasks 1-2):** commits landed 2026-09-15T07:05:17Z (`080c099`) and
  2026-09-15T07:07:16Z (`617c2da`), starting immediately after 02-04 completed at
  2026-09-15T06:52:35Z
- **Checkpoint pause (Task 3, `checkpoint:human-verify`):** 2026-09-15T07:07:16Z to the human
  operator's approval, recorded as occurring 2026-09-15 per the resume instructions; the exact
  approval timestamp was not independently captured in this record
- **Session 2 (Task 3 continuation, this agent):** verified the prior two commits, re-ran
  `scripts/nr-coverage.sh` for current figures, made the required addition, landed it at
  2026-09-15T16:00:29Z (`017a02b`), then closed out STOP-04 and the planning artifacts
- **Tasks:** 3 (all complete: 2 `auto`, 1 `checkpoint:human-verify`)
- **Files touched:** 7 across the whole plan (3 created in tasks 1-2; `emergency-stop-proof-scope.md`
  modified once more for the required addition; `REQUIREMENTS.md`, `STATE.md`, `ROADMAP.md` and
  this summary in close-out)

## Accomplishments

- `docs/proofs/emergency-stop-proof-scope.md` (D-48): six sections in the required order, every
  one of the eleven Kani harnesses named by its real name from `crates/stop/src/proofs.rs`, the
  interleaving gap naming `latch.rs`, the specific orderings, `loom` and `CHAN-06`, `CREU-01`
  named as the deferred full functional proof, and the `compare_exchange` modelling report named
  as the concrete reason the proved surface avoids atomics entirely.
- `docs/proofs/phase-6-output-gate-contract.md` (D-45): five obligations on the real Phase 6
  consumer, each naming the failure it prevents; explicitly forbids treating `Stopping` as still
  open and forbids describing `acknowledge` as the moment the system becomes safe.
- `docs/proofs/README.md`: states what `docs/proofs/` is, what is in it now, what Phase 4's loom
  work and CREU-01 will add later, and why proof results stay out of `measurements/` (D-54).
- Human review (Task 3 checkpoint) read all four documents against the four tests in the plan's
  own `<how-to-verify>` block and approved them, with one required addition.
- The required addition: the Coverage section now states function (16/17, 94.1 percent), region
  (134/137, 97.8 percent) and line (108/111, 97.3 percent) coverage beside the 100 percent branch
  figure, naming `EmergencyStop::default()` as the entire, branch-free gap. Re-derived from a
  fresh `scripts/nr-coverage.sh` run and a direct read of the JSON `totals` block in this
  session, not copied from `02-04-SUMMARY.md`, though the numbers agree exactly.
- STOP-04 closes in `.planning/REQUIREMENTS.md`: both the checkbox and the traceability table
  row, with the orchestrator's pointer note removed now that the published claim it pointed at
  exists and is approved.
- Full verification suite confirmed green after the addition: `cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --all-targets`
  (all crates, 0 failures), `scripts/nr-proofs.sh` (11 of 11 harnesses verified), and
  `scripts/nr-coverage.sh` (branches 6/6, regions 134/137, lines 108/111, exit 0).

## Task Commits

1. **Task 1: The proof scoping note** - `080c099` (docs) - completed in the prior session;
   verified present at the start of this session, not redone
2. **Task 2: The Phase 6 output gate contract** - `617c2da` (docs) - completed in the prior
   session; verified present at the start of this session, not redone
3. **Task 3: Review the published claim before it is published** - `checkpoint:human-verify`;
   approved by the human operator on 2026-09-15 with one required addition, which landed as
   `017a02b` (docs)

**Plan metadata:** commit hash recorded after this summary is written (see final commit).

## Files Created/Modified

- `docs/proofs/emergency-stop-proof-scope.md` - the D-48 scope note; created in task 1
  (`080c099`), Coverage section extended in the checkpoint's required addition (`017a02b`)
- `docs/proofs/phase-6-output-gate-contract.md` - the D-45 written contract; created in task 2
  (`617c2da`)
- `docs/proofs/README.md` - what `docs/proofs/` is; created in task 1 (`080c099`)
- `.planning/REQUIREMENTS.md` - STOP-04 checkbox and traceability row both flipped to complete;
  the orchestrator's pointer note removed from the requirement text
- `.planning/STATE.md` - position, decisions and session updated
- `.planning/ROADMAP.md` - Phase 2 plan-progress table updated
- `.planning/phases/02-proven-emergency-stop/02-05-SUMMARY.md` - this document

## Checkpoint Record (Task 3)

### Review method

The human operator read all four documents (`emergency-stop-proof-scope.md`,
`phase-6-output-gate-contract.md`, `emergency-stop-proof-report.md`, `README.md`) against the
four tests in the plan's own `<how-to-verify>` block:

1. Does the scope note claim anything the proof did not establish? No finding.
2. Is "What nothing in this phase establishes" complete, or does it read as a formality? No
   finding.
3. Would Phase 6 be able to act on the contract? No finding.
4. Does anything anywhere describe the acknowledgement as the moment the system becomes safe? No
   finding.

### Verdict

Approved, with one required addition and one process decision, both recorded below.

### Required addition: Coverage section

Before (as committed in `080c099`):

```
`crates/stop` reports 100 percent branch coverage, 6 of 6 branches, under `cargo-llvm-cov` on
the pinned dated nightly, scoped to this crate alone (D-53); the rest of the workspace carries no
coverage gate. Branch coverage says that every branch in `crates/stop` was taken by some test in
the existing suite. It does not say that every behaviour of the crate is correct: coverage is a
completeness measure on the tests that ran, not a correctness measure on the code they exercised.
The eleven Kani harnesses above are the correctness claim; this figure only says the branches
they and the ordinary test suite exercise were not skipped by accident.
```

After (as committed in `017a02b`):

```
`crates/stop` reports 100 percent branch coverage, 6 of 6 branches, under `cargo-llvm-cov` on
the pinned dated nightly, scoped to this crate alone (D-53); the rest of the workspace carries no
coverage gate. Function, region and line coverage are not 100 percent: 16 of 17 functions
(94.1 percent), 134 of 137 regions (97.8 percent), 108 of 111 lines (97.3 percent). The entire
gap is one function, `EmergencyStop::default()` in `latch.rs`, a one-line delegation to
`Self::new()` that no test calls; it has no branches of its own, so it does not affect the metric
this gate enforces. STOP-06 and D-53 both scope the gate to branches, and neither
`cargo-llvm-cov` 0.9.1 nor this script gates on the other three figures. Branch coverage says
that every branch in `crates/stop` was taken by some test in the existing suite. It does not say
that every behaviour of the crate is correct: coverage is a
completeness measure on the tests that ran, not a correctness measure on the code they exercised.
The eleven Kani harnesses above are the correctness claim; this figure only says the branches
they and the ordinary test suite exercise were not skipped by accident.
```

Reason given: the document's whole purpose is to stop a reader from assuming more was
established than actually was, and quoting only the figure that reads 100 percent would be the
one place the document did the thing it warns against. The numbers were re-derived in this
session (a fresh `scripts/nr-coverage.sh` run plus a direct read of the JSON `totals` block, not
copied from `02-04-SUMMARY.md`) and agree exactly with what that plan recorded: branches 6/6,
functions 16/17, regions 134/137, lines 108/111. `EmergencyStop::default()` in `latch.rs` was
confirmed directly (`grep -n "impl Default for EmergencyStop" -A3`) to be a one-line delegation
to `Self::new()` with no branch of its own.

### Process decision: D-07 and the two already-committed tasks

The reviewer chose the reading that D-07's "reaches main" means reaching the shared remote.
`origin/main` is behind this repository's local `main` by design (see `git status`'s own report
at the start of this session and throughout this phase), so committing tasks 1 and 2 before
task 3's approval did not publish anything past a reviewed commit; it committed locally, where a
review could still happen before anything reached the shared remote. Commits `080c099` and
`617c2da` stay exactly as they are: not squashed, not rebased, not rewritten. This sets precedent
for every future checkpoint in this project that reviews a published claim: earlier tasks in the
same plan may be committed locally ahead of a `checkpoint:human-verify` gate, and do not need to
be re-committed or amended once approval lands, provided nothing has reached the remote in the
meantime. Plan 01-03 already worked this way in practice (Task 3's own commit landed before the
human read the schema and fixture; a second, continuation-session commit followed); this plan is
the first place the reading is written down as a decision rather than left implicit.

## Decisions Made

See `key-decisions` in the frontmatter for the full list, and the Checkpoint Record above for the
D-07 precedent and the required addition in full. Nothing else required a decision beyond what
the plan's own text already specified.

## Deviations from Plan

None. The plan executed exactly as written, including task 3's own designed outcome: a returned
finding, fixed, with the fix and its reasoning recorded in the SUMMARY rather than approved with
caveats. This is not a Rule 1-4 deviation (nothing was broken, missing, blocking, or
architectural); it is the checkpoint mechanism working as the plan's own acceptance criteria for
task 3 describe.

## Issues Encountered

None.

## User Setup Required

None. No external service configuration required; this plan produced documentation only.

## Next Phase Readiness

- STOP-04 is now genuinely complete: the proof landed in plan 02-03 (eleven harnesses, 132
  checks), and the published claim (this plan) is written, human-reviewed, and approved. Both
  halves of `.planning/REQUIREMENTS.md` (the checkbox and the traceability table row) reflect
  this.
- Phase 2's remaining open requirement is STOP-07 (abort latency measured on the reference rig),
  which needs the physical Precision 3591 and the Phase 1 clean-run protocol; plans 02-06
  through 02-09 own it and none of them were touched by this plan.
- The Phase 6 output gate contract (`docs/proofs/phase-6-output-gate-contract.md`) is now
  available for whichever plan builds the real decoder and sink; its "What would tell us this
  contract was broken" section gives that plan's reviewer concrete greps to run.
- Phase 1 remains open on its own track (01-15 task 3, gated on the coming Sunday's scheduled
  rig fire per `STATE.md`); this plan touched nothing under `measurements/` or
  `.planning/phases/01-trustworthy-measurement/`, per the explicit instruction not to.
- Nothing was pushed. `origin/main` stays behind local `main` by design; pushing is the
  operator's call.

*Phase: 02-proven-emergency-stop*
*Completed: 2026-09-15*

## Self-Check: PASSED

- FOUND: docs/proofs/emergency-stop-proof-scope.md
- FOUND: docs/proofs/phase-6-output-gate-contract.md
- FOUND: docs/proofs/README.md
- FOUND: .planning/phases/02-proven-emergency-stop/02-05-SUMMARY.md (this file)
- FOUND commit: 080c099 (docs(02-05): the D-48 proof-scope note and the docs/proofs README)
- FOUND commit: 617c2da (docs(02-05): the D-45 Phase 6 output gate contract)
- FOUND commit: 017a02b (docs(02-05): record region and line coverage are not 100 percent)
- CONFIRMED: `.planning/REQUIREMENTS.md` STOP-04 checkbox reads `[x]`, its traceability row
  reads `Complete`, and the pointer note is gone from the requirement text
- CONFIRMED: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace --all-targets`, `scripts/nr-proofs.sh` (11/11 harnesses) and
  `scripts/nr-coverage.sh` (branches 6/6) all exit 0 as of this session
