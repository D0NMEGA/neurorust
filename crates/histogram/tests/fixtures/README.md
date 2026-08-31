# Histogram fixtures

## cyclictest-rt-isolated-idle-10m.hist

Byte-identical copy of measurements/2026-08-28-precision3591/cyclictest-rt-isolated-idle-10m.hist.
Kernel 7.0.0-30-realtime, isolcpus=6-11, 6 threads, SCHED_FIFO prio 99, mlockall,
200 us interval, 10 minutes. The run was contaminated (SSH activity and an active GNOME
session); it is a parser fixture, not a publishable figure.

### Format

Line 1 is the literal `# Histogram`.
Data lines are `%06d` bin index followed by one `%06d` count per column. The first separator
is a space and the rest are tabs, so split on any whitespace.
Bin index is latency in microseconds at 1 us resolution.

This file has 6 columns for 6 threads, which means it came from `-h 400` (plain histogram).
`-H` / `--histofall` adds one extra summary column on the right, so a `--histofall` capture of
6 threads has 7 columns. The parser must handle both and must not assume the column count
equals the thread count.

Footer lines, in order:
  `# Min Latencies: <one per thread>`
  `# Avg Latencies: <one per thread>`
  `# Max Latencies: <one per thread>`
  `# Histogram Overflows: <one per thread>`
  `# Histogram Overflow at cycle number:`
  `# Thread <n>: <space separated cycle numbers>`  (one line per thread)

### Expected values

Threads: 6
Bin range present: 1 to 399
Per-thread bin sample totals: 2999187, 2999145, 2999156, 2999151, 2999168, 2999149
Total binned samples: 17994956
Per-thread overflow counts: 142, 152, 146, 152, 145, 151
Total overflows: 888
Total samples including overflows: 17995844
Per-thread maxima: 3785, 3679, 3787, 3693, 3806, 3726
Global maximum: 3806
Per-thread minima: 1, 1, 1, 2, 1, 1
Overflow bound: 400 us (derived as highest present bin + 1; the run did not record histofall)

Percentiles over binned samples only, ignoring overflows (this is what the published
2026-08-28 README reported):
  p50 = 2, p95 = 4, p99 = 9, p99.9 = 10, p99.99 = 12

Percentiles with the 888 overflows recorded at the 400 us lower bound, which is correct:
  p50 = 2, p95 = 4, p99 = 9, p99.9 = 10, p99.99 = 108, p99.999 = 400 (lower bound)

Samples not under the 30 us gate (>= 30 us), the figure to publish:
  bins 30 to 399 plus the 888 overflows:  2093  (0.0116 percent of 17995844)

Two other counts, retained only so the correction can be shown side by side. Neither may be
published as the figure:
  bins 31 to 399 only:            1201  (0.0067 percent of 17994956) - what the 2026-08-28
                                        README published; strictly above 30 us, overflows
                                        omitted, and a body-only denominator
  bins 31 to 399 plus overflows:  2089  - strict boundary, superseded 2026-08-31

Why 2093 and not 2089: PLAT-03 requires latency "brought under 30 us", so a sample landing
exactly on 30 us has not met the gate. Bin 30 holds 4 samples. And 17994956 is the binned
total, which excludes the 888 overflows; counting them in the numerator but not the
denominator mixes populations. 17995844 is the true total.

Distribution shape, for the D-23 correction:
  bins 31 to 99 hold 230 samples
  bins 100 to 399 hold 971 samples
  the largest run of empty bins anywhere above 13 us is 6 bins (278 to 283)
There is no gap between 13 us and 100 us and there is no second mode.

## probe-cyclictest-h-60s.hist, probe-cyclictest-histofall-60s.hist, probe-cyclictest-60s.json

Rig-recon format probes from plan 01-02 (docs/rig/recon-2026-08-31/), captured on the installed
system, 6 threads on isolcpus 6-11, 200 us interval, 60 seconds each. These are NOT measurements:
the rig was untuned at capture time (scaling governor `powersave`, not `performance`; see
docs/rig/recon-2026-08-31/FINDINGS.md, "The rt-tuning.service contradiction"). No number from
these three files may ever be quoted or published as a latency result. Their only purpose is
pinning down cyclictest's real output formats for this parser.

`probe-cyclictest-h-60s.hist` is a `-h 400` capture (7 whitespace-separated fields per data line:
1 bin index plus 6 thread columns). `probe-cyclictest-histofall-60s.hist` is the same run with
`-H 400` (8 fields: the same 7 plus one summary column on the right), confirming the "+1 column"
rule above. The rule does not hold uniformly across every footer line, though: measured directly
on these two files, `# Min Latencies:` and `# Avg Latencies:` stay at 6 fields (per-thread only)
in both captures, while `# Max Latencies:` and `# Histogram Overflows:` gain the extra field in
the `-H` capture (6 to 7). A parser must not assume a constant `thread_count + 1` width applies to
every footer line. Full detail: docs/rig/recon-2026-08-31/FINDINGS.md, "Column counts".

`probe-cyclictest-60s.json` is the `--json` output from the same `-h` run. Per-thread `min`/`avg`/
`max` and a `cycles` sample count are present directly; there is no overflow count anywhere in the
JSON (this run did not overflow its 400 us cap), so overflow handling still requires the `.hist`
file regardless of which format is otherwise preferred. Histogram bin keys are sparse strings
(e.g. `"20"`), not a dense integer-keyed array. The real machine hostname originally present in
`sysinfo.nodename` is replaced with the literal token `[redacted]`; every other field, including
`sysinfo.release` and `.version`, is untouched. Full detail: FINDINGS.md, "What is in cyclictest
--json" and "Redaction and host identifiers".
