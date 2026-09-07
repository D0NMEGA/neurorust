---
status: PASS
agent: donny-roadmapper
phases: 8
coverage: 59/59
---

# Roadmap: neurorust

## Overview

The rig exists (RIG-01 through RIG-04 are validated and measured), so the roadmap starts at
the first unbuilt thing. It is ordered by risk against the core value: the two artifacts that
survive any schedule slip are the Kani-proven `emergency_stop` and a measurement methodology
nobody can poke holes in, so those come first, before a single node of the pipeline. Phase 1
closes the open platform question (an unexplained 3.8 ms global stall on the RT kernel) and
builds the publication machinery every later number will pass through. Phase 2 delivers the
proof. Phases 3 through 6 build the runtime bottom up: locked memory and RT scheduling, then
lock-free transport, then a declared graph carrying 1024 channels through a vectorised filter
with per-node instrumentation, then the decoder that closes the DAG and the headline latency
numbers. Phases 7 and 8 are the adoption and throughput layers, sequenced last because they
are the ones to cut first.

**Coverage:** 59 of 59 v1 requirements mapped. RIG-01 through RIG-04 are already validated and
are not scheduled.

## Sequencing

Built by one person plus Claude, sequentially. There are no parallel workstreams.

Ordering rules applied:

1. Platform integrity is an early gate. Every later latency figure would otherwise inherit an
   unexplained 3.8 ms stall, and building on top of an unexplained stall compounds it.
2. Proof and methodology precede throughput. Both are what a reviewer who knows the field
   weighs, and neither needs hardware the project does not already have.
3. Cut order, last to first: Phase 8 (wire protocol and throughput, explicitly the least
   differentiating claim), then Phase 7 (the pylsl shim, which is the adoption path and so
   outranks the wire), then Phase 6's benchmark targets. Phases 1 through 5 are the floor.
4. The Tauri monitor and the Creusot functional proof are v2 and appear nowhere here.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [ ] **Phase 1: Trustworthy measurement** - Root-cause the 3.8 ms stall and make every published figure reproducible from a raw capture on a named rig
- [ ] **Phase 2: Proven emergency_stop** - A latching abort FSM with a Kani proof gating CI and a measured abort latency
- [ ] **Phase 3: Deterministic substrate** - Locked memory, preallocated pools, an allocator hook, and a measured scheduling policy decision
- [ ] **Phase 4: Lock-free transport** - Handle-passing channels with a sub-200 ns SPSC p99, loom-verified and criterion-gated
- [ ] **Phase 5: Instrumented pipeline** - A declared graph running 1024 channels at 30 kHz through a SIMD IIR filter, with per-node histograms and Arrow export
- [ ] **Phase 6: Decoder and the end-to-end result** - The full source to sink DAG on the rig, with jitter published as the headline metric
- [ ] **Phase 7: pylsl compatibility** - Existing lab scripts adopt the runtime by changing one import line
- [ ] **Phase 8: Wire protocol and sustained throughput** - Authenticated, hardware-timestamped UDP ingest holding 480 MB/s for 24 hours

## Phase Details

### Phase 1: Trustworthy measurement
**Goal**: The rig is a credible instrument and the project has a publication contract: the 3.8 ms stall is named rather than tolerated, and every figure that follows carries its rig, its protocol, and its raw capture.
**Depends on**: Nothing (RIG-01 through RIG-04 are already validated)
**Requirements**: PLAT-01, PLAT-02, PLAT-03, BENCH-04, BENCH-05, BENCH-06, BENCH-08
**Success Criteria** (what must be TRUE):
  1. The kernel path responsible for the ~3.8 ms global stall is named, with the ftrace and `cyclictest --tracemark` capture that identifies it committed next to the claim
  2. A third party can follow the documented measurement protocol (no remote shell activity during a run, defined system state) and reproduce a run under the same conditions rather than guessing at them
  3. Worst-case scheduling latency on isolated cores is either under 30 us on a clean run, or the residual is attributed to a named platform cause and published as a stated limitation
  4. Every published figure is stamped by the capture harness with rig, kernel, BIOS revision, and tuning state, ships as a histogram with its raw capture alongside, and losing configurations appear rather than being omitted
  5. A CI job commits a weekly metrics JSON carrying p50, p95, and p99 for every stage instrumented so far
**Plans**: 23 plans in 18 waves (single operator; waves express dependency, not parallel staffing). Expanded from 15 on 2026-09-05: an external adversarial audit (`01-EXTERNAL-AUDIT.md`) found ten methodology and implementation defects, and executing D-18 showed that `hwlatdetect` cannot sample the isolated cores at all on this kernel. Plans 01-16 through 01-23 close those gaps and build the replacement instrument; plans 01-12 through 01-15 were rewritten in place and now run last. Plan numbers no longer match wave order, so each unexecuted plan below carries its wave.

Plans:
- [x] 01-01-PLAN.md - Cargo workspace, dual licence, CI skeleton, histogram fixture
- [x] 01-02-PLAN.md - Rig recon: tracer availability, tool versions, schema probes (rig runbook)
- [x] 01-03-PLAN.md - Manifest schema: the D-14 field set, provenance tier, blake3 checksums
- [x] 01-04-PLAN.md - Histogram parser: bins, overflows, hdrhistogram percentiles
- [x] 01-05-PLAN.md - Capture crate: preconditions, environment snapshot, contamination verdict
- [x] 01-06-PLAN.md - Metrics crate: series, baseline comparison, coverage record, PLAT-03 decomposition
- [x] 01-07-PLAN.md - nrmeasure run: orchestration and the stamped run directory
- [x] 01-08-PLAN.md - nrmeasure verify and reconstruct, plus the blocking provenance gate
- [x] 01-09-PLAN.md - The measurement protocol and the publication layout documents
- [x] 01-10-PLAN.md - Reconstructed manifests and the 2026-08-28 README correction
- [x] 01-11-PLAN.md - Calibration pair and the RT firmware floor re-run (rig runbook)
- [ ] 01-12-PLAN.md - PLAT-01 investigation: armed tracing, threshold calibration, three cycles (rig runbook) [wave 15]
- [ ] 01-13-PLAN.md - PLAT-03 headline capture and the verdict, with no subtraction (rig runbook) [wave 16]
- [ ] 01-14-PLAN.md - Weekly job: nrmeasure series with nullable statistics, systemd units, regression gate [wave 17]
- [ ] 01-15-PLAN.md - Rig install runbook: enable the weekly timer and observe the first fire [wave 18]
- [x] 01-16-PLAN.md - Harness identity, argv provenance, and the re-derivation guard [wave 8]
- [x] 01-17-PLAN.md - The durable attempt record and per-instrument interference windows [wave 9]
- [x] 01-18-PLAN.md - Gate audit: every precondition establishes what its name claims [wave 10]
- [x] 01-19-PLAN.md - Verification that recomputes: no substituted zeros, reports re-derived [wave 8]
- [x] 01-20-PLAN.md - rtla hwnoise parser, SMI manifest fields, CPU-distribution standing check [wave 12]
- [x] 01-21-PLAN.md - Wire hwnoise and MSR_SMI_COUNT into nrmeasure run [wave 13]
- [x] 01-22-PLAN.md - Rig entry point: sudoers rule, root scripts, msr-tools, format probes [wave 11]
- [x] 01-23-PLAN.md - Re-take D-18 on the isolated cores and correct the published firmware record [wave 14]

**Notes**: The cargo workspace and the CI pipeline are created here as enabling work for the weekly metrics job; later phases add gates to the same pipeline (Kani in Phase 2, loom and criterion in Phase 4). The contaminated `cyclictest-rt-isolated-idle-10m.hist` run is the starting evidence, not a publishable figure. The harness and provenance fixes (plans 01-16 through 01-19) run before any new rig capture, because a capture taken through a harness that can lose its raw evidence or stamp an unknown identity would have to be retaken.

### Phase 2: Proven emergency_stop
**Goal**: The safety-critical abort path is proven correct across its reachable state space rather than tested, and its worst case is a measured number on the reference rig.
**Depends on**: Phase 1 (STOP-07 needs the clean measurement protocol and the capture harness; the FSM and the proof need no rig at all and can be built on the macOS dev host)
**Requirements**: STOP-01, STOP-02, STOP-03, STOP-04, STOP-05, STOP-06, STOP-07
**Success Criteria** (what must be TRUE):
  1. `emergency_stop` is a finite state machine with an enumerated reachable state space, and from any reachable state an abort signal drives the system to the defined safe state
  2. Once stopped, no decoder output is produced for the remainder of the session, and the latch is proven rather than assumed
  3. `cargo kani` proves absence of undefined behaviour across the reachable state space and fails the build in CI when it does not
  4. The module reports 100 percent branch coverage in CI
  5. Abort latency is published as a bounded worst case measured on the reference rig under the Phase 1 protocol
**Plans**: TBD
**Notes**: STOP-03 is proven at FSM level against a modelled output gate; the decoder and sink consume that gate in Phase 6. The full functional property is CREU-01, deferred to v2.

### Phase 3: Deterministic substrate
**Goal**: The runtime process cannot page-fault, allocate, or silently pick the wrong scheduling policy on the hot path, and the policy choice is settled by measurement.
**Depends on**: Phase 1 (SUBS-06's published comparison must be measured under the clean protocol)
**Requirements**: SUBS-01, SUBS-02, SUBS-03, SUBS-04, SUBS-05, SUBS-06, SUBS-07
**Success Criteria** (what must be TRUE):
  1. A running process holds all hot-path memory locked with `mlockall` and serves transient state from buffers preallocated at startup against a declared worst-case schedule plus an arena
  2. A hot-path allocation panics in a debug build and is counted in a release build
  3. The release-build allocation counter reads zero after a 24 hour soak
  4. The same workload is measured under `SCHED_DEADLINE` (via raw `sched_setattr`) and `SCHED_FIFO` on the reference rig, and the comparison is published
  5. Buffer handoff between threads requires an explicit `transfer()`, so shared mutable ownership does not compile
**Plans**: TBD
**Notes**: The 24 hour soak here runs the substrate under synthetic load; WIRE-07 re-asserts zero allocations at full pipeline rate in Phase 8.

### Phase 4: Lock-free transport
**Goal**: Data moves between nodes without locks, without copies, and with a tail latency that is published rather than assumed.
**Depends on**: Phase 3 (handle passing needs the preallocated pools and the ownership model; the allocator hook is what proves the channels allocate nothing)
**Requirements**: CHAN-01, CHAN-02, CHAN-03, CHAN-04, CHAN-05, CHAN-06, CHAN-07, GRAPH-03
**Success Criteria** (what must be TRUE):
  1. SPSC transport between same-thread nodes measures p99 under 200 ns on the reference rig
  2. MPMC, bounded MPSC fan-in, and cross-process shared-memory transports all carry data, and each is reached through a named constructor (`channel::spsc()`, `channel::mpsc()`) so that picking the wrong primitive takes deliberate effort
  3. Rings carry handles rather than payloads, so a sample buffer entering the graph is not copied again before it leaves
  4. Loom exhaustively explores atomic interleavings for every channel type in CI, and the job fails on a violation
  5. A committed criterion benchmark runs per commit and flags a tail-latency regression
**Plans**: TBD

### Phase 5: Instrumented pipeline
**Goal**: A declared graph carries implant-scale signal end to end through a numerically validated filter, and every stage of it is visible without perturbing the hot path.
**Depends on**: Phase 4 (nodes need transport between them)
**Requirements**: GRAPH-01, GRAPH-02, WIRE-01, FILT-01, FILT-02, FILT-03, FILT-04, FILT-05, OBS-01, OBS-02, OBS-03, OBS-04
**Success Criteria** (what must be TRUE):
  1. A synthetic source sustains 1024 channels at 30 kHz, 16-bit, with zero drops
  2. Graph topology is declared at startup and cannot change while running, and each node carries a worst-case execution time annotation
  3. The causal IIR filter processes 1024 channels from a structure-of-arrays memory layout through a SIMD kernel, matches a SciPy reference within a stated tolerance, and publishes a criterion speedup figure against that reference
  4. Any session exports per-node latency histograms and an Apache Arrow IPC file that replays
  5. A flame graph of the hot path can be produced on demand, and the tonic-gRPC control plane reports telemetry and applies configuration without measurably perturbing hot-path timing
**Plans**: TBD
**Notes**: OBS-03 lands here because the control plane is infrastructure the decoder plugs into; DEC-04 (model swaps stay off the hot path) is the decoder-side half in Phase 6.

### Phase 6: Decoder and the end-to-end result
**Goal**: The complete source, filter, decode, sink graph runs on the reference rig, and its headline numbers are published against the targets whether they are met or missed.
**Depends on**: Phase 5 (the graph and its instrumentation), Phase 2 (the decoder and sink consume the abort gate), Phase 1 (every figure here is published under the methodology)
**Requirements**: GRAPH-04, DEC-01, DEC-02, DEC-03, DEC-04, BENCH-01, BENCH-02, BENCH-03, BENCH-07
**Success Criteria** (what must be TRUE):
  1. An exported model runs as a decode node, and the choice between `candle` and `ort` is settled by measured WCET on the reference rig with the comparison published
  2. Inference worst-case execution time is measured rather than averaged, fits the end-to-end budget with stated headroom, and models load and swap on the control plane while the hot path keeps running
  3. A graph of source, filter, decode, and sink runs end to end with per-node timing exported
  4. End-to-end DAG latency p99 on a replayed Indy session and source-to-first-node p99 are published against the 8 ms and 100 us targets
  5. Jitter (p99 minus p50) is published as the headline metric against the 0.5 ms target, alongside a comparison with BRAND's published figures that states the methodology differences explicitly
**Plans**: TBD

### Phase 7: pylsl compatibility
**Goal**: An existing lab acquisition script adopts the runtime by changing one import line, and the switch is worth making.
**Depends on**: Phase 5 (needs a running pipeline to expose; it does not need the decoder), Phase 1 (LSL-03's comparison is measured under the protocol)
**Requirements**: LSL-01, LSL-02, LSL-03
**Success Criteria** (what must be TRUE):
  1. A drop-in Python package satisfies the pylsl API closely enough that an existing script runs unmodified
  2. At least three real third-party LSL scripts, none written for this project, run without source changes
  3. A measured latency improvement over liblsl is shown for those scripts and published with raw captures
**Plans**: TBD

### Phase 8: Wire protocol and sustained throughput
**Goal**: Samples arrive from the network at full rate, authenticated and hardware-timestamped, without dropping one over a day.
**Depends on**: Phase 5 (ingest feeds the declared graph), Phase 3 (the no-allocation guarantee the soak asserts)
**Requirements**: WIRE-02, WIRE-03, WIRE-04, WIRE-05, WIRE-06, WIRE-07
**Success Criteria** (what must be TRUE):
  1. Ingest runs on io_uring with SQPOLL, DEFER_TASKRUN, and NAPI busy-poll
  2. The UDP frame carries an 8-byte `CLOCK_MONOTONIC_RAW` header and a FlatBuffers payload read in place with no parse step
  3. Payloads are authenticated with ChaCha20-Poly1305 using hardware crypto extensions, with the per-frame cost measured
  4. Ingest timestamps come from the `e1000e` IEEE 1588 hardware clock, and the delta against software timestamping is published
  5. 480 MB/s sustained ingest holds for 24 hours with zero drops and a zero hot-path allocation count
**Plans**: TBD
**Notes**: WIRE-06 requires an ethernet cable; the wifi adapter has no PTP clock. The PROJECT.md Constraints bullet still says "software timestamping only", which the corrected Out of Scope entry of 2026-08-28 supersedes; that bullet needs updating.

## Progress

**Execution Order:**
Phases execute in numeric order: 1 -> 2 -> 3 -> 4 -> 5 -> 6 -> 7 -> 8

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Trustworthy measurement | 19/23 | In Progress|  |
| 2. Proven emergency_stop | 0/TBD | Not started | - |
| 3. Deterministic substrate | 0/TBD | Not started | - |
| 4. Lock-free transport | 0/TBD | Not started | - |
| 5. Instrumented pipeline | 0/TBD | Not started | - |
| 6. Decoder and the end-to-end result | 0/TBD | Not started | - |
| 7. pylsl compatibility | 0/TBD | Not started | - |
| 8. Wire protocol and sustained throughput | 0/TBD | Not started | - |

---
*Roadmap created: 2026-08-28*
