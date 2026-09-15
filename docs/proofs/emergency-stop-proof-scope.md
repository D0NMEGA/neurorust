# Emergency-stop proof scope

## What this document is

This is the scope of the Phase 2 proof over `crates/stop`: what `cargo kani` passing on this
crate actually means, stated as three separate claims rather than one. The solver run behind it
is committed at `docs/proofs/emergency-stop-proof-report.md`, which names the Kani version, the
source commit, every harness, and its outcome. This document states what that report's results
are evidence of, and, just as important, what they are not.

## What the proof establishes

Eleven `#[kani::proof]` harnesses in `crates/stop/src/proofs.rs` cover four families of property
(D-46). Every harness verified: `cargo kani -p nr-stop` reports 132 of 132 checks succeeded
across all eleven, zero failures.

### Family 1: totality over the enumerated state space

`step_is_total`, `state_raw_round_trips`, `state_from_raw_is_total_and_fails_closed`, and
`cause_raw_round_trips_and_is_total` together establish that `step` returns one of the three
enumerated states for every combination of state and event, with no input left unhandled, and
that the `u8` encodings the atomic latch stores round trip for every real `State` and `Cause`
value. They also establish that both `from_raw` functions are total over every possible `u8`,
not only the values this crate itself writes, and that a value the crate never wrote decodes to
the most restrictive state or cause available: `Stopped` for the state encoding, `InternalFault`
for the cause encoding. A corrupted or future encoding fails closed rather than reopening a
stopped session.

### Family 2: no path back to Running

`abort_never_returns_to_running`, `running_is_never_reachable_from_stopping_or_stopped`, and
`stopped_is_terminal` establish that an abort, from any state, with any cause and any timestamp,
lands in `Stopping` or `Stopped` and never in `Running`, and that once a machine is out of
`Running`, no event of any kind returns it there. The structural reason is that `step`'s match
table has no arm that produces `Running` except `(Running, Acknowledged)` itself. This is not
contingent on nobody calling a reset function (D-39): there is no reset function to call.

### Family 3: the output gate invariant

`no_permit_when_not_running`, `permit_supply_closes_on_the_abort_edge`, and
`modelled_consumer_never_emits_after_a_stop` establish that no permit is issued from any state
other than `Running`; that the state an abort transitions into, whatever state it started from,
never issues a permit, so the supply closes on the transition itself rather than on reaching
`Stopped` (D-40); and that the modelled consumer's emission count cannot move once a stop has
happened, because there is no permit anywhere in that branch of the proof to hand it.

### Family 4: panic and overflow freedom

`consumer_emit_never_overflows` establishes that the one place arithmetic happens in the proved
surface, the emission counter inside `ModelledConsumer::emit`, cannot panic or overflow for any
starting value. Kani also checks for reachable panics, index out-of-bounds accesses, and
arithmetic overflow automatically inside every harness above; this harness is where that
guarantee is made explicit at the one place the crate does arithmetic at all.

"Proved" means the property holds for every value of the inputs at once, checked by a bounded
model checker over the enumerated state space, rather than for the particular values a test
happened to choose.

## What the compiler establishes

`crates/stop` forbids `unsafe` twice: once at the workspace level in the root `Cargo.toml`, and
again on the first line of `crates/stop/src/lib.rs` (D-49). Because the crate contains no
`unsafe`, the compiler excludes several classes of undefined behaviour before Kani ever runs:
data races on non-atomic memory, use after free, dangling references, out-of-bounds pointer
arithmetic, reading uninitialised memory, breaking aliasing rules, and unsound transmutes. None
of that exclusion is Kani's doing; it is the type system's, and it would hold even if this crate
carried no proof harnesses at all.

STOP-04's original wording credited Kani with excluding undefined behaviour, which overstated
what the proof adds in a crate that already forbids `unsafe`. STOP-04 and ROADMAP.md Phase 2
success criterion 3 were both amended on 2026-09-14, in one reviewed commit (af4c479), before any
plan for this phase was written. The timing matters: amending a criterion after the fact, once
its result is already known, is the exact failure ROADMAP criterion 1 was rewritten on
2026-09-07 to forbid. This document's separation of proof, compiler, and gap is the amendment's
deliverable, not a retroactive justification for it.

## What nothing in this phase establishes

Five things a reader might reasonably assume this phase covers, and does not.

- Thread interleavings. Kani does not explore them. `crates/stop/src/latch.rs` is where this
  crate's concurrency actually lives, and no harness in `crates/stop/src/proofs.rs` touches an
  atomic, a compare-exchange, or an `Ordering` value. The orderings this document means are the
  `Ordering::AcqRel`/`Ordering::Acquire` compare-exchange on the `Running` to `Stopping`
  transition inside `abort`, the `Ordering::Acquire` load on the hot-path poll that must
  synchronize-with it, and the `Ordering::Release` store to `record_published` that must
  synchronize-with the `Ordering::Acquire` load inside `abort_record`. `latch.rs`'s own module
  doc argues in prose that these hold; that argument is unverified until CHAN-06 brings loom
  into CI in Phase 4, and a loom run would check whether every interleaving the hardware and the
  compiler are permitted to produce still leaves those orderings intact, not just the one path
  the prose argument walks through.
- Functional correctness beyond the four families above. Kani proves the properties the eleven
  harnesses assert and nothing else about what the machine is for. The full functional property,
  that no decoder output follows a stop under any implementation of the surrounding system, is
  CREU-01 with Creusot, already deferred to v2 in REQUIREMENTS.md.
- Anything about the timing of the abort. Nothing here says how long the hot path takes to
  observe a closed gate after an abort. That figure is STOP-07, measured on the reference rig
  rather than proved, and published separately once plans 02-06 through 02-09 land.
- Anything outside `crates/stop`. Nothing in `nr-stop-harness` or in the five measurement crates
  (`nr-capture`, `nr-cli`, `nr-histogram`, `nr-manifest`, `nr-metrics`) is proved by anything in
  this phase.
- Any claim about the model checker itself. Kani is a bounded model checker with its own
  implementation, and this proof is only as sound as the solver and its own modelling of Rust.
  The concrete reason the proved surface was designed to avoid atomics entirely: a reported case
  of a Kani build modelling a failed `compare_exchange` as successful
  (model-checking/verify-rust-std#673). Whether that report applies to the pinned Kani 0.67.0
  used here was not determined. Keeping every harness in this crate free of atomics,
  compare-exchanges, and `Ordering` values makes that question moot for what STOP-01 through
  STOP-04 establish here, rather than resolving it.

## How to re-check this

    scripts/nr-proofs.sh
    scripts/nr-coverage.sh

Both are blocking checks in `.github/workflows/ci.yml` on every push and every pull request,
unconditionally: neither the `kani` job nor the `coverage` job carries an `if:` at the job level
or a `paths:` filter. `nr-proofs.sh` re-runs `cargo kani -p nr-stop` and refuses to pass unless
every harness named above still appears and the successful-verification count still matches.
`nr-coverage.sh` re-runs the coverage gate described below.

## Coverage

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
