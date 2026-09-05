---
kind: external-audit
phase: 01-trustworthy-measurement
auditor: codex-cli 0.153.4, model gpt-6-astra, sandbox read-only
requested_by: operator
date: 2026-09-04
scope: >
  Adversarial review of the D-18 firmware-floor comparison, the PLAT-03 decomposition,
  the D-24 contamination detector, and Phase 1 methodology generally. Prompted after
  four consecutive defects surfaced while executing plan 01-11 Task 2.
verified_independently:
  - "report.rs saturating_sub and the <= gate boundary: confirmed by reading the source"
  - "counter inversion arithmetic: recomputed from the two committed calibration manifests, matches"
  - "measurement-protocol.md still states 60 C while the code states 70 C: confirmed"
status: open
---

# External audit of Phase 1, 2026-09-04

Run non-interactively with `codex exec -m gpt-6-astra --sandbox read-only`. The auditor
read the repository and `.planning/` and made no changes.

Findings are reproduced verbatim below. Line references are to the tree at commit 178c7e8.
Nothing here has been actioned yet; see the follow-up commits that cite this file.

Reproduced verbatim, so unlike the rest of this repository it contains non-ASCII
typography (curly quotes, em dashes, a degree sign). Editing an audit record to satisfy
a house style would make it a paraphrase, so it is left exactly as received.

The findings below are ranked by threat to the project’s core claim. I made no changes, ran no builds or tests, and did not access the rig.

**1. C: The proposed “kernel contribution” is mathematically invalid, including as an upper bound.**

[01-13-PLAN.md:265](/Users/d0nmega/Developer/neurorust/.planning/phases/01-trustworthy-measurement/01-13-PLAN.md:265) explicitly says subtracting the separately measured firmware floor produces “the largest the kernel’s contribution could be.” That is false.

Even granting an idealized additive model, consider a scheduling maximum of 40 µs caused entirely by kernel activity, with no firmware interruption during that event. A separate hwlat run observes 22 µs. Subtraction produces 18 µs, while the actual kernel contribution was 40 µs. The proposed upper bound has already failed, before considering the instruments’ different semantics.

An observed maximum hardware gap is neither a fixed delay charged to every wakeup nor a guaranteed minimum firmware contribution to the worst scheduling event. Sampling different CPUs, workloads and thermal states makes the inference weaker still.

This error is implemented in [report.rs:67](/Users/d0nmega/Developer/neurorust/crates/metrics/src/report.rs:67). `saturating_sub` additionally converts an independently measured hardware maximum greater than the scheduling maximum into a zero kernel contribution. The output calls the residual “kernel contribution” and associates it with a measurement run. The total remains the gate input; the accompanying attribution is unsupported.

I would remove the subtraction, the required scalar `firmware_floor_us`, and the “1 to 8 µs left for the kernel” argument. PLAT-03 should report:

- Observed scheduling maximum, per-thread and pooled distributions, sample counts, overflow counts, and counts at or above the specified gate.
- Duration, CPU placement, workload, actual operating conditions and protocol deviations.
- Independent hwlat observations, explicitly identified as hardware-gap diagnostics under their own conditions.
- Event attribution only where corresponding traces support it; otherwise, “unattributed.”

Also fix [report.rs:71](/Users/d0nmega/Developer/neurorust/crates/metrics/src/report.rs:71): `<=` currently calls exactly 30 µs “under the 30 us gate.”

Finally, [the baseline README:47](/Users/d0nmega/Developer/neurorust/measurements/2026-08-28-precision3591/README.md:47) calls 29 µs a “worst-case bound.” A maximum observed during this finite screen establishes no such bound. Use “maximum observed under the stated test conditions.”

**2. A: D-18 cannot test the carry-over assumption as designed, and its acceptance criteria protect the assumption from rejection.**

A delta column does **not inherently imply a controlled experiment**. Publishing a descriptive difference between two explicitly different configurations is defensible. Interpreting it as the effect of PREEMPT_RT, evidence of kernel independence, or validation of transferability is not.

The plan partially recognizes this. [01-11-PLAN.md:372](/Users/d0nmega/Developer/neurorust/.planning/phases/01-trustworthy-measurement/01-11-PLAN.md:372) acknowledges changing measurement conditions. But it then instructs the author not to frame the carry-over assumption as wrong; [line 401](/Users/d0nmega/Developer/neurorust/.planning/phases/01-trustworthy-measurement/01-11-PLAN.md:401) makes that restriction an acceptance criterion. That is a methodological defect: the required interpretation has been selected before the experiment.

“BIOS services SMIs” does not entail “the distribution of firmware interruptions is independent of the OS configuration.” Software changes power states, heat, device activity and the conditions that provoke firmware intervention. Kernel documentation describes firmware handling and hardware-gap detection; it does not validate this project’s transferability claim. [Linux hardware latency detector documentation](https://docs.kernel.org/trace/hwlat_detector.html).

Your four confounds are material. There are additional operating-state differences: the baseline explicitly records **charging at 84–93%** in [README.md:53](/Users/d0nmega/Developer/neurorust/measurements/2026-08-28-precision3591/README.md:53). AC-connected, fully charged operation is another condition, not a substitute for that charging state.

I would replace the intended conclusion with:

> These results compare historical screening with installed PREEMPT_RT characterization under different recorded configurations. Differences cannot be attributed to PREEMPT_RT and do not establish equivalence or transferability. The installed-system results characterize only the tested installed configuration.

Keep the historical table if useful, but label differences descriptive. The `<10 µs` result is censored; it does not supply an exact number for subtraction. An untuned RT arm that was not measured must remain “not measured,” regardless of the acceptance criterion demanding four corresponding figures.

A controlled kernel comparison does **not require erasing the installed disk or reconstructing the USB session**. Boot generic and RT kernels against the same installed root, with matched tools, services, command line, CPU placement, power policy and workload. Alternate boot order across repeated sessions; record charging, temperature, fan and power behavior. If the distro kernels differ beyond preemption configuration, describe the estimand as the difference between those kernel builds. An RT-only claim needs more closely matched kernel builds.

Treat isolation and power policy as separate experimental factors if their effects matter. Predefine an equivalence tolerance and collect enough repeated exposure to assess it. Similar maxima from one pair cannot establish equivalence.

Also decide whether you are matching **workload** or **temperature**. Temperature can be an outcome of the configuration. Changing the workload until both kernels reach the same temperature answers a different question from applying the same workload to both. The USB-versus-installed contribution remains unidentified without an additional experiment.

**3. B: The proposed affinity fix is insufficient, and “22 CPUs saturated” is not a complete hwlat measurement condition.**

`taskset -c 0-21 stress-ng --cpu 22` gives the process and inherited workers an **allowed CPU set**. It does not assign one worker to each logical CPU or restore scheduler balancing on isolated CPUs. Your premise that explicit affinity permits execution there is correct; the broad mask is insufficient to guarantee that placement. [taskset manual](https://www.man7.org/linux/man-pages/man1/taskset.1.html).

I would use explicit singleton placement: one independently pinned CPU worker for each logical CPU 0–21, with worker affinity and actual per-CPU utilization verified throughout. Preserve the stressor method and version. The runbook specifies `--cpu-method matrixprod`; omitting that changes another experimental input. These are 22 logical CPUs on 16 physical cores, as [RIG.txt:5](/Users/d0nmega/Developer/neurorust/measurements/2026-08-28-precision3591/RIG.txt:5) records.

Loading sampled CPUs is conceptually consistent with the historical full-package arm. It still does not recreate the historical operating state.

During hwlat’s interrupt-disabled polling interval, an ordinary stress worker cannot run on that same logical CPU and cause a gap merely by winning scheduler time. The detector temporarily replaces that worker’s execution. Between polling intervals, workload can affect sampler scheduling and coverage; throughout the experiment, sibling-thread activity and package power can change the physical conditions. Consequently, saturation has no universal inflation or suppression correction. [Linux hwlat implementation](https://github.com/torvalds/linux/blob/master/kernel/trace/trace_hwlat.c).

Two measurement distinctions need to appear in the report:

- hwlat detects execution gaps; those are not uniquely identified SMIs. NMI accounting and other hardware effects matter.
- Its output counts threshold-exceeding sampling records, not a census of SMI invocations. Multiple gaps can contribute to one sampling record. Under round-robin sampling, wall-clock duration is not exposure on each CPU. The baseline records a 500 ms sampling width and 1 s window. With ideal rotation across 12 logical CPUs, 600 seconds gives roughly 300 aggregate CPU-seconds of polling, approximately 25 per CPU—not 600 seconds per CPU. Verify actual mode and coverage. [Detector documentation](https://docs.kernel.org/trace/hwlat_detector.html), [implementation](https://github.com/torvalds/linux/blob/master/kernel/trace/trace_hwlat.c).

There is a revealing detail in the raw baseline: all 13 reported P-core events name **CPU 0**, in [hwlatdetect-pcore-underload-10m.txt:16](/Users/d0nmega/Developer/neurorust/measurements/2026-08-28-precision3591/hwlatdetect-pcore-underload-10m.txt:16). That does not prove only CPU 0 was sampled—the output is thresholded—but it certainly does not establish an observed 22 µs maximum specifically on runtime CPUs 6–11. Nor does the other arm’s E-core clustering establish the README’s asserted sampling-artifact explanation.

`stress-ng` is usable here. The better experiment is one with verified placement, fixed stressor behavior, sufficient thermal stabilization, continuous named-sensor telemetry, and recorded detector configuration and exposure. Add separate steady-load, load-transition and charging-state arms if investigating triggers. Housekeeping-only load with idle target CPUs is useful too, but it is a distinct workload.

There are also two concrete runbook failures:

- [01-11-SUMMARY.md:188](/Users/d0nmega/Developer/neurorust/.planning/phases/01-trustworthy-measurement/01-11-SUMMARY.md:188) gives arm 2 an 1100-second stress timeout, followed by 60 seconds of ramp, 300 seconds of cyclictest, and 900 seconds of hwlat. The load ends **at least 160 seconds before hwlat finishes**. Arm 3 repeats the same error with `800 < 60 + 300 + 600`. Supervise load lifetime through capture completion.
- The original plan’s `--duration 0` is not a hwlat-only mode. [run.rs:328](/Users/d0nmega/Developer/neurorust/crates/cli/src/cmd/run.rs:328) always executes cyclictest first. Upstream cyclictest uses zero to disable duration-based termination. Add an explicit instrument selection instead. [cyclictest source](https://kernel.googlesource.com/pub/scm/utils/rt-tests/rt-tests/%2B/3c097fc36ca620e69eb63734305b73b0c011e24a/src/cyclictest/cyclictest.c).

**4. E: The counter inversion is incorrectly reported, and the replacement classifier can exclude the failures the project needs to discover.**

[interference.rs:24](/Users/d0nmega/Developer/neurorust/crates/capture/src/interference.rs:24) says the contaminated arm had fewer CAL, TLB, RES and device-IRQ counts. The committed manifests contradict that.

I summed their recorded deltas over CPUs 6–11 and normalized using the recorded durations: [clean manifest:572](/Users/d0nmega/Developer/neurorust/measurements/2026-09-01-precision3591-calibration-clean/manifest.json:572), [contaminated manifest:572](/Users/d0nmega/Developer/neurorust/measurements/2026-09-02-precision3591-calibration-contaminated/manifest.json:572).

| Counter | Clean, 3600 s | Contaminated, 900 s | Clean / hour | Contaminated / hour |
|---|---:|---:|---:|---:|
| CAL | 6 | 12 | 6 | 48 |
| TLB | 6 | 6 | 6 | 24 |
| RES | 55 | 24 | 55 | 96 |
| Device IRQ | 1127 | 211 | 1127 | 844 |

Only aggregate device IRQs invert after normalization. CAL is higher even before normalization; TLB is equal before normalization.

This does not establish useful counter thresholds. Tiny counts can include fixed startup effects, and two unequal-duration observations are inadequate calibration. But the stated empirical reason for abandoning the counters is wrong and should be corrected throughout the code, protocol, configuration and summaries.

The proposed explanation is also unsound. `irqaffinity` supplies a default IRQ affinity mask; it is not a general mechanism for suppressing CAL/TLB/RES IPIs to isolated CPUs. A shootdown reaching a CPU “without ever registering as per-core interrupt traffic” needs evidence, not that command-line explanation. [Kernel parameter documentation](https://cdn.kernel.org/doc/html/latest/admin-guide/kernel-parameters.html).

The counters remain salvageable as diagnostics. They count particular interrupt deliveries, not their execution cost, total scheduling interference or causal responsibility for a latency spike. Rename `context_switches`: [interference.rs:17](/Users/d0nmega/Developer/neurorust/crates/capture/src/interference.rs:17) explicitly fills it with RES interrupts because the schema wanted a per-CPU quantity. That is a semantic substitution, not a context-switch measurement.

The tail classifier has a more fundamental problem. [interference.rs:548](/Users/d0nmega/Developer/neurorust/crates/capture/src/interference.rs:548) classifies a high maximum/p99 ratio combined with similar thread maxima as contamination. A real global kernel regression or firmware stall can produce exactly that signature. Excluding it because its latency looks bad conditions acceptance on the outcome being measured. Conversely, severe interference affecting one thread can receive `Clean`; sustained degradation can raise p99 and reduce the ratio. Similar independent thread maxima do not establish simultaneous events.

The current provisional exclusion in [run.rs:806](/Users/d0nmega/Developer/neurorust/crates/cli/src/cmd/run.rs:806) prevents all such runs from entering the accepted series. That protection should remain until the methodology changes. It also means the present configuration cannot produce the accepted headline series envisaged downstream.

I would separate **protocol adherence**, **capture validity**, and **latency anomaly**. An unexplained bad tail remains evidence against a latency claim. It may trigger investigation; it cannot establish its own disqualification.

For a defensible detector, obtain independent labels for specific violations, repeated clean and perturbed sessions across boots and durations, randomized perturbation order, frozen thresholds, and held-out evaluation with false-positive and false-negative uncertainty. The September 3 positive confirmation does not validate false-positive behavior. More samples alone will not repair circular labeling.

Finally, [interference.rs:596](/Users/d0nmega/Developer/neurorust/crates/capture/src/interference.rs:596) couples validation status to algorithm: provisional uses tails; calibrated uses counters. Algorithm selection and validation status need separate fields.

**5. D: The Screen exemption repairs a real design conflict, but 70 °C is an operating policy—not an experimentally established boundary—and the promised thermal record is incorrect.**

Your change in [preconditions.rs:550](/Users/d0nmega/Developer/neurorust/crates/capture/src/preconditions.rs:550) is reasonable for an intentionally hot firmware screen. Recording the observed temperature with `NotApplicable` preserves useful evidence.

I would scope applicability to a declared hot-screen measurement profile, rather than all `Screen` runs, and determine applicability before observing whether temperature happens to exceed the threshold. Otherwise an unintentionally hot idle screen receives the same exemption.

The constant’s justification overclaims:

- [preconditions.rs:26](/Users/d0nmega/Developer/neurorust/crates/capture/src/preconditions.rs:26) calls it “derived from measurement,” while admitting the 59–91 °C interval is unmeasured.
- Zero events above 10 µs at 59 °C establishes neither absence of SMIs nor a safe boundary at 70 °C.
- The hot observations confound temperature with workload and configuration.
- The assertion that one idle RT observation confirms kernel independence repeats D-18’s unsupported inference.
- The “P-cores saturated” evidence row misdescribes an all-CPU load with P-core-restricted sampling.

Seventy degrees can be a **provisional admission threshold for a specified operating profile**. It is not currently a measured SMI-onset or throttling boundary. Using hot observations to motivate a cool operating policy is not itself circular; applying that policy to forbid the hot characterization needed to investigate it was the design error.

The larger defect is the claim that subsequent thermal excursions are auditable. [run.rs:387](/Users/d0nmega/Developer/neurorust/crates/cli/src/cmd/run.rs:387) takes its environment snapshot **after both instruments finish**. [environment.rs:656](/Users/d0nmega/Developer/neurorust/crates/capture/src/environment.rs:656) then stores that temperature as `temp_c_start`, always leaves `temp_c_end` empty, and calculates `package_temp_c_max` as the maximum across thermal zones at that one instant. It is not a maximum over the run, and the hottest zone need not be the package sensor.

Fix the timestamps and sensor semantics before using manifests to establish thermal comparability. Record actual start/end temperatures and a suitably sampled temperature trace, with monitoring overhead characterized. Correct the interpretation of existing captures rather than inventing missing history.

The protocol still specifies **60 °C**, with no Screen exception, at [measurement-protocol.md:67](/Users/d0nmega/Developer/neurorust/docs/measurement-protocol.md:67). The claimed assert-and-record contract currently disagrees with execution.

**6. F: Provenance and verification do not support “every claim is reproducible” yet.**

Several concrete failures undermine that promise:

- **Harness identity describes the checkout at execution time, not the executable.** [run.rs:690](/Users/d0nmega/Developer/neurorust/crates/cli/src/cmd/run.rs:690) runs `git rev-parse HEAD` in the current working directory. A stale executable can acquire a newer checkout’s identity; failure becomes `unknown` with `git_dirty: false`. The calibration artifacts actually contain this unknown identity, for example [clean manifest:11](/Users/d0nmega/Developer/neurorust/measurements/2026-09-01-precision3591-calibration-clean/manifest.json:11). Embed build provenance and record executable hashes.
- **Recorded commands retain deleted temporary paths.** [run.rs:438](/Users/d0nmega/Developer/neurorust/crates/cli/src/cmd/run.rs:438) relativizes against the final directory, although outputs were created elsewhere. The resulting `/tmp/.tmp…` paths are visible at [clean manifest:695](/Users/d0nmega/Developer/neurorust/measurements/2026-09-01-precision3591-calibration-clean/manifest.json:695). Preserve executed argv plus an explicit artifact-path mapping.
- **A parse failure becomes a published zero.** [verify.rs:406](/Users/d0nmega/Developer/neurorust/crates/cli/src/cmd/verify.rs:406) substitutes `(0, 0)` when histogram metrics cannot be computed. A correctly hashed malformed capture is not valid numerical evidence. Fail verification or report unavailable values.
- **Hashes do not verify the derived claims.** [validate.rs:44](/Users/d0nmega/Developer/neurorust/crates/manifest/src/validate.rs:44) verifies artifact integrity and limited structural conditions. It does not establish the complete applicable precondition set or recompute eligibility. Verification does not regenerate and compare `REPORT.md` and `hist.tsv`. [JSON reconciliation:83](/Users/d0nmega/Developer/neurorust/crates/histogram/src/json.rs:83) checks thread count and maxima, not sample-count agreement.

I would make strict verification reproduce the numerical reports, validate cross-artifact counts and applicable protocol requirements, and record the exact threshold configuration and algorithm used. Missing information must remain missing.

One historical claim already lacks its promised raw support: [README.md:55](/Users/d0nmega/Developer/neurorust/measurements/2026-08-28-precision3591/README.md:55) cites six events and a 7 µs maximum at a 1 µs threshold. The referenced tuned capture contains only the 10 µs-threshold run. I found no matching 1 µs raw capture in the measurements or rig documentation. Recover it or mark that observation as lacking a published raw capture.

**7. F: Failed attempts can disappear, and interference counters cover the wrong duration.**

In [run.rs:329](/Users/d0nmega/Developer/neurorust/crates/cli/src/cmd/run.rs:329), raw output lives in a temporary directory. Parsing, reconciliation and verdict calculation can return errors before the durable run directory is created at line 393. A malformed or incomplete capture can therefore lose its raw evidence. Warning about a nonzero exit does not guarantee preservation.

Create a durable attempt record before launching instruments. Preserve partial outputs, stderr, exit status and failure reasons even when parsing fails. “Not usable for numerical analysis” must remain a recorded outcome.

Separately, the interrupt snapshots bracket cyclictest **plus hwlat plus parsing**, but [run.rs:383](/Users/d0nmega/Developer/neurorust/crates/cli/src/cmd/run.rs:383) normalizes by requested cyclictest duration alone. A nominal 3600-second cyclictest followed by 900-second hwlat includes roughly 4500 seconds of activity divided by 3600. Early-terminated investigation runs have another denominator mismatch.

Take per-instrument snapshots and monotonic elapsed times, with parsing outside the measurement interval.

**8. F: The asserted gate checks are narrower than their names suggest.**

[preconditions.rs:384](/Users/d0nmega/Developer/neurorust/crates/capture/src/preconditions.rs:384) checks deep C-states only on **CPU 0**. It cannot establish the state of target CPUs 6–11 after per-CPU sysfs tuning.

[preconditions.rs:594](/Users/d0nmega/Developer/neurorust/crates/capture/src/preconditions.rs:594) treats `current_tracer == nop` as tracer quiescence. Event tracing has separate enable controls, so this does not establish absence of tracing activity. [Kernel event tracing documentation](https://docs.kernel.org/trace/events.html).

There is also an inconsistent fixture restriction: [run.rs:232](/Users/d0nmega/Developer/neurorust/crates/cli/src/cmd/run.rs:232) forbids the facts fixture for publishable classes, while the interrupts fixture accepted at [line 315](/Users/d0nmega/Developer/neurorust/crates/cli/src/cmd/run.rs:315) is reused for both snapshots, producing zero deltas. That test input must also be forbidden or force explicit non-publication status.

Audit each gate against the exact proposition it claims to establish. Check target CPUs, effective tracing configuration and fixture provenance explicitly.

**9. F: PLAT-01 still contains unsupported attribution, and its tracing recipe does not establish that the required trace will exist.**

[README.md:127](/Users/d0nmega/Developer/neurorust/measurements/2026-08-28-precision3591/README.md:127) assigns the approximately 3.8 ms maximum to the isolated events rather than the sustained burst. The capture records per-thread maxima and overflow cycle numbers separately; it does not map each overflow’s amplitude to its cycle. That assignment cannot be recovered from those artifacts.

Similar thread maxima suggest a hypothesis worth investigating. They do not prove a simultaneous global event or identify `stop_machine()` or TLB shootdowns. Describe the observed patterns and leave the amplitude-to-event association unresolved.

The investigation recipe has execution defects:

- [01-12-PLAN.md:183](/Users/d0nmega/Developer/neurorust/.planning/phases/01-trustworthy-measurement/01-12-PLAN.md:183) says `--tracemark` “arms ftrace.” The flags mark/stop tracing at a threshold; they do not specify and enable the intended diagnostic event set. Configure tracing explicitly and verify the resulting buffer. [cyclictest manual](https://manpages.debian.org/bookworm/rt-tests/cyclictest.8.en.html).
- [01-12-PLAN.md:273](/Users/d0nmega/Developer/neurorust/.planning/phases/01-trustworthy-measurement/01-12-PLAN.md:273) says rtla runs “alongside,” but the foreground pipeline completes before the harness command starts.

Finally, [01-13-PLAN.md:301](/Users/d0nmega/Developer/neurorust/.planning/phases/01-trustworthy-measurement/01-13-PLAN.md:301) turns non-reproduction into exclusion from the residual. Non-reproduction supplies additional exposure without an observed recurrence; it does not resolve the earlier cause or establish a deterministic recurrence bound.

**10. F: Two downstream publication instructions would manufacture confidence if implemented literally.**

[01-14-PLAN.md:162](/Users/d0nmega/Developer/neurorust/.planning/phases/01-trustworthy-measurement/01-14-PLAN.md:162) instructs unavailable hwlat percentiles to be filled with the maximum. A module comment does not make those valid percentiles. Use nullable statistics with an explicit population: percentiles of threshold-exceeding records differ from percentiles of all sampling windows or scheduling wakeups.

The proposed regression workflow at [01-14-PLAN.md:438](/Users/d0nmega/Developer/neurorust/.planning/phases/01-trustworthy-measurement/01-14-PLAN.md:438) uses `merge-base origin/main HEAD` for push comparisons. On a main-branch push where both identify the new head, the diff is empty and the “reviewed baseline” is the current baseline. The guard can miss precisely the simultaneous change it claims to prohibit. Use event-specific base revisions and an independently protected baseline.

These are planned defects, not claims that those workflows are already operational. Correct them before implementation.

The defensible near-term artifact is an **observed-latency characterization of a specified installed configuration**, retaining unexplained failures and documenting measurement exposure. The abort proof is still an unchecked requirement at [PROJECT.md:39](/Users/d0nmega/Developer/neurorust/.planning/PROJECT.md:39); neither these screens nor a successful scheduling run establishes that proof or a physical worst-case response bound.
