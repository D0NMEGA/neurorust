---
donny_state_version: 1.0
milestone: v1.0
milestone_name: milestone
status: executing
stopped_at: Completed 01-07-PLAN.md
last_updated: "2026-08-31T16:11:48.755Z"
last_activity: 2026-08-31
progress:
  total_phases: 8
  completed_phases: 0
  total_plans: 15
  completed_plans: 7
  percent: 47
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-08-28)

**Core value:** The safety-critical abort path is proven correct rather than tested, and every latency claim is reproducible from published raw captures on a named rig.
**Current focus:** Phase 01 — trustworthy-measurement

## Current Position

Phase: 01 (trustworthy-measurement) — EXECUTING
Plan: 8 of 15
Status: Ready to execute
Last activity: 2026-08-31

Progress: [█████░░░░░] 47%

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
| Phase 01 P04 | 18min | 3 tasks | 7 files |
| Phase 01 P05 | 50min | 3 tasks | 12 files |
| Phase 01 P06 | 11min | 3 tasks | 15 files |
| Phase 01 P07 | 25min | 3 tasks | 15 files |

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
- [Phase 01]: 01-04: Histogram thread count derived from "# Min Latencies:", not "# Max Latencies:" as the plan's own action text stated — FINDINGS.md's empirically measured "Column counts" section (real -h/-H probes from plan 01-02) shows "# Max Latencies:" and "# Histogram Overflows:" both gain the -H summary column (6 to 7 fields for 6 threads), while "# Min Latencies:" and "# Avg Latencies:" stay thread-count-only in both layouts. The plan's action text claimed Max Latencies is always exactly one value per thread; followed the real rig data instead, per this plan's own instruction to trust real captures over an assumed schema.
- [Phase 01]: 01-04: Added parse_hist_file(&Path, Option<u64>) and parse_json_file(&Path) ahead of being required by this plan's own task acceptance criteria — The plan's <interfaces> block names these as what plan 01-07 (nr-cli) will call, framed as "the public surface right first time." Neither is in any task's action text or acceptance criteria, but they are thin wrappers (a few lines each) around parse_hist/serde_json::from_str, and building them now avoids a known future gap. HistError gained an Io variant to carry file-read failures.
- [Phase 01]: 01-04: lib.rs module registration and a HistError enum extension in hist.rs were required outside Tasks 2/3's declared file lists — Task 2/3's <files> lists omit lib.rs, but without pub mod percentiles;/pub mod json; neither module compiles into the crate at all, so the plan's own mandated cargo test -p nr-histogram commands cannot pass. Task 3 also needed HistError::JsonDeserialize/SummaryDisagreement (defined in hist.rs) since reconcile() returns Result<(), HistError> by the plan's own design. Treated as Rule 3 (blocking): the minimum touch needed for each task's own tests to run.
- [Phase 01]: 01-05: procfs 0.18.0 has no /proc/interrupts parser at all; hand-rolled a small parser shared between the live path and every test — Verified by downloading and grepping the published procfs-0.18.0 and procfs-core-0.18.0 crate sources directly: no interrupt-table parsing exists at this pinned version, contrary to RESEARCH.md's assumption. Rather than silently working around it, interference.rs documents the finding and parses the text format itself, used identically by the live path and every test.
- [Phase 01]: 01-05: GovernorIsPerformanceOnAllCpus and TracersQuiescent correctly FAIL/refuse against the real rig's current state; not weakened — Every CPU governor currently reads powersave and GDM is active with a real Wayland session (confirmed live over SSH). The checks correctly refuse a headline-class run against this state, per FINDINGS.md's governor-race root cause. This is the D-06 assertion list doing its job.
- [Phase 01]: 01-05: nr_manifest InterferenceSnapshot.context_switches populated from /proc/interrupts RES row, not /proc/stat ctxt — That field is typed Vec<CpuCounter>, one entry per isolated CPU, but /proc/stat's ctxt is a single machine-wide counter with no per-CPU breakdown and cannot fill it at all. RES (rescheduling interrupts) is the closest true per-CPU scheduling-interference signal available without a heavier tracer.
- [Phase 01]: 01-05: D-15 contamination reason lives in a local VerdictOutcome wrapper, not a new nr_manifest field — nr_manifest::InterferenceSnapshotPair (plan 01-03's already-shipped, schema-generated type) has no reason field. Adding one would touch crates/manifest/, schemas/manifest.schema.json and the committed minimal-manifest.json fixture, none of which are in this plan's file list. interference::verdict returns a local wrapper instead.
- [Phase 01]: 01-05: argv relativization implemented as a standalone module (argv.rs), closing plan 01-03's deferred-items.md carry-forward — None of this plan's three tasks shell out to a tool or build a ToolInvocation themselves (nr-cli, plan 01-07, does that per this plan's own interfaces block). relativize_argv is the standalone, tested utility nr-cli will call, so the rewrite nr_manifest's argv doc comment promises still lives in nr-capture.
- [Phase 01]: 01-06: record_gap added as a generic single-week gap writer alongside the plan's named record_gaps, so GapReason::RefusedOnPrecondition and RunFailed have a real producer — D-08 requires a refused run to produce the same recorded gap a missed week does, and the refused_run_records_gap test (owned by this plan per 01-VALIDATION.md) had no function to call otherwise.
- [Phase 01]: 01-06: blake3 added as a direct nr-metrics dependency so render_run_report can name the manifest blake3 it was generated from — nr_manifest's public API exposes only a file-path hasher (blake3_file), not a bytes hasher, and render_run_report receives an in-memory RunManifest, not a path; adding a bytes-hasher to nr_manifest would have touched a crate outside this plan's file list.
- [Phase 01]: 01-06: no over-gate sample count (samples_above/samples_at_or_above) appears anywhere in this plan's code — PLAT-03's decomposition compares two scalars (observed max vs the 30us gate, and vs the firmware floor), not a population count, so the 2026-08-31 convention change to samples_at_or_above had no code surface to apply to in nr-metrics. Confirmed by grep after implementation.
- [Phase 01]: 01-07: interference::snapshot ships Linux-gated with no facts parameter (plan 01-05's real shape), not the &facts-parameterized signature this plan's own interfaces block assumed — added NRMEASURE_INTERRUPTS_FIXTURE, a second test-only env var mirroring NRMEASURE_FACTS_FIXTURE, rather than changing nr-capture's already-shipped signature.
- [Phase 01]: 01-07: std::env::set_var/remove_var are unsafe fn under edition 2024 and this workspace forbids unsafe_code outright — restructured cmd::run into a thin run(args) wrapper over execute(args, &Overrides), so every test constructs an Overrides value directly instead of mutating the process environment; no unsafe anywhere in nr-cli.
- [Phase 01]: 01-07: probe-sysfs-tuning.txt (named CLEAN throughout nr-capture's own tests) is the real rig's actual untuned capture, not a passing scenario — tuned_facts_text() derives a genuinely passing fixture from it by overriding only governor/systemd-target/no_turbo/thermal, rather than fabricating a synthetic fixture from nothing.
- [Phase 01]: 01-07: FixtureFacts::parse's one-assignment-per-line format cannot represent /proc/cpuinfo or /etc/os-release (embedded newlines) — added extract_multiline_blocks, a small @begin/@end preprocessing extension entirely inside cmd/run.rs, so a fixture-driven run's D-14 snapshot is genuinely populated rather than rendering mostly empty. nr-capture's own fixture format is untouched.
- [Phase 01]: 01-07: a non-Clean contamination verdict, including Uncalibrated (the shipped D-17 default), marks excluded_from_series = true — not explicitly specified by the plan's own text, chosen because an unknown contamination status is not a defensible headline figure either, matching the project's rig-discipline stance.
- [Phase 01]: 01-07: the plan's own verification command 'cargo test -p nr-cli run_pipeline' selects zero tests (cargo's bare filter matches test function names, and none of task 3's own mandated exact names contain that substring) — the equivalent, correctly-targeted 'cargo test -p nr-cli --test run_pipeline' does select and pass all 6 tests; recorded so a future reader does not mistake the plan's literal command for a real check.
- [Phase 01]: 01-07: the reference rig was network-unreachable (100% packet loss, no ARP entry, ssh TCP connect timeout) for the entire session, retried three times — the real-rig refusal verification this execution's success criteria require could not be performed; documented as PARTIAL with the exact command for a future session once the rig reconnects, rather than skipped silently or claimed as passing.

### Pending Todos

None yet.

### Blockers/Concerns

- Open question, Phase 1: a ~3.8 ms global stall appeared on all six isolated threads at nearly identical values (stop_machine or a system-wide TLB shootdown signature). Unexplained. No latency figure should be published until it is named.
- Contaminated baseline: `cyclictest-rt-isolated-idle-10m.hist` was taken with SSH activity and an active GNOME session. Not publishable; re-run under the Phase 1 protocol with `--tracemark` and ftrace armed.
- Doc inconsistency: PROJECT.md Constraints still says "software timestamping only", which the corrected Out of Scope entry (PTP hardware timestamps back in scope, WIRE-06) supersedes. Fix at the next PROJECT.md update.
- WIRE-06 needs an ethernet cable on `enp0s31f6`; the wifi adapter has no PTP clock.
- Rig boots untuned: every CPU governor reads powersave despite rt-tuning.service reporting active/enabled (power-profiles-daemon wins a boot-time race, see docs/rig/recon-2026-08-31/FINDINGS.md). Any plan that runs the real measurement protocol must fix this or rely on nr-capture's own precondition check; it must not trust rt-tuning.service's ActiveState.

## Session Continuity

Last session: 2026-08-31T16:11:48.754Z
Stopped at: Completed 01-07-PLAN.md
Resume file: None

Next: `/donny-plan-phase 1`
