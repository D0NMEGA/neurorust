# Rig recon, 2026-09-05: rtla hwnoise and MSR_SMI_COUNT

## What was probed and why

`hwlatdetect` cannot sample more than one CPU on this kernel (its hwlat tracer's `mode`
resets to `none`, matching wherever the scheduler happens to place its single thread, before
every run it launches), so no capture in this repository characterises firmware latency on
CPUs 6 to 11: three D-18 arms put all 32 events on CPU 5, and the 2026-08-28 baseline's
P-core arm put all 13 of its events on CPU 0. `rtla hwnoise` (rtla 7.0.12, from
`linux-tools-common`, already installed) runs one dedicated osnoise thread per CPU named in
its `-c` list, so every isolated core gets its own sampling thread instead of sharing one
migrating thread with the rest of the system. `rdmsr -p <cpu> 0x34` reads `MSR_SMI_COUNT`, an
exact per-CPU count of system management interrupts serviced on that CPU, with no sampling
and no threshold. Full background on why these two instruments were chosen over driving the
hwlat tracer directly is in `docs/rig/firmware-floor-rt-vs-stock.md`; this document only
records what each one's real output looks like.

## rtla hwnoise output format

Probe: `rtla hwnoise -c 6-11 -H 0-5 -P f:99 -d 60s`, taken 2026-09-05T22:49:59Z, kernel
7.0.0-30-realtime. Full output is `probe-rtla-hwnoise.txt` beside this file.

Verbatim from the final redraw:

```
duration:   0 00:01:00 | time is in us
CPU Period       Runtime        Noise  % CPU Aval   Max Noise   Max Single          HW          NMI
  6 #59         44250000            3    99.99999           2           1           3           0
```

Every CPU named in `-c 6-11` (6, 7, 8, 9, 10, 11) produces its own row in every redraw. This
is the property that replaces `hwlatdetect`: one thread per listed CPU, not one thread that
may or may not visit it.

The header block (`CPU Period ... NMI`) repeats before every redraw, once per second, and
each redraw is preceded by a full-screen clear: the raw file shows a lone `c` character
immediately before the `Hardware-related Noise` banner line on every redraw. That `c` is the
tail of a terminal reset/clear escape sequence that a non-interactive capture does not render;
a parser reading this format must skip it along with the repeated header rather than treat it
as data. `rtla hwnoise --help` (captured at the end of the same probe file) documents
`-q/--quiet: print only a summary at the end`, which avoids the repeated redraw entirely.
Plans 01-20 and 01-21 should invoke `-q` and parse one summary block, not the 585-line redraw
this probe captured to see the format once.

`Period` is a redraw counter, not a duration: it read `#59` at the last redraw of this 60
second probe. `Runtime` is that CPU's own accumulated osnoise-thread execution time in
microseconds, and it is the field that answers per-CPU exposure, not the wall-clock `duration`
header. The two disagree here: `duration` reads `00:01:00` (60 s of wall clock) while every
CPU's `Runtime` reads `44250000` us (44.25 s) at the same redraw. The ratio is constant and
exact across the whole probe: `Runtime` equals `Period` times 750000 us for every period and
every CPU checked (period 1: 750000; period 59: 44250000 = 59 x 750000), so each CPU's osnoise
thread is idle for a fixed fraction of every redraw interval. Neither the numeric period nor
runtime flag was passed to this probe, so the 750000 us figure is this build's default, not a
value pinned by an invocation; a parser must read `Runtime` directly rather than assume it
equals wall-clock duration or derive it from the requested `-d`.

`Max Single` (the largest one-shot noise event within the period) read `1` us on all six
isolated CPUs at the final redraw, and `NMI` read `0` on all six for the whole probe. `HW`
(hardware-noise event count for the period) ranged `2` (CPU 10) to `7` (CPU 9) at the final
redraw. All of this is well inside the 30 us cyclictest gate and is a schema check, not a
figure: this was a 60 second probe, not the 900 second capture D-18 wants.

Item 4 of the plan's own checklist, whether `-H 0-5` kept rtla's own control threads off
6-11, was not checked: this probe did not open a second connection to run
`ps -eLo pid,tid,psr,comm` during the run, so thread placement is not directly confirmed here.
The indirect evidence (zero NMI throughout, single-digit-microsecond `HW` noise on every
isolated CPU) is consistent with `-H` working as documented, not proof of it.

One version note: this probe's own `# rtla banner:` comment line came back blank, but
`rtla hwnoise --help`'s first line reads `rtla hwnoise: a summary of hardware-related noise
(version 7.0.12)`, confirming the same 7.0.12 build `docs/rig/recon-2026-08-31/FINDINGS.md`
recorded from a bare `rtla` invocation, just carried inline in the help banner rather than on
a separate line before it.

## MSR_SMI_COUNT

Probe: `rdmsr -p <cpu> 0x34` on CPUs 0, 5, 6, 7, 8, 9, 10, 11, taken 2026-09-05T22:49:50Z,
msr-tools 1.3+git20220805.7d78c80-1build1, kernel 7.0.0-30-realtime. Full output is
`probe-rdmsr-smi-count.txt` beside this file.

`rdmsr -p <cpu> 0x34` prints a bare hexadecimal value with no `0x` prefix and no other text,
one line per invocation, for example `fa6` for CPU 0. `rdmsr -d -p <cpu> 0x34` prints the same
register in plain decimal with no other text, for example `4006` for CPU 6.

Every CPU checked, the two housekeeping CPUs (0 and 5) and all six isolated CPUs (6 through
11), read the identical value at probe time: `0xfa6` (4006 decimal). The counter is non-zero
on CPUs 6 to 11: this establishes directly that SMIs have reached the isolated cores, the
exact question three `hwlatdetect` arms left unanswered because none of their events ever
named a CPU in that range.

What this settles: a non-zero `MSR_SMI_COUNT` on CPUs 6 to 11 establishes that SMIs reach
them, with no sampling and no threshold involved. What it does not settle: a count is not a
duration, so it says how many, never how long any individual SMI took. It is also a single
point-in-time read here, not a before/after pair around a specific capture window, so it
establishes only that the cumulative count is non-zero as of probe time, not a rate or a count
attributable to any one run. Attributing SMI activity to a specific capture requires reading
the register immediately before and immediately after that capture, as the plan's own design
for this field intends.

## What this changes for the firmware screen

Plan 01-20 (the parser and manifest fields) should parse `rtla hwnoise -q` output, not the
live redraw this probe captured, and should treat each CPU's `Runtime` column as its real
exposure figure rather than the wall-clock `duration` header; it should also add an
`MSR_SMI_COUNT` before/after pair as a new manifest provenance field alongside the existing
interference counters. Plan 01-21 (the wiring) can use the exact argv this probe confirms
works, `rtla hwnoise -c 6-11 -H 0-5 -P f:99 -d <duration>` and `rdmsr -p <cpu> 0x34` before and
after, to wire both instruments into the harness the way `nr-capture` already wires cyclictest
and hwlatdetect. Plan 01-23 (the re-taken D-18 arms) re-takes the firmware screen with both
instruments now probed and installed; every event either one reports will name a CPU in 6-11
directly, closing the coverage gap that put every event so far on CPU 0 or CPU 5.

Standing check, stated plainly so it is not lost again: any firmware capture consumed from
here on must assert which CPUs its events name, not only report the maximum. Applied to the
2026-08-28 raw capture, that single assertion, that all 13 P-core-arm events named CPU 0
rather than a target isolated core, would have caught the coverage defect on the day the
capture was taken, not three arms and 32 events later.
