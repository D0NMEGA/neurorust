# Firmware floor: PREEMPT_RT versus stock kernel

Status: re-taken twice. The first re-measurement (`hwlatdetect`, below) did not produce a
firmware floor for the cores this project cares about; the reason was a measurement-coverage
defect that also affects the 2026-08-28 screening this was meant to be compared against, and
that defect is itself the result of that attempt. The second re-take, with `rtla hwnoise`
(the section immediately below the coverage defect it fixes), does cover CPUs 6 to 11
directly: three 900-second arms, idle and under two different loads, each report a pooled
maximum single event of 1 us across the six isolated CPUs -- no CPU exceeded it, and CPU 7
recorded no noise event at all in the idle arm -- with a `MSR_SMI_COUNT` delta of zero on
every one of them in every arm. That is a bound stated with its conditions (this
duration, these loads, this machine, this firmware revision), not a floor claimed without
them, and it does not retract or replace finding 1.

## Why this was re-measured

The 2026-08-28 screening states, in `measurements/2026-08-28-precision3591/README.md`:

> SMI behaviour is a firmware property and is independent of which kernel is running, so
> these figures carry over to a PREEMPT_RT install.

All four of its runs were taken from a live USB on the stock kernel 7.0.0-30-generic
(PREEMPT_DYNAMIC), with nothing installed to disk. That assumption is load-bearing under the
entire rig decision and had never been tested, which is why D-18 required a re-run on the
installed system.

## Results

RT figures are from the installed 7.0.0-30-realtime system, tuned (governor `performance`,
`no_turbo=1`, deep C-states disabled, `multi-user.target`), with `isolcpus=6-11 nohz_full=6-11
rcu_nocbs=6-11`. Load, where present, is 22 `stress-ng --cpu-method matrixprod` workers, one
pinned per logical CPU 0-21.

Differences in this table are descriptive. They are not a delta attributable to PREEMPT_RT:
see the interpretation section for the list of variables that differ.

| Condition | Stock, live USB, 2026-08-28 | PREEMPT_RT, installed | Source of the RT figure |
|---|---|---|---|
| Untuned, idle | max 125 us, 291 events / 900 s | not measured | Optional per the plan, and only worth taking if the tuned arms surprised. They did not, and the sampling defect below makes another arm of this kind uninformative until it is fixed. |
| Tuned, idle | max under 10 us (7 us at a 1 us threshold), 0 events | max below threshold, 0 events / 900 s | `measurements/2026-09-03-precision3591-calibration-clean/hwlatdetect.txt` |
| Tuned, 22 CPUs loaded | max 29 us, 26 events / 900 s, package 91 to 93 C | max 15 us, 12 events / 900 s, package 85 to 87 C | `measurements/2026-09-05-precision3591-screen/` |
| Tuned, 22 CPUs loaded, sampling restricted to CPUs 0-11 | max 22 us, 13 events / 600 s, package 93 to 95 C | max 13 us, 5 events / 600 s, package 85 to 87 C | `measurements/2026-09-05-precision3591-screen-02/` |

The 1 us threshold figure in the stock tuned-idle row has no published raw capture in this
repository. It is quoted from the README and is not independently verifiable here.

## The result: neither set of figures characterises the isolated cores

Every `hwlatdetect` event in both RT arms named CPU 5. Not one named CPUs 6-11, which are the
cores the runtime isolates and the only ones a firmware floor for this project would be about.

    2026-09-05-precision3591-screen        12 events, all cpu:5   (no --cpu-list)
    2026-09-05-precision3591-screen-02      5 events, all cpu:5   (--cpu-list 0-11)

Arm 3 existed to distinguish two explanations for arm 2's clustering: either the sampler
rotated and only CPU 5 showed gaps above threshold, or it never sampled the isolated cores.
Passing `--hwlatdetect-cpu-list 0-11` writes `tracing_cpumask` explicitly
(`/usr/sbin/hwlatdetect` lines 491 to 502), so CPUs 6-11 were in the eligible set. Every event
still landed on CPU 5. CPUs 0-4 were equally eligible in both arms and produced nothing either.

The mechanism, confirmed by reading the rig:

    /sys/kernel/tracing/hwlat_detector/mode   ->   [none] round-robin per-cpu
    /sys/kernel/tracing/tracing_cpumask       ->   000fff

The bracketed value is the active one, so the mode is `none`. In that mode the tracer does not
migrate its kernel thread; the thread samples whichever CPU the scheduler has it on.
`hwlatdetect` exposes no option to set this: its field map covers `width`, `window`,
`tracing_on`, `tracing_thresh` and `tracing_cpumask` only (`/usr/sbin/hwlatdetect` lines 236 to
240). It does not leave the mode alone either, as the section below records: its startup clears
`current_tracer`, which resets the mode to `none` even when it was set to something else
beforehand.

`isolcpus=6-11` removes those CPUs from the scheduler's automatic placement. That is the same
mechanism that left cores 6-11 at 100% idle under `stress-ng --cpu 22` until each worker was
pinned individually. A kernel thread the tracer does not actively place, on a machine whose
scheduler will not place it there, cannot sample the isolated cores. `tracing_cpumask` grants
eligibility; in `none` mode nothing acts on it.

This reaches back into the 2026-08-28 screening, which used the same tool with the same
unset mode:

    hwlatdetect-tuned-underload-15m.txt    events on cpu 13, 14, 15, 20, 21   (no --cpu-list)
    hwlatdetect-pcore-underload-10m.txt    13 events, all cpu:0               (--cpu-list 0-11)

The whole-machine arm did sample several CPUs, because that live USB had no `isolcpus` and its
scheduler moved the thread freely. Every one of those CPUs is an E-core. The P-core arm, the
one whose 22 us figure this project has been treating as its firmware floor, reports all 13 of
its events on CPU 0 alone.

So the 22 us figure characterises CPU 0 under load. It does not characterise CPUs 6-11, and no
capture in this repository does. The README's reading of the E-core clustering as a sampling
artifact was closer to right than it knew, for a different reason than it gave.

## The re-take with rtla hwnoise, 2026-09-06

Three arms, each a 900 second `rtla hwnoise -c 6-11 -H 0-5 -P f:99 -d 900s` window on the
installed PREEMPT_RT system, one osnoise sampling thread per isolated CPU rather than the
single migrating thread `hwlatdetect` uses. `MSR_SMI_COUNT` (register `0x34`) was read on
every one of CPUs 6 to 11 once before cyclictest started and once after the hwnoise window
closed, so the delta brackets cyclictest, the hwnoise screen, and the harness's own
environment snapshots around them, not the 900 second hwnoise window by itself.

| Arm | Run directory | Load | Package temp at start | Observed CPUs | Max (population) | SMI delta |
|---|---|---|---|---|---|---|
| 1, idle | `measurements/2026-09-06-precision3591-screen-03` | none | 58.0 C | 6,7,8,9,10,11 | 1 us (largest Max Single value across the observed CPUs' final rows, 13 events) | 0 on 6,7,8,9,10,11 |
| 2, 22 CPUs loaded | `measurements/2026-09-06-precision3591-screen-04` | 22 pinned `stress-ng --cpu-method matrixprod` workers, one per logical CPU | 83.1 C | 6,7,8,9,10,11 | 1 us (largest Max Single value across the observed CPUs' final rows, 64 events) | 0 on 6,7,8,9,10,11 |
| 3, housekeeping loaded | `measurements/2026-09-07-precision3591-screen` | 16 pinned workers on cpus 0-5,12-21; cpus 6-11 otherwise idle | 79.0 C | 6,7,8,9,10,11 | 1 us (largest Max Single value across the observed CPUs' final rows, 44 events) | 0 on 6,7,8,9,10,11 |

Every arm's `rtla hwnoise` rows name all six isolated CPUs, idle and under two different loads.
The event count rises with load (13, then 64, then 44), but the size of the largest single
event does not: 1 us in every arm. `MSR_SMI_COUNT` delta is zero on every one of CPUs 6 to 11
in every arm, read before cyclictest started and after the hwnoise window closed in each arm,
a bracket wider than the 900 second hwnoise window by itself.

`rtla hwnoise`'s own three statements, identical across all three arms and stated once here
rather than per arm: it measures hardware-related noise, the execution gaps left after
software noise is accounted for, and those are not uniquely identified SMIs either -- the
`MSR_SMI_COUNT` delta above is the census, this is not. Its per-CPU figures come from one
osnoise sampling thread per CPU in the `-c` list, so a CPU absent from the observed list was
sampled and reported nothing, rather than never being sampled, which is the specific
difference from `hwlatdetect` on this rig. And the per-CPU exposure it reports (674.25 seconds
per CPU in every arm here) is what the tool itself reports, not a wall-clock duration divided
across a CPU count the way an unrestricted `hwlatdetect` run would be. None of these three
statements is the `hwlatdetect` version restated: the exposure argument in particular does not
carry over, because each listed CPU here has its own sampling thread rather than sharing one
migrating thread's polling time.

A zero SMI delta over each arm's full run, on two different loads and one idle arm, is a
stronger and simpler statement than any `hwlatdetect` arm above could make, and it is the
answer D-18 has been trying to reach since 2026-08-28: no system management interrupt reached
CPUs 6 to 11 during any of these three runs. State the bound with it, not instead of it: the
delta brackets cyclictest plus the 900 second hwnoise screen, not the hwnoise screen alone
(the manifest's own utc_start to utc_end spans about 961 seconds for each of these three
runs), under each stated load, on this machine, at this firmware revision (BIOS 1.23.0,
released 04/24/2026, microcode 0x28). It is not a guarantee that no SMI can ever reach these
cores, and a longer window, a different load, or a later firmware revision could show
otherwise.

## Interpretation

The two sets of figures differ in at least six recorded ways: kernel build (7.0.0-30-generic
PREEMPT_DYNAMIC versus 7.0.0-30-realtime), root medium (live USB versus installed disk), CPU
isolation (none versus `isolcpus`/`nohz_full`/`rcu_nocbs` on 6-11), C-state and idle policy,
achieved thermal state (91 to 95 C versus 85 to 87 C), and battery state (the baseline records
charging at 84 to 93%, the RT arms ran on AC). A difference between them cannot be attributed
to PREEMPT_RT, and agreement between them would not establish transferability either.

The SMI mechanism is not in question. The kernel documentation is explicit that SMIs are set up
and serviced by BIOS code and that Linux does not know they are occurring, so the source of the
latency genuinely is kernel-independent. What that does not establish is that the distribution
of observed firmware interruptions is independent of the OS configuration, because the OS
determines power states, heat and device activity, and those determine how often a firmware
window is entered and observed.

On the evidence here the carry-over assumption is neither confirmed nor contradicted. It was
not tested, because the instrument did not sample the cores the question is about. That is a
weaker outcome than either answer, and it is the honest one.

The RT arms reached 85 to 87 C against the baseline's 91 to 95 C under nominally the same
load. Do not read that as the RT kernel running cooler. Temperature here is an outcome of the
whole configuration, not a controlled input, and matching workload and matching temperature are
different experiments that answer different questions.

## What the numbers above can and cannot support

`hwlatdetect` detects execution gaps. Those are not uniquely identified SMIs; NMI accounting and
other hardware effects contribute. Its output counts sampling records whose polling interval
contained a gap above the threshold, which is not a census of firmware interruptions, and
several gaps can land inside one record.

Exposure is not wall-clock duration. The sample width is 500 ms inside a 1 s window, so a 600 s
run polls for roughly 300 seconds in total, and in a mode that spread across N CPUs it would be
roughly 300/N seconds per CPU. In `none` mode all of it lands on one CPU, which is the only
reason these particular numbers describe a single CPU thoroughly rather than twelve CPUs badly.

Both RT manifests record `exclusion_reason: "hwlatdetect exited with code 1"`. That is the
harness misreading a convention, not a failed capture. `/usr/sbin/hwlatdetect` line 549 is
`sys.exit(maxlatency > hardlimit)` and line 458 defaults `hardlimit` to the threshold, so any
run observing anything above 10 us exits 1 by design. Both captures ran to completion. D-12
forbids editing a published manifest, so the misleading reason stays in those two files and is
corrected here.

## What PLAT-03 should report

Not a firmware floor, and not a subtraction.

There is no `firmware_floor_us` value to hand plan 01-13. The three 2026-09-06 arms do now
establish firmware observations on CPUs 6-11 (see the re-take section above), but an observed
noise ceiling over three 900 second arms is not a floor: it bounds what those arms saw, under
those conditions, not what the machine can do. An earlier version of this document was
required to name one, and to state headroom arithmetic of the form "against a 30 us gate, a
floor of n us leaves 30 minus n us for everything the kernel does". That arithmetic is invalid
independently of the coverage problem: `cyclictest` and `hwlatdetect` measure different things,
on different CPUs, at different times, and firmware stalls do not compose additively with
scheduling delay. `render_plat03_verdict` no longer performs it.

PLAT-03 should report its own observed scheduling maximum against the 30 us gate, with the
distribution, sample count and overflow count behind it, the conditions it was taken under, and
any protocol deviations. An `hwlatdetect` observation may be reported beside it, attributed to
its run and its conditions, and labelled as a hardware-gap diagnostic. It may not be subtracted
from anything.

## Setting the mode was tried, and hwlatdetect prevents it

The obvious fix is to set `hwlat_detector/mode` to `round-robin` or `per-cpu` before sampling,
so the tracer migrates across `tracing_cpumask` instead of staying put. That was implemented in
`nr-measure-mode on` and it does not work, because `hwlatdetect` undoes it.

The kernel accepts a mode change only while `current_tracer` is not `hwlat`. Measured directly:

    tracer=nop, write round-robin            -> round-robin
    then set current_tracer=hwlat            -> round-robin   (survives)
    write round-robin while tracer=hwlat     -> none          (rejected, resets)
    write round-robin with tracing_on=0      -> none          (still rejected)

So the only working order is: set the mode, then select the tracer. `hwlatdetect` clears the
tracer as part of its own startup, which resets the mode to `none` before it selects `hwlat`,
and it exposes no option to set the mode itself. Confirmed end to end on 2026-09-05: the mode
read `round-robin` immediately before a run and `none` twice during it.

A third arm was taken after the mode was set (`measurements/2026-09-05-precision3591-screen-03`,
900 s, all 22 CPUs loaded, package 87 C). All 15 of its events named CPU 5, the same as the
other two.

Measuring the isolated cores needs a different instrument. It does not need a new one:
`rtla hwnoise` is already installed (rtla 7.0.12, from `linux-tools-common`) and does exactly
this. It takes `-c/--cpus` to run one osnoise thread per CPU in the list, so every listed CPU
is sampled by its own thread rather than sharing one migrating thread, and `-H/--house-keeping`
to keep rtla's own control threads off the cores under test. For this rig that is

    rtla hwnoise -c 6-11 -H 0-5 -P f:99 -d 900s

`msr-tools` is worth adding alongside it. `rdmsr -p <cpu> 0x34` reads `MSR_SMI_COUNT`, an exact
per-CPU SMI counter. Read before and after a run it answers "did SMIs reach these cores" with
a count rather than an inference from timing gaps, and it is cheap enough to record in the
manifest for every run.

Until that exists, no `hwlatdetect` figure from this rig characterises CPUs 6-11, and none
should be published as if it did.

Whatever instrument is used, confirm from the raw output that events name CPUs 6-11 before
treating any figure as a floor for the isolated cores. Checking the CPU distribution of the
events, rather than only the maximum, should be a standing check: it is the step that would
have caught this in the 2026-08-28 screening, where the information was present in the raw
capture all along.
