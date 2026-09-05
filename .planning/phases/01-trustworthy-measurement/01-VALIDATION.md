---
phase: 1
slug: trustworthy-measurement
status: approved
nyquist_compliant: true
wave_0_complete: true
created: 2026-08-30
updated: 2026-09-05
---

# Phase 1 - Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from `01-RESEARCH.md` "## Validation Architecture".

**Phase-specific caveat:** this phase's requirements are mostly about artifacts, provenance and a
rig measurement, not about runtime behaviour. Several requirements genuinely cannot be verified on
the dev host. The tables below separate automated coverage from rig-only verification honestly
rather than claiming a unit test covers a physical measurement. The planner must not paper over
that split.

**Current scope:** 23 plans spanning waves 0 through 18. Plans 01-01 through 01-11 (waves 0
through 7) have executed. Plans 01-12 through 01-23 (waves 8 through 18) are pending. The phase
was expanded from 15 plans to 23 on 2026-09-05 after `01-EXTERNAL-AUDIT.md` and the D-18
instrument failure; plan numbers no longer match wave order, so every table below carries the wave.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `#[test]`, plus `insta` 1.48.0 (snapshots) and `assert_cmd` 2.2.2 + `predicates` 3.1.4 (CLI) |
| **Config file** | `Cargo.toml` at the repo root; five crates (nr-manifest, nr-histogram, nr-capture, nr-metrics, nr-cli). Created by plan 01-01 in wave 0. |
| **Quick run command** | `cargo test --workspace` |
| **Full suite command** | `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` |
| **Provenance gate** | `./target/release/nrmeasure verify --strict --check-index`, referred to below as the gate. Built by plan 01-08; 20 of the 36 pending tasks include it in their verify. |
| **Estimated runtime** | ~30 seconds quick, ~90 seconds full on a cold cache |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --workspace`
- **After every task that writes to `measurements/`, `metrics/` or a manifest schema:** also run the gate
- **After every plan wave:** Run `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
- **Before `/donny-verify-work`:** Full suite green, AND the provenance gate green against the
  real `measurements/` tree including the D-16 reconstructed manifests
- **Max feedback latency:** 90 seconds

---

## Per-Task Verification Map

### Executed plans (01-01 through 01-11, waves 0 through 7)

Written before execution, stating the required coverage per requirement. Left as authored.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 01-01-T | 01-01 | 0 | infra | - | N/A | build | `cargo test --workspace` | no W0 | pending |
| 01-03-T | 01-03 | 1 | BENCH-04 | T-1-03 | Manifest rejects missing required field | unit | `cargo test -p manifest required_fields` | no W0 | pending |
| 01-08-T | 01-08 | 5 | BENCH-04 | T-1-05 | Provenance gate rejects capture with no manifest | integration | `cargo test -p cli provenance_gate_rejects_orphan_capture` | no W0 | pending |
| 01-03-T | 01-03 | 1 | BENCH-04 | T-1-03 | Checksum mismatch fails validation | unit | `cargo test -p manifest checksum_mismatch_rejected` | no W0 | pending |
| 01-04-T | 01-04 | 2 | BENCH-05 | - | Percentiles reproduce known values from committed fixture | unit | `cargo test -p histogram percentiles_from_fixture` | no W0 | pending |
| 01-04-T | 01-04 | 2 | BENCH-05 | - | Overflow samples counted in percentiles and over-gate count | unit | `cargo test -p histogram overflow_counted` | no W0 | pending |
| 01-06-T | 01-06 | 3 | BENCH-06 | - | Run marked `contaminated` is retained and rendered, not dropped | unit | `cargo test -p metrics contaminated_run_retained` | no W0 | pending |
| 01-06-T | 01-06 | 3 | BENCH-06 | - | Losing configuration appears in generated report output | unit | `cargo test -p metrics losing_config_rendered` | no W0 | pending |
| 01-06-T | 01-06 | 3 | BENCH-08 | - | Metrics JSON carries p50/p95/p99 per stage and validates against schema | unit | `cargo test -p metrics schema_roundtrip` | no W0 | pending |
| 01-06-T | 01-06 | 3 | BENCH-08 | T-1-04 | Regression check fails when p99 exceeds baseline threshold | integration | `cargo test -p metrics regression_gate` | no W0 | pending |
| 01-06-T | 01-06 | 3 | BENCH-08 | - | Missed week recorded as an explicit gap, not backfilled (D-08) | unit | `cargo test -p metrics missed_week_records_gap` | no W0 | pending |
| 01-05-T | 01-05 | 2 | PLAT-02 | T-1-02 | Precondition assertions refuse the run on a violated fixture state | integration | `cargo test -p capture preconditions_refuse_on_violation` | no W0 | pending |
| 01-05-T | 01-05 | 2 | PLAT-02 | - | Every precondition result is recorded in the manifest, pass or fail | unit | `cargo test -p capture all_precondition_results_recorded` | no W0 | pending |
| 01-06-T | 01-06 | 3 | PLAT-03 | - | Report renders total max AND kernel contribution above firmware floor (D-22) | unit | `cargo test -p metrics plat03_report_decomposition` | no W0 | pending |
| 01-03-T | 01-03 | 1 | D-16 | - | Reconstructed manifest carries `reconstructed` provenance tier, tier non-optional | unit | `cargo test -p manifest provenance_tier_required` | no W0 | pending |

*Two rows above no longer describe the shipped code. The crates were named `nr-manifest`,
`nr-histogram`, `nr-capture`, `nr-metrics` and `nr-cli` in plan 01-01, so the `-p manifest`,
`-p histogram`, `-p capture` and `-p metrics` selectors do not resolve; and the D-22 decomposition
in the last PLAT-03 row was withdrawn by finding 1 of the external audit, which is why plan 01-13
now forbids a subtraction. Both are recorded here rather than rewritten, because these rows are
the historical record of what was asked for before execution. `cargo test --workspace` is green as
of 2026-09-05.*

### Pending plans (01-12 through 01-23, waves 8 through 18)

One row per task, in wave order, recording what actually verifies each task. `gate` is
`./target/release/nrmeasure verify --strict --check-index`. Commands are abbreviated to the
selectors that matter; the plan's `<verify>` block is authoritative.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 01-16-T1 | 01-16 | 8 | BENCH-04 | T-1-46 | Harness identity is the executable's blake3 plus a build-time sha, not the working directory; a build that could not read git records `unavailable` rather than a clean tree nobody observed | unit + integration | `cargo test -p nr-manifest && cargo test -p nr-cli --test run_pipeline` + gate | yes | pending |
| 01-16-T2 | 01-16 | 8 | BENCH-04 | T-1-48 | The executed argv is preserved with its home prefix redacted, and each path is mapped to the artifact it became | unit + integration | `cargo test -p nr-manifest && cargo test -p nr-cli --test run_pipeline && cargo test --workspace` + gate | task | pending |
| 01-16-T3 | 01-16 | 8 | BENCH-05 | T-1-51 | Every published prose figure is re-derived from the calibration manifest it is quoted from | integration | `cargo test -p nr-capture --test published_figures` | task | pending |
| 01-19-T1 | 01-19 | 8 | BENCH-05 | T-1-52 | A statistic that could not be computed is recorded unavailable, never as a substituted zero | unit + integration | `cargo test -p nr-metrics && cargo test -p nr-cli --test verify && git diff --exit-code measurements/INDEX.md` + gate | yes | pending |
| 01-19-T2 | 01-19 | 8 | BENCH-04 | T-1-53 | Strict verification recomputes REPORT.md and hist.tsv from the raw capture instead of trusting them | integration | `cargo test -p nr-cli --test verify` + gate | yes | pending |
| 01-19-T3 | 01-19 | 8 | BENCH-05 | T-1-57 | Sample counts reconcile across the two parsers, and a claim with no committed capture behind it says so | unit | `cargo test -p nr-histogram && grep -q 'not saved and is not in this repository' measurements/2026-08-28-precision3591/README.md` + gate | task | pending |
| 01-17-T1 | 01-17 | 9 | BENCH-04 | T-1-58 | The attempt record is written before the first instrument starts, so a run that dies mid-capture still leaves evidence | unit + integration | `cargo test -p nr-manifest && cargo test -p nr-cli --test run_pipeline` | task | pending |
| 01-17-T2 | 01-17 | 9 | BENCH-06 | T-1-59 | A failed attempt is published in INDEX.md rather than dropped | integration | `cargo test -p nr-metrics && cargo test -p nr-cli --test verify` + gate | yes | pending |
| 01-17-T3 | 01-17 | 9 | BENCH-04 | T-1-63 | Interference counters bracket one instrument each, on a monotonic clock | unit + integration | `cargo test -p nr-manifest && cargo test -p nr-capture && cargo test --workspace` + gate | yes | pending |
| 01-18-T1 | 01-18 | 10 | PLAT-02 | T-1-64 | DeepCstatesDisabled reads the target CPUs and TracersQuiescent reads all four tracing controls, so each gate establishes what its name claims | unit | `cargo test -p nr-capture && cargo test -p nr-capture --test protocol_doc` | task | pending |
| 01-18-T2 | 01-18 | 10 | BENCH-04 | T-1-66 | A fixture-driven run names its fixtures in the manifest and cannot reach the series | unit + integration | `cargo test -p nr-manifest && cargo test -p nr-cli --test run_pipeline` + gate | yes | pending |
| 01-18-T3 | 01-18 | 10 | PLAT-02 | T-1-67 | The thermal exemption follows a declared profile, and the protocol document states the ceiling the code enforces | unit + integration | `cargo test -p nr-capture --test protocol_doc && cargo test --workspace` + gate | task | pending |
| 01-22-T1 | 01-22 | 11 | PLAT-02 | T-1-70 | Every root script is under version control, and the sudoers rule names fixed absolute paths owned by root | static | `sh -n deploy/sudoers/install.sh && sh -n scripts/nr-run-measurement && grep -c NOPASSWD deploy/sudoers/nr-measurement` | yes | pending |
| 01-22-T2 | 01-22 | 11 | PLAT-02 | T-1-74 | msr-tools and stress-ng installed, and both instruments' real output captured before a parser is written against it | **rig-only** | `MISSING` - verified by 01-22-T3 | rig | pending |
| 01-22-T3 | 01-22 | 11 | PLAT-02 | T-1-75 | Both probes are committed, non-empty and ASCII, inside the existing recon exemption rather than a new one | static | `test -s docs/rig/recon-*/probe-rtla-hwnoise.txt && test -s docs/rig/recon-*/probe-rdmsr-smi-count.txt` + gate | task | pending |
| 01-20-T1 | 01-20 | 12 | BENCH-04 | T-1-78 | Firmware fields, schema and fixture change together, and `observed_cpus` is not optional | unit | `cargo test -p nr-manifest && cargo test -p nr-cli` + gate | task | pending |
| 01-20-T2 | 01-20 | 12 | PLAT-03 | T-1-77 | The rtla hwnoise parser is written against the committed probe, not against guessed output | unit | `cargo test -p nr-capture --test hwnoise && cargo clippy -p nr-capture --all-targets -- -D warnings` | task | pending |
| 01-20-T3 | 01-20 | 12 | BENCH-06 | T-1-81 | Every firmware capture already committed is checked for CPU coverage, and none is silently rewritten | unit | `cargo test -p nr-capture --test firmware_cpu_coverage && git diff --exit-code measurements/` | task | pending |
| 01-21-T1 | 01-21 | 13 | PLAT-03 | T-1-87 | `nrmeasure run` takes an rtla hwnoise screen, and the two firmware instruments cannot be conflated in one run | integration | `cargo test -p nr-cli --test run_pipeline && cargo test --workspace` | yes | pending |
| 01-21-T2 | 01-21 | 13 | BENCH-04 | T-1-84 | An exact per-CPU SMI count brackets every run, and a failed read is recorded as failed rather than as zero | unit + integration | `cargo test -p nr-capture --test smi && cargo test -p nr-cli --test run_pipeline` + gate | task | pending |
| 01-21-T3 | 01-21 | 13 | PLAT-03 | T-1-85 | A published firmware figure states which instrument measured it and over which CPUs | unit + snapshot | `cargo test -p nr-metrics && cargo test -p nr-cli --test run_pipeline && cargo test -p nr-capture --test protocol_doc` + gate | yes | pending |
| 01-23-T1 | 01-23 | 14 | PLAT-03 | T-1-91 | Three rtla hwnoise screens on CPUs 6-11 (idle, loaded, housekeeping), each with a per-CPU SMI delta | **rig-only**, post-hoc assertion | gate + a `python3` assertion that three committed manifests carry `instrument: rtla-hwnoise` and name which of CPUs 6-11 reported nothing | rig | pending |
| 01-23-T2 | 01-23 | 14 | BENCH-06 | T-1-90 | The one-CPU figure is corrected where it was published, not superseded by a newer document beside it | integration + static | `cargo test -p nr-capture --test firmware_cpu_coverage && grep -q 'rtla hwnoise' docs/rig/firmware-floor-rt-vs-stock.md` + gate | task | pending |
| 01-23-T3 | 01-23 | 14 | PLAT-03 | T-1-94 | A human reads the corrected firmware record before it is published | **manual review**, static re-check | the 01-23-T2 greps re-run + gate | task | pending |
| 01-12-T1 | 01-12 | 15 | PLAT-01 | T-1-97 | The tracer is armed with a declared event set, the buffer is proven to hold it, and both thresholds are calibrated before the first cycle | **rig-only**, post-hoc assertion | gate + a `python3` assertion that the investigation manifest records the event set, the buffer, the traced maximum, breaktrace and CPUs 6-11, and that `tracers-quiescent` is `not-applicable` | rig | pending |
| 01-12-T2 | 01-12 | 15 | PLAT-01 | T-1-98 | Two further capture-and-analyse cycles, inside the D-20 budget of three, each leaving a note | **rig-only**, post-hoc assertion | gate + a `python3` assertion that two or three investigation manifests exist and each carries a non-empty note | rig | pending |
| 01-12-T3 | 01-12 | 15 | PLAT-01 | T-1-34 | The outcome is published as `named`, `not-reproduced` or `open-question`, and the attribution the captures cannot support is withdrawn | static | `grep -qE '^Outcome: (named\|not-reproduced\|open-question)$' docs/rig/plat01-stall-investigation.md` + gate | task | pending |
| 01-13-T1 | 01-13 | 16 | PLAT-03 | T-1-101 | The four hour headline capture runs with all fifteen preconditions passing, cyclictest alone, and no firmware screen mixed in | **rig-only**, post-hoc assertion | gate + a `python3` assertion on `run_class`, `instrument_class`, `provenance_tier`, fifteen passing preconditions and an empty `firmware_screens` | rig | pending |
| 01-13-T2 | 01-13 | 16 | PLAT-03, BENCH-05 | T-1-38 | The verdict reports the residual as unattributed and contains no subtraction and no kernel-contribution line | static | `grep -qE '^Verdict: ' && ! grep -qi 'kernel contribution\\|worst-case bound' docs/rig/plat03-scheduling-latency-verdict.md` + gate | task | pending |
| 01-13-T3 | 01-13 | 16 | PLAT-03 | T-1-18 | A human reads the headline figure before it is published | **manual review**, static re-check | the 01-13-T2 greps re-run + gate | task | pending |
| 01-14-T1 | 01-14 | 17 | BENCH-08 | T-1-103 | `nrmeasure series` carries p50, p95 and p99 per stage with nullable statistics, so an uncomputed figure is absent rather than zero | unit + integration | `cargo test -p nr-metrics && cargo test -p nr-cli --test series && cargo test --workspace` | task | pending |
| 01-14-T2 | 01-14 | 17 | BENCH-08 | T-1-40 | The oneshot is bounded by `TimeoutStartSec`, the timer does not backfill a missed week, and the wrapper takes the hwnoise screen | static | `sh -n deploy/systemd/neurorust-weekly-run.sh && grep -q 'Persistent=false' deploy/systemd/neurorust-measure.timer` | task | pending |
| 01-14-T3 | 01-14 | 17 | BENCH-08 | T-1-04 | The regression gate resolves an event-specific base revision, never a merge base, and refuses a baseline change authored by the rig identity | integration | a `python3` assertion over `.github/workflows/regression.yml` and the seeded baseline, then `nrmeasure series --compare` | task | pending |
| 01-15-T1 | 01-15 | 18 | BENCH-08 | T-1-01 | A push credential scoped to this repository exists on the rig, and the unattended checkout is deployed | **rig-only** | `MISSING` - verified by 01-15-T2 | rig | pending |
| 01-15-T2 | 01-15 | 18 | BENCH-08 | T-1-105 | One full weekly loop driven by hand reaches the repository: fifteen preconditions, cyclictest and rtla, the run present in the series | **rig-only**, post-hoc assertion | `git pull --ff-only && cargo build -p nr-cli --release` + gate + a `python3` assertion on the weekly manifest and the series | rig | pending |
| 01-15-T3 | 01-15 | 18 | BENCH-08, PLAT-02 | T-1-44 | The first scheduled fire is observed with nobody watching, and the week is recorded as a run or as an explicit gap | **rig-only**, post-hoc assertion | `git pull --ff-only` + gate + a `python3` assertion that the most recent coverage week is a run or a gap | rig | pending |

*Status: pending / green / red / flaky*

*File Exists: `yes` the command runs against infrastructure already in the repository, `task` the
file the command reads is produced by that same task, `rig` the artifact the command asserts on is
produced on the Dell Precision 3591 and committed by the operator.*

---

## Wave 0 Requirements

Complete. Plan 01-01 executed in wave 0 and `cargo test --workspace` is green as of 2026-09-05.

- [x] Cargo workspace at repo root with the crate split from RESEARCH.md, `license = "Apache-2.0 OR MIT"`
- [x] `crates/histogram/tests/fixtures/cyclictest-rt-isolated-idle-10m.hist` - seeded from the
      existing capture so the parser is tested against a real input with a known-hard case
      (888 overflows, the exact case the 2026-08-28 README got wrong)
- [x] `crates/capture/tests/fixtures/` - captured `/proc/interrupts` and sysfs snapshots so
      precondition and interference-diff logic is testable without a rig
- [x] `.github/workflows/ci.yml` - Linux and macOS legs, fmt + clippy + test
- [x] `cargo test --workspace` green on the empty workspace before any feature work

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| The named kernel path is correct | PLAT-01 | Requires the rig, a reproducing capture, and human reading of a trace | 01-12 tasks 1 and 2: follow the investigation runbook; run the tracing-overhead calibration first; commit the trace next to the claim. Budget: 3 cycles (D-20). |
| Worst-case latency under 30 us, or an attributed residual | PLAT-03 | Requires a clean rig run of the stated duration under the documented protocol | 01-13 task 1: run the headline protocol on the rig and read the verdict from the generated report. The scheduling figure and the firmware figure are reported side by side, attributed to their source runs, and are not combined: no subtraction and no kernel-contribution line, per finding 1 of the external audit, which withdrew the D-22 decomposition. |
| A third party can reproduce from the protocol document | PLAT-02 | Requires a human following prose end to end | Have a reader work through `docs/measurement-protocol.md` against a clean rig without asking questions |
| Firmware floor delta on RT vs stock kernel | D-18 | Requires the rig plus comparison against archived live-USB figures | 01-23 tasks 1 and 2: `hwlatdetect` cannot sample the isolated cores on this kernel (established by plan 01-11), so the re-take uses `rtla hwnoise -c 6-11 -H 0-5` with `rdmsr -p <cpu> 0x34` bracketing each arm. Publish the delta, correct the published one-CPU figures where they were made, and carry the interpretation note from RESEARCH.md. |
| README corrections are accurate | D-23 | Requires re-deriving figures from the raw histogram by hand | Verify the corrected over-gate count (2,093 of 17,995,844, 0.0116%, boundary >= 30 us) and the removal of the bimodality claim against the committed `.hist` |
| Published manifest exposes no unintended host identifiers | security | Requires human judgement about what is acceptable to publish | Review the first generated manifest field by field before the first push |
| The sudoers grant is safe and the root scripts run | PLAT-02 | Installing a NOPASSWD rule and validating it needs root on the machine | 01-22 task 1: copy `nr-recon` and `nr-probe` back into the repository, install every script `-o root -g root -m 0755`, validate the rule with `visudo -c -f` on a temporary copy before activating it, then confirm each grant runs without a password prompt |
| rtla hwnoise and rdmsr produce the output the parser expects | PLAT-02 | Both instruments exist only on the rig | 01-22 task 2: install msr-tools and stress-ng, run both instruments, and commit the raw output under `docs/rig/recon-*/probe-*` after grepping it for the operator username, `/home/` and the rig hostname |
| The firmware floor across the isolated cores | PLAT-03, BENCH-06 | Three 900 second screens on the rig under idle, loaded and housekeeping load | 01-23 task 1: run the three arms under the harness. A CPU absent from `rtla hwnoise` output was sampled and reported nothing, which is the distinction the eight committed `hwlatdetect` captures could not make. |
| The corrected firmware record reads as a correction | PLAT-03 | Human judgement about whether a withdrawn claim is visibly withdrawn | 01-23 task 3: read `docs/rig/firmware-floor-rt-vs-stock.md` and the 2026-08-28 README end to end before pushing |
| The headline verdict is fairly worded | PLAT-03 | Human judgement about the wording of a published figure | 01-13 task 3: read `docs/rig/plat03-scheduling-latency-verdict.md` and confirm the residual is called unattributed |
| The weekly job is deployed on the rig | BENCH-08 | Creating a scoped credential and a checkout on the machine | 01-15 task 1: create the push credential scoped to this repository, deploy the unattended checkout, and confirm it and the operator's checkout run the same harness build or record the difference |
| One full weekly loop closes by hand | BENCH-08 | The systemd units have to actually run on the rig | 01-15 task 2: install and enable the units, drive one run end to end, and confirm it reaches the repository and the series. Seed the weekly baseline in a separate commit authored by a human, because the regression gate refuses a baseline authored by the rig identity. |
| The first scheduled fire happens unattended | BENCH-08, PLAT-02 | Requires waiting a week for the timer with nobody watching | 01-15 task 3: confirm the coverage record carries the week as a run or as an explicit gap (D-08), and that the journal carries no `RuntimeMaxSec= has no effect` line |

---

## Rig-only verification convention

Some steps in this phase cannot be performed or observed from the dev host at all: they install a
package on the Dell Precision 3591, read a model-specific register, arm a kernel tracer, or wait a
week for a systemd timer to fire. Those steps still carry an `<automated>` slot, in one of two
forms. Neither form is permission to skip verification.

**Post-hoc artifact assertion.** The command runs on the dev host after the operator has committed
what the rig produced, and asserts against that artifact: the gate, plus a `python3` block that
opens the committed `manifest.json` and fails on the specific fields the task was supposed to
establish. Used by 01-12 tasks 1 and 2, 01-13 task 1, 01-15 tasks 2 and 3, and 01-23 task 1. The
rig work is not watched while it happens; it is verified by the evidence it is required to leave
behind, which is the standard every published figure in this phase is already held to.

**Deferred to a later task in the same plan.** The slot reads
`MISSING - rig-only. Verified by task N ...` and names the task carrying the real check. Only two
tasks use this: 01-15 task 1 (the push credential, verified by task 2 when the first push reaches
the repository) and 01-22 task 2 (the two instrument probes, verified by task 3 when both
committed probe files are parsed and required to be non-empty).

A `MISSING` slot is acceptable only when all three of these hold:

1. the step cannot be performed or observed from the dev host,
2. a later task in the same plan asserts the state the step was supposed to create, and
3. the plan is `autonomous: false`, so a human is at the checkpoint and reads the acceptance
   criteria before the plan advances.

Both plans that use it satisfy all three, and neither produces three consecutive tasks without an
automated verify.

---

## Validation Sign-Off

- [ ] All tasks have automated verify or a Wave 0 dependency
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING fixture references
- [ ] No watch-mode flags in any test command
- [ ] Feedback latency < 90s
- [ ] Rig-only verifications are explicitly listed as manual, not disguised as automated
- [ ] Every `MISSING` slot names the later task in the same plan that verifies it, in a plan that is `autonomous: false`
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-08-30, plans 01-01 through 01-15; re-approved 2026-09-05 for the
expanded phase, plans 01-01 through 01-23, waves 0 through 18.
