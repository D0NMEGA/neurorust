---
status: PASS
agent: donny-executor
phase: 01-trustworthy-measurement
plan: 21
subsystem: rig-instrumentation
tags: [rtla, hwnoise, msr-smi-count, rdmsr, report, plat-03, rust, cli]

# Dependency graph
requires:
  - phase: 01-trustworthy-measurement
    provides: "01-20's rtla hwnoise parser (nr_capture::hwnoise::parse_hwnoise),
      hwlatdetect coverage primitives (nr_capture::firmware), and the
      FirmwareScreen/CpuExposure/SmiCounts/ArtifactKind::RtlaHwnoise manifest
      fields this plan populates and renders for the first time"
provides:
  - "crates/cli/src/cmd/run.rs: --with-hwnoise wired end to end (its own
      InstrumentWindow, a checksummed rtla-hwnoise.txt artifact, a parsed
      FirmwareScreen, a mutual-exclusion guard against --with-hwlatdetect),
      plus an MSR_SMI_COUNT before/after bracket around every run regardless
      of which firmware instrument, if any, also ran"
  - "crates/capture/src/smi.rs: MSR_SMI_COUNT (register 0x34) parsing and
      aggregation, Linux-gated like interference::snapshot, with a CPU whose
      read fails contributing no counter entry and a stated
      unavailable_reason rather than a substituted zero"
  - "crates/metrics/src/report.rs: render_firmware_screens, a REPORT.md
      section stating three finding-3 statements written separately for
      hwlatdetect and rtla-hwnoise (never shared between them), plus the
      exact MSR_SMI_COUNT delta when the manifest carries one; omitted
      entirely, not rendered empty, when no firmware screen ran"
  - "FirmwareObservation (generalised from HwlatObservation) so
      render_plat03_verdict names whichever instrument produced a paired
      observation instead of assuming hwlatdetect, with the ece44b0 strict
      '<' gate boundary left untouched"
  - "docs/measurement-protocol.md: a paragraph under 'The two-instrument
      rule' naming both firmware instruments and which one can sample the
      isolated cores"
affects: [01-22, 01-23]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "A third fixture-text seam (NRMEASURE_SMI_FIXTURE) added to the same
      Overrides struct and fixtures_used/forced-exclusion machinery plan
      01-18 built for the facts and interrupts seams, rather than a new,
      different mechanism: one read, reused for both the before and after
      snapshot, so its own delta is always zero by construction."
    - "A rendered report section gated entirely on the data it would
      describe (manifest.firmware_screens.is_empty() short-circuits to an
      empty string) rather than rendering a heading with no content, so a
      cyclictest-only run's REPORT.md is provably unchanged rather than
      merely 'the new section happens to be short'."
    - "Per-instrument caveat text kept as two literal, non-shared string
      arrays (firmware_caveats) rather than one parameterised template,
      because the plan's own finding names two of the three statements as
      differing in substance, not just in the instrument name substituted
      into them; a shared template would have made the difference a
      runtime string-interpolation detail instead of a reviewable diff."

key-files:
  created:
    - crates/capture/src/smi.rs
    - crates/capture/tests/smi.rs
    - crates/cli/tests/fixtures/fake-rtla.sh
  modified:
    - crates/cli/src/cmd/run.rs
    - crates/cli/src/tools.rs
    - crates/cli/tests/run_pipeline.rs
    - crates/capture/src/lib.rs
    - crates/metrics/src/report.rs
    - crates/metrics/tests/report.rs
    - docs/measurement-protocol.md

key-decisions:
  - "requirements-completed left empty for both PLAT-03 and BENCH-04, this
      plan's own frontmatter requirements, matching 01-20's precedent
      exactly. BENCH-04 is already Complete in REQUIREMENTS.md from an
      earlier plan. PLAT-03 needs a published, attributed latency figure;
      this plan's own objective states it runs entirely on the macOS dev
      host against fake tool binaries and takes no rig capture at all -
      that is explicitly plan 01-23's job. Marking PLAT-03 complete here
      would assert a rig figure that does not yet exist."
  - "The run_pipeline and headline_report REPORT.md snapshots were left
      untouched rather than re-pinned. The plan's own action text
      anticipated a snapshot diff ('adding a section changes the rendered
      report'), but both snapshotted fixtures are cyclictest-only runs with
      no firmware screen, and the plan's own named behavior
      (report_omits_the_section_when_no_screen_ran) requires the section to
      be genuinely absent, not merely short, in exactly that case. Verified
      by running both suites rather than assumed: neither snapshot changed
      a single byte."
  - "FirmwareObservation carries max_us as Option<u64> (HwlatObservation's
      was a required u64), matching FirmwareScreen's own optionality for a
      run that observed nothing above threshold, and render_plat03_verdict
      prints 'none observed above threshold' rather than a fabricated 0 in
      that case."
  - "The two per-instrument caveat sets are looked up by instrument name
      with hwlatdetect's set as the match fallback (_ arm), documented
      inline as the conservative, more-caveated default rather than a
      silent placeholder for an instrument this project has not written
      statements for yet."

requirements-completed: []

# Metrics
duration: 4h32min wall clock (18:56 to 21:41 local), ~35min active work this
  session; task 1 (d621383) was committed by a prior session before an
  external API rate-limit interruption killed it mid-task-2, not by any
  code failure - see Issues Encountered
completed: 2026-09-05
---

# Phase 1 Plan 21: Wiring rtla hwnoise and MSR_SMI_COUNT into nrmeasure run, honestly captioned in REPORT.md Summary

**`--with-hwnoise` takes a real firmware screen with one osnoise thread per isolated CPU, every run brackets an exact per-CPU MSR_SMI_COUNT, and REPORT.md now states three per-instrument finding-3 caveats instead of zero.**

## Performance

- **Duration:** Task 1 committed 2026-09-05T18:56:57-05:00 by a prior executor session. That
  session was killed mid-task-2 by an external API session rate limit, not a code failure; the
  orchestrator confirmed the uncovered work compiled and its two unit tests passed before
  handing over. This session resumed from that point, finished Task 2 at 21:28:33 and Task 3
  at 21:39:33 (same day, -05:00), then wrote this SUMMARY: roughly 35 minutes of active work
  across the two tasks, reading context, finishing the interrupted task, and full verification.
- **Tasks:** 3 of 3 (1 carried in from the prior session, 2 executed this session)
- **Files modified:** 10 unique across the whole plan (4 in task 1, 5 in task 2, 3 in task 3,
  with `crates/cli/src/cmd/run.rs` and `crates/cli/tests/run_pipeline.rs` touched by both
  tasks 1 and 2)

## Accomplishments

- `nrmeasure run --with-hwnoise` takes a real `rtla hwnoise` firmware screen: its own
  `InstrumentWindow`, a checksummed `rtla-hwnoise.txt` artifact under `ArtifactKind::RtlaHwnoise`,
  and a parsed `FirmwareScreen` naming the requested and observed CPU lists, per-CPU exposure from
  the `Runtime` column, and the largest `Max Single` value. `--with-hwnoise` and
  `--with-hwlatdetect` are mutually exclusive, refused in the same early guard block as the
  project's other invariant-violating flag combinations.
- Every run now brackets an exact `MSR_SMI_COUNT` (register `0x34`) read immediately before the
  first interference snapshot and immediately after the last instrument's window closes,
  regardless of which firmware instrument, if any, also ran. A CPU whose register cannot be
  read contributes no counter entry; the manifest instead carries a stated
  `unavailable_reason` naming the CPU and the error. No code path anywhere in
  `crates/capture/src/smi.rs` substitutes `0` for a value that could not be read.
- REPORT.md gains a `## Firmware screen` section, entirely omitted (not rendered empty) for a
  cyclictest-only run, that states three finding-3 statements written separately for
  `hwlatdetect` and `rtla-hwnoise`: the two sets of text share no string, and the `rtla-hwnoise`
  exposure sentence specifically names "one osnoise sampling thread per CPU", the concrete
  difference from `hwlatdetect`'s single non-migrating tracer thread on this rig.
  `render_plat03_verdict` now names whichever instrument produced a paired observation
  (`FirmwareObservation`, generalised from the single-instrument `HwlatObservation`) while still
  refusing to combine the two figures, and the strict `<` gate boundary commit `ece44b0`
  corrected is confirmed unchanged.
- `docs/measurement-protocol.md` gains a paragraph under "The two-instrument rule" naming both
  firmware instruments, which one can sample the isolated cores on this kernel, and that
  `MSR_SMI_COUNT` is recorded for every run, linking `docs/rig/firmware-floor-rt-vs-stock.md`
  for the evidence.
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and
  `cargo test --workspace` (251 tests) all pass at HEAD. `./target/release/nrmeasure verify
  --strict --check-index` still reports 0 problems across all 8 run directories, and
  `git diff --stat measurements/` stayed empty throughout: this plan touched no published
  evidence, exactly as its own objective states ("no rig access; plan 01-23 takes the
  captures").

## Task Commits

1. **Task 1: `nrmeasure run` takes an `rtla hwnoise` firmware screen** - `d621383` (feat)
   - Committed by the prior, interrupted session; verified intact and unmodified this session.
2. **Task 2: An exact per-CPU SMI count brackets every run** - `4a27ee1` (feat)
3. **Task 3: A published firmware figure states what its instrument measured** - `364aa29` (feat)

**Plan metadata:** (this commit, made immediately after this SUMMARY) docs(01-21): complete plan

## Files Created/Modified

- `crates/cli/src/cmd/run.rs` - `--with-hwnoise`/`--hwnoise-*` args, `build_hwnoise_argv`,
  the hwnoise execution branch and its `FirmwareScreen` construction (task 1); the
  `NRMEASURE_SMI_FIXTURE` seam, `take_smi_snapshot`, the SMI bracket around the whole run, and
  the SMI delta line in `print_summary` (task 2)
- `crates/cli/src/tools.rs` - `RTLA_PATH_ENV` constant (task 1)
- `crates/cli/tests/fixtures/fake-rtla.sh` - emits the committed 2026-09-05 probe text so the
  hwnoise pipeline runs end to end with no rig (task 1)
- `crates/cli/tests/run_pipeline.rs` - five hwnoise behavior tests plus `hwnoise_warns_on_
  uncovered_requested_cpus` (task 1); `SMI_COUNTS` fixture text and
  `smi_counts_bracket_the_run`/`smi_fixture_forces_exclusion` (task 2, finished this session -
  the production wiring for task 2 was already complete and verified in the working tree; only
  these two integration tests were still missing)
- `crates/capture/src/smi.rs` - `parse_rdmsr_value`, `parse_smi_snapshot`, `read_smi_counts`
  (Linux-gated), `delta`; module doc states plainly that a count is not a duration (task 2)
- `crates/capture/src/lib.rs` - `pub mod smi;` registration (task 2)
- `crates/capture/tests/smi.rs` - `parses_a_hex_rdmsr_reading`,
  `unreadable_register_records_a_reason_not_a_zero` (task 2)
- `crates/metrics/src/report.rs` - `FirmwareObservation` (replacing `HwlatObservation`),
  `Plat03Input.firmware`, `render_firmware_screens`, `firmware_caveats`, `format_cpu_list`
  (task 3)
- `crates/metrics/tests/report.rs` - updated the two pre-existing PLAT-03 tests for the renamed
  API, added `firmware_section_states_all_three_caveats`, `firmware_caveats_differ_by_
  instrument`, `firmware_section_names_observed_cpus`, `report_omits_the_section_when_no_
  screen_ran`, `plat03_observation_names_its_instrument` (task 3)
- `docs/measurement-protocol.md` - one paragraph under "The two-instrument rule" (task 3)

## Decisions Made

See `key-decisions` in the frontmatter for the full list with rationale. In brief: neither
`PLAT-03` nor `BENCH-04` is recorded as newly completed by this plan (BENCH-04 already was;
PLAT-03 needs plan 01-23's actual rig capture); the two REPORT.md snapshots the plan expected to
need re-pinning were left untouched because both are cyclictest-only fixtures and the plan's own
section-omission behavior correctly produces zero diff for them, verified rather than assumed;
`FirmwareObservation.max_us` is `Option<u64>` to match `FirmwareScreen`'s own honesty about an
instrument that observed nothing; and the two instruments' caveat text is two separate literal
arrays, never one shared template, matching the plan's explicit instruction that copying one
onto the other "would be its own small dishonesty."

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Formatting] `cargo fmt` reformatted pre-existing uncommitted code alongside new code**
- **Found during:** Task 2, first `cargo fmt --check` after resuming
- **Issue:** `crates/capture/tests/smi.rs` (written by the prior, interrupted session) and
  `crates/cli/src/cmd/run.rs`'s SMI-fixture-reading block both had minor rustfmt drift (an
  `assert_eq!` macro call and a `match` binding rustfmt wanted reflowed differently), and my own
  newly added test code in `crates/cli/tests/run_pipeline.rs` had one similar drift.
- **Fix:** Ran `cargo fmt` across the workspace once, then confirmed `cargo fmt --check` passed
  clean.
- **Files modified:** `crates/capture/tests/smi.rs`, `crates/cli/src/cmd/run.rs`,
  `crates/cli/tests/run_pipeline.rs`
- **Verification:** `cargo fmt --check` exits 0; `cargo test` results unchanged before and after.
- **Committed in:** `4a27ee1` (task 2's own commit)

**Total deviations:** 1 auto-fixed (Rule 1, mechanical formatting only). **Impact on plan:**
none beyond keeping the plan's own `cargo fmt --check` verification gate green; no logic changed.

## Issues Encountered

The prior executor session was killed mid-Task-2 by an external API session rate limit, not by
any code defect. Before handing over, the orchestrator had already confirmed
`cargo check --workspace --all-targets` finished clean and `cargo test -p nr-capture --test smi`
passed 2/2 against the uncommitted `crates/capture/src/smi.rs` and its test file. On resuming,
reading `crates/cli/src/cmd/run.rs` end to end showed the SMI wiring itself (the fixture seam,
the before/after bracket, the manifest population, and the `print_summary` delta line) was
already complete and correctly placed; the two integration tests the task's own behavior list
names (`smi_counts_bracket_the_run`, `smi_fixture_forces_exclusion`) were the only pieces
genuinely missing from `crates/cli/tests/run_pipeline.rs`. Both were written, and the full task 2
verification block (unit tests, integration tests, workspace tests, clippy, release build,
strict verify) passed without needing to touch the already-correct production code.

## User Setup Required

None. This plan runs entirely on the macOS dev host against fake tool binaries and committed
fixtures; no rig access, no external services, no new dependencies.

## Next Phase Readiness

- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and
  `cargo test --workspace` (251 tests) all pass at HEAD (`364aa29`).
- `./target/release/nrmeasure verify --strict --check-index` reports 0 problems across all 8
  run directories; `git diff --stat measurements/` is empty.
- Plan 01-22 and plan 01-23 (re-taking the D-18 firmware screen on the real rig) now have a
  harness that can take an `rtla hwnoise` screen, brackets `MSR_SMI_COUNT` around every run, and
  publishes both honestly captioned per finding 3. The exact argv this plan wires up matches
  `docs/rig/recon-2026-09-05/FINDINGS.md`'s recommendation unchanged:
  `rtla hwnoise -c 6-11 -H 0-5 -P f:99 -d <duration>` and `rdmsr -p <cpu> 0x34` before and after.
- External-audit finding 3 is now closed: both the exposure-is-not-duration statement and the
  sampling-records-not-a-census statement are written per instrument, asserted to differ, and
  neither claims an exposure its own tool did not report.
- No blockers for wave 14 (whichever plan is sequenced there); `deferred-items.md`'s "Finding 3,
  partially open" entry should be closed in this plan's final metadata commit, alongside
  `STATE.md`.
- Noted, not actioned (pre-existing, out of scope for this plan): `crates/cli/src/cmd/run.rs`
  (2161 lines) and `crates/cli/tests/run_pipeline.rs` (1789 lines) are both well past this
  project's own 800-line file-size guideline, having grown incrementally across roughly a dozen
  plans. Splitting either is a real, out-of-scope refactor with no bearing on this plan's own
  correctness; flagging it here rather than deferred-items.md since it is not new information
  and no prior plan has treated it as blocking.

## Self-Check: PASSED

Verified directly:
- `[ -f crates/capture/src/smi.rs ]`, `[ -f crates/capture/tests/smi.rs ]`,
  `[ -f crates/cli/tests/fixtures/fake-rtla.sh ]` - all FOUND.
- `grep -q 'pub mod smi' crates/capture/src/lib.rs` - FOUND.
- `grep -q 'pub struct FirmwareObservation' crates/metrics/src/report.rs` - FOUND;
  `grep -c 'HwlatObservation' crates/metrics/src/report.rs` - 0 (fully renamed).
- `grep -q 'fn render_firmware_screens' crates/metrics/src/report.rs` - FOUND.
- `grep -n 'input.observed_max_us < input.gate_us' crates/metrics/src/report.rs` - FOUND,
  unchanged from commit `ece44b0`.
- `grep -n 'unwrap_or(0)' crates/capture/src/smi.rs` - none found.
- `git log --oneline --all | grep -q d621383` / `4a27ee1` / `364aa29` - all FOUND.
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace` (251 tests) - all pass at HEAD (`364aa29`).
- `./target/release/nrmeasure verify --strict --check-index` - exit 0, 0 problems.
- `git diff --stat measurements/` - empty.

*Phase: 01-trustworthy-measurement*
*Completed: 2026-09-05*
