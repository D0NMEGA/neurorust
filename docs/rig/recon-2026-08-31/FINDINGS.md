# Phase 1 rig recon findings, 2026-08-31

Captured against the Dell Precision 3591 reference rig (Ubuntu 26.04.1 LTS, kernel
7.0.0-30-realtime, the installed system, not a live USB) over SSH. Root-gated reads went through
two fixed, argument-free scripts granted passwordless sudo, `nr-recon` (read-only) and `nr-probe`
(the two 60 second format probes plus a few more read-only captures); every other command ran
unprivileged. Raw outputs are committed alongside this file under `docs/rig/recon-2026-08-31/`
and as fixtures under `crates/histogram/tests/fixtures/` and `crates/capture/tests/fixtures/`.

Nothing in this plan is a measurement and no rig state was changed. The two cyclictest probes
below ran on an untuned machine (governor `powersave`, see "The rt-tuning.service contradiction"),
which is fine for their purpose (pinning down output formats) and disqualifying for any other
purpose. No number from `probe-cyclictest-h-60s.hist`, `probe-cyclictest-histofall-60s.hist`, or
`probe-cyclictest-60s.json` may ever be quoted as a latency result.

## Which tracers are available

Verbatim contents of `/sys/kernel/tracing/available_tracers` (see `available-tracers.txt`):

```
timerlat osnoise hwlat blk mmiotrace function_graph wakeup_dl wakeup_rt wakeup function nop
```

`timerlat`, `osnoise` and `hwlat` are all present. The backing kernel config confirms this is not
an accident of the running instance: `CONFIG_TIMERLAT_TRACER=y`, `CONFIG_OSNOISE_TRACER=y` and
`CONFIG_HWLAT_TRACER=y` are all set (`tracer-config.txt`), alongside `CONFIG_TRACING=y`,
`CONFIG_FTRACE=y` and `CONFIG_PREEMPT_RT=y`. RESEARCH.md pitfall 4 (assuming the tracers are
available) resolves in the good direction on this kernel: no fallback to a cyclictest-only
attribution method is required.

The directory listing of `/sys/kernel/tracing/` (folded into `available-tracers.txt`, captured by
`nr-probe`) shows an `osnoise` directory but no separate `timerlat` directory. This is expected,
not a gap: timerlat shares its configuration knobs (`timerlat_period_us`, `stop_tracing_us`,
`print_stack`, and so on) under `/sys/kernel/tracing/osnoise/` rather than owning a directory of
its own, matching kernel documentation. `nr-recon`'s own per-tracer directory check reports this
plainly as `timerlat: absent`, which should be read as "no separate directory", not "unavailable".

## Method decision

`OUTCOME: rtla-available` (full log: `rtla-build-log.txt`).

RESEARCH.md's environment-availability table assumed `rtla` would need a from-source build against
the kernel tree at `tools/tracing/rtla`, because it has no standalone Ubuntu package (`apt-cache
policy rtla` confirms this again here: empty output). That half is still true. What has changed:
on this rig's Ubuntu 26.04.1 RT kernel line, `rtla` ships as a file inside the already-installed
`linux-tools-common` package (`dpkg -S /usr/bin/rtla` -> `linux-tools-common: /usr/bin/rtla`),
tied to the kernel's own tools tree rather than to `rt-tests`. No clone, no build dependencies,
and no build were needed.

Version: `rtla version 7.0.12`. `rtla --version` itself exits 1, because `--version` is not a
recognised top-level flag; rtla's only top-level arguments are its `osnoise`/`hwnoise`/`timerlat`
subcommands. Every invocation, including a bare `rtla`, `rtla --help`, or an invalid flag, prints
a `rtla version 7.0.12` banner before the usage text, and that banner is the correct way to read
the version. `rtla timerlat top --help` prints full usage (`-a/--auto`, `-c/--cpus`, `-s/--stack`,
`-t/--trace`, SCHED_FIFO/SCHED_DEADLINE priority flags among others), confirming the binary runs.

Consequence for plan 01-12: use `rtla timerlat top --cpus 6-11 --auto <us>` as the attribution
instrument for the PLAT-01 investigation runs, exactly as RESEARCH.md's primary recommendation
describes. `cyclictest --breaktrace --tracemark` remains the separate, literal capture the
roadmap success criterion names; both are produced, neither substitutes for the other, per
RESEARCH.md open question 1's resolution.

## What is in cyclictest --json

The committed fixture (`crates/histogram/tests/fixtures/probe-cyclictest-60s.json`, hostname
redacted, see "Redaction and host identifiers" below) is 200 lines, over the 60 line threshold
for pasting in full. Top-level keys plus one complete thread entry follow instead.

Top-level keys: `file_version`, `cmdline:`, `rt_test_version:`, `start_time`, `end_time`,
`return_code`, `sysinfo` (nested: `sysname`, `nodename`, `release`, `version`, `machine`,
`realtime`), `num_threads`, `resolution_in_ns`, `thread` (an object keyed by thread index as a
string, `"0"` through `"5"` for this 6 thread run).

One complete thread entry, `thread["0"]`:

```json
"0": {
  "histogram": {
    "2": 215519, "3": 52724, "4": 19761, "5": 1271, "6": 33, "7": 1080, "8": 4232,
    "9": 3916, "10": 1178, "11": 222, "12": 37, "13": 10, "14": 3, "15": 2, "16": 1,
    "17": 5, "18": 1, "19": 3, "20": 2
  },
  "cycles": 300000,
  "min": 2,
  "max": 20,
  "avg": 2.56,
  "cpu": 6,
  "node": 0
}
```

Per-thread `min`/`avg`/`max` are present directly on each thread object, so plan 01-04's parser
does not need to derive them from the `.hist` file. A sample count is present as `cycles` (the
loop count for that thread: 300000 for most threads at a 200 us interval over 60 seconds; two
threads show 299999/299998, one cycle short, ordinary startup/shutdown rounding, not a defect).

There is no `overflow` key anywhere in the JSON. `--json` only carries histogram bins that were
actually populated (sparse: absent bins are implicitly zero, and this run's highest observed bin
was 340 us, well under the 400 us `--histogram=400` cap, so it never overflowed). Overflow count
is only available from the `.hist` file's `# Histogram Overflows:` footer line. This directly
answers RESEARCH.md open question 3: plan 01-04's parser needs the `.hist` file for the full
per-bin distribution AND for the overflow count; `--json` supplies per-thread min/avg/max and a
sparse histogram as a convenience, but overflow handling, the exact defect the 2026-08-28 README
got wrong and D-23 corrects, must still come from the `.hist` file regardless of which output
format is otherwise preferred.

The JSON's histogram bin keys are strings (`"2"`, not `2`) and sparse; a parser that indexes them
must parse to integers and must not assume a contiguous key range.

## Isolation: isolcpus or nohz_full

From `cpu-isolation.txt`:

```
isolated: 6-11
nohz_full: 6-11
realtime: 1
```

Raw `/proc/cmdline` (root= redacted, see `kernel-cmdline.txt` for why):

```
BOOT_IMAGE=/boot/vmlinuz-7.0.0-30-realtime root=[redacted] ro quiet splash isolcpus=6-11 nohz_full=6-11 rcu_nocbs=6-11 irqaffinity=0-5,12-21 intel_idle.max_cstate=1 processor.max_cstate=1 nosoftlockup nowatchdog nmi_watchdog=0 tsc=reliable skew_tick=1 crashkernel=2G-4G:320M,4G-32G:512M,32G-64G:1024M,64G-128G:2048M,128G-:4096M
```

Cores 6-11 appear in `nohz_full`, not only in `isolcpus`. RESEARCH.md open question 4's concern
(isolcpus alone leaves the scheduler tick running at CONFIG_HZ, floated as a candidate explanation
for the roughly 10 ms spacing of the sustained overflow burst in CONTEXT.md) does not apply as
stated: the tick-stopping mechanism it depends on is engaged on these cores. This is recorded as a
resolved precondition, nohz_full is not the missing piece, rather than a resolved root cause: a
nohz_full core can still take a tick from an RCU callback, a cross-CPU wakeup from a housekeeping
core, or another timer source. It is a hypothesis for plan 01-12 to test with the literal
`--tracemark`/`timerlat` capture, not a finding on its own.

`irqaffinity=0-5,12-21` is also set on the real cmdline, routing device IRQs away from cores 6-11
entirely. Neither PROJECT.md nor the plan's own `<interfaces>` block mentioned this parameter,
because the live-USB `RIG.txt` capture predates the installed cmdline entirely. Recorded here as a
correction: IRQ affinity exclusion for the isolated cores is already configured on the installed
system, not merely `isolcpus`.

## rt-tests version and flag support

Installed: `rt-tests 2.9-1ubuntu1` (`dpkg -l rt-tests`), reporting itself as `cyclictest V 2.80`.
This corrects RESEARCH.md's assumption log, which expected `2.5-1` (noble) or `2.2-1` (jammy); the
actual Ubuntu 26.04.1 archive ships a newer `2.9` series build with a `2.80` internal cyclictest
version string.

`--json`, `--histofall`, `--histogram`, `--histfile`, `--breaktrace` and `--tracemark` are all
present in `cyclictest --help` on this build (`package-versions.txt`). All five flags RESEARCH.md
needed confirmed are available; no fallback capture strategy is required.

## Column counts

The plain histogram (`-h`, `--histogram=400`) probe has 7 whitespace-separated fields per data
line (`awk 'NR==2{print NF}'`): 1 bin-index column plus 6 thread columns. The histofall (`-H`,
`--histofall=400`) probe has 8: the same 7 plus one summary column on the right. This confirms the
rule already recorded in `crates/histogram/tests/fixtures/README.md`. Both files begin with the
literal `# Histogram` line.

Refinement the existing README does not cover, measured directly on both committed fixtures by
counting fields on each footer line:

| Footer line | -h field count | -H field count |
|---|---|---|
| `# Min Latencies:` | 6 | 6 |
| `# Avg Latencies:` | 6 | 6 |
| `# Max Latencies:` | 6 | 7 |
| `# Histogram Overflows:` | 6 | 7 |

`--histofall` adds its extra column to the per-bin histogram body, to `# Max Latencies:`, and to
`# Histogram Overflows:`, but not to `# Min Latencies:` or `# Avg Latencies:`, which stay
per-thread-only in both formats. A parser that assumes a uniform "+1" column count across every
footer line, not only the histogram body, will misparse the Min/Avg lines of a `-H` capture. Plan
01-04 should special-case these two lines rather than deriving their expected width from the
thread count plus a constant.

## Redaction and host identifiers

Per the threat model's T-1-06 mitigation, every probe file and this document were grepped for the
operator's unix username (`whoami`), the machine's hostname (`hostname`), and any
MAC-address-shaped string, using the same pattern shape as the plan's own verification block:
`grep -rniE "$(whoami)|$(hostname)|([0-9a-f]{2}:){5}[0-9a-f]{2}"`. The literal values are not
spelled out in this document on purpose, so that this section does not itself trip the same check
it describes.

Two files carried the real hostname and needed redaction, both replaced with the literal token
`[redacted]`:

- `crates/histogram/tests/fixtures/probe-cyclictest-60s.json`: `cyclictest --json` embeds
  `uname()` info in `sysinfo.nodename`. This is a genuine tool-output field the plan's threat model
  table did not anticipate (its file list named only the sysfs/interrupts probes and this
  document); the mitigation's own instruction to "grep the probe files" generally, not only the
  three named ones, is what catches it. Only `nodename` was touched; `sysname`, `release`,
  `version` and `machine` are untouched, since kernel/OS version strings are not host identifiers
  and RIG.txt already publishes equivalents.
- `docs/rig/recon-2026-08-31/cpu-isolation.txt` and `rt-tuning-state.txt`: both capture raw
  `uname -a` or `journalctl` output whose lines carry the hostname as a field. Same redaction,
  same rationale, applied for the same reason the kernel cmdline's `root=` value is redacted:
  this repository is public and the hostname carries no reproduction value the already-published
  "Dell Precision 3591" (PROJECT.md, RIG.txt) does not already carry.

No MAC address pattern and no other hostname or username occurrence was found in
`probe-cyclictest-h-60s.hist`, `probe-cyclictest-histofall-60s.hist`, `probe-proc-interrupts.txt`,
`probe-sysfs-tuning.txt`, or the remaining recon `.txt` files. `/proc/interrupts` device labels
(`e1000e`, `iwlwifi`, `nvme`, and similar) are hardware/driver names, not host identifiers, and
are the same class of information RIG.txt already publishes, per the threat model's own carve-out.

## The rt-tuning.service contradiction

`rt-tuning.service` reports `LoadState=loaded`, `ActiveState=active`, `SubState=exited`,
`UnitFileState=enabled` (`rt-tuning-state.txt`), meaning systemd considers it to have run
successfully at boot. Its `ExecStart` is `/usr/local/sbin/rt-tuning.sh`, a two-step oneshot
script:

```sh
for g in /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor; do
  echo performance > "$g" 2>/dev/null || true
done
echo 1 > /sys/devices/system/cpu/intel_pstate/no_turbo 2>/dev/null || true
```

Yet at recon time, roughly five and a half hours after boot, every one of the 22 CPUs reads
`scaling_governor = powersave` (`nr-recon`, and independently confirmed in the `nr-probe` sysfs
snapshot), the untuned, screening-era value, not the `performance` value the firmware-latency
screening required to reach its sub-10 us floor.

Root cause, established from read-only evidence only (`journalctl -u rt-tuning.service -b`,
`journalctl -b` broadly, `systemctl show power-profiles-daemon.service`, `uptime -s`, and direct
cpufreq/EPP/ACPI sysfs reads, all in `rt-tuning-state.txt`):

- The machine booted at `2026-08-30 23:58:51`.
- `rt-tuning.service` started and finished at `23:59:10`; its journal for this boot is exactly
  two lines, `Starting...` immediately followed by `Finished...`, no errors.
- `gdm.service` started at `23:59:07`. GDM's own login-screen session runs a full GNOME Shell
  instance (`gnome-shell --mode=gdm`) with `gsd-power`, the GNOME Settings Daemon power plugin,
  attached to it, even before any real user has logged in.
- At `23:59:09`, `gsd-power` (running as the `gdm-greeter` system account, uid 60578) requested
  the `org.freedesktop.UPower.PowerProfiles` D-Bus name, which systemd D-Bus-activates as
  `power-profiles-daemon.service`.
- `power-profiles-daemon.service`'s `ActiveEnterTimestamp` is `23:59:10`, the same wall-clock
  second as `rt-tuning.service`'s run.
- `power-profiles-daemon` persists its active profile in `/var/lib/power-profiles-daemon/state.ini`
  and currently reports (`powerprofilesctl get`) the `performance` profile.

At the same moment, every CPU reads `scaling_governor=powersave` AND
`energy_performance_preference=performance` simultaneously (`energy_performance_available_preferences`
lists `default performance balance_performance balance_power power`, a separate EPP hint distinct
from the coarse governor). Intel HWP is active on this driver
(`/sys/devices/system/cpu/intel_pstate/status = active`), and the ACPI-level
`/sys/firmware/acpi/platform_profile` also currently reads `performance`
(choices: `cool quiet balanced performance`), a separate lever `power-profiles-daemon` also
drives. Taken together, this is strong evidence, though inferred from the observed state and
timing rather than read directly from `power-profiles-daemon`'s source, that on this HWP-capable
Meteor Lake backend, `power-profiles-daemon`'s notion of a "performance" profile is expressed as
`scaling_governor=powersave` (let HWP scale autonomously) plus `energy_performance_preference=performance`
(bias that autonomous scaling toward high clocks), never as the legacy
`scaling_governor=performance` override `rt-tuning.sh` writes.

In short: `rt-tuning.service` and `power-profiles-daemon` both act on the CPU governor within the
same second at every boot, because GDM's greeter alone, independent of any real login, is enough
to D-Bus-activate `power-profiles-daemon`. Whichever runs last wins, and because
`power-profiles-daemon`'s idea of "performance" never touches `scaling_governor` on this backend,
the net effect after both have run is always `scaling_governor=powersave`, regardless of
`rt-tuning.service`'s own successful, error-free write. The service is not failing silently on a
bad write; it is winning a race it does not know it is in, against a second tool with a different
definition of "performance" for the same sysfs knob.

This does not extend to `no_turbo`: `power-profiles-daemon` does not manage
`/sys/devices/system/cpu/intel_pstate/no_turbo` at all, so `rt-tuning.sh`'s second write is
uncontested and persists (`no_turbo` currently reads `1`, consistent with `cpu6`'s
`cpuinfo_max_freq` being the 2300000 kHz P-core base clock rather than a turbo bin).

This also corrects an assumption carried into this recon session: `/sys/devices/system/cpu/intel_pstate/`
(the standard path, matching the plan's own task 3 snippet and kernel documentation) is present
and populated on this rig (`no_turbo`, `status`, `max_perf_pct`, `min_perf_pct`,
`hwp_dynamic_boost`), and reading it directly and unprivileged returns `no_turbo=1`,
`status=active`. It is only the other path, `/sys/devices/system/intel_pstate/` with no `cpu/`
segment, the one both `nr-recon` and `nr-probe` query, that does not exist on this kernel. Both
fixed root scripts therefore report `intel_pstate.no_turbo=unavailable`, which is a false negative
from querying the wrong sysfs path, not a genuine absence of the file. The committed
`crates/capture/tests/fixtures/probe-sysfs-tuning.txt` fixture carries this false `unavailable`
verbatim, because it is real, byte-identical tool output, and this document exists precisely to
record and explain a discrepancy like this rather than to quietly edit it away.

Consequence for `nr-capture`'s D-06 preconditions: a `governor-is-performance` precondition must
read `/sys/devices/system/cpu/cpu*/cpufreq/scaling_governor` directly, as this recon did, rather
than trusting `rt-tuning.service`'s `ActiveState`; a service reporting healthy tells you it ran,
not that its effect survived. If such a precondition also wants to report `no_turbo`, it must
query `/sys/devices/system/cpu/intel_pstate/no_turbo`, not `/sys/devices/system/intel_pstate/no_turbo`.
A measurement run under the current boot configuration would correctly refuse a `governor-is-performance`
precondition right now, because the rig is not actually tuned, regardless of what the service
claims. Whether and how to close the race (masking `power-profiles-daemon.service`, reordering
`rt-tuning.service` to run after the graphical session settles, or something else) is a decision
for whichever plan owns the harness's tuning story; this plan only records the mechanism and
leaves rig state untouched, per its own instructions.

## Downstream implication for plan 01-05

`cpuidle` on every CPU exposes exactly two states, `POLL` and `C1E`, both `disable=0`
(`nr-recon`, all 22 CPUs; `nr-probe`'s sysfs snapshot confirms the same for cpu0:
`cstate.POLL=disable:0`, `cstate.C1E=disable:0`). Neither `C6` nor `C10` exists as a `cpuidle`
state directory at all, because `intel_idle.max_cstate=1` on the kernel cmdline caps the driver at
registering only the first two idle states; the deeper states are never registered, not merely
disabled.

Plan 01-05's `deep-cstates-disabled` precondition (`nr-capture`) cannot read a `disable` flag for
`C6`/`C10`, because no such sysfs file exists to read. A check that looks for
`/sys/devices/system/cpu/cpu0/cpuidle/state{2,3}/disable == 1` and treats a missing file as a
failure would incorrectly refuse every run on this rig. The precondition must instead treat
"state directory absent" as a pass for that state, since deep C-states being unreachable is the
actual guarantee the precondition wants, and only treat a present deep C-state directory with
`disable=0` as a violation.
