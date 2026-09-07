# Precision 3591 real-time viability screen

Purpose: decide whether a Dell Precision 3591 laptop can serve as the neurorust reference
rig, or whether dedicated hardware is required. The deciding question is firmware-induced
latency (SMIs), which the kernel cannot observe or preempt and which therefore sets a floor
no amount of PREEMPT_RT work can lower.

Method: `hwlatdetect` parks a CPU and polls the TSC for gaps. A gap means firmware took the
core without the kernel's knowledge. All runs used the stock Ubuntu 26.04.1 kernel
(7.0.0-30-generic, PREEMPT_DYNAMIC) from a live USB. Nothing was installed to disk.
SMI behaviour is a firmware property and is independent of which kernel is running, so
these figures carry over to a PREEMPT_RT install.

**That carry-over claim was untested when written.** It was the reason D-18 was opened, and
it was neither confirmed nor contradicted by the PREEMPT_RT re-measurements: see
`docs/rig/firmware-floor-rt-vs-stock.md`'s Interpretation section for why the instrument
available at the time could not test it either.

See `RIG.txt` for the full environment: BIOS 1.23.0 (2026-04-24), microcode 0x28,
Core Ultra 9 185H, P-cores 0-11, E-cores 12-21.

## Results

| Run | Config | Load | Max | Events >10us / 900s |
|---|---|---|---|---|
| `hwlatdetect-stock-15m.txt` | powersave, turbo on, C-states to C10 | idle | **125 us** | 291 |
| `hwlatdetect-tuned-15m.txt` | performance, turbo off, C6/C10 off | idle | **<10 us** (7 us at a 1us threshold per an unsaved, unverifiable capture; see Caveats) | 0 |
| `hwlatdetect-tuned-underload-15m.txt` | performance, turbo off, C6/C10 off | 22 cores saturated, 91-93 C | **29 us** | 26 |
| `hwlatdetect-pcore-underload-10m.txt` | as above, sampling restricted to P-cores 0-11 | 22 cores saturated, 93-95 C | **22 us** | 13 / 600 s (~19 / 900 s) |

All 13 events behind the P-core-restricted row's 22 us figure named cpu 0: that figure
describes cpu 0 under load, not the machine as a whole. See Findings below, and
`docs/rig/firmware-floor-rt-vs-stock.md` for the coverage defect this reflects and its
2026-09-06 re-take on the isolated cores this project actually runs on.

Tuning was three sysfs writes: `performance` governor on all CPUs, `no_turbo=1`, and
disabling C6 and C10 while keeping POLL and C1E.

## Findings

**Tuning removes almost all firmware latency.** The observed maximum fell from 125 us to
under 10 us, at least a 12x improvement, from sysfs writes alone (the unsaved 1us-threshold
capture would put this closer to 18x, but that figure is not independently verifiable; see
Caveats). The 21 us mode present roughly once per second on the stock configuration
disappeared entirely.

**Thermal load reintroduces it, bounded.** Saturating all 22 cores to 91-93 C brought back
26 events, a maximum of 29 us. This is the load-triggered SMI behaviour that idle screening
cannot detect, and it is why the idle result alone is not sufficient evidence.

**Neither arm characterises the P-cores, or the machine.** In the whole-machine run every
event happened to land on an E-core (cpu13, 14, 15, 20, 21), which suggested that isolating
P-cores might avoid SMIs entirely. A run pinned explicitly to `--cpu-list=0-11` refuted that:
13 events in 600 s with a 22 us max, a comparable rate to the whole-machine figure. The reason
in both cases is the same: the hwlat tracer's `mode` was `none` in every one of these runs, so
its thread did not migrate and each arm sampled whichever single CPU the scheduler happened to
place it on, rather than rotating across the eligible set -- the E-cores above in one arm, cpu
0 alone in the other. Neither arm characterises the machine, and the comparison between 22 us
and 29 us is between two single-CPU observations taken under different loads, not a
P-core-versus-whole-machine comparison. See `docs/rig/firmware-floor-rt-vs-stock.md` for the
full account of the coverage defect, and its 2026-09-06 re-take with `rtla hwnoise`, an
instrument that samples every requested CPU with its own thread.

**The 22-core saturation is deliberately pessimistic.** The real workload isolates a handful
of P-cores and runs a pipeline; it does not saturate the package. 29 us should be read as the
maximum observed under the stated test conditions, not as the expected operating point.

**Corrected 2026-09-06 under Phase 1 (D-18 re-take, plan 01-23).** Three claims above did not
survive a second instrument that could actually sample CPUs 6 to 11, the cores this project
isolates: the P-core-restricted row's 22 us figure is CPU 0 alone, not the P-cores as a group;
the E-core clustering was the hwlat tracer's non-migrating `mode: none`, not a rotation
artifact; and an observed maximum does not, by itself, establish a bound on any future run.
`docs/rig/firmware-floor-rt-vs-stock.md` carries the full account and the new figures. The
raw `hwlatdetect` captures themselves are unchanged and their checksums are recorded in
manifest.json.

## Caveats

- Battery was charging (84-93%) during all runs. Charge controllers are a known SMI source.
- 15 minutes is not 24 hours. The acceptance gates call for day-long soaks.
- Detector validity was checked at the time with a `--threshold=1` run that reported 6 events
  with a 7 us maximum on the tuned idle machine. That capture was not saved and is not in this repository,
  so the observation is recorded here as an unverifiable note rather than as evidence. The
  zero-above-10us result rests on the committed 10 us captures alone.
- The `--threshold=1` detector-validity run above is worth retaking with a saved capture;
  plan 01-23 (re-taking the firmware screens on the isolated cores) is its owner.

## Disk preparation (2026-08-28)

Windows NTFS shrunk from 475.8 GiB to 355.8 GiB to free 120 GiB for Ubuntu.
`partition-table-BEFORE-resize.txt` and `-AFTER-resize.txt` capture the exact sfdisk
dumps either side, so the operation is reversible with `sfdisk < BEFORE` if the
filesystem is later shrunk back.

Preconditions verified before touching the disk: NTFS `Volume Flags: 0x0000` (clean),
no `hiberfil.sys` (so Fast Startup was not leaving the volume suspended), no BitLocker
signature, Secure Boot already disabled.

The `ntfsresize` dry run reported `Needed relocations: 0`, meaning all in-use data already
sat below the target boundary and the shrink was a metadata update rather than a 173 GB
migration. Partition was then set 1.92 GiB larger than the filesystem so the partition can
never be smaller than its contents.

Post-resize verification: read-only mount succeeded, `/Windows` and `/Users` present,
space in use unchanged at 173.4 GB.

## First PREEMPT_RT measurement (2026-08-28, CONTAMINATED - see caveat)

`cyclictest-rt-isolated-idle-10m.hist`. Kernel 7.0.0-30-realtime (`/sys/kernel/realtime` = 1),
`isolcpus=6-11`, 6 threads under SCHED_FIFO prio 99, `mlockall`, 200 us interval, 10 minutes,
18.0 M samples.

| Percentile | Latency |
|---|---|
| p50 | 2 us |
| p95 | 4 us |
| p99 | 9 us |
| p99.9 | 10 us |
| p99.99 | 108 us |
| max | 3806 us |

Percentiles include the 888 histogram overflows, recorded at the 400 us histogram bound. That
is a lower bound on their true value, so p99.99 and above are conservative. The maximum is
taken from the cyclictest footer and is exact. The previously published p99.99 of 12 us was
computed from the histogram bins alone and omitted the overflows.

Samples not under the 30 us gate (>= 30 us): 2,093 of 17,995,844 (0.0116%). This counts the
1,205 samples in bins 30 to 399 plus the 888 samples cyclictest reported separately as
histogram overflows. The previously published figure of 1,201 of 17,994,956 (0.0067%) counted
only samples strictly above 30 us, omitted the overflows entirely, and used a denominator that
excluded them.

Boundary and denominator per the 2026-08-31 operator decision: PLAT-03 requires latency
"brought under 30 us", so a sample landing exactly on 30 us has not met the gate (bin 30 holds
4 samples). And 17,994,956 is the binned total, which excludes the 888 overflows, so counting
overflows in the numerator but not the denominator would mix populations in the very
correction meant to fix a population error. The true total is 17,995,844.

**Split verdict.** Jitter (p99 - p50) is 7 us against the 500 us target, passing by ~70x.
`cyclictest max < 30 us` fails by ~127x. This is the project's own thesis in data: a 2 us
median says nothing about a 3.8 ms stall.

**Two structural clues.** The distribution has a continuous long tail rather than two modes.
Bins 31 to 99 hold 230 samples and bins 100 to 399 hold 971. The largest run of empty bins
anywhere above 13 us is 6 bins (278 to 283), which is consistent with sparse sampling at 2 to 8
counts per bin rather than with a gap. The earlier claim of a gap between 13 us and 100 us, and
the inference of a specific recurring event that rested on it, are withdrawn. The ~3.8 ms
maximum appeared on all six threads at nearly identical values (3785, 3679, 3787, 3693, 3806,
3726), the signature of a global stall such as `stop_machine()` or a system-wide TLB shootdown
rather than per-core interference.

**There are two phenomena, not one.** All six threads log roughly 140 overflows inside cycles
1,820,584 to 1,826,674, which at a 200 us interval is about 1.2 seconds of wall time carrying a
stall over 400 us roughly every 10 ms on every isolated core at once. Separately, a handful of
isolated cross-thread events appear (cycles near 2,259,348, 2,349,428 and 2,709,344, each
landing on two or three threads within about 30 cycles), and the roughly 3.8 ms maximum belongs
to that second category. The sustained burst and the isolated global spikes are different
phenomena and are investigated separately under PLAT-01.

**CAVEAT - do not publish this figure.** The run was contaminated. SSH commands were executed
against the machine during the measurement (`ps -L`, `tmux capture-pane`, `scp`), each creating
processes, taking network interrupts, and triggering TLB shootdown IPIs that broadcast to the
isolated cores; `/proc/interrupts` showed CAL (function-call interrupt) counts of ~137k on
CPUs 6-11. A GNOME session was also active with a user typing into it.

**Next measurement must**: run with no SSH activity, with the desktop idle or the system
dropped to `multi-user.target`, and with `--tracemark` plus ftrace armed so the kernel records
what it was executing at the moment of the worst spike instead of leaving us to speculate.

**Corrected 2026-08-31 under Phase 1 (D-23).** Two claims in the original text did not survive
re-derivation from the raw histogram: a claim of a gap in the distribution that is not in the
data, and an over-gate count that omitted the histogram overflows. The percentile table was
regenerated from the raw capture by the measurement harness. The raw captures themselves are
unchanged and their checksums are recorded in manifest.json.
