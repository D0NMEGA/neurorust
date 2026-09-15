# Phase 2: Proven emergency_stop - Context

**Gathered:** 2026-09-14
**Status:** Ready for planning

<domain>
## Phase Boundary

The safety-critical abort path becomes proven rather than tested, and its worst case becomes a
measured number. Phase 2 delivers: a latching abort FSM with an enumerated reachable state
space (STOP-01), an abort that drives the system to a defined safe state from any reachable
state (STOP-02), a proven guarantee that no decoder output follows a stop for the rest of the
session (STOP-03), Kani proofs over that state space (STOP-04) gating CI as a blocking check
(STOP-05), 100 percent branch coverage on the module (STOP-06), and a published abort latency
measured on the reference rig under the Phase 1 protocol (STOP-07).

This is the first runtime code in the repository. Everything committed so far is `nrmeasure`,
the measurement and provenance harness: 5 crates, no channel, no node, no graph, no decoder.

Out of scope and owned elsewhere: the real decoder and sink (Phase 6 consumes the gate this
phase proves), channels and the declared graph (Phases 4 and 5), `mlockall`, preallocated pools
and the allocator hook (SUBS-01 through SUBS-04, Phase 3), exhaustive interleaving verification
(CHAN-06 brings loom in Phase 4), and the Creusot functional proof (CREU-01, v2).

</domain>

<decisions>
## Implementation Decisions

Decision IDs continue the project-wide sequence rather than restarting at D-01. Phase 1 used
D-01 through D-29 with no phase qualifier, STATE.md and the plans refer to them that way, and
this phase's text cites several of them. Restarting the numbering would make "D-22" ambiguous.

### The abort trigger and what gets measured

- **D-30:** Phase 2 wires exactly one abort source: an in-process `EmergencyStop::abort(cause)`
  handle callable from any thread. The FSM takes an `Abort` event carrying a cause code, and
  who raises it is not the FSM's concern. The tonic control plane (OBS-03, Phase 5) and a
  deadline watchdog become callers later without touching the proven state space. No POSIX
  signal handler: registering one requires `unsafe`, and the crate whose whole value is a
  proof is the worst place to spend the project's first `unsafe` block.
- **D-31:** STOP-07 measures from the timestamp the aborting thread takes immediately before
  the latch store, to the timestamp the hot-path thread takes at the iteration where it first
  observes the gate closed. This is the interval a reviewer cares about: the last moment a
  sample could still have flowed. Timing the store alone would produce a flattering number
  with no safety meaning, and ending at a real sink output requires a decoder that does not
  exist until Phase 6.
- **D-32:** The hot path checks the latch once per node iteration, at a fixed point in the
  loop. Abort latency is then bounded by one iteration WCET plus cross-core propagation, which
  composes with the per-node WCET annotation GRAPH-01 introduces in Phase 5 and keeps the
  branch count small enough for D-53's 100 percent gate to be honest rather than gamed.
- **D-33:** The stand-in hot path for STOP-07 is a single thread on one isolated core running a
  fixed-period loop whose only body is the latch poll. Nothing else is in the way, so the
  measurement isolates the two terms D-34 reports. Its unrepresentativeness is stated as a
  published limitation, and Phase 6 re-measures against the real DAG.
- **D-34:** Phase 1's D-22 rule applies here unchanged: report the total and the decomposition
  side by side, never subtracted. The published figure is the end-to-end worst case taken from
  a uniformly random abort phase; the cross-core propagation component and the poll period are
  reported separately, each attributed to how it was measured, with an explicit sentence that
  they are not combined or netted. `render_plat03_verdict` already establishes this rendering
  pattern in `crates/metrics/src/report.rs`.
- **D-35:** Both ends read `CLOCK_MONOTONIC_RAW`, already the project's chosen clock for
  WIRE-03's frame header. Cross-core skew and clock read overhead are characterised in their
  own committed capture and published beside the figure, so the number carries its own
  instrument error rather than assuming it away.
- **D-36:** The STOP-07 harness pins to isolated cores 6-11 under `SCHED_FIFO`, because a
  scheduling-sensitive figure taken on a default-policy thread on a shared core is not
  defensible under the project's own rig discipline. It does not adopt `mlockall`, preallocated
  pools or the allocator hook, which are SUBS-01 through SUBS-04 in Phase 3. The published
  caveats state plainly that the measurement predates the no-allocation guarantee.
- **D-37:** One headline-class run under the Phase 1 protocol, published with its raw capture.
  The `emergency_stop.abort_latency` stage is not added to the weekly series or the regression
  baseline in this phase: a weekly series taken against the D-33 stand-in would track the
  stand-in, not the runtime. Wiring the stage into the weekly job is deferred until the hot
  path is real, in Phase 5 or 6.

### The FSM, the latch, and the output gate

- **D-38:** Three states: `Running`, `Stopping`, `Stopped`. `Stopping` exists because the abort
  request and the safe state are not the same instant, and that interval is precisely what
  D-31 measures. A two-state latch would model as instantaneous the thing STOP-07 is asked to
  measure.
- **D-39:** There is no reset transition at all. `Stopped` is terminal, and running again means
  constructing a fresh `EmergencyStop` value. STOP-03 therefore becomes a structural property
  Kani proves by the absence of an edge, rather than a property contingent on nobody calling
  `reset()`.
- **D-40:** The permit supply closes on the `Running` to `Stopping` edge, not on reaching
  `Stopped`. Safety never depends on anything acknowledging, so STOP-02 holds even against a
  node that hangs and never responds. `Stopping` means work already in flight is draining, not
  that new output is still permitted.
- **D-41:** An `Acknowledged` event, raised by the polling node when it first observes the gate
  closed, drives `Stopping` to `Stopped`. That event is the end of D-31's interval, so the
  FSM's own transition and the published measurement are the same instant rather than two
  things hoped to coincide. Because of D-40, a missing acknowledgement delays the record and
  never safety. A time-based transition was rejected: it would assert the bound STOP-07 exists
  to measure.
- **D-42:** The output gate is a capability, not a boolean. Producing output requires holding a
  permit the gate issues only while `Running`, so a stopped session cannot emit because there
  is nothing to emit with. This follows the project's existing habit of encoding correctness in
  the API surface (GRAPH-03's named channel constructors, SUBS-07's explicit `transfer()`). A
  boolean would make STOP-03 a convention that each node is trusted to honour.
- **D-43:** The `Abort` event carries a cause code plus the `CLOCK_MONOTONIC_RAW` reading the
  caller took. The FSM therefore holds one end of D-31's interval directly, and a published
  abort record names why it happened rather than being anonymous.
- **D-44:** The cause enum has four variants from the start: operator command, watchdog deadline
  miss, internal fault, and shutdown. Only the first has a caller in Phase 2. They are all named
  now because the enum is part of the proven state space, and adding a variant later reopens the
  proof.
- **D-45:** Phase 2 ships a modelled consumer that takes permits and emits nothing real, proves
  STOP-03 against it, and writes down what Phase 6 must not do to keep the guarantee: no
  caching a permit across iterations, no emitting without one. The roadmap already says STOP-03
  is proven at FSM level against a modelled output gate; this makes the model's obligations
  explicit rather than leaving a future phase to infer them.

### What is proven, and how the claim is stated

- **D-46:** Four families of Kani harness over the enumerated space: `step()` is total and never
  yields an unreachable state; from any reachable state an `Abort` reaches `Stopping` or
  `Stopped` and no path returns to `Running`; no permit is issued from `Stopping` or `Stopped`;
  and no arithmetic overflow, index panic or `unwrap` can fire. The first three are STOP-01
  through STOP-03 mechanically. The fourth is what STOP-04 honestly buys in safe Rust.
- **D-47:** STOP-04's requirement text and ROADMAP.md Phase 2 success criterion 3 both say Kani
  proves absence of undefined behaviour. In a crate with no `unsafe`, most UB classes are
  already excluded by the type system, so the literal wording overstates what the proof adds.
  Both documents are amended together, in one reviewed commit that records the reason, and the
  amendment happens now, before any plan is written. Amending only REQUIREMENTS.md would
  reproduce the PLAT-03 split of 2026-09-09 exactly, where the requirement and the criterion
  stated different bars and nobody noticed until closure. Amending at phase close would be how
  a criterion gets quietly reshaped to fit its result, which ROADMAP criterion 1 was rewritten
  on 2026-09-07 to forbid.
- **D-48:** A scoping note is published in a new `docs/proofs/` directory, a sibling of the
  existing `docs/rig/`. It names which UB classes the type system excludes, which Kani checks,
  and which nothing in this phase covers. `docs/proofs/` also holds D-54's proof report now and
  Phase 4's loom results later.
- **D-49:** `#![forbid(unsafe_code)]` is restated at the stop crate's root even though the
  workspace lint already sets it, and the restatement is part of the published claim. It makes
  the guarantee local, survives any future workspace change, and puts the crate's most
  important property inside the crate rather than in a file a reader has to go find.
- **D-50:** Kani does not explore thread interleavings, and the latch is inherently concurrent.
  Phase 2 states plainly that the proof covers the sequential transition function and that
  cross-thread interleavings are unverified until CHAN-06 brings loom into CI in Phase 4. The
  gap is written into the published claim and tracked into Phase 4's scope rather than
  discovered there.

### CI gates

- **D-51:** Kani enters CI through a pinned `cargo install --locked kani-verifier` at an exact
  version plus `cargo kani setup`, in its own workflow job on `ubuntu-latest`, cached the way
  the existing jobs use `Swatinem/rust-cache`. Kani's own documentation states the GitHub
  Action supports Ubuntu 20.04 and `x86_64-unknown-linux-gnu` only, and `ubuntu-latest` is well
  past 20.04. Owning the install makes the version explicit and makes a Kani upgrade a
  deliberate commit rather than a surprise on a Tuesday.
- **D-52:** The Kani job is blocking on every push and pull request, unconditionally, with the
  same trigger shape as `fmt`, `clippy`, `test` and `deny`. STOP-05 asks for a blocking gate,
  and a three-state FSM solves in seconds, so there is no cost argument for narrowing it. A
  paths filter was rejected: a required check that reports skipped on most pushes either blocks
  merges or silently stops guarding.
- **D-53:** `cargo-llvm-cov` for STOP-06, not tarpaulin. llvm-cov reports region and branch
  coverage directly from LLVM instrumentation; tarpaulin's branch coverage has never been real
  on stable, and REQUIREMENTS.md offers either. The 100 percent threshold is scoped to the stop
  crate alone. The rest of the workspace gets no coverage gate, so the bar stays meaningful
  instead of becoming a number to negotiate down on every unrelated change.
- **D-54:** The proof result is committed as evidence, not left as a green check. A generated
  report under `docs/proofs/` names the Kani version, every harness and its outcome, and is
  refreshed deliberately and reviewed like any published claim under D-07. It stays outside
  `measurements/` and the D-13 manifest gate: that schema describes rig captures with
  environment snapshots and per-file checksums, and forcing a solver run into it would distort
  both. The project's most differentiating artifact should not be its only claim with no
  committed evidence beside it.

### Layout and sequencing

- **D-55:** The FSM lives in `crates/stop` (package `nr-stop`), the first runtime crate, and it
  depends on none of the five measurement crates. The STOP-07 harness is a separate binary
  named `nr-stop-harness`, which is already the exact tool string committed in
  `crates/metrics/tests/series.rs:128` under D-04, so the name in the schema and the name on
  disk agree without either being retrofitted. The harness depends on `nr-histogram` and
  `nr-metrics`; the dependency never points from the measurement tooling into the runtime being
  measured.
- **D-56:** Phase 2 starts now against an open Phase 1, confirming the decision already recorded
  in STATE.md on 2026-09-11. Everything except STOP-07 runs on the macOS dev host and needs no
  rig, and STOP-07 is sequenced last. Phase 1's remaining work is 01-15 task 3, which became
  closable when the scheduled fire passed on 2026-09-13 and is now two `journalctl` reads plus
  a coverage check. It closes on its own track. Phase 1 must still not be marked complete until
  task 3 closes and the phase verifier runs.

### Settled after research (added 2026-09-14)

- **D-57:** STOP-06's 100 percent branch coverage is produced by `cargo-llvm-cov` running under
  a pinned, dated nightly toolchain, in the coverage job only. The rest of CI stays on the
  stable toolchain `rust-toolchain.toml` pins. Research surfaced, and the main thread confirmed
  against primary sources, that neither tool REQUIREMENTS.md names can deliver branch coverage
  on stable: cargo-llvm-cov's own README states branch coverage "is currently optional and
  requires nightly", the underlying `-Z coverage-options=branch` is an unstable compiler flag
  documented in the Rust Unstable Book, and tarpaulin's help text reads
  `-b, --branch  Branch coverage: NOT IMPLEMENTED`, so tarpaulin cannot do it at any toolchain.
  The nightly is pinned by date the same way D-51 pins the Kani version, and the objection to
  nightly is already moot because this phase adopts Kani, which ships and runs its own bundled
  `nightly-2025-11-21`. STOP-06 is therefore satisfied as literally written and is NOT amended.
  Amending it would have been this phase's second reworded requirement after D-47, and two bars
  moved in one phase is the shape of fitting criteria to results that ROADMAP criterion 1 was
  rewritten on 2026-09-07 to forbid. `cargo-llvm-cov` is Apache-2.0 OR MIT, so
  `cargo deny check licenses` passes.

### Claude's Discretion

- The poll period for the D-33 stand-in loop, and whether more than one period is measured
- The atomic orderings on the latch store and poll, subject to D-50's gap being stated
- How the D-46 harness families are split across individual `#[kani::proof]` functions
- Which Kani version D-51 pins, decided from what is current at research time
- Whether the D-54 proof report regenerates on every CI run or on demand
- Run directory naming for the STOP-07 capture, within the existing `<date>-<rig>` convention
- How the D-44 cause code surfaces in a published abort record
- Module and file decomposition inside `crates/stop`

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

ROADMAP.md carries no `Canonical refs:` line for this phase. The list below is assembled from
REQUIREMENTS.md, PROJECT.md, the Phase 1 artifacts this phase builds on, and the external
documentation checked during discussion.

### Phase scope and requirements

- `.planning/ROADMAP.md` - Phase 2 section (lines 122-134): goal, the five success criteria,
  the dependency note that the FSM and proof need no rig, and the note that STOP-03 is proven
  against a modelled output gate with the decoder and sink consuming it in Phase 6. Criterion 3
  at line 129 is the text D-47 amends.
- `.planning/REQUIREMENTS.md` - STOP-01 through STOP-07 verbatim (lines 74-80) plus the
  traceability table. STOP-04 at line 78 is the other text D-47 amends.
- `.planning/PROJECT.md` - Core Value, the Constraints block (hot-path rules, rig discipline,
  licence), and the Key Decisions table.

### The Phase 1 contract this phase writes into

- `.planning/phases/01-trustworthy-measurement/01-CONTEXT.md` - the full D-01 to D-29 decision
  record. D-04 (schema generalises, STOP-07 is its first external consumer), D-05 (systemd
  oneshot, no login session), D-06 (assert and record, never enforce), D-07 (publication split),
  D-13 (blocking provenance gate), D-22 (report both, never subtract), D-28 (run admission from
  upstream evidence) and D-29 (pushed-stamp provenance) all bind this phase.
- `docs/measurement-protocol.md` - the PLAT-02 protocol any STOP-07 run must follow
- `docs/publication-layout.md` - where a published figure and its raw capture go
- `docs/rig/plat03-scheduling-latency-verdict.md` - the worked example of D-22's report-both
  rendering, which D-34 reuses
- `schemas/metrics.schema.json` and `schemas/manifest.schema.json` - the contracts the STOP-07
  harness emits into, unchanged by this phase

### Existing code the phase builds on

- `crates/metrics/src/series.rs` - `StageMetrics`, the open `stage` string (D-04), nullable
  percentiles that are never substituted
- `crates/metrics/tests/series.rs:123-136` - the committed test that already round-trips
  `stage: "emergency_stop.abort_latency"` with tool `nr-stop-harness`. D-55's naming comes from
  here.
- `crates/capture/src/preconditions.rs` - the fifteen D-06 checks a STOP-07 run must pass
- `crates/capture/src/sources.rs` - the `SystemFacts` trait with `LiveFacts` and `FixtureFacts`,
  the pattern that makes the macOS CI leg test real logic rather than nothing
- `crates/cli/src/tools.rs` - external tool invocation with explicit argv and the
  `NRMEASURE_*_PATH` fake-tool seam used to run the pipeline on a dev host with no rig
- `.github/workflows/ci.yml` - the fmt, clippy, test and deny jobs D-52's Kani job sits beside
- `Cargo.toml` - `unsafe_code = "forbid"`, `clippy::all = deny`, edition 2024, resolver 3

### Rig state and known hazards

- `.planning/STATE.md` - Current Position (the 2026-09-11 decision D-56 confirms, and the state
  of 01-15 task 3) and Blockers/Concerns. Two matter here: the rig boots untuned because
  `power-profiles-daemon` wins a boot race against `rt-tuning.service`, so every CPU reads
  `powersave` regardless of what `systemctl status` claims; and PLAT-01's 3.8 ms stall is
  unexplained, so every published latency figure including STOP-07's carries it as an explicit
  limitation naming the investigation's exposure.
- `docs/rig/plat01-stall-investigation.md` - the exposure statement STOP-07's caveats cite
- `deploy/systemd/` and `scripts/nr-push-to-rig.sh` - how code reaches the rig and how an
  unattended run is triggered

### External documentation

- https://model-checking.github.io/kani/install-guide.html - confirms
  `aarch64-apple-darwin` is a supported platform, so the proof runs on the macOS dev host
- https://model-checking.github.io/kani/install-github-ci.html - states the GitHub Action
  supports Ubuntu 20.04 and `x86_64-unknown-linux-gnu` only, which is the basis for D-51

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable assets

- `StageMetrics` and `metrics.schema.json` need no change. D-04 built them open, and
  `stage_names_are_open` already proves a non-cyclictest stage round trips using this phase's
  exact stage and tool strings.
- `nr-histogram` parses bins, overflows and hdrhistogram percentiles. The STOP-07 harness emits
  into it rather than computing percentiles itself.
- The D-06 precondition set, the D-14 environment snapshot and the ATTEMPT.json record apply to
  a STOP-07 run unchanged. A run that violates a precondition is refused, not footnoted.
- `render_plat03_verdict` in `crates/metrics/src/report.rs` is the worked precedent for D-34's
  side-by-side rendering with an explicit not-combined statement.
- The `SystemFacts` trait pattern is how this workspace keeps Linux-only logic testable on
  macOS. The STOP-07 harness should follow it rather than inventing a second approach.

### Established patterns

- Explicit argv, never a shell, for every external process (T-1-09 in `crates/cli/src/tools.rs`)
- Env-var seams for fake tools so a pipeline runs end to end on a dev host with no rig
- A generated human-readable report beside a machine-readable source of truth, never
  hand-maintained in parallel (D-12)
- Prose style from `~/.claude/rules/common/writing-style.md`: no em dashes, ASCII only, no
  emoji, sentence case headings. Every committed document so far follows it.
- CI workflow structure built so adding a blocking gate is a small diff, stated in Phase 1's
  D-02 with Kani named as the Phase 2 addition

### Integration points

- A sixth workspace member, `crates/stop`, plus the `nr-stop-harness` binary (D-55)
- A new `kani` job in `.github/workflows/`, and a coverage job scoped per D-53
- A new `docs/proofs/` directory, sibling to `docs/rig/` (D-48, D-54)
- `measurements/<date>-<rig>` gains one STOP-07 headline run under the existing convention
- Phase 6 consumes the permit type from `nr-stop`; Phase 4's loom work closes D-50's gap

</code_context>

<specifics>
## Specific Ideas

### Safety and the measurement are deliberately two different instants

D-40 closes the permit supply at the store; D-41 ends the measured interval at the
acknowledgement. These are not the same moment, and separating them is the point. The safety
property (no new output) holds at the store and depends on nothing responding. The measured
number is an observation latency: how long until the hot path notices. A hung node makes the
number unobtainable and leaves safety intact. Any plan or review that collapses the two, or
that describes the acknowledgement as when the system becomes safe, has broken STOP-02.

### The published claim is narrower than the requirement's original wording, on purpose

The crate contains no `unsafe`. Most UB classes are excluded by the compiler before Kani runs.
What Kani adds is the latch property, totality over the enumerated space, the gate invariant,
and panic and overflow freedom. Stating that precisely is worth more to a reviewer who knows
the field than the broader sentence would be, and the broader sentence is the kind of claim
this project has already corrected twice in its own published artifacts.

### The 100 percent branch coverage gate is scoped, not global

D-53 applies the threshold to `crates/stop` alone. A workspace-wide 100 percent bar would be
abandoned within a phase, and a bar that gets lowered once is not a bar. D-32's single poll
point per iteration exists partly so the branch count stays small enough that 100 percent is a
real property of a small module rather than an exercise in writing tests for coverage's sake.

</specifics>

<deferred>
## Deferred Ideas

Nothing here is dropped. Each is owned by a named later phase.

- The `emergency_stop.abort_latency` stage in the weekly series and the regression baseline.
  Deferred to Phase 5 or 6, when the hot path is real and a weekly series would track the
  runtime rather than D-33's stand-in.
- Exhaustive interleaving verification of the latch. Phase 4, CHAN-06, when loom enters CI.
  D-50 names the gap so it is tracked rather than rediscovered.
- A POSIX signal abort source. Deferred until something needs it, because it forces the first
  `unsafe` block into the crate D-49 wants to keep free of it.
- A deadline watchdog as an abort source. The cause variant exists (D-44); the detector needs
  the WCET annotations GRAPH-01 introduces in Phase 5.
- STOP-07 re-measured under the full Phase 3 substrate (`mlockall`, preallocated pools, the
  allocator hook) and again against the real DAG in Phase 6. D-36's caveat states that the
  Phase 2 figure predates both.
- The real decoder and sink behind the permit. Phase 6, bound by D-45's written contract.
- The Creusot functional proof. CREU-01, v2, already out of scope in the roadmap.

No todos matched this phase: `donny-tools todo match-phase 2` returned zero.

</deferred>

---

*Phase: 02-proven-emergency-stop*
*Context gathered: 2026-09-14*
