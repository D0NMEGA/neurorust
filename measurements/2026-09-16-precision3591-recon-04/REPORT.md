# STOP-07 clock characterisation report (D-35)

run id: 2026-09-16-precision3591-recon-04

This run measures the clock used by the abort-latency figure, not the abort-latency figure itself: the read cost of CLOCK_MONOTONIC_RAW, and the cross-core offset between the two measurement cores. Published beside the abort-latency report per D-35, never combined with it.

clock read overhead: min 35 ns, mean 42.8 ns, max 137650 ns, from 2026-09-16-precision3591-recon-04 (1000000 samples). method: the consecutive-delta cost of reading CLOCK_MONOTONIC_RAW on the measurement host

cross-core propagation: min -831 ns, mean -7.6 ns, max 6869 ns, from 2026-09-16-precision3591-recon-04 (1000 samples). method: a Cristian's-algorithm round-trip estimate between the two measurement cores, which assumes symmetric propagation delay in both directions of the exchange

running clocksource: tsc

