# Phase 1: Trustworthy measurement - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md - this log preserves the alternatives considered.

**Date:** 2026-08-30
**Phase:** 01-trustworthy-measurement
**Mode:** discuss (advisor mode off, no USER-PROFILE.md)
**Areas discussed:** Run execution + weekly job, Provenance + enforcement, PLAT-01 stopping rule
**Area offered but not selected:** Publication surface + run matrix

---

## Assumptions pass

Claude surfaced assumptions across technical approach, implementation order, scope boundaries,
risk areas and dependencies before the gray-area menu. The user responded "Yes all good" with
no corrections, so all five areas were folded in as locked decisions (CONTEXT.md D-01 to D-04
and the scope boundary).

Two findings were surfaced during the codebase scout that preceded the assumptions, both from
re-deriving the existing README's numbers against the raw histogram: the bimodality claim is
false, and the over-gate sample count undercounts by the 888 histogram overflows. A third
finding (the overflow bursts indicate two distinct phenomena, not one) was surfaced at the same
time. All three are recorded in CONTEXT.md under Specific Ideas.

---

## Run execution + weekly job

### Where does a measurement run execute, and what starts it?

| Option | Description | Selected |
|--------|-------------|----------|
| On-rig systemd timer | Oneshot service plus timer, no login session; nothing on the wire during the measurement. GitHub Actions reduced to validating pushes | Yes |
| Self-hosted GH Actions runner | One CI system, but the agent polls GitHub throughout the job, which is the activity PLAT-02 exists to eliminate | |
| Manual operator ritual | Human runs the harness at the console; CI validates on push. Reinterprets BENCH-08's "a CI job commits" | |

### How does the harness handle system state around a run?

| Option | Description | Selected |
|--------|-------------|----------|
| Assert and refuse | Check preconditions, record every check in the manifest, refuse on violation, change nothing. Doubles as the third-party reproduction checklist | Yes |
| Assert and enforce | Harness drops to multi-user.target and stops the display manager itself. Needs root; a third party without root cannot reproduce the same path | |
| Assert by default, --enforce opt-in | Both paths. More surface to build, and the two paths can drift so figures are not strictly comparable | |

### How does a run's output reach the repo?

| Option | Description | Selected |
|--------|-------------|----------|
| Split: auto metrics, reviewed figures | Weekly JSON auto-commits; published claims reach main via reviewed commit. Satisfies BENCH-08 literally while gating claims | Yes |
| Auto-commit everything | Zero friction; a contaminated run publishes itself, which is how the current README acquired its errors | |
| Human commits everything | Strongest gate; BENCH-08's "CI job commits" is then not met in any form | |

### What happens when a week is missed?

| Option | Description | Selected |
|--------|-------------|----------|
| Record the gap, no backfill | A hole in the series is honest information about the rig. Consistent with BENCH-06 | Yes |
| Persistent timer, catch-up on next boot | Denser series, but a post-boot run is a measurably different environment | |
| Best effort, untracked | Cannot distinguish a missed week from a week with no regression | |

### What does the weekly metrics JSON carry in Phase 1?

| Option | Description | Selected |
|--------|-------------|----------|
| cyclictest + hwlatdetect both | Catches a BIOS or microcode change that shifts the firmware floor the week it happens | Yes |
| cyclictest regression watch only | Cheaper; a firmware shift would surface only as unexplained scheduling drift | |
| Schema-first, near-empty | Least work now; the weekly job's first real use would also be its first debugging session | |

### How long is a weekly run?

| Option | Description | Selected |
|--------|-------------|----------|
| Short weekly plus periodic long soak | ~1 h weekly for the series, longer soak periodically; run classes tagged distinctly | Yes |
| 10 minutes weekly | Directly comparable to the 2026-08-28 baseline; weak power for rare tail events | |
| 24 hours weekly | Soak-grade evidence every week; monopolises a machine also needed for Windows | |

### When the weekly run shows a regression, what happens?

| Option | Description | Selected |
|--------|-------------|----------|
| Record on rig, CI evaluates and fails | Judgement lives in reviewable CI config; failure visible without logging into the rig | Yes |
| Record only, no gate | Avoids a threshold guessed before there is history; failure mode is that nobody looks | |
| Rig evaluates and refuses to publish | Hides the data BENCH-06 says to publish; a bad run is evidence | |

**Notes:** Claude flagged that the chosen combination is self-consistent - an unattended timer
that refuses on a failed precondition produces a recorded gap, the same outcome as a missed
week - so no additional rule was needed for that case.

---

## Provenance + enforcement

### What form does the provenance record take?

| Option | Description | Selected |
|--------|-------------|----------|
| Sidecar JSON, human view generated | manifest.json is source of truth with checksums; RIG.txt-style view generated from it so the two cannot disagree; raw captures stay byte-identical | Yes |
| Human-readable text only | One readable file; free text is brittle to parse and hand-edited manifests drift | |
| Embedded header in the capture | Self-contained, but the capture is no longer byte-identical to tool output, weakening the "raw capture" claim | |

### What does CI enforce about provenance?

| Option | Description | Selected |
|--------|-------------|----------|
| Blocking: no capture without a valid manifest | Turns the core value claim into something mechanically true | Yes |
| Warning only | Keeps the pipeline unblocked; a warning nobody reads is not a contract | |
| Validate manifests that exist, do not require them | Catches malformed provenance; leaves the main hole open | |

### What is the required field set?

| Option | Description | Selected |
|--------|-------------|----------|
| Full environment snapshot | Superset of BENCH-04 including /proc/interrupts before and after. The contaminated run was only diagnosable because someone checked CAL counts after the fact | Yes |
| Minimal per BENCH-04 | Small schema; post-hoc diagnosis depends on state nobody captured | |
| Minimal required, extended optional | Optional provenance tends to be absent on exactly the runs that need it | |

### Should the harness render a contamination verdict?

| Option | Description | Selected |
|--------|-------------|----------|
| Yes - automatic quarantine, still published | Mechanises the lesson the 2026-08-28 baseline taught; contaminated runs published per BENCH-06 but excluded from the headline series | Yes |
| Record counters, human judges | Avoids an uncalibrated threshold; failure mode is silence | |
| Do not capture interference counters | The contaminated baseline passed every plausible precondition check | |

### How do existing captures satisfy a blocking manifest check?

| Option | Description | Selected |
|--------|-------------|----------|
| Reconstruct with explicit completeness tier | Rebuild from RIG.txt and README, mark never-recorded fields explicitly absent, tag the run contaminated, carry a reconstructed vs harness-generated tier | Yes |
| Grandfather exemption marker | Two lines of work; creates a permanent class of figures outside the contract | |
| Move to archive/, outside the enforced tree | Honest separation; makes rig-selection evidence harder to find and cite | |

### Where do contamination thresholds come from?

| Option | Description | Selected |
|--------|-------------|----------|
| Calibration pair: clean vs deliberately contaminated | Thresholds from observed separation; the contaminated arm is itself publishable and BENCH-06 asks for losing configurations anyway | Yes |
| Fixed conservative constants | Fastest; with a blocking gate downstream, a bad constant either fires constantly or never | |
| Relative to trailing median | Self-calibrating but needs history first; slow drift moves the median with it | |

### What happens to the live-USB hwlatdetect figures?

| Option | Description | Selected |
|--------|-------------|----------|
| Re-run on the installed RT system, publish the delta | Tests the README's untested claim that SMI behaviour is kernel-independent, which underpins the rig decision | Yes |
| Re-run, do not make a point of the comparison | Compliant baseline with less writing | |
| Keep as-is, the claim carries over | Saves ~30 minutes; leaves an untested assumption load-bearing | |

---

## PLAT-01 stopping rule

### What counts as "named"?

| Option | Description | Selected |
|--------|-------------|----------|
| Kernel path from ftrace, per the roadmap | Matches the success criterion verbatim; one named path per distinct phenomenon; a reproducing trigger pursued opportunistically but not required | Yes |
| Named path plus a reproducing trigger | Stronger, allows verifying a mitigation; risks an open-ended hunt if no trigger exists | |
| Subsystem-level attribution | Faster; falls short of the stated criterion, which would need rewording | |

### What is the stopping rule if the investigation does not converge?

| Option | Description | Selected |
|--------|-------------|----------|
| Fixed budget, then defer as open question | Defers rather than blocks, so an unresolved PLAT-01 never holds Phase 2's proof hostage | Yes |
| Escalating-evidence rule | Self-terminating on diminishing returns; harder to judge honestly in the moment | |
| Iterate until named, no box | Strongest Phase 1 result; accepts indefinite blocking of the work the project says matters most | |

### What if the stall does not reproduce on a clean run?

| Option | Description | Selected |
|--------|-------------|----------|
| That IS the result - publish the pair | Close PLAT-01 with the contamination finding, clean and contaminated side by side. The calibration pair already produces the contaminated arm | Yes |
| Keep hunting with longer soaks | Better power for rare events; costs rig-days and may end in the same place | |
| Leave PLAT-01 open | Leaves a known-unexplained event under every later figure | |

### How should PLAT-03 account for the narrow headroom above the firmware floor?

| Option | Description | Selected |
|--------|-------------|----------|
| Report total and decomposed | Total against the 30 us gate plus the kernel's contribution above the measured firmware floor, so a miss is interpretable | Yes |
| Total observed max only | The honest end-to-end number; a 25 us result reads the same whether the kernel contributed 3 us or 25 us | |
| Firmware-floor-subtracted | Isolates what software can influence; subtracting a floor measured by a different tool under different conditions invites scrutiny | |

**Notes:** Claude flagged before this area that the raw data suggests two distinct phenomena
(a ~1.2 s window of ~100 Hz stalls on all six threads, plus isolated cross-thread events
including the ~3.8 ms max). That framing shaped the "one named path per distinct phenomenon"
wording in the selected option.

---

## Claude's Discretion

The user selected three of the four offered gray areas. The fourth, publication surface and
run matrix, was reviewed and left to Claude: measurements/ layout, raw captures in-repo versus
pointers, histograms committed versus generated, the exact Phase 1 run matrix beyond the fixed
cadence and calibration pair, BENCH-06 operationalisation in the published layout, metrics JSON
naming and layout, regression threshold values, and the form of the systemd unit files.

The PLAT-01 investigation budget was left unset by the questions and defaulted to three
capture-and-analyse cycles, accepted by the user at the closing prompt.

## Deferred Ideas

None. Discussion stayed within the phase boundary; no scope creep was raised or redirected.
