---
status: PASS
agent: donny-executor
phase: 02-proven-emergency-stop
plan: 02

subsystem: runtime
tags: [emergency-stop, fsm, capability-gate, atomics, nr-stop]

# Dependency graph
requires:
  - phase: 02-proven-emergency-stop
    plan: 01
    provides: crates/stop (package nr-stop) as the sixth workspace member, zero dependencies,
      #![forbid(unsafe_code)] restated at its root (D-49), plus Kani 0.67.0 and cargo-llvm-cov
      0.9.1 confirmed working end to end in this exact workspace
provides:
  - "The full nr-stop public API: State/step (state.rs), Cause/Event (event.rs),
    OutputPermit/would_issue_permit (gate.rs), ModelledConsumer (consumer.rs),
    EmergencyStop/AbortRecord (latch.rs), exactly matching this plan's own <interfaces> block"
  - "27 tests under crates/stop/tests/ (never a #[cfg(test)] module in src/), touching every
    (State, Event) transition arm and every latch behavior in the plan's behavior list"
  - "workspace.lints.rust.unexpected_cfgs declaring cfg(kani) as a known cfg name, which plan
    02-03's #[cfg(kani)] proof harnesses need to compile clean under this workspace's own
    -D warnings gate"
affects: ["02-03", "02-04", "02-05", "02-06"]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "A pure, atomics-free step(state, event) -> state is the function that gets proved; the
       atomic publication layer (latch.rs) is a thin, separately-argued wrapper around it and
       is explicitly unverified this phase (D-50)"
    - "A capability type (OutputPermit: no public constructor, not Clone, not Copy, consumed by
       value) rather than a boolean flag, so a safety property is structural rather than a
       convention every caller is trusted to honour"
    - "Total, fail-closed raw encodings: from_raw never panics on an unexpected u8, it decodes
       to the most restrictive value (Stopped, InternalFault) instead"

key-files:
  created:
    - crates/stop/src/state.rs
    - crates/stop/src/event.rs
    - crates/stop/src/gate.rs
    - crates/stop/src/consumer.rs
    - crates/stop/src/latch.rs
    - crates/stop/tests/state.rs
    - crates/stop/tests/gate.rs
    - crates/stop/tests/latch.rs
  modified:
    - crates/stop/src/lib.rs
    - Cargo.toml

key-decisions:
  - "requirements-completed left empty for STOP-01/02/03 despite appearing in this plan's own
     frontmatter requirements field, matching this project's own established precedent (STATE.md:
     01-09, 01-18, 01-20 through 01-25, 01-23's PLAT-03 entry) of not checking off a requirement
     until its full evidentiary bar is met. D-46 states the first three Kani harness families
     establish STOP-01 through STOP-03 'mechanically', and the roadmap's own phase objective
     calls STOP-03 'a proven guarantee'; this plan delivers the code and example-based unit tests
     those families will be proved over, not the mechanized proof itself. Plan 02-03 should run
     requirements mark-complete STOP-01 STOP-02 STOP-03 once the Kani harnesses pass."
  - "Added workspace.lints.rust.unexpected_cfgs = { check-cfg = [\"cfg(kani)\"] } to the root
     Cargo.toml (Rule 3, blocking): the plan's own mandated #[cfg_attr(kani, ...)] annotations on
     State/Cause/Event otherwise fail rustc's check-cfg lint, which -D warnings promotes to a hard
     error under the workspace's own clippy gate every task's acceptance criteria requires."
  - "consumer.rs's emit() discards the permit with 'let _ = permit;' rather than the plan's
     literal 'drop(permit)' (Rule 1, bug): OutputPermit has no Drop impl, so an explicit drop()
     call trips clippy::drop_non_drop, denied workspace-wide via clippy::all. Same by-value
     consumption, no lint violation."
  - "Reworded one clause of latch.rs's own module doc (Rule 1, bug): the plan's literal
     prescribed text used the token 'compare_exchange(AcqRel, Acquire)' in prose, which is a
     third literal occurrence of 'compare_exchange' in the file, colliding with the plan's own
     acceptance grep expecting exactly 2 (the two real call sites). Reworded to keep identical
     technical content without the colliding token, matching this project's own established
     precedent for the same class of collision (STATE.md 01-14)."

requirements-completed: []

# Metrics
duration: 24min
completed: 2026-09-15
---

# Phase 2 Plan 2: The state machine, the capability gate and the latch Summary

**`nr-stop` now holds a six-arm, no-wildcard FSM proved out by 27 tests; a capability-typed output permit that cannot be constructed, cloned or reused outside the gate; and an atomic latch whose ordering argument is written into its own module doc, with the permit supply closing on the abort itself rather than on any acknowledgement.**

## Performance

- **Duration:** 24 min
- **Started:** 2026-09-15T05:52:00Z (approximate; STATE.md recorded 02-01's completion at 05:48:49Z)
- **Completed:** 2026-09-15T06:16:00Z
- **Tasks:** 3
- **Files modified:** 10 (8 created, 2 modified)

## Accomplishments

- `crates/stop/src/state.rs`: `State` (`Running`, `Stopping`, `Stopped`) and `step(state, event)
  -> state`, a six-arm match with no wildcard over the `(State, Event)` pair. `as_raw`/`from_raw`
  round-trip and are total over every `u8`, failing closed to `Stopped` rather than panicking.
- `crates/stop/src/event.rs`: `Cause` (`Operator`, `WatchdogDeadline`, `InternalFault`,
  `Shutdown`) and `Event` (`Abort { cause, at_raw_ns }`, `Acknowledged`), both total and
  fail-closed the same way (`from_raw` decodes an unaccounted-for code to `InternalFault`).
- `crates/stop/src/gate.rs`: `OutputPermit`, a capability with a `pub(crate)`-only field, no
  public constructor, no `Clone`, no `Copy`; `would_issue_permit(state)`, pure and true only for
  `Running`, which is D-40 (the permit supply closes on the abort edge) stated as a function.
- `crates/stop/src/consumer.rs`: `ModelledConsumer`, the D-45 stand-in for the Phase 6 decoder
  and sink. `emit` takes an `OutputPermit` by value so a permit cannot be cached across
  iterations; its counter uses `saturating_add` so overflow cannot make a panic reachable.
- `crates/stop/src/latch.rs`: `EmergencyStop`, four atomics (`state`, `cause`,
  `aborted_at_raw_ns`, `record_published`). `state` is the only one on the hot path and the only
  one compare-exchanged; the abort's `Running -> Stopping` edge and the acknowledge's
  `Stopping -> Stopped` edge both derive their destination from the proved `step()` rather than a
  second transition table. Only the compare-exchange winner writes the abort record, published
  last with a `Release` store to `record_published` that `abort_record()` reads with `Acquire`
  before trusting the `Relaxed` reads behind it. No `SeqCst` anywhere. The ordering argument, and
  D-50's stated gap (Kani does not verify interleavings; CHAN-06/loom closes it in Phase 4), live
  in this file's own module doc rather than being asserted anywhere else.
- No `fn reset`, `fn restart` or `fn rearm` anywhere in `crates/stop/src/`; `Stopped` has no
  outgoing edge in `step()`, so STOP-03 is a structural absence rather than a convention (D-39).
- 27 tests, all under `crates/stop/tests/` (never a `#[cfg(test)]` module in `src/`), so plan
  02-04's coverage gate measures production code only: 10 in `state.rs` (every transition arm,
  both raw encodings' totality), 5 in `gate.rs` (the gate invariant, the consumer's zero/count/
  saturation behavior), 12 in `latch.rs` (the full latch behavior list, including cross-thread
  abort and all 16 orderings of a length-4 abort/acknowledge sequence from both `Stopping` and
  `Stopped`, none of which reach `Running`).

## Task Commits

1. **Task 1: The state machine, the cause, and the event** - `3984aba` (feat)
2. **Task 2: The output gate as a capability, and the modelled consumer** - `5a92cab` (feat)
3. **Task 3: The latch, the abort handle, and the hot-path poll** - `712ce8b` (feat)

## Files Created/Modified

- `crates/stop/src/state.rs` - `State`, `step`, both raw encodings (63 lines)
- `crates/stop/src/event.rs` - `Cause`, `Event` (59 lines)
- `crates/stop/src/gate.rs` - `OutputPermit`, `would_issue_permit` (31 lines)
- `crates/stop/src/consumer.rs` - `ModelledConsumer` (50 lines)
- `crates/stop/src/latch.rs` - `EmergencyStop`, `AbortRecord` (155 lines)
- `crates/stop/src/lib.rs` - added `pub mod consumer; pub mod event; pub mod gate; pub mod latch;
  pub mod state;` in alphabetical order
- `crates/stop/tests/state.rs` - the ten transition/encoding tests (95 lines)
- `crates/stop/tests/gate.rs` - the five gate/consumer tests, three from task 2 plus two
  permit-consuming tests appended once `try_permit` existed (62 lines)
- `crates/stop/tests/latch.rs` - the twelve latch behavior tests (154 lines)
- `Cargo.toml` - added `workspace.lints.rust.unexpected_cfgs` declaring `cfg(kani)` as a known
  cfg name (see Deviations)

## The public API, verbatim

Exactly as declared in this plan's own `<interfaces>` block; no signature deviated.

```rust
// crates/stop/src/state.rs
pub enum State { Running, Stopping, Stopped }
impl State {
    pub const fn as_raw(self) -> u8;
    pub const fn from_raw(raw: u8) -> State;
}
pub fn step(state: State, event: Event) -> State;

// crates/stop/src/event.rs
pub enum Cause { Operator, WatchdogDeadline, InternalFault, Shutdown }
impl Cause {
    pub const fn as_raw(self) -> u8;
    pub const fn from_raw(raw: u8) -> Cause;
}
pub enum Event { Abort { cause: Cause, at_raw_ns: u64 }, Acknowledged }

// crates/stop/src/gate.rs
pub struct OutputPermit;                       // pub(crate) field only, not Clone, not Copy
pub const fn would_issue_permit(state: State) -> bool;

// crates/stop/src/consumer.rs
pub struct ModelledConsumer;
impl ModelledConsumer {
    pub const fn new() -> Self;
    pub const fn with_emitted(emitted: u64) -> Self;
    pub fn emit(&mut self, permit: OutputPermit);
    pub const fn emitted(&self) -> u64;
}

// crates/stop/src/latch.rs
pub struct AbortRecord { pub cause: Cause, pub at_raw_ns: u64 }
pub struct EmergencyStop;
impl EmergencyStop {
    pub const fn new() -> Self;
    pub fn abort(&self, cause: Cause, at_raw_ns: u64) -> bool;
    pub fn acknowledge(&self) -> bool;
    pub fn state(&self) -> State;
    pub fn try_permit(&self) -> Option<OutputPermit>;
    pub fn abort_record(&self) -> Option<AbortRecord>;
}
```

Both `EmergencyStop` and `ModelledConsumer` also derive/implement `Default` (calling their own
`new()`), required by clippy's `new_without_default` under this workspace's `clippy::all = deny`;
not in the plan's own interfaces block but not a deviation from any named signature either.

## Decisions Made

See `key-decisions` in the frontmatter for the full reasoning on all four. Summary:

- `requirements-completed` left empty for STOP-01/02/03 even though they appear in this plan's
  own frontmatter `requirements` field. Plan 02-03's Kani proof is what D-46 says establishes
  them "mechanically" and what the roadmap calls STOP-03's "proven guarantee"; this plan built
  the structure and the example-based tests that proof runs over, not the proof.
- Two blocking (Rule 3) and one bug-class (Rule 1, x2) fixes were needed because the plan's own
  literal prescribed text does not, verbatim, clear this workspace's own `-D warnings` /
  `clippy::all = deny` gate. All three are documented in Deviations below with exact locations.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `#[cfg_attr(kani, ...)]` fails rustc's `unexpected_cfgs` lint under `-D warnings`**

- **Found during:** Task 1, immediately after writing `state.rs`/`event.rs` per the plan's own
  `<interfaces>` block, which requires `#[cfg_attr(kani, derive(kani::Arbitrary))]` on `State`,
  `Cause` and `Event` so plan 02-03's proof harnesses can compile them.
- **Issue:** `cargo clippy --workspace --all-targets -- -D warnings` (mandated by every task's
  own acceptance criteria and the plan's own top-level `<verification>` block) failed with three
  `error: unexpected cfg condition name: kani`. `kani` is not a cfg this workspace's build ever
  sets; rustc's check-cfg lint (warn-by-default since around Rust 1.80, promoted to a hard error
  by `-D warnings`) flags any cfg name it was not told to expect.
- **Fix:** Added `unexpected_cfgs = { level = "warn", check-cfg = ["cfg(kani)"] }` to
  `[workspace.lints.rust]` in the root `Cargo.toml`, with a comment stating that `cargo kani`
  itself sets this cfg when it compiles proof harnesses (plan 02-03), never this workspace's own
  build. Declared at the workspace level, not per-crate, because `crates/stop/Cargo.toml` already
  has `[lints] workspace = true`, which is the only path that reaches it.
- **Files modified:** `Cargo.toml` (outside task 1's own declared `<files>` list; the minimum
  touch needed for the plan's own mandated code to build clean under the plan's own mandated
  gate, matching this project's established precedent for the same class of fix, e.g. STATE.md's
  01-04 entry on `lib.rs` module registration).
- **Verification:** `cargo clippy --workspace --all-targets -- -D warnings` exits 0, both for
  `-p nr-stop` alone and for the full workspace.
- **Committed in:** `3984aba` (Task 1 commit)

**2. [Rule 1 - Bug] `drop(permit)` trips `clippy::drop_non_drop`**

- **Found during:** Task 2, writing `consumer.rs`'s `emit()` exactly as the plan's own `<action>`
  text specifies (`drop(permit); self.emitted = ...`).
- **Issue:** `OutputPermit` has no `Drop` implementation of its own (it is a marker-style
  capability type with one `pub(crate)` unit field). Clippy's `drop_non_drop` lint, part of
  `clippy::all` (denied workspace-wide), flags an explicit `drop()` call on a type that drops
  trivially anyway, since it only extends the value's contained lifetime and adds no real
  behavior.
- **Fix:** Replaced `drop(permit);` with `let _ = permit;`, the idiomatic Rust discard pattern.
  Same by-value consumption (the parameter type is still `OutputPermit`, not `&OutputPermit`),
  same visible "this line is where the permit is consumed" signal in the diff, no lint violation.
- **Files modified:** `crates/stop/src/consumer.rs`
- **Verification:** `cargo clippy --workspace --all-targets -- -D warnings` exits 0.
- **Committed in:** `5a92cab` (Task 2 commit)

**3. [Rule 1 - Bug] The plan's own literal module-doc text collides with the plan's own acceptance grep**

- **Found during:** Task 3, after implementing `latch.rs` verbatim per the plan's `<action>`
  text and running the task's own acceptance criteria (`grep -c 'compare_exchange'
  crates/stop/src/latch.rs` outputs `2`).
- **Issue:** The count came back `3`, not `2`. The plan's own prescribed module doc (the
  "ordering argument" section) reads "...a `compare_exchange(AcqRel, Acquire)`: the release
  half publishes..." as prose, which is a third literal occurrence of the substring
  `compare_exchange` alongside the two real call sites in `abort()` and `acknowledge()`.
- **Fix:** Reworded that one clause to "...a compare-exchange with `Ordering::AcqRel` on success
  and `Ordering::Acquire` on failure: the release half publishes...", preserving the exact same
  technical content (which transition, which two orderings, why) without the literal
  underscore-joined token. This is the same class of self-collision this project's own STATE.md
  already documents and resolves the same way at 01-14 ("three of the plan's own suggested
  comments would have failed one of the plan's own greps if quoted verbatim... reworded each
  comment to state the same fact without the colliding literal token; none of the three checks
  were weakened").
- **Files modified:** `crates/stop/src/latch.rs`
- **Verification:** `grep -c 'compare_exchange' crates/stop/src/latch.rs` now outputs `2`; the
  two remaining occurrences are the real call sites in `abort()` and `acknowledge()`.
- **Committed in:** `712ce8b` (Task 3 commit)

**Total deviations:** 3 auto-fixed (1 blocking build-config fix, 2 bugs in the plan's own literal
prescribed text). **Impact on plan:** none of the three touch the crate's actual behavior,
public API, or any of the correctness properties in `02-CONTEXT.md`'s D-40/D-41/D-39/D-42/D-49/
D-50; all three are either a lint-satisfying rewrite of prose/code that means the same thing, or
a one-line addition to the workspace lint table that changes nothing at runtime.

## Issues Encountered

- The plan's own top-level `<verification>` block includes `grep -rn 'unsafe'
  crates/stop/src/  # expect no output`, run recursively over the whole `src/` directory. This
  necessarily matches `crates/stop/src/lib.rs`'s own line 1, `#![forbid(unsafe_code)]`, which
  D-49 explicitly requires to be restated at the crate root, plus the doc comment immediately
  below it that also names `unsafe_code`. This is not a defect: task 3's own acceptance criterion
  scopes the same check precisely to `crates/stop/src/latch.rs` alone (`grep -q 'unsafe'
  crates/stop/src/latch.rs` returns no match), which does pass with zero matches. Recorded so a
  future reader does not mistake the broader, imprecisely-worded top-level grep for a real gap;
  verified directly that `latch.rs` (and every other file besides `lib.rs`) contains no `unsafe`
  token at all.
- `there_is_no_path_back_to_running` (task 3) checks state after every one of the 4 steps in each
  of the 16 orderings, not only after the final step, which is a strictly stronger reading of the
  plan's own behavior-list text ("no sequence... produces state() == Running") than checking only
  the end state. Not treated as a deviation since it is a superset of what was asked, not a
  different property.

## User Setup Required

None. No external service, credential, or manual dashboard step is needed to build, test or lint
this crate; everything ran on the macOS dev host with the workspace's already-installed stable
toolchain. No rig access, no Kani install, and no coverage nightly were needed for this plan
(those are plan 02-01's already-confirmed tools and are not invoked until plans 02-03 and 02-04).

## Next Phase Readiness

- `crates/stop`'s full public surface (`State`/`step`, `Cause`/`Event`, `OutputPermit`/
  `would_issue_permit`, `ModelledConsumer`, `EmergencyStop`/`AbortRecord`) exists exactly as this
  plan's own `<interfaces>` block specified it, so plan 02-03 can write its four Kani harness
  families directly against these names with no further discovery needed.
- `workspace.lints.rust.unexpected_cfgs` already declares `cfg(kani)`, so plan 02-03's
  `#[cfg(kani)]`-gated harness module will compile clean under `-D warnings` from its first
  commit; this was the one build-config gap a fresh Kani harness file would otherwise hit again.
- `step()` is proof-ready as written: pure, no atomics, no allocation, total over the enumerated
  `(State, Event)` space, matching Architecture Pattern 1 in `02-RESEARCH.md` exactly (the
  pinned-Kani `compare_exchange`-modelling risk that pattern names never applies here, since
  nothing Kani will see contains a `compare_exchange` at all).
- `latch.rs`'s module doc already states D-50's gap in the words plan 02-05 (the published
  scoping note) will need; that plan can quote it rather than re-deriving it.
- `docs/proofs/` does not exist yet; it remains plan 02-05's responsibility (D-48), not this
  plan's. `docs/proofs/phase-6-output-gate-contract.md`, referenced in doc comments on
  `OutputPermit` and `ModelledConsumer` as where Phase 6's obligations are written down, also
  does not exist yet; it is not this plan's file list either, and nothing in this plan asserts
  it already exists (the doc comments describe a future path, matching the interfaces block).
- No blockers carried forward from this plan. Phase 1 remains open on its own track (01-15 task
  3) and this plan touched nothing under `measurements/` or the Phase 1 planning directory.
- STOP-01, STOP-02 and STOP-03 remain `Pending` in `REQUIREMENTS.md`; plan 02-03 should mark them
  complete alongside STOP-04 once its Kani harnesses pass, per the reasoning in Decisions above.

*Phase: 02-proven-emergency-stop*
*Completed: 2026-09-15*

## Self-Check: PASSED

- FOUND: crates/stop/src/state.rs
- FOUND: crates/stop/src/event.rs
- FOUND: crates/stop/src/gate.rs
- FOUND: crates/stop/src/consumer.rs
- FOUND: crates/stop/src/latch.rs
- FOUND: crates/stop/tests/state.rs
- FOUND: crates/stop/tests/gate.rs
- FOUND: crates/stop/tests/latch.rs
- FOUND commit: 3984aba (feat(02-02): the state machine, the cause, and the event)
- FOUND commit: 5a92cab (feat(02-02): the output gate as a capability, and the modelled consumer)
- FOUND commit: 712ce8b (feat(02-02): the latch, the abort handle, and the hot-path poll)
