---
status: PARTIAL
agent: donny-executor
phase: 01-trustworthy-measurement
plan: 09
subsystem: documentation
tags: [docs, measurement-protocol, publication-layout, provenance, rust, rt-tuning, power-profiles-daemon, plat-02, bench-05, bench-06]

# Dependency graph
requires:
  - phase: 01-trustworthy-measurement (plan 05)
    provides: nr-capture's 14 D-06 PreconditionCheck implementations
      (preconditions.rs) and the SystemFacts-backed sources.rs the Required
      system state table's "how the harness observes it" column was drawn from
  - phase: 01-trustworthy-measurement (plan 07)
    provides: nrmeasure run's actual CLI argument surface, its real --dry-run
      behavior, and the two Phase 1 standard invocations the "Running a
      measurement" section's commands are built from
  - phase: 01-trustworthy-measurement (plan 08)
    provides: nrmeasure verify/reconstruct, the two exempt trees, and the
      generated measurements/INDEX.md the publication-layout answers describe
provides:
  - docs/measurement-protocol.md, the PLAT-02 reproduction contract: all 14
    preconditions with their literal expected value, how to set it and how the
    harness observes it, the governor operating point settled with exact
    mask/verify/restore commands, the exact run command sequence, what
    invalidates a run, run classes and cadence against OSADL's published
    practice, the two-instrument rule, and cross-hardware reproduction
  - docs/publication-layout.md, the BENCH-05/BENCH-06 publication rules:
    directory layout, run directory contents, the 25 MiB/100 MiB size policy,
    why histograms are generated ASCII rather than committed images, how
    losing configurations are operationalised, the integrity-not-authenticity
    checksum disclosure, the two provenance-gate exempt trees, the metrics/
    series split, and the two provenance tiers
  - crates/capture/tests/protocol_doc.rs, a hand-rolled PascalCase-shape
    scanner (not a fixed string list) that fails the moment the document and
    PreconditionCheck disagree, in either direction
  - PreconditionCheck::ALL in nr-manifest, the ground truth both the document
    and the drift test check against
affects: [01-11, 01-13, phase-01-verifier]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "A whole-document shape scanner, not a fixed string list, catches a
      phantom check name: protocol_doc.rs tokenizes the document into maximal
      alnum runs, flags any multi-capital-hump PascalCase word (two or more
      capital-initiated segments, ruling out both an ordinary capitalised
      English word and an all-caps acronym), and asserts every such word is a
      real PreconditionCheck variant, with a two-line allowlist for genuine
      external identifiers (systemd's own ActiveState) that share the shape"

key-files:
  created:
    - docs/measurement-protocol.md
    - docs/publication-layout.md
    - crates/capture/tests/protocol_doc.rs
  modified:
    - crates/manifest/src/fields.rs
    - .planning/phases/01-trustworthy-measurement/deferred-items.md

key-decisions:
  - "The governor operating point (the plan's most consequential open content
    decision) is settled as: the literal scaling_governor=performance value
    GovernorIsPerformanceOnAllCpus actually checks, not power-profiles-daemon's
    own notion of a performance profile, which docs/rig/recon-2026-08-31/
    FINDINGS.md found is expressed on this Meteor Lake HWP backend as
    energy_performance_preference=performance with scaling_governor left at
    powersave. Reaching it requires masking power-profiles-daemon for the
    run's duration (systemctl mask --now, then restart rt-tuning.service);
    exact verify and restore commands are given rather than a policy statement
    with no mechanism."
  - "energy_performance_preference is not yet a manifest field, and the
    document says so plainly rather than implying the gap is closed. Adding
    it would touch crates/manifest/src/fields.rs's TuningInfo/CpuGovernor,
    the generated JSON schema, and crates/capture/src/environment.rs, none of
    which are in this plan's file list (docs/measurement-protocol.md,
    docs/publication-layout.md, crates/capture/tests/protocol_doc.rs only).
    Logged to deferred-items.md for whichever plan next touches the D-14
    snapshot (01-11's calibration pair is the most likely next owner, since it
    is the next plan that depends on the governor operating point)."
  - "requirements-completed left empty, matching plan 01-08's precedent and
    this execution's explicit instruction not to touch .planning/
    REQUIREMENTS.md. PLAT-02, BENCH-05 and BENCH-06 are shared with several
    other plans and PLAT-02 in particular is not actually complete until a
    human follows this protocol on a clean rig (ROADMAP.md success criterion
    2), which the plan's own <verification> block states cannot be automated."
  - "protocol_documents_no_phantom_check scans the whole document rather than
    only the Required system state table, on the theory that a check-name
    typo anywhere in the prose (not only the table) is worth catching. This
    surfaced two false positives against the document's own prose (the bare
    Rust identifier PreconditionCheck, and the real systemd property name
    ActiveState); the first was fixed by rewording to plain English, the
    second by a small explicit allowlist, rather than by narrowing the scan
    or raising the hump threshold (which would have let a real 2-hump variant
    name like TracersQuiescent go uncaught)."

requirements-completed: []

# Metrics
duration: ~4min commit-to-commit (context-loading and research not included)
completed: 2026-08-31
---

# Phase 01 Plan 09: Measurement protocol and publication layout documents Summary

**docs/measurement-protocol.md (all 14 D-06 preconditions, the governor operating point settled against the power-profiles-daemon race, the exact run/verify command sequence) and docs/publication-layout.md (BENCH-05/BENCH-06 publication rules with zero latency figures), kept honest by a hand-rolled drift test rather than a hardcoded string list.**

## Performance

- **Duration:** approximately 4 minutes of active implementation, measured
  commit-to-commit (`942b0a4` at 12:08:00 to `a44bc19` at 12:11:47, local time
  on 2026-08-31). The preceding context-loading pass (all `files_to_read`, plus
  exploratory reads of `sources.rs`, `run.rs`, `rundir.rs`, `verify.rs`,
  `checksum.rs`, `index.rs`, `report.rs`, `validate.rs`, `REQUIREMENTS.md`,
  `01-CONTEXT.md`, `01-RESEARCH.md`, and `ROADMAP.md`) is not included in this
  figure.
- **Started:** 2026-08-31T17:08:00Z (Task 1 RED commit)
- **Completed:** 2026-08-31T17:11:47Z (Task 2 commit)
- **Tasks:** 2 (both complete; Task 1 executed as RED then GREEN per its
  `tdd="true"` marking)
- **Files changed:** 5 (3 created, 2 modified)

## Accomplishments

- `docs/measurement-protocol.md`: a table of all 14 preconditions with each
  check's literal expected value (copied from `crates/capture/src/
  preconditions.rs`'s own `expected` strings, not paraphrased), how an
  operator sets it, and exactly which sysfs path or command the harness reads
  to observe it; the "Why it exists" section names the 2026-08-28
  contamination's actual numbers (2 us median, 3.8 ms maximum, roughly 137,000
  CAL interrupts on CPUs 6 to 11); the exact `nrmeasure run`/`verify` command
  sequence including the real `--dry-run` behavior (verified against `cmd/
  run.rs`'s actual code, not the plan's own slightly optimistic description of
  it); what invalidates a run; all eight run classes with their cadence, cited
  against OSADL's published practice without claiming parity; the
  two-instrument rule; and cross-hardware reproduction guidance
- Settled the governor operating point, the plan's single most important open
  content decision: documented that the literal `scaling_governor=performance`
  value the shipped precondition checks is not reachable through
  power-profiles-daemon's own "Performance" setting on this Meteor Lake HWP
  backend, gave the exact `systemctl mask --now power-profiles-daemon.service`
  / `systemctl restart rt-tuning.service` sequence to reach it, the exact sysfs
  read to verify it, the restore sequence, and flagged that
  `energy_performance_preference` is not yet a captured manifest field
- `docs/publication-layout.md`: all nine required sections, every stated
  number matching the shipped constants exactly (`25 MiB`/`100 MiB` from
  `crates/cli/src/rundir.rs`, the two exempt trees from `crates/cli/src/cmd/
  verify.rs`, the integrity-not-authenticity wording from `crates/manifest/
  src/checksum.rs`, the three `metrics/*.json` filenames, the two
  `harness-generated`/`reconstructed` provenance tiers), and zero latency
  figures anywhere in the document (`grep -cE` for all six banned patterns
  reports 0)
- `crates/capture/tests/protocol_doc.rs`: three tests
  (`protocol_documents_every_precondition`, `protocol_documents_no_phantom_check`,
  `protocol_is_ascii`) driven by a hand-written tokenizer and a PascalCase-hump
  heuristic rather than a fixed list of expected substrings, so a precondition
  added to the code in the future without being documented is caught
  automatically, and so is a typo of a real check name anywhere in the prose
- Added `PreconditionCheck::ALL` to `nr-manifest` (explicitly authorized by
  this plan's own action text) as the single ground truth both the document
  and the drift test check against, plus a small unit test
  (`all_contains_every_variant_exactly_once`) keeping that hand-written list
  exhaustive against the enum, since nothing in the type system does that
  automatically

## Task Commits

Each task was committed atomically (Task 1 as RED then GREEN, per its
`tdd="true"` marking):

1. **Task 1 (RED): failing test for the measurement protocol document** - `942b0a4` (test)
2. **Task 1 (GREEN): the PLAT-02 measurement protocol document** - `2caa335` (feat)
3. **Task 2: the publication layout document** - `a44bc19` (docs)

**Plan metadata:** committed separately after this SUMMARY (see final commit).

## Files Created/Modified

- `docs/measurement-protocol.md` - the PLAT-02 reproduction contract
- `docs/publication-layout.md` - the BENCH-05/BENCH-06 publication rules
- `crates/capture/tests/protocol_doc.rs` - the document/code drift test
- `crates/manifest/src/fields.rs` - `PreconditionCheck::ALL` plus its own unit
  test
- `.planning/phases/01-trustworthy-measurement/deferred-items.md` - logs the
  `energy_performance_preference` manifest-field gap for a future plan

## Decisions Made

See the frontmatter `key-decisions` block for the full list with rationale.
The governor operating point decision and the `energy_performance_preference`
gap disclosure are the two with the widest downstream effect, since plans
01-11 and 01-13 both depend on the operating point being reachable and
verifiable before they can produce a publishable figure.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] The plan's own example for the governor row predates docs/rig/recon-2026-08-31/FINDINGS.md and would have left a reader's run permanently refused**
- **Found during:** Task 1, before writing the "Required system state" table
- **Issue:** The plan's own action text illustrates the table format with
  `GovernorIsPerformanceOnAllCpus    handled by rt-tuning.service on the
  reference rig`. `docs/rig/recon-2026-08-31/FINDINGS.md` (read as part of
  this execution's own mandatory context, dated the same day as this plan)
  found that `rt-tuning.service` reporting healthy does not mean its write
  survived: `power-profiles-daemon` wins a boot-time race and never writes
  the literal `scaling_governor=performance` value the check requires on this
  hardware. Following the plan's example verbatim would have told a reader to
  trust a service whose own healthy status this project's own recon already
  proved insufficient.
- **Fix:** Wrote "The governor operating point", a dedicated section giving
  the real mechanism (mask `power-profiles-daemon`, restart `rt-tuning.service`,
  verify by reading sysfs directly, restore afterward), and pointed the
  table's "how to set it" cell at that section instead of repeating the
  stale one-line example.
- **Files modified:** `docs/measurement-protocol.md`
- **Verification:** The exact commands were checked against
  `crates/capture/src/preconditions.rs`'s `check_governor_is_performance`
  (reads `scaling_governor` directly, per CPU) and
  `check_rt_tuning_service_active` (reads only `ActiveState`, confirming it
  cannot detect this failure mode on its own)
- **Committed in:** `2caa335` (Task 1 GREEN commit)

**2. [Rule 1 - Bug] A bare Rust identifier in the prose false-positived the phantom-check test**
- **Found during:** Task 1, first GREEN run of `cargo test -p nr-capture --test protocol_doc`
- **Issue:** "Every row below is one `PreconditionCheck` variant..." named the
  enum's own Rust identifier, which the phantom-check scanner correctly
  recognised as PascalCase-shaped (`Precondition`+`Check`, two humps) and
  correctly flagged as not itself a check name.
- **Fix:** Reworded to "Every row below is one precondition check...", plain
  English rather than the bare type name, which is also more appropriate for
  a document written for a third-party operator rather than a Rust API
  consumer.
- **Files modified:** `docs/measurement-protocol.md`
- **Verification:** `cargo test -p nr-capture --test protocol_doc` passes
- **Committed in:** `2caa335` (Task 1 GREEN commit)

**3. [Rule 1 - Bug] ActiveState, a genuine systemd property name, also false-positived the phantom-check test**
- **Found during:** Task 1, second GREEN run of the same test, after fixing
  deviation 2
- **Issue:** `--property=ActiveState` (the real systemd property this
  document instructs an operator to query) is itself two-hump PascalCase
  (`Active`+`State`), the same shape as a real `PreconditionCheck` variant.
  Narrowing the shape heuristic to require three or more humps would have
  silently stopped catching a typo of a genuine two-hump variant name such as
  `TracersQuiescent`.
- **Fix:** Added `KNOWN_NON_CHECK_IDENTIFIERS`, a short, explicit, documented
  allowlist in `protocol_doc.rs` containing only `"ActiveState"`, rather than
  loosening the general-purpose heuristic.
- **Files modified:** `crates/capture/tests/protocol_doc.rs`
- **Verification:** All three tests in `protocol_doc.rs` pass; the allowlist
  is a single named constant, easy for a future maintainer to extend if
  another external property name is added to the document
- **Committed in:** `2caa335` (Task 1 GREEN commit)

**4. [Rule 1 - Bug] Two accidental markdown line-wrap breaks split literal strings the plan's own acceptance criteria grep for**
- **Found during:** Task 2, running the plan's own literal verification
  one-liner (`test -s ... && grep -q '25 MiB' ...`) before committing
- **Issue:** Manual prose wrapping split `` `25` `` and `` `MiB` `` across a
  line break, and split `` `metrics/` `` and `` `baseline.json` `` across
  another; both literal substrings the acceptance criteria require
  (`grep -q '25 MiB'`, `grep -q 'metrics/baseline.json'`) therefore failed,
  since `grep` matches within one line by default.
- **Fix:** Rewrapped both sentences so each literal string stays on one line.
- **Files modified:** `docs/publication-layout.md`
- **Verification:** Re-ran every literal acceptance-criteria grep from the
  plan's own text; all pass
- **Committed in:** `a44bc19` (Task 2 commit)

**5. [Rule 2 - Missing Critical, logged rather than fixed] energy_performance_preference is not yet a manifest field**
- **Found during:** Task 1, writing "The governor operating point" per this
  execution's own explicit instruction to record that this field must be
  captured alongside `scaling_governor`
- **Issue:** `crates/manifest/src/fields.rs`'s `TuningInfo`/`CpuGovernor`
  structs have no field for it, so the document's own stated requirement is
  not yet met by the shipped schema.
- **Fix:** Documented the requirement and the gap plainly in the document
  itself, and logged it to `deferred-items.md` for whichever plan next
  touches the D-14 environment snapshot (01-11's calibration pair is the most
  likely owner). Not implemented here: doing so would touch
  `crates/manifest/src/fields.rs`, the generated JSON schema, and
  `crates/capture/src/environment.rs`, none of which are in this plan's file
  list (`docs/measurement-protocol.md`, `docs/publication-layout.md`,
  `crates/capture/tests/protocol_doc.rs` only), and the gap is pre-existing
  (shipped by plans 01-03/01-05), not introduced by this plan's own changes.
- **Files modified:** `.planning/phases/01-trustworthy-measurement/deferred-items.md`
- **Committed in:** part of the final metadata commit (see below)

---

**Total deviations:** 5 (1 stale plan-example correction driven by a same-day
recon finding, 2 self-caught bugs in the phantom-check test's own false
positives, 1 self-caught markdown line-wrap bug caught by running the plan's
own verification commands before committing, 1 out-of-scope schema gap logged
rather than fixed per the scope boundary rule).
**Impact on plan:** All were necessary for this plan's own literal acceptance
criteria and this execution's stated success criteria to pass truthfully, or
correctly scoped out per the deviation rules' scope boundary. No scope creep:
every code change stayed inside `crates/manifest/src/fields.rs` (one
additive, plan-authorized constant plus its own test) and `crates/capture/
tests/protocol_doc.rs`; nothing in `crates/capture/src/environment.rs` or the
generated JSON schema was touched.

## Issues Encountered

None blocking. `.planning/phases/01-trustworthy-measurement/01-LEDGER.jsonl`
remains untracked in git (a donny-tools phase-tracking artifact predating this
session, noted by prior plans' own summaries); left untouched, since only
this plan's own task-related files are staged per the scope boundary.

## User Setup Required

None - no external service configuration required. This plan wrote two
documents and a test; nothing here needs the reference rig, which remains
unreachable and was not needed for any of this plan's work.

## Next Phase Readiness

- Both documents are complete for this plan's scope, and `crates/capture/
  tests/protocol_doc.rs` keeps them honest against the code going forward.
  `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D
  warnings`, and `cargo test --workspace` (145 tests: 141 before this plan,
  plus the new `all_contains_every_variant_exactly_once` unit test and the
  three `protocol_doc.rs` tests) are all green.
- Plan 01-10 (reconstructed manifests, the 2026-08-28 README correction) can
  reference `docs/publication-layout.md`'s "Directory layout" and "Provenance
  tiers" sections as the settled, tested contract for what it produces.
- Plan 01-11 (the D-17 calibration pair) and plan 01-13 (the PLAT-03 headline
  capture) both depend on "The governor operating point" being followed
  before either can produce a publishable figure; plan 01-11 is also the
  most likely owner of the deferred `energy_performance_preference` manifest
  field (see deviation 5 and `deferred-items.md`).
- `PLAT-02`, `BENCH-05` and `BENCH-06` remain open at the requirements level
  (shared with other plans per REQUIREMENTS.md's traceability table); no
  action taken here, flagged for the end-of-phase verifier, per explicit
  instruction not to touch `.planning/REQUIREMENTS.md`. PLAT-02 in particular
  is not actually complete until a human follows `docs/measurement-
  protocol.md` against a clean rig without asking questions (ROADMAP.md
  success criterion 2), which is recorded in the plan's own `<verification>`
  block as manual-only and not automatable.

---
*Phase: 01-trustworthy-measurement*
*Completed: 2026-08-31*

## Self-Check: PASSED

- All 3 created files (`docs/measurement-protocol.md`, `docs/publication-layout.md`,
  `crates/capture/tests/protocol_doc.rs`) and 2 modified files
  (`crates/manifest/src/fields.rs`, `.planning/phases/01-trustworthy-measurement/
  deferred-items.md`) confirmed present and non-empty on disk with `[ -s ]`.
- All 3 commits (`942b0a4`, `2caa335`, `a44bc19`) confirmed present in
  `git log --oneline --all`.
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  and `cargo test --workspace` (145 tests, 0 failed) all re-ran clean
  immediately before this SUMMARY was finalized.
- Every literal acceptance-criteria grep from both tasks was re-run directly
  and passes: `multi-user.target`, `137,000`, `--dry-run`,
  `instrument_class`/`two-instrument`, `OSADL`, the ASCII-clean check, the
  `## ` heading count (9, at least 7 required), the sentence-case heading
  check, `25 MiB`, `100 MiB`, `integrity, not authenticity`,
  `losing configuration` (case-insensitive), `crates/*/tests/fixtures`,
  `docs/rig/recon-`, all three `metrics/*.json` filenames, `harness-generated`,
  `reconstructed`, `zstd -19`, and the zero-count check against all six banned
  latency-figure patterns in `docs/publication-layout.md`.
- The plan's own `<verification>` block commands were all re-run directly:
  `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D
  warnings`, `cargo test -p nr-capture protocol_doc`, the ASCII/em-dash grep
  across both documents, the `25 * 1024 * 1024` / `25 MiB` threshold match,
  and `cargo test -p nr-capture protocol_documents_every_precondition --
  --nocapture`. All pass.
