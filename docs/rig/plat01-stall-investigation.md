# PLAT-01: the stall on the isolated cores

**Status: incomplete.** This document is written across plan 01-12. The instrument and
calibration sections below are cycle 1's output and are final. The outcome section is written
by task 3, after cycles 2 and 3 have run, and is deliberately absent until then rather than
filled with a provisional answer.

## Instrument

Nine ftrace events, armed by `scripts/nr-arm-trace.sh` and verified to record on the isolated
cores before any capture was spent against them. Every name was checked against this kernel's
own `available_events` on 2026-09-07:

    sched:sched_switch                which task displaced the cyclictest thread
    sched:sched_wakeup                whether the wakeup itself was late
    irq:irq_handler_entry             an interrupt reached an isolated core
    irq:irq_handler_exit              and how long it took
    timer:hrtimer_expire_entry        the tick, if phenomenon A's 10 ms spacing is a tick
    workqueue:workqueue_execute_start deferred work landing on an isolated core
    ipi:ipi_send_cpu                  a single-target IPI to an isolated core
    ipi:ipi_send_cpumask              a broadcast IPI to many CPUs at once
    tlb:tlb_flush                     a system-wide TLB shootdown

`ipi:ipi_send_cpumask` is not in the event list plan 01-12 proposed. It was added because
phenomenon B stalls all six isolated threads simultaneously, which is the signature of a send
to many CPUs rather than to one, and `ipi_send_cpu` alone cannot see it.

`tracing_cpumask` is `3fffff`, all 22 logical CPUs, so an IPI's sender is visible and not only
its arrival on the isolated core. `buffer_size_kb` is 8195 per CPU: the kernel rounded up from
the 8192 requested.

The verification that matters: 2405 of 70460 buffered lines came from CPUs 6 to 11. An earlier
version of this plan assumed `cyclictest --tracemark` armed tracing. It does not, it marks and
stops at a threshold, and a tracer that records nothing on the isolated cores cannot answer this
question. `nr-arm-trace.sh` exits non-zero when that count is zero, so the check is a machine
decision rather than an operator reading a terminal.

## Calibration

Both break thresholds are derived from a maximum observed with tracing already enabled. This is
the two-step calibration the RT wiki documents, and skipping it produces thresholds that either
never fire or fire constantly on ordinary traced operation.

| Quantity | Value | Source |
|---|---|---|
| `M_untraced` | 78 us | `2026-09-01-precision3591-calibration-clean`, 3600 s, untraced |
| `M_traced` take 1 | 75 us | `2026-09-07-precision3591-investigation`, 1800 s |
| `M_traced` take 2 | 46 us | `2026-09-07-precision3591-investigation-02`, 1800 s |

**Tracing overhead is not detectable at the maximum with this event set.** Both traced maxima
fall below the untraced one, so no inflation factor above 1 can be computed from these data. That
statement is weaker than it looks and is deliberately not strengthened: the traced runs had half
the exposure of the untraced one, and a maximum is exposure-sensitive, so this bounds any
inflation loosely rather than measuring it. The two takes also differ from each other by 29 us,
which is most of the range being discussed, so run-to-run variation on this machine is comparable
to the effect being looked for.

What it does settle is the question the plan actually asks. `M_traced` is nowhere near 400 us, so
tracing overhead has not swallowed phenomenon A on this machine and `T_A` is derivable rather
than null.

### The two thresholds

    T_A = 200 us     phenomenon A, the sustained burst over 400 us
    T_B = 3000 us    phenomenon B, the isolated multi-millisecond events

`T_A` is set from what this rig actually does when nothing is wrong, not from `M_traced` alone.
Across every committed clean run, the observed maxima are 13, 30, 35, 37, 42, 46, 46, 75, 78 and
96 us. The highest is 96 us, from `2026-09-07-precision3591-screen`. Excluded from that list:
3856 and 1901 us from the two deliberately contaminated calibration arms, and 750021 us from
`2026-09-06-precision3591-screen-02`, the capture an orphaned osnoise kthread starved.

200 us sits a little over twice the highest ordinary maximum and half of 400, so ordinary traced
operation does not trip it and a burst at or above 400 us always does. Setting it just above
`M_traced` instead, at 100 us, would have put it below a maximum this machine has already
produced twice while behaving normally.

`T_B` is 3000 us, unchanged from the plan. It is below the 3679 to 3806 us per-thread maxima
recorded on 2026-08-28, so any event in that family fires it, and far enough above `T_A` that a
phenomenon A event does not consume the trace first. Its limit is worth stating: an event at,
say, 2500 us would fall between the two thresholds and be caught by neither as phenomenon B.
`T_B` targets the observed family, not every conceivable multi-millisecond event.

## Outcome

Written by plan 01-12 task 3, after cycles 2 and 3. Not yet determined.

The first line of this section will be exactly one of `Outcome: named`, `Outcome: not-reproduced`,
or `Outcome: open-question`, so it can be grepped. Per ROADMAP success criterion 1 as rewritten on
2026-09-07, non-reproduction is not a cause and may not be recorded as one: if the stall does not
reproduce, this section states the protocol, the total exposure, and what that exposure does and
does not bound, PLAT-01 stays open, and every published latency figure carries the unexplained
stall as a stated limitation.
