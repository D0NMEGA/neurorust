# Measurement protocol

## What this document is

This document states the system state a PREEMPT_RT machine must be in before a latency
figure taken on it is publishable, and the exact commands that produce a run. `nrmeasure
run` asserts every condition listed under "Required system state" before it executes
anything, and refuses to run the moment one is violated, naming every offending check by
name with its observed and expected values. The harness never changes the machine's state
on the operator's behalf: every row below is something the operator brings the machine to
before invoking `nrmeasure run`, not something the tool sets for them (D-06).

## Why it exists

On 2026-08-28, a 10 minute cyclictest run on this reference rig's isolated cores
(`isolcpus=6-11`) produced a 2 us median and a 3.8 ms maximum in the same run. Neither
number is publishable, because the run was contaminated. SSH commands (`ps -L`, `tmux
capture-pane`, `scp`) were executed against the machine during the measurement, each
spawning a process, taking a network interrupt, and triggering a TLB shootdown
interprocessor interrupt that broadcasts to every isolated core. By the end of the run,
`/proc/interrupts` showed CAL (function-call interrupt) counts of roughly 137,000 on CPUs
6 to 11, and a GNOME desktop session was active with a user typing into it throughout. See
`measurements/2026-08-28-precision3591/README.md` for the full account.

This protocol exists to make that specific failure structurally impossible, not to guard
against contamination in the abstract. Every row in "Required system state" below traces
back to a real way an unprotected run went wrong, or plausibly could: an open login
session, a background package-manager timer, a laptop's own charge controller, a tracer
left armed from an earlier investigation. Naming the exact numbers above is what makes
this protocol a record of a specific failure rather than a generic checklist.

A second gap surfaced on 2026-09-01: partway through a calibration run, an operator logged
in at the rig's own keyboard and display, at the physical console (a text-mode session on
`tty1`), and got a shell. That login started a session on the machine under measurement,
triggered `update-motd.d`'s own apt and fwupd queries ("2 updates can be applied
immediately", "2 devices have a firmware upgrade available"), and is exactly the class of
disturbance this protocol exists to exclude. None of the checks above caught it:
`NoActiveSshSessions` counts established SSH connections only, `NoGraphicalSession` counts
x11/wayland sessions only, and `DisplayManagerInactive` inspects the display-manager unit
only. A local console login falls through all three. Sitting down at the machine's own
keyboard is the most direct way to perturb it, so a local console login counts as activity
just as much as a remote one; that is what `NoActiveLoginSessions` (below) now asserts.

## Required system state

Every row below is one precondition check `nrmeasure run` evaluates, in this order, before
it runs any measurement tool. Every check is recorded in the run's manifest
whether it passes or fails (D-06); the run refuses if any of them fails, or, for a
`headline-series` run specifically, if any of them could not even be evaluated. The
"Required state" column is each check's own literal expected value, taken directly from
`crates/capture/src/preconditions.rs`, not paraphrased.

| Check | Required state | How to set it | How the harness observes it |
|---|---|---|---|
| `NoActiveSshSessions` | `0` | Close every SSH session and every terminal connected over SSH before the run starts. | Counts established TCP connections to local port 22 (`ss -Htn state established ( sport = :22 )`). |
| `SystemdDefaultTargetIsMultiUser` | `multi-user.target` | `sudo systemctl isolate multi-user.target` | `systemctl get-default` |
| `DisplayManagerInactive` | `inactive` | `sudo systemctl stop gdm.service` (or `sddm.service` / `lightdm.service`, whichever this machine runs). | `systemctl show <unit> --property=ActiveState` for `gdm.service`, `sddm.service` and `lightdm.service`; every one of these present on the machine must report `inactive`. |
| `NoGraphicalSession` | `0` | Log out of any desktop session. Isolating `multi-user.target` (the row above) normally satisfies this too. | Lists sessions with `loginctl list-sessions`, then `loginctl show-session <id> -p Type --value` for each, counting sessions of type `x11` or `wayland`. |
| `NoActiveLoginSessions` | `no local console login session` | Log out of any session opened at the machine's own keyboard and display (for example, a text-mode login at `tty1`), and do not log in at the console while a run is in progress. | Lists sessions with `loginctl list-sessions`, then `loginctl show-session <id>` for each, naming any session where `Type=tty`, `Class=user` and `Seat=seat0`. An SSH session's allocated pty also reports `Type=tty`, but carries no seat, so one SSH connection is never counted here as well as by `NoActiveSshSessions`. |
| `GovernorIsPerformanceOnAllCpus` | `performance on all CPUs` | See "The governor operating point" below. | Reads `/sys/devices/system/cpu/cpu{N}/cpufreq/scaling_governor` directly, per CPU, for every CPU with a `cpufreq` directory. Never trusts `rt-tuning.service`'s reported state. |
| `NoTurboEnabled` | `1` | `echo 1 \| sudo tee /sys/devices/system/cpu/intel_pstate/no_turbo` | Reads `/sys/devices/system/cpu/intel_pstate/no_turbo` (the path with a `cpu/` segment; the sibling path with no `cpu/` segment does not exist on this kernel and always reads as absent, a false negative `docs/rig/recon-2026-08-31/FINDINGS.md` documents). |
| `DeepCstatesDisabled` | `C6 disabled, C10 disabled` | Boot with `intel_idle.max_cstate=1`, which prevents C6/C10 from ever registering (satisfies this check by absence), or `echo 1` to each state's own `disable` file if they are registered. | Probes `/sys/devices/system/cpu/cpu0/cpuidle/state{N}/{name,disable}` for increasing `N`. A present `C6` or `C10` must read `disable=1`; an absent one passes. |
| `IsolcpusCoversTargetCpus` | the run's own `--cpus` range (e.g. `6-11`) | Set `isolcpus=<range>` plus `nohz_full=<range> rcu_nocbs=<range>`, and steer other IRQs away with `irqaffinity=<the rest>`, on the kernel command line, covering whole physical cores rather than half of a hyperthread pair. | Reads `/sys/devices/system/cpu/isolated` and checks every CPU in `--cpus` is present. |
| `KernelIsRealtime` | `1` | Boot a PREEMPT_RT kernel, for example `sudo apt install ubuntu-realtime` on Ubuntu 26.04 LTS. | Reads `/sys/kernel/realtime`. |
| `RtTuningServiceActive` | `active` | `sudo systemctl enable --now rt-tuning.service` | `systemctl show rt-tuning.service --property=ActiveState`. This confirms the service ran, not that its writes survived; see "The governor operating point". |
| `OnAcPower` | `AC=1` | Connect AC power. A charging or discharging battery is a documented source of firmware interrupts (`measurements/2026-08-28-precision3591/README.md`'s caveats). | Reads the first populated `online` file among `/sys/class/power_supply/{AC,AC0,ADP0,ADP1}/online`. |
| `ThermalHeadroomAtStart` | `package temp <= 70 C`, and not applicable to a `screen` run | Let the machine idle until every thermal zone cools to 70 C or below before starting the run. A `screen` run is exempt: a firmware screen saturates the machine on purpose, so the check is recorded as not applicable, carrying its real observed temperature, rather than refusing the run. | Reads every `/sys/class/thermal/thermal_zone{N}/temp` and takes the maximum across zones, which is not necessarily the package sensor. |
| `NoPackageManagerActivity` | `no apt, dpkg, unattended-upgrade or snapd process` | `sudo systemctl stop unattended-upgrades.service`, and let any `apt`/`dpkg`/`snapd` operation already in progress finish. | Scans `/proc/*/comm` for exactly these four process names. |
| `TracersQuiescent` | `nop` for a `headline-series` run; not applicable for an `investigation` run | `echo nop \| sudo tee /sys/kernel/tracing/current_tracer` | Reads `/sys/kernel/tracing/current_tracer`. |

## The governor operating point

`GovernorIsPerformanceOnAllCpus` (the row above) is the most consequential row in this
table and the one most likely to surprise an operator, so it gets its own section.

The 2026-08-28 firmware-latency screening reached its sub-10 us floor with
`scaling_governor=performance` written literally to every CPU. That literal value is what
`nrmeasure run` checks, per CPU, by reading `/sys/devices/system/cpu/cpu{N}/cpufreq/
scaling_governor` directly, never by trusting a service's reported state.

On the installed reference rig, this exact value is not reachable through the desktop's
own power setting. GNOME's power-profiles-daemon owns the CPU governor on this machine's
Meteor Lake HWP (hardware P-state) backend, and its own idea of a "Performance" profile is
expressed as `energy_performance_preference=performance` with `scaling_governor` left at
`powersave`, never as the literal `scaling_governor=performance` override this protocol
requires. Switching the GNOME power mode setting to Performance changes the
`energy_performance_preference` hint and leaves every CPU's governor at `powersave`; this
was confirmed by hand on 2026-08-31 (`docs/rig/recon-2026-08-31/FINDINGS.md`, "The
rt-tuning.service contradiction").

The rig's own `rt-tuning.service` writes the literal `performance` value at boot, but
power-profiles-daemon is D-Bus-activated by GDM's own login-greeter session within the
same boot second, and whichever of the two writes last wins. Because
power-profiles-daemon's notion of performance never touches `scaling_governor` on this
backend, the practical effect is that every boot ends with `scaling_governor=powersave`
regardless of `rt-tuning.service` reporting itself healthy (`ActiveState=active`, the
`RtTuningServiceActive` row above). A service reporting healthy tells you it ran, not that
its effect survived; that is exactly why `GovernorIsPerformanceOnAllCpus` reads sysfs
directly rather than trusting `RtTuningServiceActive`.

To reach the literal `performance` operating point for a run, remove power-profiles-daemon
from contention for the run's duration rather than fighting it:

```
sudo systemctl mask --now power-profiles-daemon.service
sudo systemctl restart rt-tuning.service
```

Verify every CPU actually took the value, rather than trusting either service's own
reported state:

```
cat /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor | sort -u
```

A single line reading `performance` confirms the run can proceed. Any other output (most
likely a lingering `powersave`) means `GovernorIsPerformanceOnAllCpus` will correctly
refuse the run, and masking did not take hold; check `systemctl status
power-profiles-daemon.service` for `masked`.

Restore normal desktop behaviour once the run is finished:

```
sudo systemctl unmask power-profiles-daemon.service
sudo systemctl start power-profiles-daemon.service
```

The manifest's `tuning.per_cpu_governor` field records the literal `scaling_governor`
value this protocol requires, alongside `energy_performance_preference`, since this finding
shows that value is a second, independent lever power-profiles-daemon drives on this HWP
backend: on a machine like this one, the governor value alone does not fully describe the
tuning state a figure was taken under. EPP is recorded next to the governor, never as a
substitute for it; `GovernorIsPerformanceOnAllCpus` still requires the literal
`scaling_governor=performance` value regardless of what EPP reads. An operator who wants to
check the live value directly, without waiting for a manifest, still reads it by hand:

```
cat /sys/devices/system/cpu/cpu*/cpufreq/energy_performance_preference | sort -u
```

Plan 01-11's calibration pair and plan 01-13's headline capture both depend on this
operating point being reached and verified before they run; neither can produce a
publishable figure otherwise. The D-18 firmware-floor re-run (comparing against the
2026-08-28 screening's 22 to 29 us under-load figures) happens on this installed RT system
rather than the original live-USB session, so it is also subject to this same governor race
and must reach and verify the same operating point first.

## Running a measurement

Build the harness once: `cargo build -p nr-cli --release`. Every command below assumes the
resulting `nrmeasure` binary (`./target/release/nrmeasure`) is invoked as `nrmeasure`,
either on your `PATH` or by its full path; substitute your own `<rig-slug>` (a short label
you choose for the machine, for example `precision3591` for the reference rig) and your
own `--cpus`/`--main-cpus` (see "Reproducing on different hardware" below).

1. Check the environment without measuring:

   ```
   nrmeasure run --rig-slug <rig-slug> --class headline --cpus 6-11 --main-cpus 0,1 \
     --duration 14400 --dry-run
   ```

   This runs every precondition check first. If any check fails, `nrmeasure run` refuses
   immediately, prints every failing check by name with its observed and expected values,
   and exits 2, exactly as a real run would; nothing is written either way. Fix everything
   it reports before continuing. If every check passes, `--dry-run` prints a one-line
   confirmation and the exact `cyclictest` (and `hwlatdetect`, if `--with-hwlatdetect` is
   given) command line the real run would execute, then exits 0 without running either tool
   or writing anything.

2. Disconnect in a way that does not depend on your own shell staying open.
   `NoActiveSshSessions` is asserted the moment `nrmeasure run` starts, so the process about
   to check it must not itself still be running inside the SSH session you are about to
   close. The weekly and soak cadence (D-10) is designed to make this a non-issue
   permanently: once plan 01-14's systemd oneshot unit and timer are installed
   (`deploy/systemd`), a run launches with no login session in the first place, which is
   what makes "no remote shell activity during a run" true by construction (D-05) rather
   than by an operator remembering to log out. Until that unit is installed on your
   machine, detach the run from your shell before disconnecting, for example with
   `systemd-run --collect --property=Type=oneshot -- nrmeasure run ...`, then close every
   SSH session and terminal.

3. Take the run. The two Phase 1 standard invocations:

   Weekly regression run (D-10, roughly 1 hour, with `hwlatdetect` per D-09 to catch
   firmware or BIOS drift):

   ```
   nrmeasure run --rig-slug <rig-slug> --class weekly --cpus 6-11 --main-cpus 0,1 \
     --duration 3600 --with-hwlatdetect
   ```

   Headline run (the publishable, tracer-free capture behind a stated performance claim,
   4 hours):

   ```
   nrmeasure run --rig-slug <rig-slug> --class headline --cpus 6-11 --main-cpus 0,1 \
     --duration 14400
   ```

4. Bring back the run directory and commit it. Regenerate the index so the new run appears
   in it:

   ```
   nrmeasure verify --write-index
   ```

   Commit the run directory (`measurements/<the new run>/`) together with the regenerated
   `measurements/INDEX.md`. Before pushing, confirm the same check CI runs also passes:

   ```
   nrmeasure verify --strict --check-index
   ```

   This must exit 0. `.github/workflows/provenance.yml` runs the identical command on every
   push and pull request and blocks the merge if it does not.

## What the contamination detector measures

The checklist above (`nrmeasure run`'s 15 preconditions) is asserted before a
measurement starts. D-15 is the complementary check taken around the measurement
itself: a `/proc/interrupts` snapshot before and after the run, diffed per isolated CPU,
plus (D-24, 2026-09-02) a set of metrics computed from the run's own cyclictest
histogram.

The `/proc/interrupts` diff alone is not a sufficient detector. Two real one-hour-class
runs, taken under identical tuning and differing only in contamination, prove this
directly:

- `measurements/2026-09-01-precision3591-calibration-clean` (3600s, no login session)
- `measurements/2026-09-02-precision3591-calibration-contaminated` (900s, SSH activity
  plus an active GDM greeter on seat0)

The two runs do not separate on the CAL/TLB/RES/device-IRQ counters at any usable
magnitude, while producing a worst-case latency 50 times higher (78 us clean vs 3856 us
contaminated).

An earlier version of this section said the contaminated run recorded FEWER counts than
the clean run. That was wrong. Summing each arm's recorded per-CPU deltas over the
isolated cores and normalising by the recorded durations gives, per run hour: CAL 6 clean
vs 48 contaminated, TLB 6 vs 24, RES 55 vs 96, device IRQ 1127 vs 844. Only device IRQs
invert, and CAL is higher on the contaminated arm even before normalising. The earlier
reading compared raw totals across a 4x duration difference (3600 s against 900 s).

The counters are still not a usable detector, for a better reason: the magnitudes are
implausible. One to two CAL IPIs per isolated CPU per hour, against roughly 137,000 on
the contaminated 2026-08-28 baseline. Whatever the snapshots are counting is not the
interference the D-15 design assumed, and two runs of unequal duration are not a
calibration set. The previously offered `irqaffinity=0-5,12-21` explanation is withdrawn:
that parameter sets a default affinity mask for device IRQs and does not suppress
CAL/TLB/RES accounting on the cores those IPIs are delivered to.

What separates the two runs instead is the tail of the latency distribution, not its
bulk: p50, p95, p99 and p99.9 are nearly identical between them, but the global maximum
diverges sharply from p99 (a ratio of 8.7 clean vs 428.4 contaminated), and every
isolated thread's own maximum lands within a narrow band of every other thread's (a
spread of 3.6% contaminated vs 76.9% clean, where independent per-core noise scatters
the six threads' maxima instead of clustering them). D-24 computes three metrics from
the histogram every run already produces:

- `tail_excursion_ratio`: the global maximum divided by p99.
- `thread_max_spread`: `(max(per-thread max) - min(per-thread max)) / max(per-thread
  max)`. Low spread combined with a high excursion ratio is the global-stall signature;
  low spread alone is not evidence of contamination, since a clean run with six small,
  near-identical maxima also scores low.
- `overflow_rate_per_s`: overflow samples divided by run duration. Coarser than the
  other two (it depends on the histogram bound, 400 us in this project), so it is
  recorded as evidence but is never the primary signal.

The `/proc/interrupts` counters are still recorded on every run: they remain useful
evidence for a different class of contamination (for example a genuinely busy IRQ
storm bleeding onto an isolated core), just not for the global-stall pattern above.
Neither signal is deleted in favour of the other.

`config/contamination-thresholds.json` ships with `tail_excursion_ratio_max` and
`thread_max_spread_min` set from exactly this one calibration pair (n=2). Two runs are
not a calibration set: their durations differ (3600s vs 900s), and the global maximum
is duration-sensitive (a longer clean run has more chances to catch a rare excursion,
so the clean arm's own 8.7 ratio is a lower bound on what a clean 4 hour headline run
might show). The shipped thresholds are therefore marked `"status": "provisional"`,
both in that file and in every manifest's `interference.thresholds_provisional` field,
and a provisional verdict is never enough on its own to admit a run to the headline
series (see "What invalidates a run" below); only a properly calibrated threshold set,
derived from many runs, may do that.

## What invalidates a run

- Any of the 15 preconditions failing: `nrmeasure run` refuses before running any tool or
  writing anything (exit code 2), naming every offending check. The week is recorded as a
  coverage gap (D-08) rather than backfilled.
- A tool (`cyclictest` or `hwlatdetect`) exiting non-zero: the run directory is still
  written in full (BENCH-06: a losing configuration is retained, never dropped), but the
  manifest marks `excluded_from_series: true` with the tool's name and exit code as the
  reason.
- A non-clean contamination verdict, or a verdict reached against provisional
  thresholds: the same treatment, excluded from the series with a reason, never omitted
  from `measurements/INDEX.md`. `config/contamination-thresholds.json` ships with
  `"status": "provisional"`, derived from exactly the D-17 calibration pair (n=2, see
  "What the contamination detector measures" above); every run is marked excluded from
  the series regardless of whether its own D-24 tail metrics score clean or
  contaminated, because a provisional threshold set is not trusted to admit a run on
  its own. Only a properly calibrated threshold set (many runs, not two) changes this.
- A tracer armed during a `headline-series` run: refused outright by `TracersQuiescent`,
  because tracer overhead inflates the very numbers being published.
- Running cyclictest and hwlatdetect concurrently: not possible through `nrmeasure run`
  itself, which always executes them in sequence (see "The two-instrument rule" below).
  Running a second measurement tool by hand alongside an `nrmeasure run` invocation has the
  same interference effect the 2026-08-28 contamination demonstrated, and is an operator
  error the harness cannot detect after the fact.

## Run classes and cadence

| Run class | Purpose |
|---|---|
| `recon` | Passive, read-only environment recon; no measurement tool runs (plan 01-02's rig recon). |
| `screen` | A one-off screening run deciding hardware viability (the 2026-08-28 firmware-latency screen). Never part of the regression series. |
| `calibration-clean` | One half of the D-17 calibration pair: a run taken under this protocol, used to derive contamination thresholds. |
| `calibration-contaminated` | The other half of the D-17 pair: deliberately reproduces the 2026-08-28 conditions (SSH activity, an active GNOME session), published as a documented example of what contamination looks like. |
| `investigation` | A tracer-instrumented capture (`instrument_class: investigation`) for root-cause work such as PLAT-01. Never feeds the headline series. |
| `headline` | The tracer-free, publishable capture behind a stated performance claim. This protocol's primary subject. |
| `weekly` | The roughly 1 hour regression-tracking run (D-10), including `hwlatdetect` (D-09) to catch firmware or BIOS drift. |
| `soak` | A longer run, monthly or on demand before a published claim. |

Cadence (D-10): a roughly 1 hour `weekly` run for the regression series, plus a longer
`soak` monthly or on demand before a published claim. A weekly 24 hour soak was rejected
because the rig is also a Windows machine; a reader deserves to know that constraint rather
than wondering why the cadence is what it is.

OSADL's public QA farm runs cyclictest for 5 hours 33 minutes twice daily, continuously
since 2011, at 1 us resolution, and publishes worst-case latency as the headline result.
This protocol is a much smaller version of the same idea: a roughly 1 hour run weekly plus
a periodic longer soak, on a single dual-boot laptop rather than a dedicated farm of
always-on machines. OSADL's twice-daily, decade-plus series is a materially stronger
evidentiary base than a single weekly sample, and this protocol does not claim parity with
it, only that the same underlying principle, that a long observed series beats one run,
applies at a much smaller scale.

## The two-instrument rule

`headline-series` runs use cyclictest alone, bracketed by `/proc/interrupts` diffing
(D-15), which adds no measurement overhead of its own. `investigation` runs may
additionally arm the `timerlat` or `osnoise` tracer (`rtla`, confirmed available and
requiring no build on this rig's kernel per `docs/rig/recon-2026-08-31/FINDINGS.md`) to
attribute a spike to a specific kernel path; both tracers run their own per-CPU kernel
threads and measurably inflate the very latency being investigated. The two are tagged
distinctly in every manifest by `instrument_class` (`headline-series` or `investigation`),
and a `headline-series` run refuses outright if a tracer is armed (`TracersQuiescent`
above). An investigation run's numbers never feed the regression series
(`metrics/latency-series.json`) or count toward a published headline figure; they answer a
different question (which code path is responsible) under different, heavier
instrumentation, and are never averaged together with a headline-series run.

## Reproducing on different hardware

A third party reproducing this protocol on different hardware changes: the CPU list
(`--cpus`, `--main-cpus`), the `isolcpus`/`nohz_full`/`rcu_nocbs`/`irqaffinity` kernel
command line values that make those CPUs actually isolated, and the rig slug
(`--rig-slug`) that identifies the machine in every run directory name and manifest.

A third party must not change, if the resulting figure is meant to be compared against
this project's own: the cyclictest sampling interval (200 us), the histogram bound (400
us), or the run duration for a given class (3600 s weekly, 14400 s headline). Changing any
of these produces a different measurement, not a directly comparable one.

A figure taken on different hardware is a different figure, full stop; this protocol does
not claim portability of the numbers, only of the method. Every manifest records exactly
which machine a run came from (`host.rig_slug`, `host.cpu_model`, `host.bios_version`, and
the rest of the D-14 environment snapshot), so a reader is never left guessing which rig
produced a given histogram.
