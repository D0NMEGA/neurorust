# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-08-28)

**Core value:** The safety-critical abort path is proven correct rather than tested, and every latency claim is reproducible from published raw captures on a named rig.
**Current focus:** Phase 1 - Trustworthy measurement

## Current Position

Phase: 1 of 8 (Trustworthy measurement)
Plan: 0 of TBD in current phase
Status: Ready to plan
Last activity: 2026-08-28 - Roadmap created, 59 v1 requirements mapped across 8 phases

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

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- Rig: Dell Precision 3591 chosen and tuned; RIG-01 through RIG-04 are validated, so the roadmap starts at Phase 1 with the first unbuilt work
- Roadmap: sequenced by risk for a single operator. Proof (Phase 2) and methodology (Phase 1) precede the runtime; the wire protocol (Phase 8) and the pylsl shim (Phase 7) are last because they are first to cut
- PLAT-01/02/03 made an early gate: every later latency figure would otherwise inherit an unexplained 3.8 ms stall

### Pending Todos

None yet.

### Blockers/Concerns

- Open question, Phase 1: a ~3.8 ms global stall appeared on all six isolated threads at nearly identical values (stop_machine or a system-wide TLB shootdown signature). Unexplained. No latency figure should be published until it is named.
- Contaminated baseline: `cyclictest-rt-isolated-idle-10m.hist` was taken with SSH activity and an active GNOME session. Not publishable; re-run under the Phase 1 protocol with `--tracemark` and ftrace armed.
- Doc inconsistency: PROJECT.md Constraints still says "software timestamping only", which the corrected Out of Scope entry (PTP hardware timestamps back in scope, WIRE-06) supersedes. Fix at the next PROJECT.md update.
- WIRE-06 needs an ethernet cable on `enp0s31f6`; the wifi adapter has no PTP clock.

## Session Continuity

Last session: 2026-08-28
Stopped at: ROADMAP.md and STATE.md written, REQUIREMENTS.md traceability populated
Resume file: None

Next: `/donny-plan-phase 1`
