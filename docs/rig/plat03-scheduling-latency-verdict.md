# PLAT-03: worst-case scheduling latency on isolated cores

## Verdict

Verdict: documented limitation

Observed maximum: 81 us against the 30 us gate

Samples at or above 30 us: 283 of 431999988 (0.0000655%)

The boundary is `>=`. A sample of exactly 30 us is at the gate, not under it, and
`Percentiles::samples_at_or_above` is the function that counts them. The renderer's own gate
comparison is the matching strict `<` (`crates/metrics/src/report.rs:106`), which commit ece44b0
corrected from `<=`; the old form reported a maximum of exactly 30 us as under the 30 us gate.
Both boundaries agree, so a reader does not need to check them separately.

ROADMAP.md Phase 1 success criterion 3 permits three closes: under the gate, a residual attributed
to a named platform cause, or a residual published as an explicitly unattributed limitation with
the run, its raw capture, and the reason attribution failed committed alongside. This verdict takes
the third. The reason attribution failed is stated in "Attribution" below, and it is a fact about
the instruments used, not a judgement about the residual.

## What was measured

    run directory:  measurements/2026-09-08-precision3591-headline/
    duration:       14400 s requested, 14400.057081594 s measured (0.0004% over)
    window:         2026-09-08T05:17:26Z to 2026-09-08T09:17:26Z
    placement:      --cpus 6-11 --main-cpus 0,1
    workload:       cyclictest alone, SCHED_FIFO priority 99, 200 us interval,
                    400 us histogram bound, 6 threads, --mlockall, --distance=0
    tool:           cyclictest V 2.80
    harness:        nrmeasure 0.1.0, source 6ae0e01, clean tree, provenance pushed-stamp,
                    executable blake3 dfeff7f69bb515ea7b4542581b78741014c7d6f99d4905e5e4bc59b98f2d4b81

No firmware instrument shared this window. `--with-hwnoise` and `--with-hwlatdetect` were both
withheld deliberately: a firmware instrument running concurrently changes the conditions the
scheduling maximum is measured under, and the firmware screens were taken as their own runs. That
separation is what makes the two figures independently interpretable.

Operating conditions, from the manifest rather than from the runbook:

    machine:        Dell Precision 3591, Intel Core Ultra 9 185H, 22 logical / 16 cores
    bios:           1.23.0 released 2026-04-24, microcode 0x28
    kernel:         7.0.0-30-realtime, PREEMPT_RT
    os:             Ubuntu 26.04.1 LTS, installed, display manager inactive
    systemd target: multi-user.target
    cmdline:        isolcpus=6-11 nohz_full=6-11 rcu_nocbs=6-11 irqaffinity=0-5,12-21
                    intel_idle.max_cstate=1 processor.max_cstate=1 nosoftlockup nowatchdog
                    nmi_watchdog=0 tsc=reliable skew_tick=1
    governor:       performance on all 22 CPUs, energy_performance_preference default
    no_turbo:       1
    c-states:       C6 not present, C10 not present on cpus 6-11
    package temp:   60.0 C at start, 65.0 C at end, 67.05 C maximum over the run
    power:          AC online, battery at 100% and "Not charging"

There were no protocol deviations. All 15 precondition results carry status `pass`, including
`no-active-ssh-sessions` (observed 0) and `tracers-quiescent` (observed `current_tracer=nop
events/enable=0 set_event=(empty) tracing_on=0 instances=none samplers=none`). The second of those
matters for this particular run: the PLAT-01 investigation immediately preceding it left nine
ftrace events armed with `tracing_on` at 1, and a headline-series run measured through a live
tracer would report an inflated maximum. The launcher refuses rather than measuring through it.

The series admission gate recorded `admitted: yes` with no exclusions, on eight evidence sources,
every one of them upstream of the measured latency. `excluded_from_series` is false.

## Distribution

| percentile | latency (us) |
|------------|---------------|
| p50 | 2 |
| p95 | 4 |
| p99 | 8 |
| p99.9 | 10 |
| p99.99 | 11 |
| maximum | 81 |

    total samples:    431999988
    binned samples:   431999988
    overflow samples: 0

p50, p95, p99 and p99.9 are the values the generated `REPORT.md` prints. p99.99 is not in that
table; it was computed from the committed `cyclictest.json` under the same convention
(`hdrhistogram` at 3 significant figures, `value_at_quantile`, overflows included), and that
computation reproduces all four published values exactly before it is trusted for the fifth.

Per-thread maxima, listed individually because six similar maxima and one outlier mean different
things and a pooled figure hides which happened:

| thread | cpu | maximum (us) | samples at or above 30 us |
|--------|-----|--------------|---------------------------|
| 0 | 6 | 34 | 251 |
| 1 | 7 | 70 | 14 |
| 2 | 8 | 62 | 11 |
| 3 | 9 | 81 | 7 |
| 4 | 10 | 21 | 0 |
| 5 | 11 | 22 | 0 |

The six maxima are not similar. Two threads never exceeded 22 us across four hours; one reached 81.

Overflow convention: overflow samples are recorded at the histogram bound, which is a lower bound
on their true value, so percentiles at or above the overflow fraction are conservative. This run
recorded zero overflows against a 400 us bound and an 81 us maximum, so no percentile here is
affected by it.

The raw capture is committed beside the figures: `cyclictest.hist` and `cyclictest.json` as
cyclictest wrote them, `hist.tsv` as the pooled bin table, and `manifest.json` carrying a blake3
for each. Every figure in this document is recomputable from those files.
`nrmeasure verify --strict --check-index` re-derives the report from the capture and fails if they
disagree, so the check is mechanical rather than a promise.

## Jitter

p99 minus p50 is 6 us.

## The independent firmware observation

This is a separate measurement, reported here so both figures are visible together. It is not a
decomposition of the figure above.

    instrument:      rtla hwnoise (rtla version 7.0.12)
    run directory:   measurements/2026-09-06-precision3591-screen-03/
    invocation:      rtla hwnoise -c 6-11 -H 0-5 -P f:99 -d 900s
    observed cpus:   6, 7, 8, 9, 10, 11
    maximum:         1 us, being the largest Max Single value across the observed CPUs' final
                     rtla hwnoise rows
    population:      13 recorded events
    exposure:        674.25 s per CPU, as rtla itself reports it
    conditions:      idle arm, no additional load, package temperature 58.0 C at start,
                     installed PREEMPT_RT system
    MSR_SMI_COUNT:   delta 0 on each of cpus 6, 7, 8, 9, 10 and 11

Three arms were taken. This is the idle arm, cited because its operating conditions are the closest
of the three to this headline run: no additional load, and a starting package temperature of 58.0 C
against this run's 60.0 C. The other two arms ran under stress load at 83.1 C and 79.0 C, further
from these conditions rather than nearer. All three are recorded in
[docs/rig/firmware-floor-rt-vs-stock.md](firmware-floor-rt-vs-stock.md), which also states what
`rtla hwnoise` does and does not measure and why its per-CPU exposure figure is not wall-clock
duration divided by a CPU count.

The headline run itself recorded an `MSR_SMI_COUNT` delta of 0 on each of cpus 6 to 11 across its
own four hours, read before cyclictest started and after it finished. That is a statement about
this run, from this run's own manifest, and it is not combined with anything.

These two figures are not subtracted, and combining them would be invalid rather than merely
inelegant. `cyclictest` measures wakeup latency on the isolated CPUs; `rtla hwnoise` counts
execution gaps left after software noise is accounted for, sampled by its own threads, at a
different time, under a different load and a different thermal state. Firmware stalls and
scheduling delay do not compose additively. A scheduling maximum produced entirely by kernel
activity, minus an unrelated hardware-gap observation, is neither an upper nor a lower bound on
anything: the subtraction can understate by any amount, and `saturating_sub` would flatten a
firmware figure larger than the scheduling maximum to zero. `render_plat03_verdict` used to perform
this arithmetic and no longer does. There is no difference, ratio or percentage anywhere in this
document that takes a number from one of these instruments and a number from the other.

## Attribution

[docs/rig/plat01-stall-investigation.md](plat01-stall-investigation.md) records
`Outcome: not-reproduced`. The roughly 3.8 ms stall observed on 2026-08-28 did not recur across
6.5 hours of clean running on this rig, 4.5 of them with a nine-event tracer armed and break limits
set at 200 us and 3000 us, with the highest excursion seen anywhere in that exposure being 96 us.

Non-reproduction adds exposure without an observed recurrence. It does not resolve the earlier
cause, and it does not establish a bound on how often the event can occur. The stall remains
unexplained and PLAT-01 remains open. Nothing in this section may be read as evidence that the
stall is absent from the population this run sampled.

The residual in this run is unattributed. No trace supports any cause for it, and the reason is
specific rather than a shrug:

  - This is a headline-series run, so under the two-instrument rule it was taken with tracing
    quiescent. A traced run inflates the latency it measures, which is why an investigation run can
    never feed the series and why the harness now refuses to admit one. The consequence is that no
    ftrace evidence exists for this capture, by design.
  - The traced cycles that do exist used break limits of 200 us and 3000 us, both derived to catch
    the 3.8 ms phenomenon. Every excursion in this run is below 200 us, so those cycles could not
    have captured this population even had it occurred during them.

Attributing any part of the 283 samples at or above 30 us would therefore require a traced run
designed around this population rather than around the 3.8 ms one. That run has not been taken.
Describing the residual does not attribute it.

What can be said, as observation rather than attribution, is that the tail has visible structure:

  - 701 samples fall in the 25 to 34 us range and every one of them is on cpu6. The population ends
    sharply at 34 us; cpu6 produced nothing above that in four hours.
  - There are no samples at all between 35 and 41 us, on any thread.
  - The 24 samples from 42 to 81 us are on cpu7, cpu8 and cpu9 only, and never on cpu6, cpu10 or
    cpu11.
  - The `/proc/interrupts` deltas over the run are uneven across the isolated cores: cpu6 took 4158
    irqs and 87 rescheduling IPIs, against 56 irqs and 9 rescheduling IPIs on cpu9.

The coincidence between cpu6's interrupt count and cpu6's exclusive ownership of the 25 to 34 us
population is a correlation within a single run, with no trace behind it. It is recorded here as a
lead for a future traced investigation and it is not an attribution. Both populations are
unattributed, and they stay unattributed until a capture names a cause.

Every published latency figure in this phase also carries the unexplained 3.8 ms stall as an
explicit limitation, naming the exposure over which it was not observed: 6.5 hours of clean running
on this rig, 4.5 of them traced, no event above 96 us. This figure inherits that limitation.

## What this figure does and does not support

It supports: a maximum observed for wakeup latency on cpus 6 to 11 of this named rig, under the
protocol in [docs/measurement-protocol.md](../measurement-protocol.md), over four hours and
431999988 samples, at this kernel, firmware revision and tuning state.

It does not support a claim about any other machine, including another Precision 3591. It does not
support a claim about durations longer than four hours; a longer run samples a larger population
and can only find the same or worse. It does not support a claim that anything is bounded below
81 us. Every number here is a maximum observed under the stated test conditions, and a finite
screen establishes no bound.

## Losing configurations

`measurements/INDEX.md` lists every run in the repository. Eighteen runs are committed and fifteen
carry `excluded_from_series: true`. None was deleted, and no exclusion is expressed by a run being
absent. Per BENCH-06, the excluded runs are:

  - `2026-08-28-precision3591`, the original screen, reconstructed rather than harness-generated,
    excluded because SSH commands ran against the machine during the cyclictest run.
  - The two deliberately contaminated calibration arms from 2026-09-02 and 2026-09-03, which waived
    precondition violations on purpose to give the contamination detector something true to be
    measured against.
  - The two clean calibration arms from 2026-09-01 and 2026-09-03, excluded because no calibrated
    contamination thresholds existed when they were taken.
  - Seven firmware and instrument screens from 2026-09-05 through 2026-09-07, including the three
    `rtla hwnoise` arms cited above and the run that was starved by an orphaned rtla sampling
    thread.
  - Three of the four PLAT-01 investigation runs, excluded by instrument class: a traced run
    inflates the latency it measures.

Three runs carry `excluded_from_series: false`. This headline run is one. The second is
`2026-09-07-precision3591-recon`, a short recon-class run taken to confirm the admission fix on
real hardware. The third is `2026-09-07-precision3591-investigation`, the first PLAT-01 cycle 1
take, and its false value is wrong: it was captured before commit a646407 taught the admission gate
that instrument class `investigation` excludes a run outright. D-12 forbids editing a published
manifest, so the field stays as recorded, the run was re-taken as
`2026-09-07-precision3591-investigation-02` under the corrected gate, and the error is stated here
and in that run's own note rather than quietly repaired. Its measurement was sound; only its
admission field was wrong.

`nrmeasure series` is not implemented until plan 01-14, so `excluded_from_series` currently records
an admission decision and nothing consumes it yet. What the weekly series selects from the admitted
runs is that plan's to state.
