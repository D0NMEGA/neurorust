---
status: PARTIAL
agent: donny-phase-researcher
phase: 2
confidence: MEDIUM
---

# Phase 2: Proven emergency_stop - Research

**Researched:** 2026-09-14
**Domain:** Rust formal verification (Kani model checking), LLVM source-based coverage instrumentation, real-time scheduling and clocks from Rust, safe atomic state publication, and measurement-harness integration
**Confidence:** MEDIUM overall. Individual findings range HIGH (Kani syntax, cargo-llvm-cov branch coverage, crate licenses) to LOW (Kani plus edition 2024/resolver 3, `cargo kani setup` download size). See Metadata.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

Decision IDs continue the project-wide sequence rather than restarting at D-01. Phase 1 used
D-01 through D-29 with no phase qualifier, STATE.md and the plans refer to them that way, and
this phase's text cites several of them. Restarting the numbering would make "D-22" ambiguous.

#### The abort trigger and what gets measured

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

#### The FSM, the latch, and the output gate

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

#### What is proven, and how the claim is stated

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

#### CI gates

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

#### Layout and sequencing

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

### Claude's Discretion

- The poll period for the D-33 stand-in loop, and whether more than one period is measured
- The atomic orderings on the latch store and poll, subject to D-50's gap being stated
- How the D-46 harness families are split across individual `#[kani::proof]` functions
- Which Kani version D-51 pins, decided from what is current at research time
- Whether the D-54 proof report regenerates on every CI run or on demand
- Run directory naming for the STOP-07 capture, within the existing `<date>-<rig>` convention
- How the D-44 cause code surfaces in a published abort record
- Module and file decomposition inside `crates/stop`

### Deferred Ideas (OUT OF SCOPE)

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
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| STOP-01 | `emergency_stop` is a finite state machine with an explicit reachable state space | Architecture Patterns: pure `step()` function design; Code Examples: Kani harness family 1 (totality) |
| STOP-02 | From any reachable state, an abort signal drives the system to a defined safe state | Code Examples: Kani harness family 2 (reachability, no return to `Running`); Architecture Patterns: atomic ordering argument |
| STOP-03 | No decoder output is produced for the remainder of a session after a stop | Code Examples: Kani harness family 3 (no permit from `Stopping`/`Stopped`); Architecture Patterns: capability-typed gate, D-45's modelled consumer |
| STOP-04 | Kani proves totality, invariants, and panic/overflow freedom; the crate forbids `unsafe`; the published claim is scoped | Standard Stack: Kani; Common Pitfalls: the `compare_exchange` modelling concern; Don't Hand-Roll: proof vs. type-system-excluded UB |
| STOP-05 | `cargo kani` passes in CI as a blocking gate | Standard Stack + Code Examples: the CI job YAML, install/setup/caching |
| STOP-06 | The module has 100 percent branch coverage (tarpaulin or llvm-cov) | Common Pitfalls: cargo-llvm-cov branch coverage requires nightly; Open Questions: the STOP-06-as-written tension |
| STOP-07 | Abort latency is bounded and measured on the reference rig | Architecture Patterns: harness-as-standalone-binary, CLOCK_MONOTONIC_RAW reading, SCHED_FIFO/affinity, histogram/manifest reuse |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

From the project's `CLAUDE.md` (Constraints block) and `PROJECT.md`, binding on this phase:

- **Hot path**: no allocation, no locks, no logging, no blocking syscalls, no unbounded queue,
  no work-stealing scheduler; burden of proof is on whoever adds one. STOP-07's stand-in loop
  (D-33) is a hot-path stand-in and should be written to this bar even though SUBS-01
  through SUBS-04 (allocation locking, arenas) are explicitly out of scope until Phase 3;
  D-36 already states the caveat that the measurement predates the no-allocation guarantee, so
  the loop should avoid gratuitous allocation without claiming the SUBS guarantees it does not
  yet have.
- **Platform**: Linux only for deployment; PREEMPT_RT required for any published RT number.
  STOP-07's rig capture must run on the Dell Precision 3591 under the RT kernel; the FSM and
  Kani proof need no rig (already stated in ROADMAP.md and D-56).
- **Rig discipline**: every quoted figure names the machine it was measured on. D-34/D-37 already
  bind STOP-07 to this.
- **Licence**: Apache-2.0 OR MIT dual. Every new dependency this phase adds (Kani's own crates,
  any CPU-affinity/scheduling crate, `cargo-llvm-cov`) must clear `deny.toml`'s allow-list
  (`Apache-2.0`, `MIT`, `Apache-2.0 WITH LLVM-exception`, `BSD-2-Clause`, `BSD-3-Clause`, `ISC`,
  `Unicode-3.0`, `Zlib`). Verified in this research: `kani-verifier` (Apache-2.0 OR MIT per its
  own repository convention, not independently re-checked here beyond crates.io metadata),
  `thread-priority` 3.1.1 (MIT), `core_affinity` 0.8.3 (MIT/Apache-2.0), `nix` 0.31.3 (MIT),
  `cargo-llvm-cov` 0.9.1 (Apache-2.0 OR MIT) all clear the allow-list as-is.
- **Team**: one person plus Claude, sequential, no parallel workstreams. This phase's plan should
  not assume any task can run concurrently with another.
- **Provenance**: no public artifact may imply a specification obtained via an advisor or industry
  contact. Not directly triggered by this phase's content, noted for completeness.

From `~/.claude/CLAUDE.md` and `~/.claude/rules/` (user-level, apply to all work in this
repository): no em dashes, ASCII only, no emoji, sentence case headings (already the house style
for every committed document, confirmed by reading existing `docs/` and `CONTEXT.md` files);
`thiserror` for library errors and `anyhow` for application errors (already the pattern in every
existing crate, e.g. `crates/histogram/src/hist.rs`'s `HistError`, `crates/cli`'s `anyhow::Result`);
immutable-by-default, small files, YAGNI (resist abstracting `nr-stop-harness`'s pinning code
behind a trait unless a second Linux/non-Linux implementation is genuinely needed, per the
"Simplicity" rule); surgical diffs (any refactor identified below, such as promoting a private
helper to a public one, should be the smallest change that unblocks reuse, not a rewrite).
`~/.claude/rules/context7.md` requires Context7 before asserting library API behaviour; Context7
was queried for Kani (no coverage: only an unrelated "Kanidm" identity-server library resolved)
and for `nix` (resolved to `/nix-rust/nix` but returned no matching snippets for
`clock_gettime`/`sched_setaffinity`); both gaps are covered instead by WebFetch of the crates'
own docs.rs pages, cited below.

## Summary

Phase 2 asks for two things that do not normally travel together: a machine-checked proof over
a tiny state machine, and a rig measurement of how long that machine takes to close under load.
Both are well within reach with the stack the operator already locked in (Kani, cargo-llvm-cov,
`crates/stop` plus `nr-stop-harness`), but two of the seven requirements have a real technical
snag that the locked decisions do not yet resolve, and this research surfaces both plainly
rather than papering over them.

The first snag is STOP-06. `cargo-llvm-cov` reports **region** coverage on the stable toolchain
already pinned in `rust-toolchain.toml`. True **branch** coverage (the literal text of STOP-06
and D-53) is gated behind an unstable `--branch` flag that requires a nightly toolchain dated
2024-03-16 or later, tracked upstream at `rust-lang/rust#79649` and unresolved as of this
research. D-53 correctly rejects tarpaulin (whose branch coverage "has never been real on
stable") but does not name this because cargo-llvm-cov's own stable-vs-nightly split was not
yet surfaced. The three ways forward are laid out in Common Pitfalls and Open Questions; this
research recommends pinning a nightly toolchain for the coverage CI job only, which is a small,
well-precedented addition given the project already pins exact tool versions elsewhere (D-51's
Kani pin is the same discipline applied to compilers).

The second is architectural: how `nr-stop-harness` plugs into the existing `nrmeasure`
machinery without violating D-55's rule that measurement tooling never depends on the runtime
crate. Reading the actual code (not just the schema) shows `nrmeasure run`'s orchestration in
`crates/cli/src/cmd/run.rs` is a 2900-line function deeply specific to cyclictest/hwlatdetect/
rtla run classes, and that several of its environment-snapshot helpers (`harness_info()`,
`RunDir::create`) are private to a binary-only crate with no library target at all, so nothing
outside `crates/cli` can import them regardless of dependency direction. The clean answer is
for `nr-stop-harness` to be a standalone binary that imports `nr-capture`, `nr-manifest`,
`nr-histogram` and `nr-metrics` as libraries directly (mirroring, not calling, what `nrmeasure
run` does), never invoked as a subprocess by `nrmeasure`. This is detailed in Architecture
Patterns.

For the proof itself, Kani's syntax is stable, well documented, and directly supports the
D-46 harness families: `#[derive(kani::Arbitrary)]` on the FSM's enums, `kani::any()`/
`kani::assume()` for exhaustive-but-constrained inputs, and `#[cfg(kani)]` to keep harnesses out
of normal builds. One genuine risk surfaced in research (a `model-checking/verify-rust-std`
issue reporting Kani mis-modelling a failed `compare_exchange` as successful) argues for a
specific design choice, not a rejection of Kani: keep the function Kani actually proofs a pure,
non-atomic `step(state, event) -> state` with no atomics in it at all, and keep the atomic
publication (the concurrent wrapper D-50 says is unverified this phase) as a thin, separately
argued layer around it. That split is good practice independent of whether the CAS bug still
affects Kani 0.67.0, and it also gives Phase 4's loom retrofit the smallest possible atomic
surface to explore.

For STOP-07's mechanics: `CLOCK_MONOTONIC_RAW` is not what `std::time::Instant` uses on Linux
(it uses `CLOCK_MONOTONIC`), is vDSO-accelerated only since Linux 5.3 (well before the rig's
kernel 7.0.0-31-realtime, so this should be fast, but is worth an empirical check per D-35's own
instrument-error framing), and the rig's kernel command line already carries `tsc=reliable`
(found in the phase's own recon, not asserted from training knowledge), meaning the kernel is
configured to trust the TSC as the clock source without runtime cross-core verification. Two
small, safe, cross-platform (including a documented macOS no-op, so the workspace-wide macOS CI
leg keeps compiling and testing real logic rather than being cfg'd out) crates cover D-36's
scheduling and affinity needs without introducing `unsafe` anywhere: `thread-priority` for
`SCHED_FIFO`, `core_affinity` for CPU pinning. `nix` covers the safe `clock_gettime` call if the
team prefers one fewer crate over two purpose-built ones.

**Primary recommendation:** build `crates/stop` as a pure, atomics-free, `forbid(unsafe_code)`
state machine that Kani proofs directly; wrap it in a thin, separately-argued atomic layer;
ship `nr-stop-harness` as a standalone binary (not an `nrmeasure run` subcommand) that imports
`nr-capture`/`nr-manifest`/`nr-histogram`/`nr-metrics` directly; and resolve the STOP-06 branch
coverage question explicitly, in writing, before Wave 0 closes, the same way D-47 resolved
STOP-04's overclaim.

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|---------------|
| `kani-verifier` (installs `cargo-kani`, `kani-driver`) | 0.67.0 (2026-01-16, latest on crates.io as of 2026-09-15) [VERIFIED: crates.io API] | Bounded model checking of the FSM's transition function (STOP-04) | Only actively maintained Rust model checker with first-class `#[kani::proof]`/`Arbitrary` ergonomics and a documented aarch64-apple-darwin build, matching the dev host [CITED: model-checking.github.io/kani/install-guide.html] |
| `cargo-llvm-cov` | 0.9.1 (2026-09-06, latest on crates.io) [VERIFIED: crates.io API] | Region/line coverage for STOP-06, via LLVM source-based instrumentation | Works on the pinned stable toolchain for the base coverage report (`cargo +stable install cargo-llvm-cov --locked`); this is exactly the reason D-53 preferred it over tarpaulin [CITED: raw README, taiki-e/cargo-llvm-cov] |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `thread-priority` | 3.1.1 (2026-06-20, MIT) [VERIFIED: crates.io API] | Safe `SCHED_FIFO` real-time policy + priority from Rust, no `unsafe` in caller code | In `nr-stop-harness` to satisfy D-36's scheduling requirement without touching raw `libc` |
| `core_affinity` | 0.8.3 (2025-03-01, MIT/Apache-2.0) [VERIFIED: crates.io API] | Safe per-thread CPU pinning, cross-platform including a documented macOS no-op fallback | In `nr-stop-harness` to satisfy D-36's affinity requirement; the macOS fallback keeps `cargo test --workspace --all-targets` (which runs on `macos-latest`, per `ci.yml`) exercising real harness code rather than a `cfg`'d-out stub |
| `nix` | 0.31.3 (2026-05-11, MIT) [VERIFIED: crates.io API] | Safe `clock_gettime(ClockId::CLOCK_MONOTONIC_RAW)` (Linux-only variant) [CITED: docs.rs/nix ClockId page] and, if preferred over `core_affinity`, safe `sched_setaffinity` | Only the `time` module is strictly needed if `thread-priority`+`core_affinity` are used for scheduling/affinity; `nix::sched` has no `sched_setscheduler`, so it cannot replace `thread-priority` for the `SCHED_FIFO` requirement |
| `hdrhistogram` | 7.6.0 (already a workspace dependency) [VERIFIED: local `Cargo.toml`] | Percentile computation for the STOP-07 headline figure | Already used identically by `crates/histogram/src/percentiles.rs`; reuse the same pattern rather than a second histogram library |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `thread-priority` + `core_affinity` (two crates) | `nix::sched` alone | `nix` has no `sched_setscheduler` binding [VERIFIED: docs.rs/nix sched module page], so it cannot set `SCHED_FIFO` by itself; would still need `thread-priority` or a raw `unsafe` `libc::sched_setscheduler` call |
| Safe wrapper crates | Shell out to `chrt`/`taskset`, matching the existing `crates/cli/src/tools.rs` explicit-argv pattern | Matches an established codebase convention and needs zero new Rust dependencies, but only pins at process granularity; D-36's two-thread design (abort thread and hot-path thread, potentially on different cores, per D-35's cross-core skew framing) needs per-thread pinning that `taskset` cannot express once threads already exist inside one process. Viable only if the harness is restructured as two separate OS processes, which adds cross-process IPC complexity nothing here currently needs |
| `cargo-llvm-cov` region coverage on stable | `cargo-llvm-cov --branch` on a pinned nightly, for the coverage job only | Literally satisfies STOP-06's "branch coverage" wording, at the cost of a second, less-stable toolchain pin scoped to one CI job. See Common Pitfalls and Open Questions |
| Kani's own bundled toolchain | N/A | Not really an alternative: `cargo kani setup` downloads its own nightly compiler into `~/.kani/` (or `$KANI_HOME`), entirely separate from `rustup`/`rust-toolchain.toml` [CITED: model-checking.github.io/kani/install-guide.html, corroborated by a third-party CI discussion, `leynos/chutoro#244`, LOW confidence on the exact size figure]. There is no conflict with the workspace's pinned `channel = "stable"` to design around |

**Installation:**
```bash
# Kani (pin the exact version the plan settles on; 0.67.0 was current at research time)
cargo install --locked kani-verifier@0.67.0
cargo kani setup

# Coverage
cargo +stable install cargo-llvm-cov --locked
rustup component add llvm-tools-preview   # cargo-llvm-cov can also prompt for this itself
```

**Version verification:** confirmed directly against the crates.io registry API on 2026-09-15
(commands run in this session: `curl -H "User-Agent: ..." https://crates.io/api/v1/crates/<name>`
for `kani-verifier`, `cargo-llvm-cov`, `thread-priority`, `core_affinity`, `affinity`, `nix`).
`affinity` (0.1.2, last published 2021-10-16) was rejected as stale; `core_affinity` (last
published 2025-03-01) and `thread-priority` (last published 2026-06-20) are both current.

## Architecture Patterns

### Recommended project structure

```
crates/
|-- stop/                  # package nr-stop; forbid(unsafe_code) restated (D-49)
|   |-- src/
|   |   |-- lib.rs
|   |   |-- state.rs       # pure step(state, event) -> state; the thing Kani proofs
|   |   |-- event.rs       # Abort { cause, at_raw_ns }, Acknowledged; Cause enum (D-44)
|   |   |-- latch.rs       # thin atomic wrapper around state.rs; NOT Kani-proofed (D-50)
|   |   `-- gate.rs        # capability-typed output permit (D-42)
|   `-- tests/
|       `-- proofs.rs      # #[cfg(kani)] harnesses, or split per D-46 family; see below
`-- stop-harness/          # package nr-stop-harness; a separate binary, NOT forbid(unsafe_code)
    `-- src/
        `-- main.rs        # standalone: depends on nr-stop, nr-capture, nr-manifest,
                            # nr-histogram, nr-metrics. Never invoked by nrmeasure.
```

### Pattern 1: a pure, atomics-free transition function is what Kani proofs (research question 3 and 8)

**What:** `crates/stop`'s core is a plain function, no `Atomic*`, no `OnceLock`, operating only
on owned enum values:

```rust
// Illustrative; the plan owns the exact signature and module split.
pub fn step(state: State, event: Event) -> State {
    match (state, event) {
        (State::Running, Event::Abort(_)) => State::Stopping,
        (State::Stopping, Event::Acknowledged) => State::Stopped,
        // Every other (state, event) pair is a self-loop or is structurally
        // unreachable; D-46 family 1 requires this match to be total with no
        // wildcard arm for a business-critical enum (matches this project's
        // Rust patterns rule: "Always match exhaustively -- no wildcard `_`
        // for business-critical enums").
        (s, _) => s,
    }
}
```

**When to use:** for exactly the properties D-46 lists (totality, reachability, gate invariant,
panic/overflow freedom). A thin `EmergencyStop` type wraps an `AtomicU8` (or similar) encoding
of `State`, calls `step()` internally on its own loaded value, and performs the actual
publish/observe via `compare_exchange`/`load`. That wrapper is **not** what Kani proofs against
in this phase (D-50); it is hand-argued.

**Why this split, concretely:** research surfaced `model-checking/verify-rust-std` issue #673,
which reports a pinned Kani build reporting a failed `compare_exchange` as `Ok` instead of `Err`
[CITED: GitHub issue #673, model-checking/verify-rust-std; the affected Kani version was not
confirmed against the 0.67.0 this phase pins, so treat as MEDIUM-confidence motivation, not a
proven defect in the exact pinned version]. If true of the pinned version, any proof that
branches on a `compare_exchange`'s `Err` arm for a safety-relevant property would be unsound in
exactly the way D-50 already says is out of scope (concurrency modelling), so keeping the
proofed function atomics-free sidesteps the question entirely rather than depending on an
unverified claim about solver internals. This is good design hygiene regardless of whether the
bug affects 0.67.0: it also gives Phase 4's loom retrofit (CHAN-06) the smallest possible number
of independent atomics to exhaustively explore, avoiding state-space blowup from day one.

**Recommended orderings for the thin wrapper (Claude's Discretion item, argued not proved, per
D-50):**
- Abort's `Running -> Stopping` transition: `compare_exchange(Running, Stopping,
  Ordering::AcqRel, Ordering::Acquire)`. `Release` on success publishes the cause/timestamp
  payload (written just before the CAS) to any thread that later `Acquire`-loads the state;
  `Acquire` on failure lets a losing caller still safely observe the winner's payload.
- The hot-path poll: `load(Ordering::Acquire)`. This is what must synchronize-with the abort's
  `Release` store; a `Relaxed` load here would be an actual correctness bug, not just excess
  caution, because D-43's design makes the safety-relevant payload (cause, timestamp) something
  that must become visible together with the state flip, and `Relaxed` gives no such guarantee.
- The `Acknowledged` transition (`Stopping -> Stopped`, D-41): the same `AcqRel`/`Acquire`
  shape, by the same argument.
- Do not reach for `SeqCst`: nothing else in the crate needs a total cross-atomic order, so
  `SeqCst` would be paid for with no argued benefit, and would leave an unexamined choice for
  Phase 4 to either keep (paying the cost) or revisit (redoing this argument anyway).
- What would make Phase 4 harder: shipping `Relaxed` anywhere safety-relevant (loom will almost
  certainly catch this the moment CHAN-06 lands, which would mean Phase 2's "proven" crate gets
  its first red flag from Phase 4, not from Kani, undermining the claim's credibility even
  though Kani itself never claimed to check this); or encoding the FSM state as ad hoc bit
  tricks across more than one atomic, which multiplies the interleavings loom must enumerate.

This entire subsection is **argued reasoning**, not a claim sourced from documentation; it
follows directly from D-50's own framing ("thread interleavings are NOT verified in this
phase... recommend the orderings... and state the argument for them plainly").

### Pattern 2: `nr-stop-harness` is a standalone binary, never an `nrmeasure run` tool (research question 7)

**What:** Research question 7 poses this as a choice between "a tool invoked by `nrmeasure run`"
and "a standalone binary that calls the nr-capture library itself." Reading the actual code
resolves this in favour of the standalone binary, for three concrete reasons found in this
session, not just D-55's abstract dependency-direction rule:

1. **`nrmeasure run`'s orchestration is not generic.** `crates/cli/src/cmd/run.rs::execute()` is
   about 2900 lines total, and its first ~150 lines alone are guard clauses specific to the
   cyclictest/hwlatdetect/rtla domain: mutually-exclusive firmware-instrument flags, a
   `RunClass`/`ThermalProfile` pair with rules like "`--thermal-profile hot-screen` is accepted
   only for `--class screen`", and fixture-path guards that forbid `NRMEASURE_FACTS_FIXTURE` on
   headline/weekly/soak runs. Teaching this function a sixth, structurally different concept
   (a small number of discrete abort trials, not a continuous periodic histogram) would grow an
   already-large function rather than add a clean case to it, which the project's own coding
   style rules single out by name ("Kitchen Sink: restructuring half the codebase while you are
   in there" / "Runaway Refactor").
2. **The reusable pieces are already correctly library-ified, but the run-specific glue is not,
   and cannot be, because `crates/cli` is a binary-only crate.** `nr_capture::preconditions::
   run_all`/`refuse_on_violation`, `nr_capture::environment::snapshot()`, and
   `nr_capture::interference::snapshot()` are all `pub fn` in `nr-capture` [VERIFIED: local
   `crates/capture/src/{preconditions,environment,interference}.rs`] and directly importable by
   a new binary. But `RunDir::create` (the `<date>-<rig>-<run-class>[-NN]` directory-naming and
   creation logic) lives in `crates/cli/src/rundir.rs`, declared `mod rundir;` (not `pub mod`)
   in `crates/cli/src/main.rs` [VERIFIED: local source]. `crates/cli`'s `Cargo.toml` defines only
   a `[[bin]]` target, no `[lib]`, so this is not a visibility oversight to fix by adding `pub`;
   a binary crate has nothing else can depend on. `harness_info()` (binary name, git SHA,
   executable checksum) is similarly a private function in `run.rs` and is inherently specific
   to the `nrmeasure` binary's own identity, so a `nr-stop-harness`-specific equivalent (a few
   lines, following the same `env!("CARGO_PKG_VERSION")`/`git_output(&["rev-parse", "HEAD"])`
   shape) is the right amount of new code, not a gap to close upstream.
3. **The schema is designed for this.** `StageMetrics.stage`/`.tool` are open strings by design
   (D-04), and `crates/metrics/tests/series.rs`'s `stage_names_are_open` test already round-trips
   `stage: "emergency_stop.abort_latency"`, `tool: "nr-stop-harness"` with zero code changes to
   `nr-metrics` [VERIFIED: local `crates/metrics/tests/series.rs:123-136`]. `ArtifactKind`
   already has an `Other` variant [VERIFIED: local `crates/manifest/src/fields.rs:681-688`], so
   the harness's raw capture (see Pattern 3 below) needs no new `ArtifactKind`. `RunClass::
   Headline` and `InstrumentClass::HeadlineSeries` already exist [VERIFIED: local
   `crates/manifest/src/fields.rs:99-118`], matching D-37's "one headline-class run" exactly.
   Nothing in `nr-manifest` or `nr-metrics` needs a new variant for this phase.

**Recommended shape of `nr-stop-harness main()`:** build its own `EnvironmentSnapshot` via
`nr_capture::environment::snapshot()`, run `nr_capture::preconditions::run_all()` /
`refuse_on_violation()` exactly as `nrmeasure run` does, create its own run directory (a small,
new, local equivalent of `RunDir::create`, following the same `<date>-<rig>-<run-class>[-NN]`
naming convention the rest of the project uses), pin threads and set `SCHED_FIFO` (Pattern 4),
run the D-33 stand-in loop for N abort trials, compute percentiles (Pattern 3), write
`manifest.json` (via `nr_manifest::RunManifest` and `blake3_file`) and a `StageMetrics` entry
(via `nr_metrics::series::append`), and stamp its own `HarnessInfo`. This satisfies D-55's
dependency-direction rule exactly (nothing in `nr-manifest`, `nr-histogram`, `nr-capture`,
`nr-metrics`, or `nr-cli` gains a dependency on `nr-stop` or `nr-stop-harness`) and matches the
existing "reusable assets" already named in `02-CONTEXT.md`.

**Flag for the plan:** this means Wave 0 (or an early task) should include a small, additive
task to give `crates/cli`'s run-directory-naming logic a public home if the team wants
`nr-stop-harness` to reuse it byte-for-byte rather than reimplement an equivalent ~40-line
function. Given `RunDir` is genuinely small (roughly 100 lines including its private helpers)
and the project's own YAGNI rule ("copy-paste twice before you abstract"), a reasonable, smaller
alternative is to let `nr-stop-harness` implement its own small equivalent now and only promote
`RunDir` into `nr-manifest` if a third caller ever needs it. Either is defensible; this is
recorded as an open decision, not a research gap.

### Pattern 3: emit through `nr-histogram`'s generic percentile machinery, not the cyclictest `.hist` text format (research question 6)

**What was checked:** `crates/histogram/src/hist.rs` parses cyclictest's `.hist` text format
specifically: per-thread columns, a footer with per-thread min/avg/max/overflow lines, and
overflow-cycle-number lists [VERIFIED: local source, full file read]. `crates/histogram/src/
percentiles.rs` computes percentiles from a parsed `CyclictestRun` via `hdrhistogram`, using
`Histogram::<u64>::new(SIGFIG)` with `SIGFIG = 3` [VERIFIED: local source].

**Recommendation:** do not force STOP-07's raw samples (N abort-trial latencies, one thread, no
overflow-cycle-number concept) through the `.hist` text format just so `parse_hist_file` can
re-parse them; that is a pure round-trip with no benefit and a real cost (cyclictest's format
assumes multi-thread columns that do not fit a "N trials on one path" measurement). Instead, add
one small, additive, non-breaking function to `crates/histogram/src/percentiles.rs` that
computes the same `Percentiles` struct from a flat `&[u64]` of raw samples (no `CyclictestRun`,
no per-thread structure, min/max computed as the exact slice min/max rather than through
hdrhistogram's own quantization, matching the existing "exact, never the overflow bound"
philosophy already documented on `CyclictestRun::max_us()`). This is the smaller change: it is
purely additive to a crate that already depends on `hdrhistogram`, needs no changes to
`nr-manifest` (`ArtifactKind::Other` already covers the raw capture file, which can be a simple
one-value-per-line or CSV text file of raw nanosecond deltas), and keeps `nr-stop-harness`'s use
of `nr-histogram` consistent with why D-55 named that crate as a dependency in the first place.

**Alternative considered and rejected:** having `nr-stop-harness` depend on `hdrhistogram`
directly and duplicate the dozen or so lines of `Histogram`-building logic already in
`nr-histogram`. Rejected because it would violate the project's own Don't Hand-Roll instinct
(reimplementing logic a sibling crate already has) and would silently diverge from D-55's stated
dependency on `nr-histogram` if that dependency ends up unused.

### Pattern 4: safe, cross-platform SCHED_FIFO and CPU affinity (research question 4)

```rust
// Illustrative shape; exact API surface should be re-checked against whatever
// thread-priority/core_affinity versions the plan pins at implementation time.
use thread_priority::{RealtimeThreadSchedulePolicy, ThreadPriority, ThreadSchedulePolicy};

// Pin the current thread to a specific isolated core (no unsafe; no-ops with a
// documented, non-panicking fallback on macOS, so `cargo test --workspace
// --all-targets` on macos-latest still exercises this code path).
let core_ids = core_affinity::get_core_ids().unwrap_or_default();
if let Some(core) = core_ids.iter().find(|c| c.id == target_cpu) {
    core_affinity::set_for_current(*core);
}

// Request SCHED_FIFO at a given priority (Linux; other platforms get thread_priority's
// own cross-platform fallback behaviour, not independently verified in this session).
thread_priority::set_thread_priority_and_policy(
    thread_priority::thread_native_id(),
    ThreadPriority::Crossplatform(50u8.try_into().unwrap()),
    ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::Fifo),
)?;
```

**Sources:** `thread-priority` supports `Linux, Android, DragonFly, FreeBSD, OpenBSD, NetBSD,
macOS, iOS, Windows` and exposes `ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::
Fifo)` with no `unsafe` required by the caller [CITED: raw README, iddm/thread-priority, fetched
via WebFetch; the exact function name/signature above is reconstructed from the search-indexed
docs and should be re-verified against the pinned version's actual docs.rs page before coding,
LOW-MEDIUM confidence on the precise signature, HIGH confidence on the capability existing].
`core_affinity` supports "Linux, Mac OSX, and Windows", exposes `get_core_ids()`/
`set_for_current(id)` with no `unsafe`, and on macOS aarch64 specifically (the dev host's exact
architecture) "it's not possible to pin a thread to a specific core, but the library will still
try to request the highest performance for the thread" rather than failing [CITED: WebSearch
synthesis of Elzair/core_affinity_rs, MEDIUM confidence, not independently re-verified against
the raw source]. Neither crate needs a raw `libc` call or an `unsafe` block in `nr-stop-harness`
itself; both clear `deny.toml`'s license allow-list.

### Pattern 5: `CLOCK_MONOTONIC_RAW`, not `Instant`, read at both ends (research question 5)

`std::time::Instant::now()` on Linux is implemented via `libc::clock_gettime(CLOCK_MONOTONIC)`,
**not** `CLOCK_MONOTONIC_RAW` [CITED: multiple secondary sources plus the existence of the open
upstream tracking issue `rust-lang/rust#37902`, "time.rs should consider using
CLOCK_MONOTONIC_RAW instead of CLOCK_MONOTONIC on Linux", whose own title presupposes current
behaviour is `CLOCK_MONOTONIC`; MEDIUM-HIGH confidence, a direct fetch of the current
`library/std/src/sys/pal/unix/time.rs` source did not conclusively show the `Instant`-specific
call site in this session]. This means D-35's requirement (both ends read
`CLOCK_MONOTONIC_RAW`) cannot be satisfied with `std::time::Instant`; it requires an explicit
`clock_gettime(2)` call with `CLOCK_MONOTONIC_RAW`, either via `nix::time::clock_gettime(nix::
time::ClockId::CLOCK_MONOTONIC_RAW)` (safe, no `unsafe` in caller code, confirmed available on
Linux/Android/Emscripten/Fuchsia in `nix`'s `ClockId` [CITED: docs.rs/nix `ClockId` page, fetched
via WebFetch]) or a narrow `unsafe` `libc::clock_gettime` call if the team prefers not to add
`nix` as a dependency just for this one call.

**vDSO acceleration:** `CLOCK_MONOTONIC_RAW` gained vDSO acceleration (avoiding a real syscall)
on Linux **5.3** [CITED: berthub.eu, "On Linux vDSO and clock_gettime", a well-regarded systems
blog corroborated by a Hacker-News-linked LKML patch thread]. Before that patch, the syscall
fallback measured 300-700ns on a Haswell CPU; after, roughly 100ns [CITED: same source, MEDIUM
confidence: single source, numbers not independently reproduced on the reference rig]. The
reference rig runs kernel `7.0.0-31-realtime`, far past 5.3, so vDSO acceleration should apply,
but D-35 itself asks for this to be *measured*, not assumed: recommend the STOP-07 plan include
a cheap, direct verification (`/sys/devices/system/clocksource/clocksource0/current_clocksource`
reads `tsc`, and a tight loop calling `clock_gettime(CLOCK_MONOTONIC_RAW)` a few million times to
get an empirical per-call overhead) as part of the "clock read overhead... characterised in their
own committed capture" D-35 already requires. One directly relevant local fact found in this
research: the rig's kernel command line already includes `tsc=reliable skew_tick=1` [VERIFIED:
local `docs/rig/recon-2026-08-31/FINDINGS.md:123`], meaning the kernel is configured to trust the
TSC as the clock source without its own runtime cross-core verification, which is exactly why
D-35's own empirical cross-core skew measurement is the right call rather than an assumption
that skew is zero.

**Cross-core skew methodology:** no source specific to "same-machine, invariant-TSC, cross-core
`CLOCK_MONOTONIC_RAW` skew measurement" was found in this session (general academic search
returned NTP/PTP-scale, cross-machine clock-sync literature, which solves a related but
differently-scaled problem [CITED: WebSearch results including arXiv papers on TSN/PTP clock
sync, LOW relevance, not cited further as prior art for this specific measurement]). The
well-established general technique that does transfer directly is Cristian's / NTP-style
round-trip offset estimation: have the two threads (or the abort thread and the hot-path thread
themselves, instrumented for this one calibration run) exchange a timestamped ping-pong over a
shared atomic, recording T1 (sender send time), T2 (receiver's own clock reading on receipt),
and T3 (sender's clock reading on the reply's receipt); the one-way offset estimate is
`((T2 - T1) + (T2 - T3)) / 2`, assuming symmetric propagation delay. This is **argued reasoning
applying a well-known general algorithm**, not a claim sourced from prior art specific to this
exact setup; flagged honestly as such (Assumptions Log A5).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| SCHED_FIFO from Rust | A raw `libc::sched_setscheduler`/`sched_setparam` FFI call with a hand-written `unsafe` block | `thread-priority` | Safe wrapper, cross-platform (including a documented non-panicking macOS path), MIT-licensed, actively maintained (last published 2026-06-20); a hand-rolled FFI call adds `unsafe` for no benefit given a maintained alternative exists |
| CPU affinity from Rust | A raw `libc::sched_setaffinity` FFI call, or `nix::sched::sched_setaffinity` wrapped in a hand-rolled Linux/macOS `cfg` split | `core_affinity` | Already handles the Linux/macOS/Windows split internally, including graceful degradation on macOS aarch64, matching the project's own `SystemFacts` (`LiveFacts`/`FixtureFacts`) philosophy of "test real logic on macOS rather than `cfg`-ing it out" |
| Percentile computation over raw latency samples | A hand-rolled sort-and-index-into-array percentile function inside `nr-stop-harness` | `hdrhistogram` via a small addition to `nr-histogram::percentiles` (Pattern 3) | `nr-histogram` already has this exact logic (SIGFIG, `value_at_quantile`); duplicating it risks the two implementations drifting on rounding/quantization behaviour |
| Environment snapshot / precondition checks for the STOP-07 rig run | A second precondition checklist specific to `nr-stop-harness` | `nr_capture::preconditions::run_all`/`refuse_on_violation`, `nr_capture::environment::snapshot()` | Already public library functions in `nr-capture`; already explicitly named in `02-CONTEXT.md`'s "Existing Code Insights" as applying to a STOP-07 run "unchanged" |
| The Kani-proved property statement | A broad, hand-written claim like "Kani proves the crate is free of undefined behaviour" | The four D-46 families, stated exactly as they are (totality, reachability, gate invariant, panic/overflow freedom), plus the `docs/proofs/` scoping note (D-48) | D-47 already corrected this exact overclaim once at the requirement/criterion level; restating it more broadly in code comments or a README would reopen the same problem the phase already fixed |

**Key insight:** every "don't hand-roll" item above has a maintained, safe, license-compatible
library already available, except the STOP-06 branch-coverage gap, which has no library
workaround: it is a genuine toolchain-capability boundary, not a case of missing a crate.

## Common Pitfalls

### Pitfall 1: assuming `cargo-llvm-cov`'s default output is branch coverage

**What goes wrong:** a plan or a CI script reads `cargo llvm-cov`'s summary table, sees numbers
under a column, and reports "100 percent branch coverage" when the tool actually reported
**region** coverage (a finer-grained unit than lines, but not the same as true branch coverage).

**Why it happens:** `cargo-llvm-cov`'s `--branch` flag exists and is discoverable, but is marked
`(unstable)` in the tool's own `--help` and README, requires a nightly toolchain dated
2024-03-16 or later (added in cargo-llvm-cov 0.6.8) [CITED: taiki-e/cargo-llvm-cov issue #8,
fetched via WebFetch], and is backed by the upstream compiler flag `-Z coverage-options=branch`,
which lives in the Rust Unstable Book, not the stable compiler flag reference [CITED:
doc.rust-lang.org/stable/unstable-book/compiler-flags/coverage-options.html, fetched via
WebFetch]. Region coverage, by contrast, works end to end on the stable toolchain already pinned
in this repository's `rust-toolchain.toml` [CITED: raw README, taiki-e/cargo-llvm-cov, verbatim
quote fetched: "cargo +stable install cargo-llvm-cov --locked", "Currently, installing
cargo-llvm-cov requires rustc 1.87+"]. **This directly affects STOP-06 and D-53 as written; see
Open Questions for the three ways forward.**

**How to avoid:** name the exact coverage kind (region vs. branch) in every place STOP-06 is
discussed in the plan and in the generated report; do not let "llvm-cov reports region and
branch coverage directly" (D-53's own phrasing) stand unqualified, since on the pinned stable
toolchain it reports only the former without the nightly-gated flag.

**Warning signs:** a CI job that runs `cargo llvm-cov --fail-under-lines 100` (or an equivalent
region-based threshold) and calls it done against STOP-06's literal "100 percent branch
coverage" text without the `--branch` flag and a nightly toolchain to back it.

### Pitfall 2: assuming Swatinem/rust-cache caches Kani's setup download

**What goes wrong:** D-51 says the Kani CI job is "cached the way the existing jobs use
`Swatinem/rust-cache`", and a plan takes this to mean the whole job, including `cargo kani
setup`'s download, is cached.

**Why it happens:** `Swatinem/rust-cache` caches `~/.cargo/registry`, `~/.cargo/git`, and
`target/` [CITED: Swatinem/rust-cache's own README, as summarized via WebSearch, MEDIUM
confidence, not independently re-fetched in full in this session]. `cargo kani setup` downloads
its own bundled toolchain into `~/.kani/` (or `$KANI_HOME`) [CITED: model-checking.github.io/
kani/install-guide.html], a path `Swatinem/rust-cache` does not know about. The one-time
download was reported by a third party as roughly 1.3 GB [CITED: a GitHub PR discussion,
`leynos/chutoro#244`, LOW confidence, third-party and not independently reproduced here since
Kani is not installed on the dev host].

**How to avoid:** add a second, explicit cache step (`actions/cache@v4`, or equivalent) targeting
`~/.kani` (or `$KANI_HOME`), keyed on the exact pinned `kani-verifier` version, alongside
`Swatinem/rust-cache` rather than instead of it. See Code Examples for the full job.

**Warning signs:** the Kani CI job takes roughly the same wall-clock time on every run despite
`Swatinem/rust-cache` reporting a cache hit; that is consistent with the compile-time cache
working while the setup download re-runs every time.

### Pitfall 3: writing Kani harnesses that lean on `compare_exchange`'s failure branch for a proved property

**What goes wrong:** a proof harness asserts a property that is only true because a
`compare_exchange` call is assumed to correctly return `Err` on a losing race, and Kani's solver
(at least in one reported, unconfirmed-for-this-exact-version case) models a losing
`compare_exchange` as returning `Ok` instead.

**Why it happens:** `model-checking/verify-rust-std` issue #673 reports exactly this: "a pinned
Kani reports a failed `compare_exchange` as a success" [CITED: GitHub issue title, fetched via
WebSearch; the specific Kani version pinned by that project was not confirmed against this
phase's planned 0.67.0 pin in this session, so treat this as a flagged risk, not a confirmed
defect in 0.67.0].

**How to avoid:** per Architecture Pattern 1, keep the function Kani actually proofs (`step()`)
free of atomics entirely; prove the pure transition table, and treat the atomic
publish/observe wrapper as hand-argued (which D-50 already establishes as out of scope this
phase). If the plan later wants Kani to reason about the atomic wrapper directly (e.g. in a
future phase), first write a minimal, throwaway harness asserting a deliberately-failing
`compare_exchange` returns `Err` with the correct "actual" value, and treat a wrong result as a
blocking finding before building anything on top of it.

**Warning signs:** a Kani harness that passes but whose corresponding property, when
re-expressed as a plain `#[test]` under `loom` in Phase 4, fails; that combination is exactly
what an unmodelled CAS-failure bug would produce.

### Pitfall 4: assuming Kani, the GitHub Action, and the ambient toolchain must agree

**What goes wrong:** a plan tries to make Kani's bundled compiler match the project's pinned
`stable` `rust-toolchain.toml`, or worries that installing Kani will "downgrade" the workspace to
nightly.

**Why it happens:** Kani's own documentation and a corroborating third-party discussion both
describe `cargo kani setup` as downloading a **separate, self-contained** nightly build into
`~/.kani/`, independent of `rustup`'s toolchain list [CITED: model-checking.github.io/kani/
install-guide.html]. `cargo kani` (the subcommand) invokes this bundled compiler directly; it
does not change what `cargo build`/`cargo test` use elsewhere in the same repository.

**How to avoid:** do not add any `rust-toolchain.toml` override or `rustup override set` step to
the Kani CI job; the existing `channel = "stable"` pin is unaffected by installing Kani.

**Warning signs:** none expected if the above is followed; flagged here because the research
prompt explicitly asked whether this conflict exists, and the answer (no conflict, by design) is
worth stating plainly rather than leaving implicit.

## Code Examples

### Kani proof harness shape (research question 3)

```rust
// crates/stop/src/state.rs (illustrative)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(kani, derive(kani::Arbitrary))]
pub enum State { Running, Stopping, Stopped }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(kani, derive(kani::Arbitrary))]
pub enum Cause { Operator, WatchdogDeadline, InternalFault, Shutdown }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(kani, derive(kani::Arbitrary))]
pub enum Event {
    Abort { cause: Cause, at_raw_ns: u64 },
    Acknowledged,
}

pub fn step(state: State, event: Event) -> State { /* Pattern 1 */ todo!() }
```

```rust
// crates/stop/tests/proofs.rs, or a #[cfg(kani)] module inside src/ -- kept out of a
// normal `cargo build`/`cargo test` entirely by the `cfg(kani)` gate, which is only
// set when compiling under the kani-compiler [CITED: model-checking.github.io/kani/
// tutorial-first-steps.html, verbatim example fetched].
#![cfg(kani)]
use nr_stop::state::{step, Cause, Event, State};

// Family 1 (STOP-01): step is total and never yields an unreachable state.
#[kani::proof]
fn step_is_total() {
    let state: State = kani::any();
    let event: Event = kani::any();
    let next = step(state, event);
    assert!(matches!(next, State::Running | State::Stopping | State::Stopped));
}

// Family 2 (STOP-02): from any state, Abort reaches Stopping or Stopped, and no
// path returns to Running.
#[kani::proof]
fn abort_never_returns_to_running() {
    let state: State = kani::any();
    let cause: Cause = kani::any();
    let at_raw_ns: u64 = kani::any();
    let next = step(state, Event::Abort { cause, at_raw_ns });
    assert!(next != State::Running);
}

// Family 3 (STOP-03): no permit is issued from Stopping or Stopped. Illustrative;
// the real assertion calls into gate.rs's permit-issuing function.
#[kani::proof]
fn no_permit_when_not_running() {
    let state: State = kani::any();
    kani::assume(state != State::Running);
    assert!(!nr_stop::gate::would_issue_permit(state));
}
```

Successful and failed output shapes, quoted from the official tutorial [CITED:
model-checking.github.io/kani/tutorial-first-steps.html]:

```
VERIFICATION:- SUCCESSFUL
```

```
RESULTS:
Check 3: estimate_size.assertion.1
         - Status: FAILURE
         - Description: "Oh no, a failing corner case!"
[...]
VERIFICATION:- FAILED
```

D-54's generated report can be built by capturing `cargo kani`'s text output and extracting the
`Check N: <harness>.<check_kind>.<n>` / `Status: SUCCESS|FAILURE` lines plus the final
`VERIFICATION:- SUCCESSFUL|FAILED` line per harness. Whether a newer, more structured output
format (e.g. a `--output-format` value) exists on the pinned version was not confirmed in this
session; check `cargo kani --help` directly during Wave 0 before committing to text-scraping.

### Kani CI job (research questions 2 and D-51)

```yaml
# .github/workflows/ci.yml -- new job, alongside fmt/clippy/test/deny
  kani:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
        with:
          key: kani-cargo   # separate cache key from other jobs' cargo caches
      # Swatinem/rust-cache does not know about Kani's own toolchain download;
      # see Common Pitfalls #2. Cache it explicitly, keyed on the pinned version.
      - uses: actions/cache@v4
        with:
          path: ~/.kani
          key: kani-setup-0.67.0   # bump this key whenever the pinned version changes
      - run: cargo install --locked kani-verifier@0.67.0
      - run: cargo kani setup
      - run: cargo kani -p nr-stop
```

This deliberately does **not** use `model-checking/kani-github-action`, because that Action's own
documentation states it supports "Ubuntu 20.04 with `x86_64-unknown-linux-gnu`" only [CITED:
model-checking.github.io/kani/install-github-ci.html, verbatim], and `ubuntu-latest` is well past
20.04 -- exactly the reasoning D-51 already gives.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|-------------------|---------------|--------|
| `tarpaulin` for Rust coverage (ptrace-based, x86_64 Linux only, branch coverage never fully real) | `cargo-llvm-cov` (LLVM source-based instrumentation, cross-platform) | Established well before this research; D-53 already made this call correctly | STOP-06's implementation detail changes (see Pitfall 1), not the tool choice |
| `hwlatdetect` as the project's own firmware instrument | `rtla hwnoise` (Phase 1, D-27) | 2026-09 (within this project's own history) | Not directly relevant to Phase 2, noted because `crates/capture/src/hwnoise.rs` exists and is a precedent for "build the real instrument rather than reinterpret a broken one," which is the same posture this research recommends for STOP-06 |

**Deprecated/outdated:** `-Z instrument-coverage` (the old unstable name for what is now the
stable `-C instrument-coverage` flag) appears in some older blog posts found during this
research; the current stable invocation goes through `-C instrument-coverage`, with only
`-Z coverage-options=branch`/`--mcdc` remaining behind the unstable `-Z` flag family [CITED:
doc.rust-lang.org/stable/unstable-book/compiler-flags/coverage-options.html and the rustc book's
"Instrumentation-based Code Coverage" page, cross-referenced via WebSearch].

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `cargo kani setup`'s download is approximately 1.3 GB | Standard Stack, Common Pitfalls #2 | Low: only affects CI cache-sizing expectations, not correctness; easy to confirm empirically the first time the job runs |
| A2 | Kani 0.67.0 (or whichever version is pinned) correctly supports edition 2024 and resolver 3 | Open Questions | Medium: if wrong, the Kani job fails outright at setup/build time; cheap to falsify early with a trivial one-file proof harness before writing the real ones |
| A3 | The `model-checking/verify-rust-std#673` `compare_exchange` modelling issue still affects the pinned Kani version | Common Pitfalls #3, Architecture Pattern 1 | Low: the recommended architecture (pure, atomics-free proved function) makes this moot regardless of whether the underlying bug is still present |
| A4 | `thread-priority`'s exact function name/signature for setting `SCHED_FIFO` (`set_thread_priority_and_policy` as shown) matches the pinned version | Code Examples, Architecture Pattern 4 | Low: cosmetic; re-check the exact API against the pinned version's own docs.rs page during implementation |
| A5 | A Cristian's-algorithm-style round-trip offset estimate is a reasonable methodology for measuring cross-core `CLOCK_MONOTONIC_RAW` skew on this specific rig | Architecture Pattern 5 | Medium: if the measurement methodology is unsound, D-35's "clock read overhead and cross-core skew... characterised" requirement could report a number that misrepresents the true skew; no source specific to this exact scenario was found, so this is reasoned from general first principles, not sourced prior art |
| A6 | `core_affinity`'s macOS aarch64 no-op behaviour ("will still try to request the highest performance for the thread" rather than failing) is accurate for the current released version | Standard Stack, Architecture Pattern 4 | Low: sourced from a WebSearch synthesis of the crate's README, not independently re-verified against the raw source in this session; if wrong, the macOS CI leg might need a small `cfg` guard around the affinity call |
| A7 | `Swatinem/rust-cache`'s cached paths (`~/.cargo/registry`, `~/.cargo/git`, `target/`) do not overlap with `~/.kani` | Common Pitfalls #2 | Low: the recommended mitigation (a second explicit cache step) is safe to add regardless; worst case it is redundant, not harmful |

## Open Questions

1. **Is STOP-06 achievable as literally written on the pinned stable toolchain?**
   - What we know: `cargo-llvm-cov` on stable reports region coverage, not true branch coverage.
     `--branch` requires a nightly toolchain dated 2024-03-16 or later [CITED: taiki-e/
     cargo-llvm-cov issue #8]. D-53 names cargo-llvm-cov correctly (over tarpaulin) but the
     discussion that produced D-53 does not appear to have surfaced this stable/nightly split.
   - What's unclear: whether the operator, presented with this split, would rather (a) pin a
     nightly toolchain for the coverage job only (literally satisfies STOP-06's wording; adds
     one more pinned, moving-target toolchain, mitigated by dating the pin the same way D-51
     dates the Kani version), (b) amend STOP-06 the same way D-47 amended STOP-04 (state "100
     percent region coverage" plainly, since D-32's small-branch-count design already argues the
     practical difference between region and branch coverage is small for a 3-state, ~4-event
     machine), or (c) run both: region coverage as the blocking stable-toolchain gate, and branch
     coverage as a non-blocking, best-effort nightly job that is allowed to fail without blocking
     merges.
   - Recommendation: resolve this explicitly, in writing (mirroring D-47's own precedent), before
     Wave 0 closes. This research's own preference is option (a) given the module is small and
     the project already tolerates one pinned-and-dated toolchain (Kani's own bundled compiler);
     but this is a judgment call the operator should make deliberately, not one this research
     should settle unilaterally on the operator's behalf.

2. **Does Kani 0.67.0 fully support this workspace's `edition = "2024"` and `resolver = "3"`?**
   - What we know: Kani's own "Rust feature support" page does not mention editions or resolver
     versions at all [CITED: model-checking.github.io/kani/rust-feature-support.html, checked
     directly]. Kani "releases every month and synchronizes with a recent nightly release of
     Rust" per its own FAQ-adjacent framing found via WebSearch, and edition 2024 stabilized with
     Rust 1.85 (February 2025), well before Kani 0.65.0-0.67.0 (2025-08 through 2026-01).
   - What's unclear: no direct, dated confirmation that a workspace using `edition = "2024"` /
     `resolver = "3"` has been exercised by Kani 0.67.0 specifically.
   - Recommendation: the cheapest possible Wave 0 spike is a single trivial `#[kani::proof]`
     function verified end to end in this actual workspace before any of the four real harness
     families are written. This also incidentally verifies Assumption A2 and gives an early,
     cheap signal on the CI job shape (Code Examples) before it is load-bearing.

3. **What does the rig's actual clocksource read, and what is the true `CLOCK_MONOTONIC_RAW`
   read overhead on this specific machine?**
   - What we know: the kernel command line already pins `tsc=reliable` [VERIFIED: local
     `docs/rig/recon-2026-08-31/FINDINGS.md:123`], and Linux 5.3+ vDSO-accelerates
     `CLOCK_MONOTONIC_RAW` in general [CITED: berthub.eu].
   - What's unclear: whether `/sys/devices/system/clocksource/clocksource0/current_clocksource`
     actually reads `tsc` on this rig (not recorded in any existing capture found in this
     session), and the rig-specific overhead number D-35 requires.
   - Recommendation: this is exactly what D-35 already asks the STOP-07 plan to measure and
     commit; flagged here only so the plan does not skip it as "probably fine given the kernel
     version."

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|--------------|-----------|---------|----------|
| `kani-verifier` / `cargo kani` | STOP-04, STOP-05 | no (dev host) [VERIFIED: `cargo kani --version` fails, "no such command"] | - | Install locally (`cargo install --locked kani-verifier@0.67.0 && cargo kani setup`); supported on `aarch64-apple-darwin` per the dev host's own architecture, so a full local dev loop is possible, not just CI |
| `cargo-llvm-cov` | STOP-06 | no (dev host) [VERIFIED: `cargo llvm-cov --version` fails, "no such command"] | - | Install locally (`cargo +stable install cargo-llvm-cov --locked`); works on stable for region coverage, see Open Questions for the branch-coverage nightly gap |
| `thread-priority`, `core_affinity`, `nix`, `hdrhistogram` crates | STOP-07 | yes, via crates.io on first `cargo build` | see Standard Stack | none needed; ordinary Cargo dependency resolution |
| `chrt`, `taskset` (util-linux) | STOP-07, only if the shell-out alternative in "Alternatives Considered" is chosen instead | no on macOS dev host [VERIFIED: `command -v chrt taskset` empty]; expected present on the Ubuntu rig (util-linux is part of the base Ubuntu install, not independently re-verified on the rig itself in this session) | - | Not needed if the recommended safe-crate approach (Pattern 4) is used; the dev host's absence is expected and not a blocker since these tools are Linux-only and STOP-07 only runs for real on the rig |
| `rustup` nightly toolchain | Only if Open Question 1 resolves to option (a) or (c) for STOP-06 | no (dev host currently has only stable + the `aarch64-apple-darwin`/`x86_64-unknown-linux-gnu` targets) [VERIFIED: `rustup component list --installed`] | - | `rustup toolchain install nightly-<date>`, scoped to the coverage CI job only if chosen; not a blocker for any other part of the phase |

**Missing dependencies with no fallback:** none. Every missing tool has either a straightforward
local install path or, for `chrt`/`taskset`, is simply not the recommended route.

**Missing dependencies with fallback:** `kani-verifier` and `cargo-llvm-cov` (install locally,
both are supported on the dev host's own architecture); a nightly toolchain if the coverage
gate needs one (install and pin, scoped to one job).

## Validation Architecture

### Test framework

| Property | Value |
|----------|-------|
| Framework | `#[test]` (built-in `cargo test`), plus `cargo kani` as a distinct, non-`cargo-test` verification surface |
| Config file | none dedicated; workspace-level `Cargo.toml` lints apply. A Kani-specific config, if any, lives in `crates/stop/Cargo.toml`'s own `[package.metadata.kani]` table if the plan needs one (not confirmed necessary in this research; check `cargo kani --help` during Wave 0) |
| Quick run command | `cargo test -p nr-stop` (unit tests); `cargo kani -p nr-stop` (proofs; these are excluded from `cargo test` by the `cfg(kani)` gate) |
| Full suite command | `cargo test --workspace --all-targets && cargo kani -p nr-stop && cargo llvm-cov -p nr-stop --fail-under-regions 100` |

### Phase requirements -> test map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|---------------------|--------------|
| STOP-01 | `step()` is total over the enumerated state space | Kani proof | `cargo kani -p nr-stop --harness step_is_total` | no, Wave 0 |
| STOP-02 | From any reachable state, `Abort` reaches `Stopping`/`Stopped`, never back to `Running` | Kani proof | `cargo kani -p nr-stop --harness abort_never_returns_to_running` | no, Wave 0 |
| STOP-03 | No permit issued from `Stopping`/`Stopped`; modelled consumer never emits after a stop | Kani proof + unit test against the modelled consumer (D-45) | `cargo kani -p nr-stop --harness no_permit_when_not_running`; `cargo test -p nr-stop modelled_consumer` | no, Wave 0 |
| STOP-04 | Totality, invariants, panic/overflow freedom; scoped published claim | Kani proof (4 families) + `docs/proofs/` scoping note (manual/doc, not automated) | `cargo kani -p nr-stop` | no, Wave 0 |
| STOP-05 | `cargo kani` blocks CI | CI job (integration-level, not a unit test) | the `kani` job in `.github/workflows/ci.yml`, see Code Examples | no, Wave 0 |
| STOP-06 | 100 percent coverage of `crates/stop` | Coverage run (see Open Question 1 for region-vs-branch) | `cargo llvm-cov -p nr-stop --fail-under-regions 100` (or `--fail-under-branches 100` on a nightly job, pending Open Question 1's resolution) | no, Wave 0 |
| STOP-07 | Abort latency bounded and measured on the reference rig | Rig capture (manual-only, rig-gated) | `nr-stop-harness` run on the Precision 3591, per the Phase 1 protocol; not automatable in CI since it requires the physical rig, isolated cores, and `SCHED_FIFO`/root-adjacent privileges | no, Wave 0 (the harness itself does not exist yet) |

### Sampling rate

- **Per task commit:** `cargo test -p nr-stop && cargo kani -p nr-stop` (fast: a 3-state, ~4-event
  machine should solve in seconds, per D-52's own reasoning for making the gate unconditional).
- **Per wave merge:** the full suite command above, plus a local `cargo llvm-cov -p nr-stop`
  report reviewed by hand before the STOP-07 harness work begins (STOP-06 should be closed on
  the FSM before STOP-07's rig work starts, since STOP-07 depends on nothing in STOP-06 but
  reordering would let coverage gaps hide behind "we'll get to it").
- **Phase gate:** full suite green, `docs/proofs/` report committed and reviewed (D-54), one
  headline-class STOP-07 rig capture published with its raw capture, before `/donny-verify-work`.

### Wave 0 gaps

- [ ] `crates/stop/` does not exist yet: package scaffold, `[lints] workspace = true`,
  `#![forbid(unsafe_code)]` restated (D-49).
- [ ] `crates/stop/tests/proofs.rs` (or equivalent `#[cfg(kani)]` module) - covers STOP-01
  through STOP-04.
- [ ] A trivial one-file Kani spike proof, run once manually, to settle Open Question 2 (edition
  2024/resolver 3 compatibility) before committing to the four real harness families.
- [ ] `crates/stop-harness/` does not exist yet - covers STOP-07; depends on Pattern 2/3/4/5
  decisions being settled first.
- [ ] Kani CI job (`.github/workflows/ci.yml`) - covers STOP-05.
- [ ] Coverage CI job/step - covers STOP-06, blocked on Open Question 1's resolution.
- [ ] `docs/proofs/` directory and its scoping note - covers STOP-04's published-claim
  requirement (D-48).

## Security Domain

`crates/stop` in this phase has an unusually small attack surface for an ASVS review: it takes
no network input, no user-supplied strings, and no untrusted deserialization (the `Abort` event
is constructed by trusted in-process Rust callers per D-30; there is no external caller of any
kind until OBS-03's control plane arrives in Phase 5). Most ASVS categories are therefore not
applicable yet, and this section says so explicitly rather than forcing a checklist onto code
that does not have the matching attack surface.

### Applicable ASVS categories

| ASVS Category | Applies | Standard Control |
|----------------|---------|--------------------|
| V1 Architecture, design and threat modeling | yes | `docs/proofs/` scoping note (D-48) is itself a threat-model-adjacent artifact: it names what is and is not covered, which is the ASVS V1 spirit even though this is not a web application |
| V2 Authentication | no | No caller identity concept exists in this phase; `EmergencyStop::abort(cause)` is callable from any in-process thread by design (D-30) |
| V3 Session management | no | Not applicable; no session concept |
| V4 Access control | no | Not applicable this phase; becomes relevant once OBS-03's control plane (Phase 5) can call `abort()` remotely, at which point who may trigger a remote abort becomes a real access-control question, explicitly out of scope here |
| V5 Input validation | partial | The `Cause` enum (D-44) is a closed, exhaustively-matched Rust enum, not a string or externally-parsed value, so "validation" is enforced by the type system rather than a runtime check; Kani's totality proof (STOP-01) is the closest analogue to a validation guarantee here |
| V6 Cryptography | no | Not applicable; no secrets, no encoded data |
| V7 Error handling and logging | yes | D-46 family 4 (no panic, no arithmetic overflow, no `unwrap`), proved by Kani rather than tested; D-30 explicitly rejects a POSIX signal handler partly because it would require the crate's first `unsafe` block, which is itself an error-handling-adjacent design decision |
| V11 Business logic | yes | This is where most of the real "security" content of this phase lives: STOP-01 through STOP-03 are business-logic invariants (no output after stop, no return to `Running`, totality) proved by Kani rather than asserted; D-39's "no reset transition" is a business-logic control against a whole class of "was it actually reset" bugs |

### Known threat patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|-----------------------|
| A stale or torn read of the FSM state lets a node believe it still holds `Running` after an abort | Tampering (of the effective safety property, not of data at rest) | `Acquire`/`Release` ordering on the state atomic (Architecture Pattern 1); the permit-as-capability design (D-42) makes "believing you can emit" and "being able to emit" the same thing by construction |
| A future caller adds a reset path to `Stopped`, silently reopening a session that should require re-construction | Elevation of privilege (a stopped session regaining the ability to emit) | D-39: no reset transition exists at all; STOP-03 is proved by the absence of an edge, not by a runtime check that would need to be remembered and re-verified on every future change |
| A future `unsafe` block (e.g. a hand-rolled scheduling or clock call) is added directly to `crates/stop` to "simplify" the harness | Tampering / loss of the crate's central guarantee | `#![forbid(unsafe_code)]` restated locally at the crate root (D-49); this phase's own research found two safe wrapper crates (`thread-priority`, `core_affinity`) and one safe clock-reading crate (`nix`) specifically so that no `unsafe` is needed anywhere in `nr-stop-harness` either, even though the harness is not bound by D-49 |
| A model-checker modelling gap (e.g. the `compare_exchange` concern in Common Pitfalls #3) is silently relied upon by a proof | Tampering (an unsound proof passing when the property is actually false) | Architecture Pattern 1: keep the proved function atomics-free so this class of solver-modelling gap cannot affect what STOP-01 through STOP-04 actually establish |

This is a small, honest table on purpose: forcing STRIDE categories that do not fit a
network-free, single-operator, in-process FSM crate would be worse than naming plainly which
ones do not yet apply.

## Sources

### Primary (HIGH confidence)

- Local codebase, read directly in this session: `crates/histogram/src/{hist,json,percentiles}.rs`,
  `crates/capture/src/{preconditions,sources,environment,interference}.rs`,
  `crates/cli/src/{tools.rs,rundir.rs,cmd/run.rs,cmd/series.rs}`, `crates/manifest/src/{fields,
  attempt,checksum}.rs`, `crates/metrics/src/{series,report,baseline,coverage}.rs`,
  `Cargo.toml`, `rust-toolchain.toml`, `deny.toml`, `.github/workflows/ci.yml`,
  `.planning/{REQUIREMENTS,ROADMAP,STATE}.md`, `docs/rig/recon-2026-08-31/FINDINGS.md`.
- crates.io registry API, queried directly via `curl` in this session, for `kani-verifier`,
  `cargo-llvm-cov`, `thread-priority`, `core_affinity`, `affinity`, `nix` (versions, license,
  publish dates).
- model-checking.github.io/kani/{install-guide.html, install-github-ci.html,
  tutorial-first-steps.html, reference/attributes.html, reference/arbitrary.html,
  rust-feature-support.html} - fetched via WebFetch in this session.
- raw.githubusercontent.com/taiki-e/cargo-llvm-cov/main/README.md - fetched via WebFetch, quoted
  verbatim in places.
- doc.rust-lang.org/stable/unstable-book/compiler-flags/coverage-options.html - fetched via
  WebFetch.
- docs.rs `/nix/latest/nix/{time,sched}` module and `ClockId` pages - fetched via WebFetch.

### Secondary (MEDIUM confidence)

- berthub.eu/articles/posts/on-linux-vdso-and-clockgettime - vDSO/`CLOCK_MONOTONIC_RAW` history
  and kernel version.
- raw.githubusercontent.com/iddm/thread-priority/master/README.md and
  raw.githubusercontent.com/Elzair/core_affinity_rs/master/README.md - fetched via WebFetch,
  platform support and API safety claims.
- WebSearch synthesis of `taiki-e/cargo-llvm-cov` issue #8 (nightly requirement and version
  gating for `--branch`).
- `rust-lang/rust` issue #37902 (title corroborates `Instant` on Linux uses `CLOCK_MONOTONIC`,
  not `CLOCK_MONOTONIC_RAW`) and issue #79649 (branch coverage tracking issue), found via
  WebSearch.
- `python3 ~/Developer/scrapers/research_topic.py` digests for "cargo-llvm-cov branch coverage
  stable toolchain rust", "Kani Rust model checker proof harness kani::proof Arbitrary", and
  "rust CPU affinity SCHED_FIFO thread priority crate safe" - used to identify which primary
  sources to fetch directly, per this agent's required tool order.

### Tertiary (LOW confidence)

- `model-checking/verify-rust-std` issue #673 (`compare_exchange` modelling concern) - found via
  WebSearch, version-of-Kani-affected not confirmed against this phase's 0.67.0 pin.
- `leynos/chutoro` PR #244 - third-party source for the approximate 1.3 GB `cargo kani setup`
  download size.
- WebSearch synthesis regarding `core_affinity`'s macOS aarch64 fallback behaviour, and regarding
  `Swatinem/rust-cache`'s cached directory list - not independently re-fetched from the crates'
  own primary sources in full.

## Metadata

**Confidence breakdown:**
- Standard stack (Kani, cargo-llvm-cov, versions and licenses): HIGH - every version/license
  claim was verified directly against the crates.io registry API in this session.
- The STOP-06 branch-vs-region coverage finding: HIGH - corroborated by two independent primary
  sources (the Rust Unstable Book and cargo-llvm-cov's own README, both fetched and quoted
  directly) plus a numbered upstream tracking issue.
- Kani proof harness syntax: HIGH - all syntax shown was fetched from the official Kani
  documentation in this session, not reconstructed from training data.
- Architecture recommendation for `nr-stop-harness` (Pattern 2): HIGH on the factual claims
  (which functions are public/private, where they live) since these were read directly from the
  local codebase; MEDIUM on the "standalone binary" recommendation itself, since that is this
  research's synthesis of those facts against D-55's stated rule, not a fact with an external
  source.
- CPU affinity / SCHED_FIFO crate recommendation: MEDIUM - capability claims (safe API, platform
  list, license) are sourced from the crates' own README/docs.rs pages; exact function
  signatures in the Code Examples section are reconstructed from search-indexed documentation
  and should be re-verified against the pinned versions before coding.
- Cross-core clock skew methodology: LOW - no source specific to this exact scenario was found;
  the recommendation applies a well-known general algorithm (Cristian's/NTP-style round-trip
  offset estimation) by analogy, stated as argued reasoning, not sourced prior art.
- Kani plus edition 2024/resolver 3 compatibility: LOW - no direct confirmation found either way;
  flagged as Open Question 2 with a cheap, concrete way to resolve it empirically in Wave 0.
- `cargo kani setup` download size: LOW - single third-party source, not independently
  reproduced since Kani is not installed on the dev host.

**Research date:** 2026-09-14 (session date 2026-09-15 per tool timestamps)
**Valid until:** 30 days for the codebase-derived findings (stable unless the codebase changes
underneath this phase); 14 days for the external tool-version findings (Kani and cargo-llvm-cov
both showed active, roughly monthly release cadences historically, though Kani has had no
release since 0.67.0 on 2026-01-16, an 8-month gap worth re-checking before the plan locks a
version, in case a newer release has shipped by execution time).
