# neurorust

A Rust runtime for ingesting, routing, filtering, and decoding intracortical neural data on a
PREEMPT_RT Linux kernel, built for bounded worst-case latency rather than good average latency.
It targets implant-scale streams (1024 channels at 30 kHz, 16-bit) and is intended to expose a
Lab Streaming Layer compatible interface, so an existing lab acquisition script can adopt it by
changing one import line.

This is a real-time systems project, not an edge AI project. A decoder model runs as one node in
the middle of the graph, but the engineering problem is scheduling determinism. The nearest
neighbours are pro audio, industrial motion control, and low-latency trading, not TinyML.

Two artifacts define the project. If everything else is cut, these are what remain:

- The safety-critical abort path is proven correct rather than tested.
- Every latency claim is reproducible from published raw captures on a named rig.

## Current state

Early. The runtime itself does not exist yet. What exists is the measurement methodology that
every later latency claim depends on, and it is not finished either.

| Area | State |
|---|---|
| Measurement harness and provenance contract | Built, in use, still gaining fixes |
| Reference rig characterised | Yes, with committed raw captures |
| Proven `emergency_stop` | Not started |
| Node graph, transport, filters, decoder | Not started |
| Wire protocol, LSL compatibility | Not started |

Phase 1 of 8 is open. The headline scheduling-latency figure has not been taken, and the ~3.8 ms
global stall seen on all six isolated threads is not yet explained. Run admission is decided
from evidence upstream of the measurement, not from the shape of the latency it measured; the
contamination thresholds are still marked provisional, and a provisional verdict is recorded on
every run and rendered in the report, but it plays no part in that decision. Those are tracked,
not glossed.

## What the harness does

`nrmeasure` takes a measurement on a named rig and refuses to produce one that would not be
defensible later.

```
nrmeasure run          Execute a run and write a stamped run directory
nrmeasure verify       Validate every run directory against its manifest
nrmeasure reconstruct  Build a manifest for a capture taken before the harness existed
nrmeasure series       Append runs to the metrics series and compare against a baseline
nrmeasure attempt      Mark an orphaned in-progress attempt failed, with a stated reason
```

Before a run starts, fifteen preconditions are asserted against the live machine: governor,
turbo, deep C-states on every target CPU, isolation, tracing quiescence across all four control
files, absence of SSH and graphical sessions, thermal headroom against a declared profile, and
more. A run that violates one is refused rather than recorded with a footnote.

Each run directory carries the raw tool output, a manifest naming the rig, kernel, BIOS revision,
tuning state, exact argv, and blake3 digest of every artifact, plus an attempt record written
before the first instrument starts so that a capture killed midway leaves its evidence behind
rather than taking it with it. `nrmeasure verify --strict` re-derives the published percentile
table from the raw histogram instead of trusting the number that was written down. Losing and
contaminated runs stay in `measurements/` and appear in the index; nothing is quietly retaken.

## Reference rig

Every figure in this repository names the machine it was measured on. No number from an unpinned
or shared machine is accepted.

| | |
|---|---|
| Machine | Dell Precision 3591 |
| CPU | Intel Core Ultra 9 185H, 16 cores / 22 threads |
| Memory | 32 GB DDR5-5600 |
| Kernel | 7.0.0-30-realtime (PREEMPT_RT) |
| Isolation | `isolcpus=6-11 nohz_full=6-11 rcu_nocbs=6-11` |

## Results so far

The firmware floor on the isolated cores, measured 2026-09-06 with `rtla hwnoise` running one
osnoise sampling thread per CPU, across three 900-second arms (idle, all 22 logical CPUs
saturated, and housekeeping-only load):

- No isolated CPU observed a single event above 1 us in any arm.
- `MSR_SMI_COUNT` delta was zero on every one of CPUs 6-11 in every arm.
- NMI count was zero throughout.

This corrects a previously published 22 us figure. That number came from `hwlatdetect`, whose
tracer runs a single non-migrating kernel thread that `isolcpus` keeps off CPUs 6-11 entirely,
so all thirteen of its events landed on CPU 0. It described one housekeeping core, not the cores
the runtime isolates. The measurement is bounded by its conditions: three quarter-hour arms, one
machine, one firmware revision. An observed maximum is not a guaranteed worst case, and this
document does not claim otherwise.

## Repository layout

```
crates/manifest    Manifest and attempt schema, provenance tier, checksums
crates/histogram   cyclictest parsing, overflow accounting, percentiles
crates/capture     Preconditions, environment snapshot, contamination verdict, instruments
crates/metrics     Series, baseline comparison, coverage, report rendering
crates/cli         The nrmeasure binary
docs/              Measurement protocol, publication layout, rig recon and findings
measurements/      Run directories with raw captures, manifests, and INDEX.md
schemas/           Generated JSON Schema for manifest, attempt, and metrics
scripts/           Rig-side root scripts, version controlled and installed by deploy/
```

## Building

```sh
cargo build --workspace
cargo test --workspace
```

Deployment targets Linux only, and PREEMPT_RT is required for any published real-time figure.
The harness builds and its tests run on macOS, which is where development happens; taking a
measurement requires the rig.

## Reproducing a measurement

`docs/measurement-protocol.md` states the system state a run requires and how to reach it. The
short version is that the rig is put into measurement mode (performance governor, no turbo,
multi-user target, display manager stopped, timers masked), every SSH and console session is
closed, and the capture is launched detached so no interactive session is open while it runs.

To check the published artifacts without a rig:

```sh
cargo build -p nr-cli --release
./target/release/nrmeasure verify --strict --check-index
```

## Licence

Dual licensed under Apache 2.0 and MIT, matching Rust ecosystem convention.
