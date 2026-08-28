# Precision 3591 real-time viability screen

Purpose: decide whether a Dell Precision 3591 laptop can serve as the neurorust
reference rig, or whether dedicated hardware is required. The deciding question is
firmware-induced latency (SMIs), which the kernel cannot observe or preempt and which
therefore sets a floor no amount of PREEMPT_RT work can lower.

Method: `hwlatdetect` parks a CPU and polls the TSC for gaps. A gap means firmware took
the core without the kernel's knowledge. Run on the stock Ubuntu 26.04.1 kernel from a
live USB, so nothing was installed to disk.

See `RIG.txt` for the full environment, including BIOS revision and microcode.

## Captures

| File | Config | Duration |
|---|---|---|
| `hwlatdetect-stock-15m.txt` | governor=powersave, turbo on, C-states to C10 | 900 s |
| `hwlatdetect-tuned-15m.txt` | governor=performance, turbo off, C6/C10 disabled | 900 s |

## Result, stock

    max                125 us   (1 occurrence)
    dominant mode       21 us   (207 of 291 events, ~1/s)
    events > 10 us     291
    affected CPUs      cpu2 (16), cpu4 (275)

Against the project's gates this splits: `cyclictest max < 30 us` fails by roughly 4x,
while the jitter gate (p99 - p50 < 0.5 ms) passes easily, since a single event in ~900
samples sits beyond p99.9. That split matters because the project's thesis is that the
bounded worst case is what counts.

Caveat on the stock run: the battery was actively charging (84%), and charge controllers
are a known SMI source.
