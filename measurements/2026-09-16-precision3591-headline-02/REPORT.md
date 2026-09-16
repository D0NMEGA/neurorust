# STOP-07 abort latency report

run id: 2026-09-16-precision3591-headline-02
poll period: 33000 ns
trials: 200000

## Abort observation latency (end to end)

Measured from the timestamp the aborting thread takes immediately before the latch store to the timestamp the hot-path thread takes at the iteration where it first observes the gate closed (D-31), over a uniformly random abort phase within one poll period (D-34). Nothing sits between either timestamp read and the call it brackets.

| statistic | value (ns) |
|-----------|------------|
| p50 | 16691 |
| p95 | 31535 |
| p99 | 32842 |
| p99.9 | 33188 |
| maximum (worst case) | 33434 |

sample count: 200000

## Decomposition (D-34)

poll period: 33000 ns, stated as its own term. The hot path checks the latch once per iteration at a fixed point in the loop (D-32), so the end-to-end observation latency above is bounded by one iteration plus cross-core propagation.

cross-core propagation: min -831 ns, mean -7.6 ns, max 6869 ns, from 2026-09-16-precision3591-recon-04 (1000 samples). method: a Cristian's-algorithm round-trip estimate between the two measurement cores (crates/stop-harness/src/characterise.rs::offset_estimate_ns)

clock read overhead: min 35 ns, mean 42.8 ns, max 137650 ns, from 2026-09-16-precision3591-recon-04 (1000000 samples). method: the consecutive-delta cost of reading CLOCK_MONOTONIC_RAW on the measurement host (crates/stop-harness/src/characterise.rs::read_overhead_ns)

running clocksource: tsc

These figures are reported side by side and are not combined, netted or subtracted from one another. The end-to-end worst case above already includes whatever cross-core propagation and poll-period delay actually occurred in each trial; the decomposition names the terms that compose it, and arithmetic between the two would double-count or discard part of what was actually measured.

## Caveats

- The measured hot path is the D-33 stand-in: a single thread on one isolated core whose loop body is only the latch poll, nothing else. Phase 6 re-measures against the real DAG.
- This measurement predates the Phase 3 substrate: no mlockall, no preallocated pools, no allocator hook (D-36).
- Thread interleavings are unverified until CHAN-06 brings loom into CI in Phase 4 (D-50); this figure says nothing about them.
- The PLAT-01 approximately 3.8 ms stall is unexplained and not reproduced. This figure carries it as an explicit limitation; see docs/rig/plat01-stall-investigation.md for the exposure it was not observed over.
- The cross-core offset estimate assumes symmetric propagation delay in both directions of the ping-pong exchange.

