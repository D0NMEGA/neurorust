---
status: PASS
agent: donny-executor
phase: 01-trustworthy-measurement
plan: 18
subsystem: benchmark-methodology
tags: [rust, cli, manifest, schemars, tdd, preconditions, thermal-profile, fixtures]

# Dependency graph
requires:
  - phase: 01-trustworthy-measurement
    provides: "01-16's build-time harness identity and artifact_paths mapping, 01-19's
      Option<u64> RunSummary tail figures and check_derived_figures, 01-17's durable
      ATTEMPT.json and per-instrument interference windows - all read or extended by
      this plan's changes to run.rs and preconditions.rs"
provides:
  - "DeepCstatesDisabled reads every CPU in the run's own --cpus list (discover_cstates
    per target CPU) instead of cpu0 alone, collapsing to a range when every CPU agrees
    and naming every CPU explicitly the moment one disagrees"
  - "TracersQuiescent requires current_tracer, events/enable, set_event and tracing_on
    all quiescent for a headline-series run, naming the armed control by name on
    failure"
  - "RunManifest.fixtures_used: Vec<String>, naming any NRMEASURE_*_FIXTURE env var
    active for a run; NRMEASURE_INTERRUPTS_FIXTURE is now refused for
    headline/weekly/soak, matching the existing facts-fixture guard"
  - "A fixture-driven run (any non-publishable class) is unconditionally forced
    excluded_from_series, with the contamination verdict's own recorded reason and the
    manifest's exclusion_reason both explaining that the deltas are zero by
    construction, never a quiet machine"
  - "ThermalProfile (Normal | HotScreen) declared via --thermal-profile, recorded on
    RunManifest, refused outside --class screen; the ThermalHeadroomAtStart exemption
    now follows this declaration instead of RunClass::Screen and the observed
    temperature"
  - "protocol_states_the_code_thermal_ceiling: a mechanical guard keeping
    docs/measurement-protocol.md's stated ceiling and THERMAL_HEADROOM_CEILING_C in
    sync"
affects: [01-20, 01-21, 01-22, 01-23]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Declared-before-observed exemption: applicability of a precondition decided
      from an operator declaration made before anything is read (ThermalProfile),
      rather than from the run class or the reading itself, so the exemption cannot
      be reverse-engineered from an unwanted observation"
    - "Uniform-collapse / disagreement-expand observed-string rendering
      (format_cstate_observations): a per-CPU check reports one range summary when
      every CPU agrees and expands to a fully explicit per-CPU listing the moment any
      CPU disagrees, so a disagreement can never be lost inside a summary"
    - "One forcing site: a new override condition (fixture usage) joins the existing
      excluded_from_series decision expression as another arm, rather than a second,
      separate post-hoc override, so only one place in the code can ever force
      exclusion"

key-files:
  created: []
  modified:
    - crates/capture/src/preconditions.rs
    - crates/capture/tests/preconditions.rs
    - crates/capture/tests/protocol_doc.rs
    - crates/manifest/src/fields.rs
    - crates/cli/src/cmd/run.rs
    - crates/cli/src/cmd/reconstruct.rs
    - crates/cli/tests/run_pipeline.rs
    - crates/cli/tests/snapshots/run_pipeline__full_run_report_matches_snapshot.snap
    - crates/metrics/tests/snapshots/report__headline_report.snap
    - schemas/manifest.schema.json
    - docs/measurement-protocol.md
    - .planning/phases/01-trustworthy-measurement/deferred-items.md

key-decisions:
  - "DeepCstatesDisabled reports Unavailable, not Pass, when a fixture never captured
    any target CPU's cpuidle tree (RIG_AS_FOUND/VIOLATED/CLEAN_FACTS all predate
    per-CPU capture): this is the honest answer for those historical fixtures, per
    the plan's own instruction, and required extending tuned_facts_text() in
    crates/cli/tests/run_pipeline.rs with real per-CPU POLL/C1E data for cpus 6-11 so
    the whole full-pipeline test suite kept passing"
  - "screen_spec() stays Normal-profile by default; a new hot_screen_spec() declares
    HotScreen explicitly, so thermal_headroom_still_passes_a_cold_screen (a
    Normal-profile screen run) keeps testing a still-true claim while
    thermal_headroom_does_not_refuse_a_firmware_screen (whose premise - any
    Screen-class run is exempt when hot - is exactly what finding 5 asked to change)
    is replaced by two profile-scoped tests covering the same motivating scenario
    correctly"
  - "Reused outcome.reason (the contamination verdict's own recorded reason)
    verbatim as exclusion_reason for a fixture-forced exclusion, rather than building
    two separate strings, so the two can never disagree about why a fixture-driven
    run's deltas are zero"

requirements-completed: [BENCH-04]

# Metrics
duration: 45min
completed: 2026-09-05
---

# Phase 1 Plan 18: Widened D-06 gates and a declared thermal profile Summary

**DeepCstatesDisabled and TracersQuiescent now read what their names claim across every isolated CPU and all four tracing controls, no fixture seam can reach a publishable run, and the Screen thermal exemption follows a declared `--thermal-profile` instead of the run class and the observed temperature.**

## Performance

- **Duration:** ~45 min
- **Started:** 2026-09-05T21:15:00Z (estimated from session start; a prior attempt at
  this plan was killed by an API session rate limit with zero commits and no
  uncommitted changes, so this session started fresh from 113c522)
- **Completed:** 2026-09-05T21:59:29Z
- **Tasks:** 3 (all `tdd="true"`, each executed as a genuine RED then GREEN commit
  pair)
- **Files modified:** 12

## Accomplishments

- `check_deep_cstates_disabled` now loops `discover_cstates` over every CPU in the
  run's own `--cpus` list instead of `cpu0` alone. The rig tunes C-states per CPU
  through sysfs, so `cpu0`'s tree said nothing about the isolated cores (6-11) a
  headline run actually uses; the observed string collapses to a range
  (`"C6 not present, C10 not present on cpus 6-11"`) when every CPU agrees and
  expands to a fully explicit per-CPU listing the moment any CPU disagrees, so a
  disagreeing CPU can never be lost inside a summary.
- `check_tracers_quiescent` now requires `current_tracer`, `events/enable`,
  `set_event` and `tracing_on` to all read their quiescent value for a
  `headline-series` run. `current_tracer == nop` alone used to pass with event
  tracing active, since the other three controls arm tracing independently; the
  observed string names each control and its value (or `unavailable` if the kernel
  does not expose it), so a reader always sees which controls were actually
  readable.
- `RunManifest.fixtures_used: Vec<String>` names any `NRMEASURE_FACTS_FIXTURE` /
  `NRMEASURE_INTERRUPTS_FIXTURE` active for a run. `NRMEASURE_INTERRUPTS_FIXTURE` is
  now refused for `headline`/`weekly`/`soak`, matching the pre-existing facts-fixture
  guard: reusing one fixture text for both interference snapshots made every delta
  exactly zero, which used to read as a perfectly quiet machine. Every class where a
  fixture is legitimately allowed now forces `excluded_from_series` in the same
  decision site `--allow-precondition-violation` already uses, and the
  contamination verdict's own recorded reason (mirrored into `exclusion_reason`)
  explicitly explains that the deltas are zero by construction.
- `ThermalProfile` (`Normal` | `HotScreen`), declared via a new `--thermal-profile`
  flag and recorded on the manifest, replaces `RunClass::Screen` as what
  `ThermalHeadroomAtStart`'s exemption keys on. The `HotScreen` arm performs no
  temperature comparison at all - applicability is settled by the declaration,
  before anything is read - and `hot-screen` is refused outright for any class other
  than `screen`. An unintentionally hot idle screen (declared `normal`, the default)
  now correctly fails the gate instead of being exempted just for being
  `--class screen`.
- `protocol_states_the_code_thermal_ceiling` pins `docs/measurement-protocol.md`'s
  stated thermal ceiling to the now-`pub` `THERMAL_HEADROOM_CEILING_C` constant, the
  mechanical guard for the class of defect (a prose claim the code does not
  implement) the audit found three times.
- Two of the fifteen `PreconditionCheck` variants are strictly harder to pass than
  before; none were weakened, and `PreconditionCheck::ALL` is unchanged at 15
  entries.

## Task Commits

Each task was committed atomically as a genuine RED -> GREEN pair:

1. **Task 1: DeepCstatesDisabled reads the target CPUs, TracersQuiescent reads every
   tracing control**
   - `722ae57` test(01-18): add failing tests for widened DeepCstatesDisabled and TracersQuiescent
   - `c4f6fa5` feat(01-18): read every target CPU for DeepCstatesDisabled, all four controls for TracersQuiescent
2. **Task 2: A fixture-driven run names its fixtures and cannot reach the series**
   - `72475ca` test(01-18): add failing tests for fixture-driven runs naming their fixtures
   - `44d585f` feat(01-18): no fixture can drive a publishable run, and every fixture-driven run says so
3. **Task 3: The thermal exemption follows a declared profile, and the document
   says what the code says**
   - `4b65f0c` test(01-18): add failing tests for the declared thermal profile
   - `f91bb8e` feat(01-18): the thermal exemption follows a declared profile, not the observed run class

**Plan metadata:** (this commit, made immediately after this SUMMARY) docs(01-18): complete plan

_All three RED commits were genuine failing states: Task 1's seven new tests failed
their own assertions against the unmodified implementation (a real runtime RED);
Tasks 2 and 3 referenced manifest/precondition-spec fields and a public constant
that did not exist yet, so the test crate failed to compile (the valid RED state for
new, statically-typed API surface, per this project's own established convention
from plans 01-16/01-19). No refactor commit was needed for any task; each GREEN
commit also carried the mechanical, unavoidable consequences of its own schema/type
change (see Deviations)._

## Files Created/Modified

- `crates/capture/src/preconditions.rs` - widened `check_deep_cstates_disabled`
  (`CstateStatus`, `format_cstate_observations`) and `check_tracers_quiescent`
  (`TRACING_CONTROLS`); `PreconditionSpec.thermal_profile`; rewrote
  `check_thermal_headroom_at_start` to take the declared profile; made
  `THERMAL_HEADROOM_CEILING_C` `pub`
- `crates/capture/tests/preconditions.rs` - 7 new Task 1 tests, 4 new + 1 replaced
  Task 3 tests, updated `tracers_quiescent_refuses_headline_run`'s assertion,
  `screen_spec()`/`hot_screen_spec()` additions
- `crates/capture/tests/protocol_doc.rs` - `protocol_states_the_code_thermal_ceiling`
- `crates/manifest/src/fields.rs` - `RunManifest.fixtures_used`,
  `RunManifest.thermal_profile`, `ThermalProfile` enum
- `crates/cli/src/cmd/run.rs` - `INTERRUPTS_FIXTURE_ENV` publishable-class guard,
  `fixtures_used_names`/`fixture_usage_reason`, one-forcing-site exclusion decision,
  `ThermalProfileArg`/`--thermal-profile` flag and its early guard,
  `PreconditionSpec`/`RunManifest` wiring
- `crates/cli/src/cmd/reconstruct.rs` - mechanical `fixtures_used: Vec::new()`,
  `thermal_profile: None` at the standalone `RunManifest` construction site
- `crates/cli/tests/run_pipeline.rs` - 4 new Task 2 tests plus
  `run_fixture_driven_calibration` helper; `interrupts_fixture_is_refused_...`;
  extended `tuned_facts_text()` with per-CPU cstate data (Rule 3 deviation)
- `crates/cli/tests/snapshots/run_pipeline__full_run_report_matches_snapshot.snap` -
  re-pinned `deep-cstates-disabled`/`tracers-quiescent` rows (Task 1), then the
  `reason:` line (Task 2)
- `crates/metrics/tests/snapshots/report__headline_report.snap` - re-pinned manifest
  blake3 hex (Task 2's always-serialized `fixtures_used` field)
- `schemas/manifest.schema.json` - regenerated twice (Task 2, Task 3)
- `docs/measurement-protocol.md` - `DeepCstatesDisabled`, `TracersQuiescent` and
  `ThermalHeadroomAtStart` rows rewritten to match the code
- `.planning/phases/01-trustworthy-measurement/deferred-items.md` - new entry: the
  rig's `nr-measure-mode` script needs three more writes to keep satisfying the
  widened `TracersQuiescent`, owned by plan 01-22

## Decisions Made

- `DeepCstatesDisabled`'s `Unavailable` path fires when NO target CPU has any
  cpuidle tree at all (per the plan's own instruction), which is the honest reading
  of every existing fixture (`RIG_AS_FOUND`, `VIOLATED`, `CLEAN_FACTS`): none of them
  ever captured anything beyond `cpu0`. Verified this is correct for the live rig
  path too (real sysfs registers cpuidle state directories for every CPU
  unconditionally; only the *fixture* was ever cpu0-only).
- `screen_spec()` kept its default `ThermalProfile::Normal` rather than defaulting
  to `HotScreen`, so the pre-existing `thermal_headroom_still_passes_a_cold_screen`
  test (a plain, non-thermally-exempt screen run) keeps testing a claim that is
  still true after this plan. A separate `hot_screen_spec()` declares `HotScreen`
  explicitly for the tests that need it.
- `interference::verdict()`'s own `VerdictOutcome.reason` (which the manifest never
  stores directly, only via `exclusion_reason`) is overwritten with the fixture
  explanation whenever any fixture was used, then that same string is reused
  verbatim as `exclusion_reason`, so "the contamination verdict's recorded reason"
  and the published `exclusion_reason` can never disagree about why a fixture-driven
  run's deltas are zero.
- `requirements-completed` lists only `BENCH-04` (already `Complete` from prior
  phase-1 work; this plan does not regress it), not `PLAT-02` from the plan's own
  frontmatter. Following 01-09-SUMMARY.md's established precedent: PLAT-02 ("A
  clean measurement protocol is defined and followed") is not actually complete
  until a human follows the protocol on a clean rig, which this plan cannot do -
  it runs entirely on the macOS dev host against committed fixtures, per its own
  objective. `PLAT-02` in `REQUIREMENTS.md` stays `Pending`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Extended `tuned_facts_text()` with per-CPU cstate data
(`crates/cli/tests/run_pipeline.rs`)**
- **Found during:** Task 1, `cargo test --workspace`
- **Issue:** Widening `DeepCstatesDisabled` to read every target CPU (6-11) exposed
  that this integration-test fixture, like the real 2026-08-31 probe capture it
  derives from, only ever recorded `cpu0`'s cpuidle states. Every full-pipeline
  test using the default `HeadlineSeries` instrument class (the large majority of
  the file) started failing with `DeepCstatesDisabled: ... Unavailable`, which
  `refuse_on_violation` treats as a blocking offense for that instrument class.
- **Fix:** Added the same POLL/C1E-only registration (no C6/C10, the
  `intel_idle.max_cstate=1` rationale) for every CPU in `6..=11`, matching what a
  genuinely tuned rig would show uniformly across cores.
- **Files modified:** `crates/cli/tests/run_pipeline.rs`
- **Verification:** `cargo test --workspace` (18 previously-failing tests pass)
- **Committed in:** `c4f6fa5` (Task 1 GREEN commit)

**2. [Rule 1 - Bug] Re-pinned `run_pipeline__full_run_report_matches_snapshot.snap`'s
`deep-cstates-disabled`/`tracers-quiescent` rows**
- **Found during:** Task 1, `cargo test --workspace`
- **Issue:** The pinned `## Preconditions` table rows for these two checks reflected
  the pre-widening observed-string shape.
- **Fix:** Updated the two changed cells to the new, correctly-computed strings. No
  test logic changed.
- **Files modified:** `crates/cli/tests/snapshots/run_pipeline__full_run_report_matches_snapshot.snap`
- **Verification:** `cargo test -p nr-cli --test run_pipeline full_run_report_matches_snapshot`
- **Committed in:** `c4f6fa5` (Task 1 GREEN commit)

**3. [Rule 3 - Blocking] Mechanical `RunManifest` construction-site fix
(`crates/cli/src/cmd/reconstruct.rs`)**
- **Found during:** Task 2 and again at Task 3
- **Issue:** `reconstruct.rs`'s own standalone `RunManifest { ... }` literal (not in
  either task's declared file list) failed to compile once `fixtures_used` (Task 2)
  and then `thermal_profile` (Task 3) were added as required fields.
- **Fix:** `fixtures_used: Vec::new()` (a reconstructed run was never driven by a
  live fixture seam) and `thermal_profile: None` (a reconstructed run never
  declared one - this concept did not exist for it).
- **Files modified:** `crates/cli/src/cmd/reconstruct.rs`
- **Verification:** `cargo build --workspace`
- **Committed in:** `44d585f` (Task 2), `f91bb8e` (Task 3)

**4. [Rule 1 - Bug] Re-pinned `run_pipeline__full_run_report_matches_snapshot.snap`'s
`reason:` line**
- **Found during:** Task 2, `cargo test --workspace`
- **Issue:** `run_full_pipeline()` (used by most of `run_pipeline.rs`'s snapshot and
  full-pipeline tests) always sets both fixture env vars, so once fixture usage
  correctly overrides the contamination verdict's reason, the previously-pinned
  generic "provisional thresholds" reason line changed to the new fixture
  explanation - the correct, intended consequence of Task 2 rippling through every
  fixture-driven pipeline test, not only the four new ones.
- **Fix:** Updated the one changed `reason:` line to the new, correctly-computed
  text.
- **Files modified:** `crates/cli/tests/snapshots/run_pipeline__full_run_report_matches_snapshot.snap`
- **Verification:** `cargo test -p nr-cli --test run_pipeline full_run_report_matches_snapshot`
- **Committed in:** `44d585f` (Task 2 GREEN commit)

**5. [Rule 1 - Bug] Re-pinned `report__headline_report.snap`'s manifest blake3
hex**
- **Found during:** Task 2, `cargo test --workspace`
- **Issue:** `fixtures_used` has no `skip_serializing_if` (it must always serialize,
  including as `[]`, so a fixture-driven run's manifest is unambiguous), which
  changed the re-serialized byte content - and therefore the pinned content-hash -
  of `headline_report_snapshot`'s hand-built sample manifest. (`thermal_profile` in
  Task 3, by contrast, uses `skip_serializing_if = "Option::is_none"` and left this
  same snapshot's hash byte-identical, since the plan's own snippet specifies it
  that way.)
- **Fix:** Updated the one changed hex digest line. No test logic or `nr-metrics`
  behavior changed.
- **Files modified:** `crates/metrics/tests/snapshots/report__headline_report.snap`
- **Verification:** `cargo test -p nr-metrics --test report headline_report_snapshot`
- **Committed in:** `44d585f` (Task 2 GREEN commit)

**6. [Rule 3 - Blocking] Mechanical `Args`/`PreconditionSpec` construction-site
fixes for the new `thermal_profile` field**
- **Found during:** Task 3
- **Issue:** `crates/cli/src/cmd/run.rs`'s own `#[cfg(test)]` `base_args()` helper
  (used by `refusal_writes_nothing`/`raw_capture_is_byte_identical`, outside this
  task's declared file list beyond `run.rs` itself) failed to compile once `Args`
  gained a required `thermal_profile` field.
- **Fix:** `thermal_profile: ThermalProfileArg::Normal`.
- **Files modified:** `crates/cli/src/cmd/run.rs`
- **Verification:** `cargo test --workspace`
- **Committed in:** `f91bb8e` (Task 3 GREEN commit)

**Total deviations:** 6 auto-fixed (2 Rule 3 blocking construction-site fixes, 1
Rule 3 blocking test-fixture extension, 3 Rule 1 snapshot re-pins). **Impact on
plan:** all six are direct, mechanical, or newly-and-correctly-exposed consequences
of the three tasks' own mandated schema/behavior changes; none expand scope beyond
this plan's three tasks and their threat register.

Separately, not counted as a deviation because it directly implements the plan's
own explicit instruction rather than fixing an unplanned problem:
`thermal_headroom_does_not_refuse_a_firmware_screen` (whose premise - any
`Screen`-class run is exempt when hot - is exactly the finding-5 defect) was
replaced by `hot_screen_profile_exempts_a_hot_start` and
`normal_profile_refuses_a_hot_screen`, two of the plan's own named tests that cover
the same motivating scenario under the corrected, profile-scoped design.

## Issues Encountered

None beyond the deviations documented above. Each task's RED phase was reached and
verified cleanly on the first attempt; the only compiler feedback needed during the
GREEN phase was two straightforward `&str`/`&&str` deref mismatches in
`format_cstate_observations`'s closures, fixed immediately and not worth a separate
entry.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- All eight committed run directories still pass
  `nrmeasure verify --strict --check-index` with 0 problems (7 re-derived, 1
  reconstructed run correctly reported not re-derivable by name); `measurements/` is
  untouched (`git diff --stat measurements/` is empty), per D-12.
- Finding 8 of `01-EXTERNAL-AUDIT.md` is fully closed: `DeepCstatesDisabled` and
  `TracersQuiescent` now establish exactly the propositions their names claim, and
  no fixture seam can drive a headline/weekly/soak run or hide behind an
  unexplained zero delta.
- Finding 5 of `01-EXTERNAL-AUDIT.md` is fully closed (the documentation half closed
  in 01-11's commit `137c3c1`; this plan closes the exemption-scoping half): the
  thermal exemption now follows a declared, manifest-recorded profile rather than
  the run class evaluated after the fact.
- `deferred-items.md` names plan 01-22 as the owner of updating the rig's
  `nr-measure-mode` script with the three additional tracing-control writes the
  widened `TracersQuiescent` now requires; until then, a real run against the
  unpatched script will correctly (not spuriously) fail the check if event tracing,
  `set_event`, or `tracing_on` happen to be armed.
- No blockers for the next wave. All six external-audit findings STATE.md tracked
  as open before this plan (partial findings 3, 5, 6, 7, 8, and 9/10 in unexecuted
  plans) now have findings 5 and 8 fully closed.

## Self-Check: PASSED

All 12 modified files confirmed present on disk with the expected content; all 6
commit hashes (`722ae57`, `c4f6fa5`, `72475ca`, `44d585f`, `4b65f0c`, `f91bb8e`)
confirmed present in `git log`.

*Phase: 01-trustworthy-measurement*
*Completed: 2026-09-05*
