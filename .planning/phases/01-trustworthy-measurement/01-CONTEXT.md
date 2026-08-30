# Phase 1: Trustworthy measurement - Context

**Gathered:** 2026-08-30
**Status:** Ready for planning

<domain>
## Phase Boundary

The rig becomes a credible instrument and the project gets a publication contract. Phase 1
delivers: the ~3.8 ms global stall named rather than tolerated (PLAT-01), a documented and
mechanically enforced clean measurement protocol (PLAT-02), a resolution of the 30 us
scheduling gate (PLAT-03), a capture harness that stamps rig, kernel, BIOS and tuning state
onto every figure and publishes histograms with raw captures and losing configurations
(BENCH-04, BENCH-05, BENCH-06), and a weekly committed metrics JSON (BENCH-08).

The cargo workspace and CI pipeline are created here as enabling work. No runtime code is in
scope: no channels, no nodes, no `emergency_stop`. The `SCHED_DEADLINE` versus `SCHED_FIFO`
comparison is SUBS-06 in Phase 3, not here.

</domain>

<decisions>
## Implementation Decisions

### Harness and toolchain

- **D-01:** The capture harness is a Rust binary in a new cargo workspace, not a set of shell
  scripts. It shells out to `cyclictest` and `hwlatdetect` but owns run-manifest generation,
  histogram parsing, percentile computation and JSON emission. Rationale: BENCH-04, BENCH-05
  and BENCH-08 are a data contract, and a typed parser that later phases reuse for their own
  histograms beats bash that gets copy-pasted per phase.
- **D-02:** CI is GitHub Actions targeting `x86_64-unknown-linux-gnu` plus the macOS dev host.
  Phase 1 gates are fmt, clippy, test, the provenance check (D-13) and the regression check
  (D-11). Hooks are left for Kani in Phase 2 and loom plus criterion in Phase 4.
- **D-03:** Order of work: workspace and CI skeleton first, then manifest schema, histogram
  parser and metrics JSON, then the written protocol (PLAT-02), then the first clean run,
  then PLAT-01, then the PLAT-03 verdict, then the weekly job last. The harness's first
  customer is PLAT-01's own investigation, so the protocol is dogfooded before it becomes a
  contract.
- **D-04:** The manifest and metrics schema must generalise beyond cyclictest now, not later.
  STOP-07 (Phase 2 abort latency) and SUBS-06 (Phase 3 scheduling comparison) are its first
  external consumers.

### Run execution and the weekly metrics job

- **D-05:** Measurement runs execute on the rig from a systemd oneshot service plus timer,
  with no login session. This satisfies PLAT-02 by construction: nothing is on the wire during
  the measurement. GitHub Actions is reduced to validating what gets pushed. A self-hosted
  Actions runner was rejected because its agent polls GitHub throughout the job, which is the
  exact background activity PLAT-02 exists to eliminate.
- **D-06:** The harness asserts preconditions and refuses to run on violation. It changes
  nothing about system state. Checks cover active sshd sessions, systemd default target,
  display manager state, governor, `no_turbo`, C-states, `isolcpus`, and
  `/sys/kernel/realtime`. Every check result is recorded in the manifest. The assertion list
  doubles as the third-party reproduction checklist, which an enforce-mode harness needing
  root would not provide.
- **D-07:** Publication path is split. The weekly p50/p95/p99 JSON is a regression-tracking
  artifact and auto-commits. Anything that becomes a published claim (a histogram, a README
  figure, a stated verdict) reaches main only through a reviewed commit. This satisfies
  BENCH-08 literally while keeping a human gate on every claim.
- **D-08:** A missed week is recorded as a gap in the coverage record. No backfill, no
  catch-up run. A post-boot catch-up would be a measurably different environment, and a gap in
  the series is itself honest information about the rig. A run refused on a failed
  precondition (D-06) produces the same recorded gap.
- **D-09:** The weekly watch covers both `cyclictest` and `hwlatdetect`. The rig is a vendor
  laptop that takes BIOS and microcode updates via fwupd, and a firmware change could silently
  move the 22 to 29 us floor that every later figure sits on top of. Catching that the week it
  happens is worth more than catching it a phase later.
- **D-10:** Run cadence is a roughly 1 hour weekly run for the regression series plus a
  periodic longer soak (monthly or on demand before a published claim). The two run classes
  are tagged distinctly in the manifest so they are never averaged together. A weekly 24 hour
  soak was rejected because the rig is also a Windows machine.
- **D-11:** The rig only measures and pushes. A GitHub Actions workflow triggered on that push
  compares against a committed baseline and fails the check when p99 or max regresses beyond a
  stated threshold. Judgement lives in reviewable CI config, not in a script on one laptop,
  and the failure is visible without logging into the rig. The harness never withholds a
  regressed run, because a bad run is evidence (BENCH-06), not something to suppress.

### Provenance and enforcement

- **D-12:** A sidecar `manifest.json` per run directory is the source of truth, carrying
  checksums over each raw capture. The human-readable RIG.txt-style summary is generated from
  it rather than hand-maintained, so the two cannot disagree. Raw captures stay byte-identical
  to what the tool emitted, which an embedded-header approach would break.
- **D-13:** CI enforcement is blocking. Any capture or figure under `measurements/` must have
  a manifest with all required fields present and checksums matching, or the check fails and
  the commit cannot merge. This is what turns the project's core value claim into something
  mechanically true rather than aspirational.
- **D-14:** The required field set is a full environment snapshot, a superset of BENCH-04's
  literal ask: every field currently in `RIG.txt`, plus kernel cmdline, `isolcpus`,
  `nohz_full` and `rcu_nocbs`, per-CPU governor, `no_turbo` and C-state configuration,
  `/sys/kernel/realtime`, `rt-tuning.service` state, thermal and AC/battery state,
  `/proc/interrupts` before and after the run, precondition assertion results (D-06), harness
  version, git SHA, UTC start and end, and per-file checksums. Rationale: the 2026-08-28
  contaminated run was only diagnosable because someone thought to check CAL counts after the
  fact.
- **D-15:** The harness renders an automatic contamination verdict. It diffs interference
  counters across the run (CAL and TLB IPIs, context switches, IRQs on isolated cores) and
  marks a run `contaminated` when thresholds are exceeded. Contaminated runs are kept and
  published per BENCH-06 but excluded from the headline series. This mechanises the lesson the
  2026-08-28 baseline taught instead of relying on someone noticing again.
- **D-16:** Existing captures get reconstructed manifests rather than an exemption. Fields are
  rebuilt from `RIG.txt` and the README; fields that were never recorded are marked explicitly
  absent, never guessed; the cyclictest run is tagged `contaminated`. The schema carries a
  provenance tier, `reconstructed` versus `harness-generated`, so a reader can tell at a glance
  which figures meet the full contract. This keeps the rig-selection evidence inside the
  contract instead of beside it.
- **D-17:** Contamination thresholds (D-15) are derived from a deliberate calibration pair:
  one run under the clean protocol, one deliberately reproducing the 2026-08-28 conditions
  (SSH activity plus an active GNOME session), with thresholds set from the observed
  separation. The contaminated arm is itself a publishable result: it documents what
  contamination looks like, which is what PLAT-02 needs third parties to be able to avoid, and
  BENCH-06 asks for losing configurations to be reported anyway.
- **D-18:** The `hwlatdetect` firmware baseline is re-run on the installed PREEMPT_RT system
  under the harness, and the delta against the live-USB stock-kernel figures is published. The
  existing README asserts that SMI behaviour is kernel-independent and that the screening
  figures therefore carry over. That assumption is load-bearing under the entire rig decision
  and has never been tested.

### PLAT-01 root cause and the PLAT-03 verdict

- **D-19:** "Named" means a specific kernel path identified from the ftrace and
  `cyclictest --tracemark` capture, with that capture committed next to the claim, one named
  path per distinct phenomenon. This matches the roadmap's success criterion verbatim. A
  reproducing trigger is pursued opportunistically but is not the bar, so the investigation
  cannot run unbounded chasing one.
- **D-20:** The investigation is budgeted at three capture-and-analyse cycles. On exhaustion,
  what is known is published as a documented open question and work proceeds to Phase 2. This
  defers rather than blocks: Phase 2 is the proof, the single most differentiating artifact,
  and it needs no rig at all, so an unresolved PLAT-01 must never hold it hostage.
- **D-21:** If the stall does not reproduce on a clean run, that is the result, not a failure.
  PLAT-01 closes with "the 3.8 ms stall was an artifact of measurement contamination", showing
  the clean and contaminated runs side by side. The calibration pair in D-17 already produces
  the contaminated arm, so the comparison falls out at no extra cost. This outcome demonstrates
  the project's own thesis about measurement discipline better than a kernel bug would.
- **D-22:** PLAT-03 reports both the total observed max against the 30 us gate and, separately,
  the kernel's contribution above the independently measured firmware floor (D-18). The gate
  sits barely above a 22 to 29 us firmware floor under load, leaving roughly 1 to 8 us for
  everything the kernel does, so an undecomposed number makes a miss uninterpretable. Reporting
  both lets a reader see which layer consumed the budget.

### Corrections to existing published artifacts

- **D-23:** `measurements/2026-08-28-precision3591/README.md` is corrected in place as Phase 1
  work. Two of its claims do not survive re-derivation from the raw histogram; see Specific
  Ideas below for the exact errors and the corrected figures. Correcting a published artifact
  that misstates its own raw data is this phase's subject matter, not scope creep.

### Claude's Discretion

The publication surface and run matrix were reviewed and left to Claude:

- `measurements/` directory layout beyond the existing `<date>-<rig>` convention
- Whether raw captures live in the repo or behind pointers, and any size threshold
- Whether histograms are committed as rendered images or generated on demand from raw captures
- The exact composition of the Phase 1 run matrix (which configurations, which durations)
  beyond the cadence fixed in D-10 and the calibration pair fixed in D-17
- How BENCH-06 ("report losing configurations") is operationalised in the published layout
- Metrics JSON file naming, on-disk layout, and the regression threshold values in D-11
- Exact form of the systemd unit files and how they are installed on the rig from the repo

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

ROADMAP.md carries no `Canonical refs:` line for this phase. The list below is assembled from
REQUIREMENTS.md, PROJECT.md, and the artifacts discovered during the codebase scout.

### Phase scope and requirements

- `.planning/ROADMAP.md` - Phase 1 section: goal, the five success criteria, and the note that
  the cargo workspace and CI pipeline are enabling work created here
- `.planning/REQUIREMENTS.md` - PLAT-01, PLAT-02, PLAT-03, BENCH-04, BENCH-05, BENCH-06,
  BENCH-08 verbatim text, plus the traceability table
- `.planning/PROJECT.md` - Core Value, the Constraints block (rig discipline, hot-path rules,
  budget, licence, provenance), and the Key Decisions table

### Project state and open questions

- `.planning/STATE.md` - Blockers and Concerns section: the unexplained 3.8 ms stall, the
  contaminated baseline, the PROJECT.md timestamping inconsistency, and the WIRE-06 cable
  dependency

### Existing measurement evidence

- `measurements/2026-08-28-precision3591/README.md` - method, results table, findings and
  caveats for the firmware screen and the first PREEMPT_RT run. Contains two errors corrected
  under D-23; read alongside the Specific Ideas section below
- `measurements/2026-08-28-precision3591/RIG.txt` - the current de facto manifest schema and
  roughly 90 percent of the D-14 field set
- `measurements/2026-08-28-precision3591/cyclictest-rt-isolated-idle-10m.hist` - PLAT-01's
  starting evidence and the reference capture for the D-17 calibration pair
- `measurements/2026-08-28-precision3591/hwlatdetect-stock-15m.txt` - untuned firmware
  baseline, 125 us worst case
- `measurements/2026-08-28-precision3591/hwlatdetect-tuned-15m.txt` - tuned idle, under 10 us
- `measurements/2026-08-28-precision3591/hwlatdetect-tuned-underload-15m.txt` - 22 cores
  saturated, 29 us worst case
- `measurements/2026-08-28-precision3591/hwlatdetect-pcore-underload-10m.txt` - P-cores only
  under load, 22 us worst case. This is the floor D-22 decomposes against

</canonical_refs>

<code_context>
## Existing Code Insights

The repository is greenfield. There is no `Cargo.toml`, no `src/`, no CI configuration. Only
`.planning/`, `measurements/`, and `CLAUDE.md` exist. Everything below is convention and data,
not code.

### Reusable assets

- `RIG.txt` field set: already covers system, BIOS, CPU, topology, microcode, kernel, OS,
  session, memory, cmdline, governor, no_turbo, cstates, power, and NIC state. It is the
  starting point for the D-14 manifest schema, not a blank page.
- `measurements/<ISO-date>-<rig-slug>/` directory convention, established by the existing
  capture set and worth keeping.
- The contaminated cyclictest capture serves double duty: PLAT-01's starting evidence and the
  reference arm for the D-17 threshold calibration.
- `rt-tuning.service` already exists on the rig and persists governor, `no_turbo` and C-state
  settings across reboots. It is the authoritative source to query for tuning state in D-14,
  rather than re-deriving it.

### Established patterns

- None in code. The conventions this phase sets (workspace layout, manifest schema, CI gate
  structure) become the patterns every later phase inherits, so they are worth getting right
  once rather than reworking in Phase 4.
- Prose style is set by `~/.claude/rules/common/writing-style.md`: no em dashes, ASCII only,
  no emoji, sentence case headings. The existing README and RIG.txt already follow it.

### Integration points

- New cargo workspace at the repository root, alongside the existing `.planning/` and
  `measurements/` trees.
- `.github/workflows/` for the fmt, clippy, test, provenance and regression checks. Phase 2
  appends Kani, Phase 4 appends loom and criterion, so the workflow structure should make
  adding a blocking gate a small diff.
- systemd unit files (D-05) live in the repo and are installed onto the rig, so they are
  version-controlled artifacts, not machine-local configuration.
- The metrics JSON schema is the contract that STOP-07 and SUBS-06 write into from Phase 2
  and Phase 3.

</code_context>

<specifics>
## Specific Ideas

### Two errors in the existing README, found by re-deriving from the raw histogram

Both are corrected under D-23. Recorded here with the exact numbers so the planner does not
have to re-derive them.

1. **The bimodality claim is false.** The README states "zero samples between 13 and 100 us
   and then a distinct 101-399 us population, which indicates a specific recurring event
   rather than gradual noise." The histogram shows a continuous long tail: bins 31 to 99 hold
   230 samples, bins 100 to 399 hold 971, and the largest empty run anywhere above 13 us is
   6 bins (278 to 283), consistent with sparse sampling at 2 to 8 counts per bin. There is no
   gap and no second mode. The "specific recurring event" inference rests on a gap that is not
   in the data.

2. **The over-gate count undercounts.** The README states "Samples above the 30 us gate: 1,201
   of 17,994,956 (0.0067%)". 1,201 is bins 31 to 399 only. cyclictest counts the 888 histogram
   overflows (>= 400 us) separately, and those are also above 30 us. The correct figure is
   2,089 of 17,994,956, or 0.0116%.

### A structural finding the README misses: there are two phenomena, not one

The overflow cycle numbers cluster hard. All six threads log roughly 140 overflows inside
cycles ~1,820,584 to ~1,826,674, which at a 200 us interval is about 1.2 seconds of wall time
carrying a >400 us stall roughly every 10 ms on every isolated core at once. Separately, a
handful of isolated cross-thread events appear (cycles ~2,259,348, ~2,349,428, ~2,709,344,
each landing on two or three threads within ~30 cycles), and the ~3.8 ms maximum belongs to
that second category.

This is why D-19 says "one named path per distinct phenomenon". Treating the sustained
100 Hz burst and the isolated global spikes as a single event is likely to produce a wrong
root cause for at least one of them.

### The 30 us gate has almost no headroom

`hwlatdetect` measured a 22 us firmware floor on P-cores and 29 us whole-machine under
adversarial thermal load. Against a 30 us gate, that leaves roughly 1 to 8 us for everything
the kernel does. PLAT-03 landing on "documented limitation" rather than "under 30 us" is a
live and reasonable outcome, and D-22 exists so that outcome is interpretable rather than
just a fail.

</specifics>

<deferred>
## Deferred Ideas

None. Discussion stayed within the phase boundary. The one area from the original gray-area
menu that was not selected for discussion (publication surface and run matrix) is not deferred
to a later phase; it is in scope for Phase 1 and recorded under Claude's Discretion above.

</deferred>

---

*Phase: 01-trustworthy-measurement*
*Context gathered: 2026-08-30*
