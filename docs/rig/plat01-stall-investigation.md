# PLAT-01: the stall on the isolated cores

## Outcome

Outcome: not-reproduced

Neither phenomenon appeared during the three-cycle investigation. Across 6.5 hours of clean
running on the reference rig, 4.5 hours of it with a nine-event tracer armed and break limits set
at 200 us and 3000 us, no event above 96 us was observed on any isolated core. The roughly 3.8 ms
stall recorded on 2026-08-28 remains unexplained, and PLAT-01 stays open. Non-reproduction is not
a cause, and nothing below records it as one.

## What was observed

The 2026-08-28 capture (`measurements/2026-08-28-precision3591`) shows two distinct patterns on
the isolated cores.

A sustained burst: all six threads log roughly 140 overflows inside cycles 1,820,584 to
1,826,674, which at a 200 us interval is about 1.2 seconds of wall time carrying a stall over
400 us roughly every 10 ms on every isolated core at once. Called phenomenon A here.

Isolated global spikes: a handful of cross-thread events near cycles 2,259,348, 2,349,428 and
2,709,344, each landing on two or three threads within about 30 cycles. Called phenomenon B here.
The per-thread maxima in that capture run from 3679 to 3806 us.

**The association between them is not available from the artifacts.** cyclictest records
per-thread maxima in its summary and overflow cycle numbers in its overflow list, and nothing in
either ties an amplitude to a cycle. Which of the two patterns produced the 3806 us maximum
therefore cannot be determined from this capture. An earlier version of that run's README
assigned the maximum to phenomenon B; that assignment was not recoverable and has been withdrawn.

What the investigation cycles saw is stated in the Captures table below: nothing above 89 us in
either cycle, against break limits of 3000 us and 200 us, neither of which fired.

## Method

### Instrument

Nine ftrace events, armed by `scripts/nr-arm-trace.sh`. Every name was checked against this
kernel's own `available_events` before use:

    sched:sched_switch                which task displaced the cyclictest thread
    sched:sched_wakeup                whether the wakeup itself was late
    irq:irq_handler_entry             an interrupt reached an isolated core
    irq:irq_handler_exit              and how long it took
    timer:hrtimer_expire_entry        the tick, if phenomenon A's 10 ms spacing is a tick
    workqueue:workqueue_execute_start deferred work landing on an isolated core
    ipi:ipi_send_cpu                  a single-target IPI to an isolated core
    ipi:ipi_send_cpumask              a broadcast IPI to many CPUs at once
    tlb:tlb_flush                     a system-wide TLB shootdown

`ipi:ipi_send_cpumask` is not in the set plan 01-12 proposed. It was added because phenomenon B
stalls all six isolated threads simultaneously, which is the signature of a send to many CPUs
rather than to one, and `ipi_send_cpu` alone cannot see it.

`tracing_cpumask` was `3fffff`, all 22 logical CPUs, so an IPI's sender is visible and not only
its arrival on the isolated core. `buffer_size_kb` was 8195 per CPU, the kernel having rounded up
from the 8192 requested.

**The buffer was proved to hold those events on the isolated cores before any cycle was spent:
2405 of 70460 buffered lines came from CPUs 6 to 11.** This check exists because an earlier
version of this plan assumed `cyclictest --tracemark` armed tracing. It does not; it marks and
stops at a threshold. A tracer that records nothing on the isolated cores cannot answer this
question, and `nr-arm-trace.sh` exits non-zero when that count is zero rather than leaving the
judgement to an operator reading a terminal.

### Threshold calibration

Both break limits were derived from a traced maximum, that is, from a maximum observed **with
tracing already enabled**. This is the two-step calibration the RT wiki documents. It matters
because a threshold derived from an untraced baseline is set against a machine that no longer
exists once the tracer is armed: it will either fire constantly on ordinary traced operation or
never fire at all.

| Quantity | Value | Source |
|---|---|---|
| `M_untraced` | 78 us | `2026-09-01-precision3591-calibration-clean`, 3600 s, untraced |
| `M_traced` take 1 | 75 us | `2026-09-07-precision3591-investigation`, 1800 s |
| `M_traced` take 2 | 46 us | `2026-09-07-precision3591-investigation-02`, 1800 s |

Tracing overhead is not detectable at the maximum with this event set: both traced maxima fall
below the untraced one, so no inflation factor above 1 can be computed. That is weaker than it
sounds and is not strengthened here. The traced runs had half the exposure of the untraced one
and a maximum is exposure-sensitive, so this bounds any inflation loosely rather than measuring
it, and the two takes differ from each other by 29 us, which is most of the range being
discussed. What it settles is only the question that governs the next step: `M_traced` is nowhere
near 400 us, so overhead has not swallowed phenomenon A and `T_A` is derivable rather than null.

    T_A = 200 us     phenomenon A, the sustained burst over 400 us
    T_B = 3000 us    phenomenon B, the isolated multi-millisecond events

`T_A` is set from what this rig does when nothing is wrong, not from `M_traced` alone. Across
every committed clean run the observed maxima are 13, 30, 35, 37, 42, 46, 46, 72, 75, 78, 89 and
96 us. 200 us sits a little over twice the highest and half of 400, so ordinary traced operation
does not trip it and a burst at or above 400 us always does. Setting it just above `M_traced`
instead, near 100 us, would have put it below a maximum this machine has already produced twice
while behaving normally.

`T_B` is 3000 us: below the 3679 to 3806 us per-thread maxima recorded on 2026-08-28 so any event
in that family fires it, and far enough above `T_A` that a phenomenon A event does not consume
the trace first. Its limit is worth stating: an event at, say, 2500 us would fall between the two
thresholds and be caught by neither as phenomenon B. `T_B` targets the observed family, not every
conceivable multi-millisecond event.

### Instrument ordering

Ordering (a), two separate runs taken sequentially, was used for every cycle. This is a departure
from plan 01-12's stated preference for (b), rtla running concurrently in its own transient unit
so both instruments describe one window.

(b) is self-defeating on this machine. `rtla timerlat top --cpus 6-11` runs sampling threads at
SCHED_FIFO on exactly the cores cyclictest measures at priority 99, which is the configuration
that destroyed `measurements/2026-09-06-precision3591-screen-02`: 75 percent of cyclictest cycles
lost, every thread stalled at roughly 750000 us, which is `osnoise/runtime_us`. An investigation
run records `TracersQuiescent` as `NotApplicable`, so the precondition that would otherwise catch
it does not apply.

What ordering (a) means for reading the results: the ftrace buffer and the cyclictest histogram
describe the same run and the same window, because ftrace is in-kernel and adds no threads. No
rtla numbers are reported here at all, so there are no two instruments' figures to reconcile.
Recorded explicitly because silently assuming (b) while executing (a) is what
`01-EXTERNAL-AUDIT.md` finding 9 caught.

### One hypothesis removed before any cycle was spent

Phenomenon A's roughly 10 ms spacing suggested a 100 Hz scheduler tick. This kernel is
`CONFIG_HZ=1000`, so the tick period is 1 ms and a 10 ms spacing is ten tick periods rather than
one. `nohz_full=6-11` is confirmed in `/proc/cmdline`, so the tick is in any case largely
suppressed on the measured cores. The hypothesis is removed, not supported. It cost one minute of
reading and it is recorded because a removed candidate is worth as much to the next person as a
surviving one.

## Result

Outcome: not-reproduced.

### 1. The protocol

Every non-reproducing run was taken in measurement mode: performance governor, turbo disabled,
`multi-user.target` with the display manager stopped, `ppd` and `snapd` masked, 17 of 17 timers
masked, suspend blocked, on AC power. Fifteen preconditions were asserted before each run started;
in the investigation runs `TracersQuiescent` records `NotApplicable`, because tracing is the point
of an investigation run, and every other check passed. Every run was launched detached, with the
capture beginning only after the last SSH session had closed.

CPU placement was `--cpus 6-11 --main-cpus 0,1` throughout, against a kernel booted with
`isolcpus=6-11 nohz_full=6-11 rcu_nocbs=6-11 irqaffinity=0-5,12-21 intel_idle.max_cstate=1
processor.max_cstate=1 nosoftlockup nowatchdog nmi_watchdog=0 tsc=reliable skew_tick=1`. The
cyclictest interval was 200 us at priority 99 with a 400 us histogram bound. Load was idle in
every cycle: no stress-ng, no competing workload.

The armed event set, the cpumask and the buffer size are as stated under Method, and each run
directory's manifest note records them for that run individually. Break limits were 3000 us in
cycle 2 and 200 us in cycle 3; cycle 1 used 100000 us specifically so it could not fire.

One condition differed between cycles and is recorded because the D-14 snapshot does not capture
it: a USB peripheral was charging from the machine throughout cycles 1 and 2 and was removed
before cycle 3. It is judged immaterial on evidence rather than argument. The two routes by which
a charging peripheral could reach CPUs 6-11 are a device interrupt, which `irqaffinity=0-5,12-21`
excludes by construction, and an SMI from legacy USB emulation, and `MSR_SMI_COUNT` read zero on
all six isolated CPUs in every capture taken while it was attached.

### 2. The total exposure

6.5 hours of clean running, summed over thirteen runs. The arithmetic:

| Run directory | Seconds | Instrument |
|---|---:|---|
| `2026-09-01-precision3591-calibration-clean` | 3600 | headline-series |
| `2026-09-03-precision3591-calibration-clean` | 900 | headline-series |
| `2026-09-05-precision3591-screen-02` | 600 | headline-series |
| `2026-09-05-precision3591-screen-03` | 900 | headline-series |
| `2026-09-05-precision3591-screen` | 900 | headline-series |
| `2026-09-06-precision3591-screen-03` | 60 | headline-series |
| `2026-09-06-precision3591-screen-04` | 60 | headline-series |
| `2026-09-07-precision3591-recon` | 120 | headline-series |
| `2026-09-07-precision3591-screen` | 60 | headline-series |
| `2026-09-07-precision3591-investigation` | 1800 | investigation, traced |
| `2026-09-07-precision3591-investigation-02` | 1800 | investigation, traced |
| `2026-09-07-precision3591-investigation-03` | 5400 | investigation, traced |
| `2026-09-08-precision3591-investigation` | 7200 | investigation, traced |

3600 + 900 + 600 + 900 + 900 + 60 + 60 + 120 + 60 + 1800 + 1800 + 5400 + 7200 = 23400 seconds,
which is 6.5 hours. Of that, 1800 + 1800 + 5400 + 7200 = 16200 seconds, 4.5 hours, was taken with
the nine-event tracer armed.

Excluded from the sum and why: `2026-09-02` and `2026-09-03-precision3591-calibration-contaminated`
were deliberately contaminated; `2026-09-06-precision3591-screen-02` was starved by an orphaned
osnoise kthread and lost 75 percent of its cycles; `2026-08-28-precision3591` is the reconstructed
pre-protocol capture that is the subject of this investigation rather than evidence in it.

The highest maximum anywhere in that 6.5 hours is 96 us, from
`2026-09-07-precision3591-screen`.

### 3. What that exposure does and does not bound

It establishes that no event of the stated magnitude was observed on the isolated cores across
23400 seconds of clean running, under the conditions in section 1, with a nine-event tracer armed
and break limits at 200 us and 3000 us, on this rig, on kernel 7.0.0-30-realtime.

It does not establish that the stall does not occur. It does not establish a rate, a frequency,
or any bound expressed in hours or in events per unit time. A bound of that kind would require
assumptions about how the event is distributed in time, and this investigation neither supplies
those assumptions nor can test them: an event that occurs rarely and irregularly is entirely
consistent with everything observed here. And it does not identify a cause, of the stall or of
its absence.

### 4. What is still true

The roughly 3.8 ms stall was observed. It is in `measurements/2026-08-28-precision3591`, on this
rig, on the PREEMPT_RT kernel, and the capture is committed. It was not an artifact of the
histogram bound, not an overflow accounting error, and not a figure anyone typed: it is in the
raw per-thread output.

The calibration pair puts the clean and contaminated conditions side by side:

| Run | Duration | Max | CAL | TLB | RES | device IRQ |
|---|---:|---:|---:|---:|---:|---:|
| `2026-09-01-...-calibration-clean` | 3600 s | 78 us | 6 | 6 | 55 | 1127 |
| `2026-09-02-...-calibration-contaminated` | 900 s | 3856 us | 12 | 6 | 24 | 211 |
| `2026-09-03-...-calibration-contaminated` | 3600 s | 1901 us | 12 | 6 | 41 | 986 |

The stall co-occurred with the contaminated conditions and did not appear under the clean ones.
**That is an association across three runs. It is not a demonstrated mechanism, and it is not an
attribution.** The interference counters in that table do not support one either: they barely
separate the arms, and `config/contamination-thresholds.json` already records that their
magnitudes are implausible against the roughly 137,000 CAL interrupts seen on the contaminated
2026-08-28 baseline, so whatever these snapshots count is not the interference D-15 assumed.

PLAT-01 stays Pending. The stall remains unexplained.

### What this outcome does demonstrate

One thing, and it is a claim about method rather than about the platform. The original number was
real and reproducible under the original conditions. Its meaning depended entirely on conditions
nobody had recorded: no protocol, no precondition list, no interference census, no record of what
else was running on the machine. Six months later that capture would have been indistinguishable
from a platform property. A documented protocol is what makes that difference visible, and the
difference between a 78 us clean arm and a 3856 us contaminated one is the whole argument for
building the harness before publishing the figure.

That is a statement about measurement discipline. It is not an explanation of the stall.

## What this means for published figures

The stall is unexplained and PLAT-01 remains open. Every latency figure published in this phase
carries it as an explicit limitation, naming the exposure over which it was not observed: 6.5
hours of clean running on the reference rig, 4.5 of them traced, with no event above 96 us.

Nothing here supports an expectation about future runs. A non-observation over 23400 seconds is
not a prediction that a headline run will be free of the stall, and no figure published in this
phase may be worded as though it were.

Plan 01-13 writes the PLAT-03 headline figure next and inherits this limitation verbatim.

## Captures

| Run directory | Cycle | Purpose | Break limit | Fired | Max | Outcome |
|---|---|---|---:|---|---:|---|
| `2026-09-07-precision3591-investigation` | 1 | Tracing-overhead calibration | 100000 us | no | 75 us | `M_traced` take 1. Superseded: taken by a harness whose admission gate did not enforce the two-instrument rule |
| `2026-09-07-precision3591-investigation-02` | 1 | Tracing-overhead calibration, re-take | 100000 us | no | 46 us | `M_traced` take 2 |
| `2026-09-07-precision3591-investigation-03` | 2 | Phenomenon B | 3000 us | no | 72 us | Not reproduced in 90 minutes |
| `2026-09-08-precision3591-investigation` | 3 | Phenomenon A | 200 us | no | 89 us | Not reproduced in 2 hours |

Four run directories, three cycles. The D-20 budget is three capture-and-analyse cycles and three
were spent; cycle 1 has two directories because it was re-taken after a harness defect was found
and fixed between the takes, and the re-take performed no new analysis. Both are published: the
first capture's measurement was sound and only its admission field was wrong.

No investigation run appears in `metrics/latency-series.json`, and none can: the admission gate
excludes every run whose `instrument_class` is `investigation`, naming the two-instrument rule as
the reason.
