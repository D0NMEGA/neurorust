---
status: PASS
agent: main-thread
phase: 01-trustworthy-measurement
plan: 12

subsystem: investigation
tags: [ftrace, cyclictest, breaktrace, plat01, tracing-calibration, not-reproduced, D-19, D-20, D-21]

requires:
  - phase: 01-trustworthy-measurement
    provides: "the measurement protocol and measurement mode (01-09, 01-11), the sudoers grant
      and nr-run-measurement (01-22), the durable attempt record (01-17), the D-28 admission
      gate and its instrument-class rule (01-24, a646407), the widened TracersQuiescent
      (01-25), the scoped rig collection script (3dd2de5)"
provides:
  - "docs/rig/plat01-stall-investigation.md: the PLAT-01 result, its method, its exposure and
    what that exposure does and does not bound"
  - "T_A = 200 us and T_B = 3000 us, derived from a traced baseline"
  - "scripts/nr-arm-trace.sh, nr-plat01-cycle1.sh, nr-plat01-cycle2.sh, nr-plat01-cycle3.sh,
    nr-plat01-capture-trace.sh: the investigation procedure under version control"
  - "four committed investigation captures, including the two that found nothing"
affects: [01-13, 01-15]
---

# Phase 1 Plan 12: PLAT-01, the stall on the isolated cores

## Outcome

**Stop condition 3: not-reproduced.** Neither phenomenon appeared across the three-cycle D-20
budget. PLAT-01 stays Pending and the stall remains unexplained.

6.5 hours of clean running on the rig, 4.5 of them with a nine-event tracer armed and break
limits at 200 us and 3000 us. Highest excursion anywhere: 96 us. The 3679 to 3806 us family
recorded on 2026-08-28 did not occur once.

| Cycle | Run | Duration | Break limit | Fired | Max |
|---|---|---:|---:|---|---:|
| 1 | `2026-09-07-...-investigation` | 1800 s | 100000 us | no | 75 us |
| 1 (re-take) | `2026-09-07-...-investigation-02` | 1800 s | 100000 us | no | 46 us |
| 2 | `2026-09-07-...-investigation-03` | 5400 s | 3000 us | no | 72 us |
| 3 | `2026-09-08-...-investigation` | 7200 s | 200 us | no | 89 us |

## What was found along the way

Two harness defects, both found by running the instrument rather than by review.

**An investigation run was series-eligible.** The D-28 admission gate read `instrument_class`
only as a source of latitude and never as a disqualifier, so cycle 1 recorded
`excluded_from_series: false` on a traced run, against the two-instrument rule. Fixed in
`a646407` by making instrument class an eighth evidence source, which fits D-28 exactly: which
instrument was used is a fact about how the run was taken, fixed before it started. Cycle 1 was
re-taken rather than corrected in a footnote; both captures are published and their manifests
carry the two harness shas, so which binary produced which verdict is readable from the record.

**A guard that could only ever refuse.** `nr-plat01-cycle2.sh` checked `tracing_on` by reading
tracefs, but the launcher runs as the operator and tracefs is root-only, so the read failed and
the `|| echo 0` fallback refused every launch. It cost 90 minutes of elapsed time. It now asks
through `nr-measure-mode`, one of the four NOPASSWD grants. A guard whose failure mode is a
silent false refusal is worse than no guard, because it looks like the thing it guards against.

One candidate was raised and deflated within the same session. A snapshot showed five TLB flushes
landing on five isolated CPUs at one timestamp, which is exactly phenomenon B's signature. The
next capture, taken during a real measured run, showed `ipi_send_cpumask` at 1 against 2250; the
earlier snapshot had been taken while rsync activity from a source push was running. Recorded
here because it was reported as a lead before it was checked.

## Deviations

1. **Instrument ordering (a), not the plan's preferred (b).** (b) runs rtla sampling threads at
   SCHED_FIFO on cpus 6-11, the cores the run measures at priority 99, which is what starved
   `2026-09-06-precision3591-screen-02`. An investigation run records `TracersQuiescent` as
   `NotApplicable`, so nothing would have refused it. Recorded in every manifest note.

2. **`ipi:ipi_send_cpumask` added to the plan's eight events.** Phenomenon B stalls all six
   threads at once, which is a send to many CPUs; `ipi_send_cpu` alone cannot see it.

3. **Four run directories, three cycles.** The acceptance criterion says never four. Cycle 1 has
   two directories because of the re-take, which performed no new analysis. The budget is three
   capture-and-analyse cycles and three were spent.

4. **Task 1's acceptance criterion cannot be satisfied and is recorded as such.** It asks the
   manifest note to carry `M_traced`, the inflation factor, `T_A` and `T_B`, all derived from the
   run the manifest describes. The note is an argument at launch and the manifest is sealed at
   the end, so the criterion asks for the one thing D-12 forbids. The derivation is in the
   write-up instead, which task 3's own `read_first` already expected to restate it. Logged in
   `deferred-items.md` as a wording fix rather than treated as met.

5. **The trace snapshot is not a manifest artifact.** Same cause: the manifest is sealed when the
   run ends and `verify --strict` rejects an unlisted capture-shaped file in a run directory. No
   break ever fired, so no trace of an event exists to store; the question of where such bytes
   would live is left for whenever one does.

## Self-Check: PASSED

- `docs/rig/plat01-stall-investigation.md` exists, ASCII only, and its first Outcome line is
  `Outcome: not-reproduced`
- The forbidden wording is absent: no "rather than the kernel", no attribution of the stall to
  contamination or anything else, no rate, frequency or bound in hours
- `measurements/2026-08-28-precision3591/README.md` no longer contains "belongs to that second
  category", and points at the investigation document without restating it
- Four investigation manifests, all `instrument_class: investigation`, all with notes
- No investigation run can reach `metrics/latency-series.json`: the admission gate excludes them
  by instrument class
- `cargo test --workspace` green; `nrmeasure verify --strict --check-index` exits 0
- The plan's own automated verify for task 3 exits 0
