# Phase 6 output gate contract

## What this contract is

STOP-03 is proved at FSM level against `ModelledConsumer`, which takes an `OutputPermit` and
emits nothing. The proof recorded in `docs/proofs/emergency-stop-proof-report.md` establishes
that no permit exists outside `Running`. It does not and cannot establish that a future, real
consumer uses permits the way the model does; no type can enforce that across a phase boundary
that does not exist yet. This document names the difference and states what Phase 6 must do to
close it without breaking the guarantee STOP-03 already earned.

## What the type system already enforces

Phase 6 gets the following for free, each because of a specific mechanism rather than a promise:

- `OutputPermit` has a private field and no public constructor, so it cannot be forged from
  outside `nr-stop`.
- `OutputPermit` is neither `Clone` nor `Copy`, so one issued permit cannot become two.
- `ModelledConsumer::emit` takes an `OutputPermit` by value, so using one consumes it; there is
  no way to hold onto it and use it again.
- There is no method anywhere in `nr-stop` that emits, or claims to emit, without an
  `OutputPermit`.
- `EmergencyStop` has no reset, restart, or rearm method. A stopped session cannot be reopened;
  a new session means constructing a new `EmergencyStop` value.

## Obligations on the real consumer

Five rules. Each names the failure it exists to prevent.

1. Do not cache a permit across iterations. Acquire it inside the iteration that uses it, and
   let it be consumed there. A permit held from a previous iteration is a permit that was issued
   while the machine was `Running` and then used after it stopped, which is exactly the output
   STOP-03 forbids. `OutputPermit` is not `Copy` and is consumed by value, so the natural way to
   write this loop is already correct; caching one requires deliberate effort, for example
   storing it in a struct field or in a long-lived `Option`.
2. Do not emit without one. Every code path that produces output must take an `OutputPermit` by
   value. A convenience path that skips the permit, for a retry, a flush, a test, or an error
   path, deletes the guarantee for every other path, because the guarantee STOP-03 states is
   that no such path exists anywhere, not that most paths are safe.
3. Do not add a public constructor to `OutputPermit`, and do not add a `Clone` or `Copy` derive
   to it, for any reason, including making a test easier to write. Either change reopens the
   forgery and double-use gaps the type system currently closes for free.
4. Poll the gate at one fixed point per node iteration (D-32), and treat any state other than
   `Running` as closed. Do not branch on `Stopping` versus `Stopped` and treat `Stopping` as
   still open: the permit supply closes at the abort itself, not at `Stopped`, and treating
   `Stopping` as an open state is the single most likely way to break this contract.
5. Raise `acknowledge` when the gate is first observed closed, and understand what it is for.
   It ends the interval STOP-07 measures. It does not make the system safe, and the system does
   not wait for it: the permit supply already closed at the abort (D-40). A consumer that never
   calls `acknowledge` is safe and simply unmeasured, which is the correct trade if it ever
   comes up.

## What would tell us this contract was broken

Concretely: a Phase 6 test that emits output after a stop; a grep for `OutputPermit` appearing
as a struct field or inside a long-lived `Option`; any `pub fn` anywhere outside `nr-stop` that
returns an `OutputPermit`; and any output path whose function signature does not consume an
`OutputPermit` by value. These are the checks a Phase 6 reviewer should actually run, so that
this document is something to act on rather than something to have read once.

## What this contract does not cover

Thread interleavings, which are unverified until Phase 4's loom work lands (CHAN-06), and the
timing of the abort, which is STOP-07. See `docs/proofs/emergency-stop-proof-scope.md` for the
full statement of both; this document does not restate it.
