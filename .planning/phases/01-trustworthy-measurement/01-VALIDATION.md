---
phase: 1
slug: trustworthy-measurement
status: approved
nyquist_compliant: true
wave_0_complete: false
created: 2026-08-30
---

# Phase 1 - Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from `01-RESEARCH.md` "## Validation Architecture".

**Phase-specific caveat:** this phase's requirements are mostly about artifacts, provenance and a
rig measurement, not about runtime behaviour. Several requirements genuinely cannot be verified on
the dev host. The tables below separate automated coverage from rig-only verification honestly
rather than claiming a unit test covers a physical measurement. The planner must not paper over
that split.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `#[test]`, plus `insta` 1.48.0 (snapshots) and `assert_cmd` 2.2.2 + `predicates` 3.1.4 (CLI) |
| **Config file** | none - `cargo test` is the entry point. Wave 0 creates the workspace. |
| **Quick run command** | `cargo test --workspace` |
| **Full suite command** | `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` |
| **Estimated runtime** | ~30 seconds quick, ~90 seconds full on a cold cache |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --workspace`
- **After every plan wave:** Run `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
- **Before `/donny-verify-work`:** Full suite green, AND the provenance gate green against the
  real `measurements/` tree including the D-16 reconstructed manifests
- **Max feedback latency:** 90 seconds

---

## Per-Task Verification Map

Task IDs are assigned by the planner. This table states the required coverage per requirement;
the planner fills the Task ID and Plan columns when plans are written.

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
| 01-12-T | 01-12 | 8 | PLAT-01 | - | Named kernel path with committed trace | **manual, rig-only** | not automatable | n/a | pending |

*Status: pending / green / red / flaky*

---

## Wave 0 Requirements

- [ ] Cargo workspace at repo root with the crate split from RESEARCH.md, `license = "Apache-2.0 OR MIT"`
- [ ] `crates/histogram/tests/fixtures/cyclictest-rt-isolated-idle-10m.hist` - seeded from the
      existing capture so the parser is tested against a real input with a known-hard case
      (888 overflows, the exact case the 2026-08-28 README got wrong)
- [ ] `crates/capture/tests/fixtures/` - captured `/proc/interrupts` and sysfs snapshots so
      precondition and interference-diff logic is testable without a rig
- [ ] `.github/workflows/ci.yml` - Linux and macOS legs, fmt + clippy + test
- [ ] `cargo test --workspace` green on the empty workspace before any feature work

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| The named kernel path is correct | PLAT-01 | Requires the rig, a reproducing capture, and human reading of a trace | Follow the investigation runbook; run the tracing-overhead calibration first; commit the trace next to the claim. Budget: 3 cycles (D-20). |
| Worst-case latency under 30 us, or an attributed residual | PLAT-03 | Requires a clean rig run of the stated duration under the documented protocol | Run the headline protocol on the rig; read the verdict from the generated report; decompose against the firmware floor per D-22 |
| A third party can reproduce from the protocol document | PLAT-02 | Requires a human following prose end to end | Have a reader work through `docs/measurement-protocol.md` against a clean rig without asking questions |
| Firmware floor delta on RT vs stock kernel | D-18 | Requires the rig plus comparison against archived live-USB figures | Run hwlatdetect under the harness on the installed RT system; publish the delta and the interpretation note from RESEARCH.md |
| README corrections are accurate | D-23 | Requires re-deriving figures from the raw histogram by hand | Verify the corrected over-gate count (2,093 of 17,995,844, 0.0116%, boundary >= 30 us) and the removal of the bimodality claim against the committed `.hist` |
| Published manifest exposes no unintended host identifiers | security | Requires human judgement about what is acceptable to publish | Review the first generated manifest field by field before the first push |

---

## Validation Sign-Off

- [ ] All tasks have automated verify or a Wave 0 dependency
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING fixture references
- [ ] No watch-mode flags in any test command
- [ ] Feedback latency < 90s
- [ ] Rig-only verifications are explicitly listed as manual, not disguised as automated
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-08-30, plans 01-01 through 01-15
