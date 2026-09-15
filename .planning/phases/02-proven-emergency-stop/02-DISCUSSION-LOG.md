# Phase 2: Proven emergency_stop - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md - this log preserves the alternatives considered.

**Date:** 2026-09-14
**Phase:** 02-proven-emergency-stop
**Mode:** discuss (interactive), invoked with --chain
**Areas discussed:** Abort trigger and the latency clock, FSM shape and the output gate, Proof
scope and wording, CI gate mechanics, Layout and sequencing

Two facts were verified during discussion rather than assumed, both from Kani's own
documentation: `aarch64-apple-darwin` is a supported platform, so the proofs run on the macOS
dev host; and the GitHub Action documents Ubuntu 20.04 with `x86_64-unknown-linux-gnu` only.
`kani-verifier` 0.67.0 (2026-01-16) is the current release. Neither Kani nor `cargo-llvm-cov`
is installed on the dev host today.

An assumptions block covering technical approach, implementation order, scope boundaries, risk
areas and dependencies was presented before any question was asked. It was confirmed with no
corrections. One self-correction followed: the block stated the rig was off-limits until Sunday
2026-09-13 03:00 CDT, which had already passed.

---

## Abort trigger and the latency clock

| Option | Description | Selected |
|--------|-------------|----------|
| One in-process handle, no OS sources | `abort(cause)` from any thread; control plane and watchdog become callers later | x |
| Handle plus a POSIX signal path | SIGTERM/SIGINT handler; forces `unsafe` into the safety-critical crate | |
| Handle plus a deadline watchdog | Pulls GRAPH-01 WCET work forward from Phase 5 | |

| Option | Description | Selected |
|--------|-------------|----------|
| Request to gate observed on the hot path | Caller's pre-store timestamp to the hot path's first observation | x |
| Request to latch store retired | Tens of nanoseconds, no safety meaning | |
| Request to last sample emitted from the sink | Most meaningful, needs the Phase 6 decoder | |

| Option | Description | Selected |
|--------|-------------|----------|
| One poll per node iteration | Bound is one iteration WCET plus propagation | x |
| Multiple checkpoints per iteration | Tighter bound, more branches, ambiguous attribution | |
| You decide | | |

| Option | Description | Selected |
|--------|-------------|----------|
| Headline run now, weekly series later | Weekly stage deferred until the hot path is real | x |
| Headline run plus weekly series immediately | Extends D-09 from firmware drift to code drift | |
| Headline run only, never weekly | No baseline entry ever | |

| Option | Description | Selected |
|--------|-------------|----------|
| Minimal periodic loop, one poll per period | Isolates the two terms being reported | x |
| Loop shaped like Phase 5's node | 1024 channels at 30 kHz; bakes in a filter that does not exist | |
| Two threads, no period at all | Drops the dominant term | |

| Option | Description | Selected |
|--------|-------------|----------|
| Report total and decomposition, never subtract | Reuses Phase 1's D-22 precedent | x |
| Publish the total only | A miss becomes uninterpretable | |
| Publish propagation, state the period analytically | Part measurement, part arithmetic | |

| Option | Description | Selected |
|--------|-------------|----------|
| CLOCK_MONOTONIC_RAW, skew measured and published | Already the project's clock for WIRE-03 | x |
| Raw TSC reads, converted afterwards | Needs constant_tsc plus a calibration step | |
| You decide | | |

| Option | Description | Selected |
|--------|-------------|----------|
| SCHED_FIFO and isolated cores yes, mlockall and pools no | Caveat states it predates SUBS-01..04 | x |
| Adopt the full substrate now | Moves Phase 3 scope into the proof phase | |
| Neither, run it plain | Not defensible under the project's rig discipline | |

**Notes:** the user selected the recommended option in all eight. The follow-up round was
prompted by a gap the first four answers opened: the hot path does not exist yet, so the poll
loop has to be a stand-in and the two timestamps cross a thread boundary.

---

## FSM shape, the latch, and the output gate

| Option | Description | Selected |
|--------|-------------|----------|
| Three states: Running, Stopping, Stopped | Stopping is the interval STOP-07 measures | x |
| Two states: Running and Stopped | Models as instantaneous the thing being measured | |
| Four states, adding Armed | Covers a pre-start abort where nothing is producing anyway | |

| Option | Description | Selected |
|--------|-------------|----------|
| Never cleared; a new session is a new value | STOP-03 proven by the absence of an edge | x |
| An explicit reset(), guarded by a session token | Latch proof becomes conditional on the guard | |
| You decide | | |

| Option | Description | Selected |
|--------|-------------|----------|
| A capability the gate hands out, checked by type | Matches GRAPH-03 and SUBS-07's API-surface habit | x |
| A boolean the node checks before emitting | Makes STOP-03 a convention | |
| Both: permit for output, boolean for the poll | Two mechanisms to keep consistent | |

| Option | Description | Selected |
|--------|-------------|----------|
| A cause code plus the requesting timestamp | FSM holds one end of the measured interval | x |
| A cause code only | Harness carries the timestamp separately | |
| Nothing; a bare event | Every abort looks identical in the record | |

| Option | Description | Selected |
|--------|-------------|----------|
| Stopping denies permits immediately | STOP-02 holds against a hung node | x |
| Output stays legal until Stopped | One hung node leaves the system permanently un-stopped | |
| You decide | | |

| Option | Description | Selected |
|--------|-------------|----------|
| An acknowledgement event from the hot path | Transition and measurement are the same instant | x |
| Time: Stopped after a declared bound elapses | Asserts the bound STOP-07 exists to measure | |
| Nothing: entered and left in the same call | Discards the interval three states were chosen for | |

| Option | Description | Selected |
|--------|-------------|----------|
| A modelled consumer plus a written contract | Makes the model's obligations on Phase 6 explicit | x |
| The type alone | Guarantee depends on a future phase inferring rules | |
| A modelled consumer, no written contract | A permit-caching mistake would silently void STOP-03 | |

**Notes:** the follow-up round was prompted by two edges the three-state choice left undefined,
both bearing on STOP-02. The pair of answers separates safety (at the store) from the measured
observation (at the acknowledgement), which is recorded in CONTEXT.md under Specific Ideas.

---

## Proof scope and how the claim is worded

| Option | Description | Selected |
|--------|-------------|----------|
| Latch, totality, gate, and panic-freedom | Four harness families over the enumerated space | x |
| The latch property only | STOP-01 and STOP-03 would be tested, not proven | |
| All of the above plus the atomic wrapper | Kani models atomics sequentially; reads as more than it is | |

| Option | Description | Selected |
|--------|-------------|----------|
| Amend the requirement, publish a scoping note | Follows the 2026-09-09 PLAT-03 precedent | x |
| Keep the requirement, publish the note only | Traceability closes an overstated requirement | |
| Keep the requirement and claim it plainly | A reviewer who knows the field catches it first | |

| Option | Description | Selected |
|--------|-------------|----------|
| Restate forbid(unsafe_code) at the crate root | Makes the guarantee local and part of the claim | x |
| Rely on the workspace lint | Property stated somewhere other than the crate | |
| Allow narrow unsafe where it buys latency | Gives Kani real UB, costs the simplest true sentence | |

| Option | Description | Selected |
|--------|-------------|----------|
| Name the gap now, close it with loom in Phase 4 | Tracked into CHAN-06 scope | x |
| Pull loom forward into Phase 2 for the latch | Second verification stack, two phases early | |
| Argue the orderings in prose | Below the bar this project set for itself | |

| Option | Description | Selected |
|--------|-------------|----------|
| Amend REQUIREMENTS.md and ROADMAP criterion 3 together | One commit, reason recorded | x |
| Amend REQUIREMENTS.md only | Reproduces the PLAT-03 split exactly | |
| Amend the criterion only | Closes a requirement whose text was never satisfied | |

| Option | Description | Selected |
|--------|-------------|----------|
| Now, before any plan is written | One bar visible to researcher, plans and verifier | x |
| At phase close, once the proof exists | How a criterion gets reshaped to fit its result | |
| Only if the proof turns out weaker than hoped | Makes the amendment look like an excuse | |

| Option | Description | Selected |
|--------|-------------|----------|
| docs/proofs/, mirroring docs/rig/ | New sibling for verification artifacts | x |
| In the crate's README or module docs | Harder to cite as a canonical reference | |
| You decide | | |

**Notes:** the follow-up round was prompted by a drift risk the amendment decision opened.
ROADMAP.md:129 carries the same wording as STOP-04, and it is the bar the verifier checks at
phase close. This project has already been bitten by that exact split twice.

---

## CI gate mechanics

| Option | Description | Selected |
|--------|-------------|----------|
| Pinned cargo install, own workflow job | The action documents a runner image GitHub no longer offers | x |
| The official action, pinned to a commit SHA | Less to maintain if it still works on current runners | |
| You decide | | |

| Option | Description | Selected |
|--------|-------------|----------|
| Blocking on every push and PR, unconditionally | Same shape as fmt, clippy, test, deny | x |
| Blocking, filtered to the stop crate | A required check reporting skipped on most pushes | |
| Blocking on PR, nightly on main | Weaker than STOP-05's literal wording | |

| Option | Description | Selected |
|--------|-------------|----------|
| cargo-llvm-cov, gate scoped to the stop crate | Tarpaulin branch coverage is not real on stable | x |
| cargo-llvm-cov, workspace floor plus 100 percent on stop | A second threshold to argue about | |
| Tarpaulin, as REQUIREMENTS.md names first | Would not support the claim STOP-06 makes | |

| Option | Description | Selected |
|--------|-------------|----------|
| Commit a proof report, outside the measurements contract | Evidence beside the claim, without distorting the schema | x |
| A green CI check is the record | The headline artifact would have no committed evidence | |
| Bring the proof under the measurements contract | Forces environment fields meaningless for a solver run | |

---

## Layout and sequencing

| Option | Description | Selected |
|--------|-------------|----------|
| crates/stop plus a separate nr-stop-harness binary | Name already committed in the metrics test under D-04 | x |
| crates/stop plus a sixth nrmeasure subcommand | Points the dependency from tooling into the runtime | |
| You decide | | |

| Option | Description | Selected |
|--------|-------------|----------|
| Start Phase 2 now, close Phase 1 in parallel | Confirms the 2026-09-11 decision in STATE.md | x |
| Close Phase 1 fully first | Stalls the proof on two ssh reads | |
| Start Phase 2 and defer STOP-07 | Drops one of five success criteria | |

| Option | Description | Selected |
|--------|-------------|----------|
| Four causes: operator, watchdog, internal fault, shutdown | Enum is part of the proven state space | x |
| One variant now, extend as callers appear | Every later source reopens the proof | |
| An opaque code the caller supplies | FSM can no longer enumerate its own inputs | |

---

## Claude's Discretion

Recorded in CONTEXT.md. The poll period for the stand-in loop, the atomic orderings, how the
harness families split across `#[kani::proof]` functions, the pinned Kani version, whether the
proof report regenerates every CI run, run directory naming, how the cause code surfaces in a
published abort record, and module decomposition inside `crates/stop`.

## Deferred Ideas

No scope creep was raised. Seven items were deferred to named later phases and are listed in
CONTEXT.md: the weekly abort-latency stage, loom interleaving verification, a POSIX signal
source, the deadline watchdog detector, re-measurement under the Phase 3 substrate and against
the Phase 6 DAG, the real decoder behind the permit, and the Creusot functional proof.

`donny-tools todo match-phase 2` returned zero pending todos, so none were folded or reviewed.
