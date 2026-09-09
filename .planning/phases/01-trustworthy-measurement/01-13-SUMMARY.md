---
status: PASS
agent: main-thread
phase: 01-trustworthy-measurement
plan: 13

subsystem: measurement
tags: [plat03, headline, cyclictest, verdict, documented-limitation, unattributed, bench-05, bench-06, D-28]

requires:
  - phase: 01-trustworthy-measurement
    provides: "the measurement protocol and measurement mode (01-09, 01-11), nr-run-measurement
      and the sudoers grant (01-22), the D-28 admission gate and its instrument-class rule
      (01-24, a646407), the widened TracersQuiescent (01-25), firmware re-derivation in strict
      verify (01-26), the pushed-stamp harness identity (01-27, D-29), the PLAT-01 outcome
      (01-12), the rtla hwnoise screens (01-23), the scoped rig collection script (3dd2de5)"
provides:
  - "measurements/2026-09-08-precision3591-headline: the PLAT-03 headline capture, four hours,
    clean, admitted to the series"
  - "docs/rig/plat03-scheduling-latency-verdict.md: the PLAT-03 verdict against the 30 us gate,
    with no arithmetic between the two instruments"
  - "the first run in the repository with excluded_from_series false at headline class, which is
    the seed the weekly series needs"
affects: [01-14, 01-15]
---

# Phase 1 Plan 13: the PLAT-03 headline figure

## Outcome

**Verdict: documented limitation.** Observed maximum 81 us against the 30 us gate.

ROADMAP criterion 3 offers three closes and this takes the third: the residual is published as
an explicitly unattributed limitation, with the run, its raw capture and the reason attribution
failed committed beside it.

`measurements/2026-09-08-precision3591-headline`, four hours on cpus 6-11 with no login session,
cyclictest alone.

| figure | value |
|---|---:|
| maximum | 81 us |
| p50 | 2 us |
| p95 | 4 us |
| p99 | 8 us |
| p99.9 | 10 us |
| p99.99 | 11 us |
| jitter (p99 minus p50) | 6 us |
| total samples | 431999988 |
| binned samples | 431999988 |
| overflow samples | 0 |
| `samples_at_or_above(30)` | 283 (0.0000655%) |

Per-thread maxima, and the count at or above the gate on each:

| thread | cpu | max | at or above 30 us |
|---|---|---:|---:|
| 0 | 6 | 34 us | 251 |
| 1 | 7 | 70 us | 14 |
| 2 | 8 | 62 us | 11 |
| 3 | 9 | 81 us | 7 |
| 4 | 10 | 21 us | 0 |
| 5 | 11 | 22 us | 0 |

Contamination verdict `Clean` against provisional thresholds. `MSR_SMI_COUNT` delta 0 on each of
cpus 6, 7, 8, 9, 10 and 11. All 15 preconditions `pass`. `excluded_from_series` false with no
exclusions on any of the eight admission evidence sources. Requested duration 14400 s against a
measured `elapsed_seconds` of 14400.057081594, 0.0004 percent over.

## What was found along the way

**The D-28 admission split works on real hardware.** This is the first headline-class run taken
since 01-24, and the gate admitted it while excluding all three post-fix PLAT-01 investigation
runs by instrument class. Admission was decided from eight sources every one of which is upstream
of the measured latency; the contamination verdict was computed, recorded and rendered, and
decided nothing. That was the whole claim of the replan and it now has a hardware demonstration
rather than a unit test.

**The tail has structure, and none of it is attributed.** 701 samples fall between 25 and 34 us
and every one is on cpu6; the population stops dead at 34 and cpu6 produced nothing above it in
four hours. There are no samples at all between 35 and 41 us on any thread. The 24 samples from
42 to 81 us are on cpu7, cpu8 and cpu9 only. cpu10 and cpu11 never exceeded 22 us. The
`/proc/interrupts` deltas are correspondingly uneven: 4158 irqs and 87 rescheduling IPIs on cpu6
against 56 and 9 on cpu9. The coincidence between cpu6's interrupt load and cpu6's exclusive
ownership of the 25 to 34 us population is one run with no trace behind it, and it is written
down as a lead for a future traced investigation rather than as a cause.

**Why attribution failed is specific, not a shrug.** A headline-series run is taken with tracing
quiescent under the two-instrument rule, so no ftrace evidence exists for this capture by design.
The traced cycles that do exist used break limits of 200 us and 3000 us, both derived to catch
the 3.8 ms phenomenon, and both sit far above every excursion in this run: the PLAT-01
investigation could not have captured this population even had it occurred during it. Naming a
cause needs a traced run designed around 25 to 81 us, and nobody has taken one. That is now the
strongest candidate for a Phase 1 follow-on or a Phase 6 item.

**The pre-run guard earned its place immediately.** PLAT-01 cycle 3 left nine ftrace events armed
with `tracing_on` at 1. `scripts/nr-plat03-headline.sh` carries the inverse of the investigation
launchers' guard and refuses when tracing is armed, because a headline-series run measured through
a live tracer reports an inflated maximum and `TracersQuiescent` would have failed four hours in
rather than at launch. It read the state through `nr-measure-mode status` rather than tracefs
directly, which is the fix from 2026-09-07 that cost ninety minutes to learn.

**Both audit properties are still where the audit left them.** `observed_max_us < input.gate_us`
at `crates/metrics/src/report.rs:106` is still the strict boundary ece44b0 corrected from `<=`,
and the only `saturating_sub` in code is the jitter line; the second grep hit is inside the doc
comment that explains why the subtraction was removed. Checked before the verdict was written,
because both are easy to reopen in a refactor.

## Deviations

1. **p99.99 is not in the generated `REPORT.md`.** Task 2's acceptance criterion asks for a
   percentile table of p50, p95, p99, p99.9, p99.99 and the maximum with "every value matching
   the headline run's `REPORT.md`", but the report prints only four of them. p99.99 was computed
   from the committed `cyclictest.json` under the same convention the harness uses
   (`hdrhistogram` at 3 significant figures, `value_at_quantile`, overflows included), and the
   computation was validated by reproducing all four published values exactly before being
   trusted for the fifth. The verdict document says where the value came from in place rather
   than presenting it as if the harness printed it. Approved at the task 3 checkpoint.

2. **The runbook in task 1 is stale in two places and was not followed literally.** It says
   `git pull` on the rig, which has no git and is an rsync mirror (D-29, plan 01-27); source
   reaches the rig through `scripts/nr-push-to-rig.sh` instead. It also predates
   `scripts/nr-collect-from-rig.sh`, which the same task's own collection step already uses. The
   capture itself followed `docs/measurement-protocol.md` exactly.

3. **The dry run reports one failing precondition by construction.** `NoActiveSshSessions`
   observes 1 while the operator is connected to read the result. The other fourteen passed, and
   the real run recorded all fifteen at `pass`. No check was weakened to make anything pass.

4. **One STATE.md line changed beyond the two the plan named.** The plan asked for the
   contaminated-baseline blocker and `PROJECT.md:155`. The adjacent blocker, "PROJECT.md
   Constraints still says software timestamping only", became false once that row was corrected,
   since it was the last copy the 2026-08-28 correction had not reached. Marked RESOLVED using
   the section's own existing vocabulary rather than left standing as a known-false entry.
   Flagged at the checkpoint and approved.

5. **`chmod go-w` is needed after every rebuild on the rig.** `cargo build` recreates the binary
   at mode 775 and `nr-run-measurement` refuses to exec a group-writable executable. Already
   recorded in `deferred-items.md` under 01-15; it cost one launch attempt here.

## Self-Check: PASSED

- `docs/rig/plat03-scheduling-latency-verdict.md` exists, ASCII only, sentence case headings, and
  line 3 is exactly `Verdict: documented limitation`
- `Observed maximum:` and `Samples at or above 30 us:` lines present with count, total and
  percentage; both boundaries stated as agreeing (`>=` for the count, strict `<` in the renderer)
- `grep -ci 'kernel contribution'` and `grep -ci 'worst-case bound'` both report 0
- No arithmetic anywhere combines a cyclictest figure with a firmware figure; the firmware section
  states that combining them would be invalid, not that it was declined
- The firmware section names the instrument, its run directory, its observed cpus, its maximum
  with the population, its per-cpu exposure, its conditions and the per-cpu SMI delta, and gives
  the reason the idle arm is the one cited
- The attribution section matches `plat01-stall-investigation.md`'s `Outcome: not-reproduced`,
  states that non-reproduction adds exposure without resolving the cause or bounding recurrence,
  and calls everything unexplained `unattributed`
- All six per-thread maxima are listed individually; the overflow convention and the zero overflow
  count are both stated
- `PROJECT.md:155` reads `- Corrected 2026-08-28` and names the wired NIC's PTP hardware clock;
  the `- Pending` form is gone
- STATE.md no longer describes the baseline as not publishable
- `nrmeasure verify --strict --check-index` exits 0 with 0 problems, 17 of 19 re-derived, the
  headline run among them and the 2026-08-28 reconstructed screen the only exception
- The plan's own automated verify for tasks 1, 2 and 3 all exit 0
- A human reviewed the figure against its capture at the task 3 checkpoint and approved it
