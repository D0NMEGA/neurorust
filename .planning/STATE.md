---
donny_state_version: 1.0
milestone: v1.0
milestone_name: milestone
status: executing
stopped_at: Completed 01-02-PLAN.md
last_updated: "2026-08-31T05:50:19.145Z"
last_activity: 2026-08-31
progress:
  total_phases: 8
  completed_phases: 0
  total_plans: 15
  completed_plans: 3
  percent: 20
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-08-28)

**Core value:** The safety-critical abort path is proven correct rather than tested, and every latency claim is reproducible from published raw captures on a named rig.
**Current focus:** Phase 01 — trustworthy-measurement

## Current Position

Phase: 01 (trustworthy-measurement) — EXECUTING
Plan: 4 of 15
Status: Ready to execute
Last activity: 2026-08-31

Progress: [██░░░░░░░░] 20%

## Performance Metrics

**Velocity:**

- Total plans completed: 0
- Average duration: n/a
- Total execution time: 0.0 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| - | - | - | - |

**Recent Trend:**

- Last 5 plans: n/a
- Trend: n/a

*Updated after each plan completion*
| Phase 01 P01 | 11min | 3 tasks | 20 files |
| Phase 01 P03 | 6min | 3 tasks | 12 files |
| Phase 01 P02 | 25min | 3 tasks | 15 files |

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- Rig: Dell Precision 3591 chosen and tuned; RIG-01 through RIG-04 are validated, so the roadmap starts at Phase 1 with the first unbuilt work
- Roadmap: sequenced by risk for a single operator. Proof (Phase 2) and methodology (Phase 1) precede the runtime; the wire protocol (Phase 8) and the pylsl shim (Phase 7) are last because they are first to cut
- PLAT-01/02/03 made an early gate: every later latency figure would otherwise inherit an unexplained 3.8 ms stall
- [Phase 01]: 01-01: Symlinked /opt/homebrew/bin/cargo-fmt to the Homebrew rustup formula's own cargo-fmt binary (host-local fix, not a repo change) — cargo fmt --check failed with no such command: fmt even though rustfmt was an installed rustup component. Homebrew's rustup formula symlinks cargo, cargo-clippy, rustc, rustdoc, rustfmt, rustup into /opt/homebrew/bin but omits cargo-fmt. Does not affect CI, which installs Rust via the official rustup action.
- [Phase 01]: 01-01: deny.toml bans section gets allow-wildcard-paths = true, and all five nr-* crates get publish = false — cargo-deny flagged every internal workspace path dependency as a wildcard dependency (bans FAILED). allow-wildcard-paths is cargo-deny's documented exemption for this pattern, but only applies to crates marked publish = false, which is also correct on its own since none of these crates publish to crates.io independently. Verified: cargo deny check advisories licenses bans sources exits 0.
- [Phase 01]: 01-03: kernel.cmdline redacts root= and resume= values to the literal token [redacted]; every other parameter (BOOT_IMAGE, isolcpus, nohz_full, rcu_nocbs, irqaffinity, intel_pstate, etc.) stays verbatim — Human review (Task 3 checkpoint:human-verify) approved this after reading the generated schema and worked-example fixture. The root/swap filesystem UUIDs carry zero reproduction value and are the only machine-instance identifiers in the manifest; redaction is visible (the literal token appears) rather than a silent drop. Implemented as KernelInfo::redact_cmdline, unit tested against a realistic cmdline including a resume_offset near-miss. Does not touch D-12 (raw captures).
- [Phase 01]: 01-03: tools[].argv documented as run-directory-relative rather than absolute home-directory paths; actual path rewrite deferred to nr-capture (plan 01-05) — Human review (Task 3 checkpoint:human-verify) approved this so a recorded argv is a command a third party can actually paste and run. nr-capture does not exist yet and owns the actual rewrite at capture time; nr-manifest only documents the convention in the doc comment on ToolInvocation.argv and carries it forward in deferred-items.md for plan 01-05 to implement.
- [Phase 01]: 01-02: Executed all three tasks directly over SSH instead of stopping at checkpoint:human-action — PLAN.md's task type assumed only a human at the rig console could run these commands. Passwordless SSH plus two fixed, argument-free sudo scripts (nr-recon, nr-probe) were set up after the plan was written, making direct execution possible with no architectural change.
- [Phase 01]: 01-02: Root-caused the rt-tuning.service/powersave contradiction: power-profiles-daemon wins a boot-time race — GDM's own greeter session D-Bus-activates power-profiles-daemon within the same boot second as rt-tuning.service; ppd's performance profile is expressed as scaling_governor=powersave plus energy_performance_preference=performance on this HWP backend, never as the legacy governor=performance override rt-tuning.sh writes, so the rig silently boots untuned. Recorded in FINDINGS.md and deferred-items.md, not fixed, per explicit instruction not to mutate rig state.
- [Phase 01]: 01-02: T-1-06 hostname redaction applied beyond the threat model's named three-file list — cyclictest --json's sysinfo.nodename field, plus raw uname -a and journalctl lines captured into two recon text files, all carried the real hostname; none of the three files were in T-1-06's component list, but its own mitigation text says to grep the probe files generally. Redacted with the same visible [redacted] token KernelInfo::redact_cmdline uses.
- [Phase 01]: 01-02: rtla needs no source build on this rig; ships inside linux-tools-common (v7.0.12) — Corrects RESEARCH.md's assumption that rtla would need a from-source build against the kernel tree. dpkg -S confirms /usr/bin/rtla is owned by the already-installed linux-tools-common package. Settles plan 01-12's PLAT-01 attribution instrument as rtla timerlat with no build step.
- [Phase 01]: 01-02: roadmap update-plan-progress silently no-ops on zero-padded phase args; hand-corrected ROADMAP.md and STATE.md's stale body progress line — roadmap.cjs's table-row regex requires the row to start with the zero-padded arg (01.), but ROADMAP.md's own table/heading use unpadded numbers (1., Phase 1:), so String.replace finds no match while the function still reports updated:true. Same class of format-mismatch bug 01-03 found in state.cjs's progress-bar regex, recurring because the shared CLI was not patched. Not patched here either (out of scope for this plan); the specific stale cells were corrected by hand.

### Pending Todos

None yet.

### Blockers/Concerns

- Open question, Phase 1: a ~3.8 ms global stall appeared on all six isolated threads at nearly identical values (stop_machine or a system-wide TLB shootdown signature). Unexplained. No latency figure should be published until it is named.
- Contaminated baseline: `cyclictest-rt-isolated-idle-10m.hist` was taken with SSH activity and an active GNOME session. Not publishable; re-run under the Phase 1 protocol with `--tracemark` and ftrace armed.
- Doc inconsistency: PROJECT.md Constraints still says "software timestamping only", which the corrected Out of Scope entry (PTP hardware timestamps back in scope, WIRE-06) supersedes. Fix at the next PROJECT.md update.
- WIRE-06 needs an ethernet cable on `enp0s31f6`; the wifi adapter has no PTP clock.
- Rig boots untuned: every CPU governor reads powersave despite rt-tuning.service reporting active/enabled (power-profiles-daemon wins a boot-time race, see docs/rig/recon-2026-08-31/FINDINGS.md). Any plan that runs the real measurement protocol must fix this or rely on nr-capture's own precondition check; it must not trust rt-tuning.service's ActiveState.

## Session Continuity

Last session: 2026-08-31T05:48:05.832Z
Stopped at: Completed 01-02-PLAN.md
Resume file: None

Next: `/donny-plan-phase 1`
