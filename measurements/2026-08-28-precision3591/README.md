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

See `RIG.txt` for the full environment: BIOS 1.23.0 (2026-04-24), microcode 0x28,
Core Ultra 9 185H, P-cores 0-11, E-cores 12-21.

## Results

| Run | Config | Load | Max | Events >10us / 900s |
|---|---|---|---|---|
| `hwlatdetect-stock-15m.txt` | powersave, turbo on, C-states to C10 | idle | **125 us** | 291 |
| `hwlatdetect-tuned-15m.txt` | performance, turbo off, C6/C10 off | idle | **<10 us** (7 us at 1us threshold) | 0 |
| `hwlatdetect-tuned-underload-15m.txt` | performance, turbo off, C6/C10 off | 22 cores saturated, 91-93 C | **29 us** | 26 |
| `hwlatdetect-pcore-underload-10m.txt` | as above, sampling restricted to P-cores 0-11 | 22 cores saturated, 93-95 C | **22 us** | 13 / 600 s (~19 / 900 s) |

Tuning was three sysfs writes: `performance` governor on all CPUs, `no_turbo=1`, and
disabling C6 and C10 while keeping POLL and C1E.

## Findings

**Tuning removes almost all firmware latency.** Worst case fell from 125 us to under 10 us,
an 18x improvement, from sysfs writes alone. The 21 us mode present roughly once per second
on the stock configuration disappeared entirely.

**Thermal load reintroduces it, bounded.** Saturating all 22 cores to 91-93 C brought back
26 events, worst case 29 us. This is the load-triggered SMI behaviour that idle screening
cannot detect, and it is why the idle result alone is not sufficient evidence.

**P-cores are not firmware-clean.** In the whole-machine run every event happened to land on
an E-core (cpu13, 14, 15, 20, 21), which suggested that isolating P-cores might avoid SMIs
entirely. A run pinned explicitly to `--cpu-list=0-11` refuted that: 13 events in 600 s with a
22 us max, a comparable rate to the whole-machine figure. The E-core clustering was a sampling
artifact of `hwlatdetect` rotating across CPUs, not a property of the hardware. P-cores are
modestly better (22 us versus 29 us) but not exempt, and no core-isolation choice removes the
firmware floor.

**The 22-core saturation is deliberately pessimistic.** The real workload isolates a handful
of P-cores and runs a pipeline; it does not saturate the package. 29 us should be read as a
worst-case bound under adversarial thermal conditions, not as the expected operating point.

## Caveats

- Battery was charging (84-93%) during all runs. Charge controllers are a known SMI source.
- 15 minutes is not 24 hours. The acceptance gates call for day-long soaks.
- Detector validity was confirmed: at `--threshold=1` the tuned idle machine reported 6
  events with a 7 us max, so the zero-above-10us result is a real measurement rather than a
  dead instrument.

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
