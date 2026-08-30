---
donny_state_version: 1.0
milestone: v1.0
milestone_name: milestone
status: executing
stopped_at: Completed 01-01-PLAN.md
last_updated: "2026-08-30T22:11:44.174Z"
last_activity: 2026-08-30
progress:
  total_phases: 8
  completed_phases: 0
  total_plans: 15
  completed_plans: 1
  percent: 7
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-08-28)

**Core value:** The safety-critical abort path is proven correct rather than tested, and every latency claim is reproducible from published raw captures on a named rig.
**Current focus:** Phase 01 — trustworthy-measurement

## Current Position

Phase: 01 (trustworthy-measurement) — EXECUTING
Plan: 2 of 15
Status: Ready to execute
Last activity: 2026-08-30

Progress: [░░░░░░░░░░] 0%

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

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- Rig: Dell Precision 3591 chosen and tuned; RIG-01 through RIG-04 are validated, so the roadmap starts at Phase 1 with the first unbuilt work
- Roadmap: sequenced by risk for a single operator. Proof (Phase 2) and methodology (Phase 1) precede the runtime; the wire protocol (Phase 8) and the pylsl shim (Phase 7) are last because they are first to cut
- PLAT-01/02/03 made an early gate: every later latency figure would otherwise inherit an unexplained 3.8 ms stall
- [Phase 01]: 01-01: Symlinked /opt/homebrew/bin/cargo-fmt to the Homebrew rustup formula's own cargo-fmt binary (host-local fix, not a repo change) — cargo fmt --check failed with no such command: fmt even though rustfmt was an installed rustup component. Homebrew's rustup formula symlinks cargo, cargo-clippy, rustc, rustdoc, rustfmt, rustup into /opt/homebrew/bin but omits cargo-fmt. Does not affect CI, which installs Rust via the official rustup action.
- [Phase 01]: 01-01: deny.toml bans section gets allow-wildcard-paths = true, and all five nr-* crates get publish = false — cargo-deny flagged every internal workspace path dependency as a wildcard dependency (bans FAILED). allow-wildcard-paths is cargo-deny's documented exemption for this pattern, but only applies to crates marked publish = false, which is also correct on its own since none of these crates publish to crates.io independently. Verified: cargo deny check advisories licenses bans sources exits 0.

### Pending Todos

None yet.

### Blockers/Concerns

- Open question, Phase 1: a ~3.8 ms global stall appeared on all six isolated threads at nearly identical values (stop_machine or a system-wide TLB shootdown signature). Unexplained. No latency figure should be published until it is named.
- Contaminated baseline: `cyclictest-rt-isolated-idle-10m.hist` was taken with SSH activity and an active GNOME session. Not publishable; re-run under the Phase 1 protocol with `--tracemark` and ftrace armed.
- Doc inconsistency: PROJECT.md Constraints still says "software timestamping only", which the corrected Out of Scope entry (PTP hardware timestamps back in scope, WIRE-06) supersedes. Fix at the next PROJECT.md update.
- WIRE-06 needs an ethernet cable on `enp0s31f6`; the wifi adapter has no PTP clock.

## Session Continuity

Last session: 2026-08-30T22:11:44.172Z
Stopped at: Completed 01-01-PLAN.md
Resume file: None

Next: `/donny-plan-phase 1`
