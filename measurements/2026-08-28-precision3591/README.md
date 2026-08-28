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
| `hwlatdetect-pcore-underload-10m.txt` | as above, sampling restricted to P-cores 0-11 | 22 cores saturated, 93-95 C | see file | see file |

Tuning was three sysfs writes: `performance` governor on all CPUs, `no_turbo=1`, and
disabling C6 and C10 while keeping POLL and C1E.

## Findings

**Tuning removes almost all firmware latency.** Worst case fell from 125 us to under 10 us,
an 18x improvement, from sysfs writes alone. The 21 us mode present roughly once per second
on the stock configuration disappeared entirely.

**Thermal load reintroduces it, bounded.** Saturating all 22 cores to 91-93 C brought back
26 events, worst case 29 us. This is the load-triggered SMI behaviour that idle screening
cannot detect, and it is why the idle result alone is not sufficient evidence.

**Under load, every event landed on an E-core.** Distribution by CPU was cpu13 (4), cpu14 (2),
cpu15 (17), cpu20 (1), cpu21 (2). All are in the E-core range 12-21. No event occurred on a
P-core. Since the intended design isolates P-cores for the hot path and leaves E-cores to the
OS, this suggests the isolated cores may be materially cleaner than the whole-machine figure
implies. The P-core-restricted run tests that directly rather than inferring it.

**The 22-core saturation is deliberately pessimistic.** The real workload isolates a handful
of P-cores and runs a pipeline; it does not saturate the package. 29 us should be read as a
worst-case bound under adversarial thermal conditions, not as the expected operating point.

## Caveats

- Battery was charging (84-93%) during all runs. Charge controllers are a known SMI source.
- 15 minutes is not 24 hours. The acceptance gates call for day-long soaks.
- Detector validity was confirmed: at `--threshold=1` the tuned idle machine reported 6
  events with a 7 us max, so the zero-above-10us result is a real measurement rather than a
  dead instrument.
