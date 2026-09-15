# Requirements: neurorust

**Defined:** 2026-08-28
**Core Value:** The safety-critical abort path is proven correct rather than tested, and every
latency claim is reproducible from published raw captures on a named rig.

## Validated

Completed and measured 2026-08-28. Raw captures and rig manifest in
`measurements/2026-08-28-precision3591/`.

- [x] **RIG-01**: A PREEMPT_RT reference rig exists and is pinned (Dell Precision 3591,
      Core Ultra 9 185H, BIOS 1.23.0, microcode 0x28, Ubuntu 26.04.1, kernel 7.0.0-30-realtime,
      `/sys/kernel/realtime` = 1)
- [x] **RIG-02**: Hot-path cores are isolated on whole physical cores (`isolcpus=6-11`,
      `nohz_full`, `rcu_nocbs`, `irqaffinity` steering), derived from measured hyperthread
      sibling pairs rather than assumed numbering
- [x] **RIG-03**: Firmware latency is characterised (`hwlatdetect`: 125 us stock, under 10 us
      tuned idle, 22 us on P-cores under 95 C load) and the tuning that achieves it persists
      across reboots via `rt-tuning.service`
- [x] **RIG-04**: A stock-kernel baseline exists on identical hardware, enabling a controlled
      before/after comparison rather than two numbers from two configurations

## v1 Requirements

### Platform integrity

- [ ] **PLAT-01**: The 3.8 ms global stall observed on the RT kernel is root-caused with
      ftrace and `cyclictest --tracemark`, and the responsible kernel path is named
- [ ] **PLAT-02**: A clean measurement protocol is defined and followed: no remote shell
      activity during a run, desktop idle or system at `multi-user.target`, documented in the
      repo so third parties reproduce the conditions rather than guessing
- [x] **PLAT-03**: Worst-case scheduling latency on isolated cores is either brought under
      30 us, or the residual is attributed to a named platform cause, or published as an
      explicitly unattributed limitation with the run, its raw capture, and the reason
      attribution failed committed alongside

### Scheduling and memory substrate

- [ ] **SUBS-01**: All hot-path memory is locked at startup with `mlockall` so no page can
      fault mid-stream
- [ ] **SUBS-02**: All buffers are preallocated at startup from a declared schedule
      (worst-case message rate times WCET times a safety factor)
- [ ] **SUBS-03**: An arena allocator serves transient hot-path state
- [ ] **SUBS-04**: A global allocator hook panics on hot-path allocation in debug builds and
      counts it in release builds
- [ ] **SUBS-05**: The release-build allocation count is asserted to be zero across a 24 hour soak
- [ ] **SUBS-06**: Hot-path threads run under both `SCHED_DEADLINE` (via raw `sched_setattr`)
      and `SCHED_FIFO`, both are measured on the reference rig, and the comparison is published
- [ ] **SUBS-07**: Buffer ownership is single-threaded by default with an explicit `transfer()`
      for handoff

### Node and graph model

- [ ] **GRAPH-01**: A `Node` trait carries a worst-case execution time annotation
- [ ] **GRAPH-02**: Graph topology is declared at startup and cannot change on the hot path
- [ ] **GRAPH-03**: Channel selection is encoded in the API surface (`channel::spsc()`,
      `channel::mpsc()`) so the correct primitive is the path of least resistance
- [ ] **GRAPH-04**: A graph of source, filter, decode, and sink runs end to end with per-node
      timing exported

### Inter-node transport

- [ ] **CHAN-01**: SPSC transport between same-thread nodes via `rtrb`, p99 under 200 ns
- [ ] **CHAN-02**: MPMC transport via `crossbeam-queue::ArrayQueue`
- [ ] **CHAN-03**: Bounded MPSC fan-in (Vyukov)
- [ ] **CHAN-04**: Shared memory plus futex transport across process boundaries
- [ ] **CHAN-05**: Rings carry handles rather than payloads, so the path is zero copy end to end
- [ ] **CHAN-06**: Loom exhaustively explores atomic interleavings for every channel type in CI
- [ ] **CHAN-07**: A criterion benchmark is committed and tracked per commit, gating regressions

### emergency_stop

- [ ] **STOP-01**: `emergency_stop` is a finite state machine with an explicit reachable state space
- [ ] **STOP-02**: From any reachable state, an abort signal drives the system to a defined safe state
- [ ] **STOP-03**: No decoder output is produced for the remainder of a session after a stop
- [ ] **STOP-04**: Kani proves, across the enumerated reachable state space, that the transition
      function is total, that the latch and output-gate invariants hold, and that no panic or
      arithmetic overflow is reachable. The module forbids `unsafe`, so the undefined-behaviour
      classes a model checker would otherwise carry are excluded by the type system instead;
      the published claim says which properties the proof establishes, which the compiler
      establishes, and which nothing in the phase establishes
- [ ] **STOP-05**: `cargo kani` passes in CI as a blocking gate
- [ ] **STOP-06**: The module has 100 percent branch coverage (tarpaulin or llvm-cov)
- [ ] **STOP-07**: Abort latency is bounded and measured on the reference rig

### Filter node

- [ ] **FILT-01**: Causal IIR filtering over 1024 channels
- [ ] **FILT-02**: SIMD vectorised (`wide` or `pulp`)
- [ ] **FILT-03**: Structure-of-arrays layout so per-channel access is linear and vectorisable
- [ ] **FILT-04**: Numerically validated against a SciPy reference within a stated tolerance
- [ ] **FILT-05**: A criterion benchmark against the SciPy path published as a speedup figure

### Decoder node

- [ ] **DEC-01**: An exported decoder model runs through `candle` or `ort`
- [ ] **DEC-02**: The runtime choice is decided on measured WCET on the reference rig, not
      ecosystem taste, and the comparison is published
- [ ] **DEC-03**: Inference WCET is measured rather than averaged and fits the end-to-end
      budget with stated headroom
- [ ] **DEC-04**: Model loading and swapping happen on the control plane, never the hot path

### Ingest and wire protocol

- [ ] **WIRE-01**: A synthetic source sustains 1024 channels at 30 kHz, 16-bit, with zero drops
- [ ] **WIRE-02**: Ingest uses io_uring with SQPOLL, DEFER_TASKRUN, and NAPI busy-poll
- [ ] **WIRE-03**: A custom UDP wire format with an 8-byte `CLOCK_MONOTONIC_RAW` header
- [ ] **WIRE-04**: FlatBuffers payloads read in place with no parse step
- [ ] **WIRE-05**: ChaCha20-Poly1305 AEAD using hardware crypto extensions
- [ ] **WIRE-06**: IEEE 1588 hardware timestamps captured at ingest via the `e1000e` PTP clock
- [ ] **WIRE-07**: 480 MB/s sustained ingest with zero drops over a 24 hour soak

### pylsl compatibility

- [ ] **LSL-01**: A drop-in Python package such that an existing pylsl script runs unmodified
- [ ] **LSL-02**: At least three real third-party LSL scripts run without source changes
- [ ] **LSL-03**: A measured latency improvement over liblsl is shown for those scripts

### Observability and storage

- [ ] **OBS-01**: Per-node latency histograms exported
- [ ] **OBS-02**: Flame graphs generated for the hot path
- [ ] **OBS-03**: A tonic-gRPC control plane for configuration, telemetry, and model swaps
- [ ] **OBS-04**: Session export to Apache Arrow IPC for replay

### Benchmark methodology

- [ ] **BENCH-01**: End-to-end DAG latency p99 under 8 ms on a replayed Indy session
- [ ] **BENCH-02**: Jitter (p99 minus p50) under 0.5 ms, published as the headline metric
- [ ] **BENCH-03**: Source to first node p99 under 100 us
- [x] **BENCH-04**: Every published figure names the rig, kernel, BIOS revision, and tuning
      state it was measured on
- [x] **BENCH-05**: Latency histograms are published rather than means, with raw captures
      alongside so numbers can be recomputed
- [x] **BENCH-06**: Losing configurations and failure cases are reported, not omitted
- [ ] **BENCH-07**: A comparison against BRAND's published figures, with methodology
      differences stated explicitly
- [ ] **BENCH-08**: A weekly committed JSON metrics file carrying p50, p95, and p99 for every
      instrumented stage

## v2 Requirements

### Formal verification (extended)

- **CREU-01**: Creusot proves the functional property that no decoder output follows a stop.
  Deferred because Kani alone already exceeds what any comparable runtime offers, and a second
  proof toolchain on the critical path of the highest-priority component is avoidable risk.

### Tooling

- **TAURI-01**: Tauri desktop monitor showing live per-node latency distributions.
  Deferred per the brief's own stated priority: cut the monitor before cutting the shim.

### Multi-host

- **PTP-01**: Full PTP clock distribution across hosts. v1 ships the documented hook and
  hardware timestamp capture only.

## Out of Scope

| Feature | Reason |
|---|---|
| Implant firmware, electrodes, device-side code | The runtime starts at the host boundary |
| Decoder training infrastructure | We consume exported models |
| Windows and macOS as deployment targets | Linux only; macOS is the dev host |
| Clinical or regulatory qualification | Research-grade software with a safety-critical posture |
| Live BRAND installation | Standing up someone else's Redis stack is not on a solo critical path; compare against published figures instead |
| Tokio or work-stealing on the hot path | Control plane only; nothing there has a deadline |
| A new wire protocol for community adoption | The adoption path is the LSL shim, not evangelism |
| Whole-system formal proof | State explosion; proofs are scoped to `emergency_stop` and the SPSC ring |

## Traceability

Populated from ROADMAP.md on 2026-08-28. RIG-01 through RIG-04 were validated before the
roadmap existed and map to no phase; they are listed for completeness.

| Requirement | Phase | Status |
|-------------|-------|--------|
| RIG-01 | Complete (pre-roadmap) | Validated |
| RIG-02 | Complete (pre-roadmap) | Validated |
| RIG-03 | Complete (pre-roadmap) | Validated |
| RIG-04 | Complete (pre-roadmap) | Validated |
| PLAT-01 | Phase 1 | Pending |
| PLAT-02 | Phase 1 | Pending |
| PLAT-03 | Phase 1 | Complete |
| SUBS-01 | Phase 3 | Pending |
| SUBS-02 | Phase 3 | Pending |
| SUBS-03 | Phase 3 | Pending |
| SUBS-04 | Phase 3 | Pending |
| SUBS-05 | Phase 3 | Pending |
| SUBS-06 | Phase 3 | Pending |
| SUBS-07 | Phase 3 | Pending |
| GRAPH-01 | Phase 5 | Pending |
| GRAPH-02 | Phase 5 | Pending |
| GRAPH-03 | Phase 4 | Pending |
| GRAPH-04 | Phase 6 | Pending |
| CHAN-01 | Phase 4 | Pending |
| CHAN-02 | Phase 4 | Pending |
| CHAN-03 | Phase 4 | Pending |
| CHAN-04 | Phase 4 | Pending |
| CHAN-05 | Phase 4 | Pending |
| CHAN-06 | Phase 4 | Pending |
| CHAN-07 | Phase 4 | Pending |
| STOP-01 | Phase 2 | Pending |
| STOP-02 | Phase 2 | Pending |
| STOP-03 | Phase 2 | Pending |
| STOP-04 | Phase 2 | Pending |
| STOP-05 | Phase 2 | Pending |
| STOP-06 | Phase 2 | Pending |
| STOP-07 | Phase 2 | Pending |
| FILT-01 | Phase 5 | Pending |
| FILT-02 | Phase 5 | Pending |
| FILT-03 | Phase 5 | Pending |
| FILT-04 | Phase 5 | Pending |
| FILT-05 | Phase 5 | Pending |
| DEC-01 | Phase 6 | Pending |
| DEC-02 | Phase 6 | Pending |
| DEC-03 | Phase 6 | Pending |
| DEC-04 | Phase 6 | Pending |
| WIRE-01 | Phase 5 | Pending |
| WIRE-02 | Phase 8 | Pending |
| WIRE-03 | Phase 8 | Pending |
| WIRE-04 | Phase 8 | Pending |
| WIRE-05 | Phase 8 | Pending |
| WIRE-06 | Phase 8 | Pending |
| WIRE-07 | Phase 8 | Pending |
| LSL-01 | Phase 7 | Pending |
| LSL-02 | Phase 7 | Pending |
| LSL-03 | Phase 7 | Pending |
| OBS-01 | Phase 5 | Pending |
| OBS-02 | Phase 5 | Pending |
| OBS-03 | Phase 5 | Pending |
| OBS-04 | Phase 5 | Pending |
| BENCH-01 | Phase 6 | Pending |
| BENCH-02 | Phase 6 | Pending |
| BENCH-03 | Phase 6 | Pending |
| BENCH-04 | Phase 1 | Complete |
| BENCH-05 | Phase 1 | Complete |
| BENCH-06 | Phase 1 | Complete |
| BENCH-07 | Phase 6 | Pending |
| BENCH-08 | Phase 1 | Pending |

**Coverage:**
- v1 requirements: 59 total
- Mapped to phases: 59
- Unmapped: 0
- Duplicated across phases: 0

**Per phase:**

| Phase | Name | Requirements | IDs |
|-------|------|--------------|-----|
| Phase 1 | Trustworthy measurement | 7 | PLAT-01, PLAT-02, PLAT-03, BENCH-04, BENCH-05, BENCH-06, BENCH-08 |
| Phase 2 | Proven emergency_stop | 7 | STOP-01, STOP-02, STOP-03, STOP-04, STOP-05, STOP-06, STOP-07 |
| Phase 3 | Deterministic substrate | 7 | SUBS-01, SUBS-02, SUBS-03, SUBS-04, SUBS-05, SUBS-06, SUBS-07 |
| Phase 4 | Lock-free transport | 8 | CHAN-01, CHAN-02, CHAN-03, CHAN-04, CHAN-05, CHAN-06, CHAN-07, GRAPH-03 |
| Phase 5 | Instrumented pipeline | 12 | GRAPH-01, GRAPH-02, WIRE-01, FILT-01, FILT-02, FILT-03, FILT-04, FILT-05, OBS-01, OBS-02, OBS-03, OBS-04 |
| Phase 6 | Decoder and the end-to-end result | 9 | GRAPH-04, DEC-01, DEC-02, DEC-03, DEC-04, BENCH-01, BENCH-02, BENCH-03, BENCH-07 |
| Phase 7 | pylsl compatibility | 3 | LSL-01, LSL-02, LSL-03 |
| Phase 8 | Wire protocol and sustained throughput | 6 | WIRE-02, WIRE-03, WIRE-04, WIRE-05, WIRE-06, WIRE-07 |

---
*Requirements defined: 2026-08-28. Traceability populated at roadmap creation, 2026-08-28.*
