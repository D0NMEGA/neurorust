# neurorust

## What This Is

An open-source Rust runtime that ingests, routes, filters, and decodes intracortical
neural data on a PREEMPT_RT Linux kernel, built for bounded worst-case latency rather
than good average latency. It targets implant-scale streams (1024 channels at 30 kHz,
16-bit) and exposes a Lab Streaming Layer compatible interface so an existing lab
acquisition script can adopt it by changing one import line.

It is a real-time systems project, not an edge AI project. A decoder model runs as one
node in the middle of the graph, but the engineering problem is scheduling determinism.
The nearest neighbours are pro audio, industrial motion control, and low-latency
trading, not TinyML.

## Core Value

The safety-critical abort path is proven correct rather than tested, and every latency
claim is reproducible from published raw captures on a named rig.

If everything else is cut, those two artifacts are the project.

## Requirements

### Validated

(None yet - ship to validate)

### Active

- [ ] Reference rig stood up on PREEMPT_RT with a documented cyclictest baseline
- [ ] Node graph (DAG) runtime with per-node WCET annotations and deadline-aware scheduling
- [ ] Lock-free SPSC intra-process transport with published p99, verified exhaustively under loom
- [ ] No heap allocation on the hot path, enforced by an allocator hook and asserted across a soak
- [ ] Synthetic source sustaining 1024 channels at 30 kHz with zero drops
- [ ] Causal IIR filter node, SIMD vectorised, structure-of-arrays layout, numerically validated against SciPy
- [ ] Decoder node running an exported model with a measured (not averaged) WCET
- [ ] `emergency_stop` primitive with a Kani proof passing as a blocking CI gate
- [ ] Custom UDP wire protocol with FlatBuffers payload and ChaCha20-Poly1305 AEAD
- [ ] pylsl-compatible shim running at least three unmodified third-party LSL scripts
- [ ] Jitter (p99 minus p50) measured and published as the headline metric
- [ ] Benchmark methodology published with raw captures, histograms rather than means, and named rigs
- [ ] Per-node latency histograms and session export to Apache Arrow IPC

### Out of Scope

- Implant firmware, electrodes, any device-side code - the runtime starts at the host boundary
- Decoder training infrastructure - we consume exported models
- Full multi-host PTP distribution - a documented hook only, v2
- Windows and macOS as deployment targets - Linux only (macOS is the dev host)
- Clinical or regulatory qualification - research-grade software with a safety-critical posture
- Live Stanford BRAND installation - downgraded to a comparison against BRAND's published
  figures, because standing up someone else's Redis-based stack is not on the critical path
  for a solo build
- IEEE 1588 hardware timestamps - no NIC available to this project reports a PTP hardware
  clock, so ingest uses software timestamping with the added uncertainty measured and stated
- Tokio or any work-stealing scheduler on the hot path - control plane only
- General-purpose async framework - Tokio already exists
- A new wire protocol for the community to adopt - the adoption path is the LSL shim

## Context

**Who is building it.** One person plus Claude, not the four to six engineers the original
brief was staffed for. Roughly a 6x staffing gap against the source document, so the
roadmap sequences work for a single operator rather than six parallel workstreams.

**Development host.** MacBook Pro, Apple M5 Pro, arm64, 18 cores, 24 GB, macOS 26.5.
Lima and colima are installed. macOS cannot produce any real-time number, but it can carry
the Kani and Creusot proofs, loom, criterion microbenchmarks, the filter and its SciPy
parity check, the pylsl shim, gRPC, Tauri, and Arrow IPC. The proofs, which are the single
most differentiating artifact, need no special hardware at all.

**Reference rig, unresolved.** Two candidates:

- Dell Precision 3591, Intel Core Ultra 7/9 185H (Meteor Lake), 16 cores, x86_64. Free and
  fast. Risks: laptop embedded-controller SMIs cause 50-300 us spikes that are invisible to
  the kernel and would put the 30 us cyclictest gate out of reach; hybrid P/E cores
  complicate isolcpus and SCHED_DEADLINE admission control, which assumes uniform CPU
  capacity; thermal throttling under sustained load; and the machine is needed for Windows,
  which conflicts with repeated 24-hour soaks.
- Raspberry Pi 5, 45 to 55 USD plus about 10 USD for the active cooler it needs under
  sustained load. No SMIs at all, an officially supported Ubuntu `raspi` real-time variant,
  arm64 matching the dev laptop (one NEON target, one ARMv8 crypto target), and dedicated
  so a 24-hour soak costs nothing. Limits: 4 cores, and Gigabit Ethernet caps the wire at
  about 125 MB/s.

Resolution path is a zero-cost screening run: `hwlatdetect` plus stock-kernel `cyclictest`
from an Ubuntu 26.04.1 live USB on the Dell, which touches no disk. Clean result means a
free 16-core x86 rig. Dirty result means buy the Pi, and the measurement is itself worth
publishing since few people document why laptops fail as RT platforms.

**Machine explicitly off-limits.** `bicfcomp01.ccbb.utexas.edu` (UT Austin CCBB): dual Xeon
Gold 5420+, 112 threads, 754 GB RAM, 2x10G bonded, passwordless sudo. Verified 2026-08-28 to
be running kernel 6.8.0-137-generic (PREEMPT_DYNAMIC, not RT) with no isolcpus, load average
9.0, four to five users, and roughly thirty Docker containers. It is a shared preprocessing
node. No kernel changes, no reboot, no isolcpus, and no monopolising soaks. It may be used
lightly as an unmodified Linux target, and no real-time number may ever be attributed to it.

**Kernel availability, verified 2026-08-28.** PREEMPT_RT was merged into mainline Linux at
6.12, so no out-of-tree patch is needed. Canonical's support matrix: Ubuntu 24.04 LTS ships
RT 6.8 (generic and raspi) via Ubuntu Pro; 24.04 HWE ships RT 6.17; 25.10 ships RT 6.17 from
`universe`; and **Ubuntu 26.04 LTS ships RT 7.0 from the main archive**, installable with
`apt install ubuntu-realtime` and no Pro subscription. Both candidate rigs are covered.

**Competitive reference.** Stanford BRAND. The claim is not lower absolute latency but
materially tighter jitter, plus a safety guarantee BRAND does not offer.

## Constraints

- **Team**: one person plus Claude - sequence for a single operator, no parallel workstreams
- **Hot path**: no allocation, no locks, no logging, no blocking syscalls, no unbounded queue,
  no work-stealing scheduler; the burden of proof is on whoever wants to add one
- **Platform**: Linux only for deployment; PREEMPT_RT required for any published RT number
- **Rig discipline**: every quoted figure names the machine it was measured on, and no number
  from an unpinned or shared machine is accepted
- **Timestamps**: software timestamping only - no available NIC has a PTP hardware clock
- **Network**: if the rig is a Pi, the wire is capped near 125 MB/s and the 480 MB/s figure
  applies to the in-process and shared-memory path, stated plainly rather than blurred
- **Budget**: 0 to 55 USD of hardware; the only item money buys is throughput over a fast NIC,
  which is the least differentiating claim in the project
- **Licence**: Apache 2.0 and MIT dual, matching Rust ecosystem convention
- **Provenance**: 1024 channels is a publicly reported electrode count used as a sizing target;
  no public artifact may imply a specification obtained via an advisor or industry contact

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Proof and methodology before throughput records | Both cost nothing and are what a reviewer who knows the field actually weighs. The brief says so itself: the differentiating signal is methodology and the proof, not headline numbers | - Pending |
| Do not touch bicfcomp01's kernel | Shared preprocessing node under real load; rebooting it for a side project risks access and disrupts other users | ✓ Good |
| Ubuntu real-time kernel over a hand-built PREEMPT_RT | On 26.04 LTS it is in the main archive, needs no Pro token, and ships RT 7.0, which is well past the 6.12 mainline merge | - Pending |
| BRAND parity downgraded to published-figures comparison | Standing up someone else's Redis-based stack is real work that is not on the critical path for a solo build | - Pending |
| Software timestamps, gap documented | No available NIC reports a PTP hardware clock; measure and publish the added uncertainty rather than pretend | - Pending |
| Screen the Dell before buying anything | `hwlatdetect` from a live USB is free, risks nothing on disk, and decides the question with data | - Pending |
| Rig chosen for determinism and availability over speed | The rig is a measurement instrument. The project's own thesis, that a bounded worst case beats a good average, applies to choosing it | - Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (applied automatically):
1. Requirements invalidated? -> Move to Out of Scope with reason
2. Requirements validated? -> Move to Validated with phase reference
3. New requirements emerged? -> Add to Active
4. Decisions to log? -> Add to Key Decisions
5. "What This Is" still accurate? -> Update if drifted

**After each milestone** (via `/donny-complete-milestone`):
1. Full review of all sections
2. Core Value check - still the right priority?
3. Audit Out of Scope - reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-08-28 after initialization*
