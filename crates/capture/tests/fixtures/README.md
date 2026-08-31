# Capture fixtures

## probe-proc-interrupts.txt, probe-sysfs-tuning.txt

Rig-recon format probes from plan 01-02 (docs/rig/recon-2026-08-31/), captured on the installed
Dell Precision 3591 system (kernel 7.0.0-30-realtime) alongside the two `crates/histogram`
cyclictest probes. These are NOT measurements. No number in either file may ever be quoted or
published as a latency, thermal, or interference result; they exist only so the D-14/D-15 parsers
in this crate have real rig-produced input to parse, rather than a synthetic guess.

`probe-proc-interrupts.txt` is a verbatim `/proc/interrupts` snapshot, all 22 CPUs, including the
`CAL` (function call IPI) and `TLB` (shootdown) rows the 2026-08-28 post-mortem found diagnostic
for D-15's interference-counter diffing.

`probe-sysfs-tuning.txt` is a stable, line-oriented `key=value` snapshot of the D-06 preconditions
and tuning state: per-CPU `cpuN.governor`, `intel_pstate.no_turbo`, per-cpu0 C-state `disable`
flags, `kernel.realtime`, `systemd.default_target`, `tracing.current_tracer`, AC/battery state,
and per-zone thermal readings. It was captured while the rig was untuned (every CPU's governor
reads `powersave`; see docs/rig/recon-2026-08-31/FINDINGS.md, "The rt-tuning.service
contradiction", for why a service that reports itself active does not guarantee this). Its
`intel_pstate.no_turbo=unavailable` line is a known false negative: the capture command queries
`/sys/devices/system/intel_pstate/no_turbo` (no `cpu/` segment), which does not exist on this
kernel, rather than the real, populated path `/sys/devices/system/cpu/intel_pstate/no_turbo`. The
value is kept verbatim because it is genuine tool output and the discrepancy itself is a recorded
finding, not an error to silently correct; see FINDINGS.md for the full explanation and its
consequence for how `nr-capture`'s own preconditions must query these paths.
