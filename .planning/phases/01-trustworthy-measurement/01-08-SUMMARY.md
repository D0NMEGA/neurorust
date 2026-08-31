---
status: PASS
agent: donny-executor
phase: 01-trustworthy-measurement
plan: 08
subsystem: measurement-provenance
tags: [rust, clap, cli, provenance, ci, github-actions, assert_cmd, bench-04]

# Dependency graph
requires:
  - phase: 01-trustworthy-measurement (plan 03)
    provides: nr-manifest's RunManifest/ProvenanceTier/AbsentField, validate(),
      blake3_file, and KernelInfo::redact_cmdline
  - phase: 01-trustworthy-measurement (plan 06)
    provides: nr-metrics' index::render_index/RunSummary
  - phase: 01-trustworthy-measurement (plan 07)
    provides: the nrmeasure CLI skeleton (main.rs, cmd/mod.rs), the
      RunClassArg/InstrumentClassArg value enums (reused here), and the
      Verify/Reconstruct stub bodies this plan replaces
provides:
  - nrmeasure verify, the D-13 provenance gate, walking measurements/ for
    per-run manifest validity (check 1), the whole git-tracked tree for
    capture-shaped files outside two named exempt trees (check 2, the
    T-1-05 mitigation), and measurements/INDEX.md write/check (check 3)
  - nrmeasure reconstruct, the D-16 path that builds an honest manifest for
    a pre-harness capture from RIG.txt and a README, hardcoded to
    provenance_tier reconstructed with every unrecorded field marked absent
  - .github/workflows/provenance.yml, a separate blocking CI gate (push to
    main and every pull_request, read-only token) running
    `nrmeasure verify --strict --check-index`
  - Confirmed, by actually running the release binary against the real
    repository tree, that the gate genuinely fails today (naming
    measurements/2026-08-28-precision3591 as lacking a manifest) and that
    the stray-capture scan catches a figure copied out of measurements/
affects: [01-10-reconstruct-2026-08-28, 01-09-protocol-doc, 01-14-nr-cli,
  phase-01-verifier]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Tree-wide scan scoped by file shape, not directory: verify's check 2
      classifies every git-tracked (or, absent a repo, plain-walked) file by
      a small set of capture-shaped glob patterns, then requires it to be
      either listed+checksummed inside a valid run directory or under one
      of two named exempt trees. A file moved out of measurements/ cannot
      evade the gate by relocation alone (T-1-05)."
    - "git ls-files with a plain-walk fallback: the stray-capture scan
      shells out to `git -C <root> ls-files --cached --others
      --exclude-standard` to honour .gitignore for free, falling back to a
      hardcoded .git/target skip list when root is not a git repository
      (exercised directly by every integration test, since a tempdir is
      never inside this repo's own .git)."
    - "Reconstruct never fabricates a struct it cannot fill from a source:
      InterferenceSnapshotPair is a required (non-Option) manifest field
      with no D-15 data available pre-harness, so reconstruct populates it
      with empty counter vectors, the operator's own --verdict value (a
      real, explicitly-supplied input, not a guess), and three absent_fields
      entries (interference.before/after/delta) naming exactly what could
      not be recovered, rather than omitting the struct or inventing counts."

key-files:
  created:
    - crates/cli/tests/verify.rs
    - crates/cli/tests/reconstruct.rs
    - .github/workflows/provenance.yml
  modified:
    - crates/cli/src/cmd/verify.rs
    - crates/cli/src/cmd/reconstruct.rs

key-decisions:
  - "RIG.txt's system: line (\"Dell Inc. Precision 3591\") has no
    double-space separator, unlike bios:/kernel: (both confirmed via `cat
    -evt` to carry a literal double space). Implemented split at the first
    single space per the plan's literal instruction, giving
    system_vendor=\"Dell\", system_model=\"Inc. Precision 3591\": an
    imperfect but genuinely traceable, non-fabricated split, called out here
    so plan 01-10 (or a human) can override with a cleaner value if wanted
    when it reconstructs the real directory."
  - "host.bios_release_date is recorded verbatim from RIG.txt
    (\"04/24/2026\", the DMI-native MM/DD/YYYY form), not reformatted to
    ISO 8601. nr-capture's own live path (environment.rs's build_host) does
    not reformat /sys/class/dmi/id/bios_date either; matching that
    convention keeps the field's format consistent across provenance tiers.
    Plan 01-03's minimal-manifest.json fixture shows a hand-typed
    \"2026-04-24\", which is inconsistent with the actual live code and was
    not treated as authoritative for this field's format."
  - "kernel.release/version_string/preempt_model come from RIG.txt's own
    kernel: line (\"7.0.0-30-generic\", PREEMPT_DYNAMIC), verbatim per the
    plan's literal mapping table, while kernel.is_realtime comes from the
    README's separate statement (\"/sys/kernel/realtime\" = 1) per the
    plan's explicitly carved-out exception. The two sources describe
    different points in time (the live-USB screening vs. the later
    installed-RT capture), so the reconstructed manifest visibly disagrees
    with itself on kernel identity, exactly as RIG.txt's own staleness
    implies; reconciling the two would be inventing a fact neither source
    states together."
  - "Reused cmd::run::{RunClassArg, InstrumentClassArg} in reconstruct.rs
    rather than duplicating the RunClass/InstrumentClass value-enum mapping
    a second time; both were already `pub` from plan 01-07."
  - ".github/workflows/provenance.yml, not ci.yml: this execution's own
    success criteria said 'wired into .github/workflows/ci.yml', but the
    plan's task 3 explicitly and repeatedly specifies a separate
    provenance.yml file, with acceptance criteria that check for that exact
    filename, and a stated rationale (one file per gate keeps a future
    Kani/loom/criterion gate's diff small and a failure attributable at a
    glance). Followed the plan's explicit, reasoned instruction over the
    task prompt's shorthand; ci.yml itself was not touched."
  - "BENCH-04 traceability: shared with plans 01-01/01-03/01-07/01-10. Per
    explicit instruction, requirements mark-complete was not run and
    .planning/REQUIREMENTS.md was left untouched; the end-of-phase verifier
    owns marking BENCH-04 complete once all named plans have landed."

requirements-completed: []

# Metrics
duration: ~10min commit-to-commit (context-loading and research not included)
completed: 2026-08-31
---

# Phase 01 Plan 08: nrmeasure verify and reconstruct, the D-13 blocking provenance gate Summary

**`nrmeasure verify` scans measurements/ for manifest validity and the whole git-tracked tree for capture-shaped files outside two named exempt trees; `nrmeasure reconstruct` builds an honest, D-16-tiered manifest from RIG.txt and a README for a pre-harness capture; a new .github/workflows/provenance.yml wires verify in as a blocking gate, confirmed (by actually running it) to fail today on the one directory plan 01-10 will fix.**

## Performance

- **Duration:** approximately 10 minutes of active implementation, measured
  commit-to-commit (`53fd636` at 11:31:48 to `5df4461` at 11:41:16, local
  time on 2026-08-31). Context-loading (all `files_to_read`, plus
  exploratory reads of `fields.rs`, `sources.rs`, `environment.rs`,
  `preconditions.rs`, `interference.rs`, `percentiles.rs`, the real
  `RIG.txt`/`README.md`, and a byte-level whitespace check of `RIG.txt`) is
  not included in this figure.
- **Started:** 2026-08-31T11:31:48-05:00 (Task 1 commit)
- **Completed:** 2026-08-31T11:41:16-05:00 (gap-closing test commit,
  after Task 3)
- **Tasks:** 3 planned, all complete, plus one immediate self-correction
  (see Deviations)
- **Files changed:** 5 (3 created, 2 modified)

## Accomplishments

- `nrmeasure verify`: three checks, every failure collected before exiting.
  Check 1 validates every immediate subdirectory of `measurements/` (missing
  manifest, parse failure, or any `nr_manifest::validate` error, all
  reported with the run directory named). Check 2 walks the whole
  git-tracked tree (via `git ls-files --cached --others --exclude-standard`,
  falling back to a plain walk skipping `.git`/`target` when not a git
  repository) for seven capture-shaped glob patterns, requiring each match
  to be listed-and-checksummed inside a valid run directory or under
  `crates/*/tests/fixtures/` or `docs/rig/recon-*/` with a `probe-` prefix
  (this is the T-1-05 mitigation: scoped by what a file *is*, not only
  where it sits). Check 3 regenerates or checks `measurements/INDEX.md`
  against `nr_metrics::index::render_index`, computing each run's p99/max
  by locating its `CyclictestHist` artifact and parsing it via
  `nr_histogram`.
- `nrmeasure reconstruct`: parses RIG.txt's `key: value` lines (splitting on
  the first `:` only, so `captured_utc`'s RFC3339 timestamp survives
  intact) into every `HostInfo`/`KernelInfo`/`OsInfo`/`TuningInfo`/
  `PowerInfo`/`NetworkInfo` field the plan's mapping table names, covering
  all 20 real RIG.txt keys (19 mapped, `captured_utc` deliberately reported
  as unmapped, matching every other genuinely-unrecognised key). Every
  field the sources do not carry is recorded in `absent_fields` with a
  specific reason rather than defaulted; `provenance_tier` is hardcoded to
  `Reconstructed` with no flag able to change it; `--verdict` is a required
  argument (`--exclusion-reason` additionally required when it is
  `contaminated`); `--force` gates overwriting an existing `manifest.json`.
- `.github/workflows/provenance.yml`: a separate workflow (push to `main`
  and every `pull_request`, `permissions: contents: read`) that builds
  `nr-cli --release` and runs `nrmeasure verify --strict --check-index`.
  Validated with `actionlint` (newly installed via Homebrew) alongside the
  plan's own Python/PyYAML verification snippet.
- **Ran the gate for real against the actual repository tree** (not just
  temp-directory tests): `cargo build -p nr-cli --release && ./target/release/nrmeasure
  verify --strict --check-index` exits 1, reporting exactly two problems -
  `measurements/2026-08-28-precision3591` lacking a manifest, and
  `measurements/INDEX.md` not yet generated - with no other false
  positives anywhere in the tree. Also ran the plan's own stray-capture
  verification (`cp .../cyclictest-rt-isolated-idle-10m.hist ./stray.hist`),
  confirming the scan names it and cleaning it up immediately after.
- 19 new tests: 11 integration tests for `verify` (10 named by the plan
  plus one closing a gap against this execution's own success criteria;
  see Deviations) and 8 integration tests for `reconstruct` (all named by
  the plan, driven against a temporary copy of
  `measurements/2026-08-28-precision3591/`, never the real directory),
  plus 10 unit tests (4 in `verify.rs`, 6 in `reconstruct.rs`). Full
  workspace: 141 tests (112 before this plan), `fmt`/`clippy --all-targets
  -D warnings` both clean.

## Task Commits

Each task was committed atomically:

1. **Task 1: nrmeasure verify, scoped by file shape rather than by directory alone** - `53fd636` (feat)
2. **Task 2: nrmeasure reconstruct, which never guesses a field** - `80be097` (feat)
3. **Task 3: The blocking provenance workflow** - `7095661` (feat)
4. **Gap-closing coverage** - `5df4461` (test, see Deviations)

**Plan metadata:** committed separately after this SUMMARY (see final commit).

## Files Created/Modified

- `crates/cli/src/cmd/verify.rs` - the three checks, `CAPTURE_GLOBS`, the
  git-ls-files/plain-walk tree scan, `build_summaries`/`compute_percentiles`
  for the index, 4 unit tests
- `crates/cli/tests/verify.rs` - 11 integration tests driving the compiled
  `nrmeasure` binary against temporary trees
- `crates/cli/src/cmd/reconstruct.rs` - RIG.txt/README parsing, the six
  field builders, artifact collection, `harness_info`, 6 unit tests
- `crates/cli/tests/reconstruct.rs` - 8 integration tests against a
  temporary copy of the real 2026-08-28 measurement directory
- `.github/workflows/provenance.yml` - the blocking CI gate

## Decisions Made

See the frontmatter `key-decisions` block for the full list with rationale.
The two most load-bearing for a future reader of the reconstructed
2026-08-28 manifest (plan 01-10):

**`system:`'s split produces `system_vendor="Dell"`,
`system_model="Inc. Precision 3591"`.** RIG.txt's `bios:` and `kernel:`
lines both carry a genuine double-space field separator (confirmed with
`cat -evt`, not assumed); `system:` does not. The plan's instruction ("split
at the first space run") was followed literally as "the first run of
whitespace", which for a line with no double-space degenerates to the
first single space. The result is a real, non-fabricated substring of the
source line, just not a clean vendor/model boundary; flagged explicitly so
it is not mistaken for a bug when the real directory is reconstructed.

**The reconstructed manifest's `kernel.release` and `kernel.is_realtime`
describe different moments.** RIG.txt's `kernel:` line documents the
live-USB screening kernel (`7.0.0-30-generic`, `PREEMPT_DYNAMIC`); the
README separately documents the later installed kernel used for the actual
cyclictest capture (`7.0.0-30-realtime`, real-time). The plan's mapping
table sources `release`/`version_string`/`preempt_model` from RIG.txt
verbatim and carves out `is_realtime` as a named exception sourced from the
README, so the reconstructed manifest visibly (and correctly) does not
resolve this tension - resolving it would mean inventing a fact neither
source states in one place.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Added an 11th verify test not named by the plan, required by this execution's own success criteria**
- **Found during:** Final review of `<success_criteria>` in this execution's
  own prompt, after Task 1's ten plan-named tests were already committed
- **Issue:** This execution's `<success_criteria>` names four distinct
  `verify` failure modes, each requiring "its own test": an orphan capture,
  a missing required field, a checksum mismatch, and "a manifest naming a
  file that does not exist". The plan's own task 1 fixes exactly ten test
  names, and none of them cover the fourth mode - `ArtifactMissing`
  (the file the manifest names was never written at all), distinct from
  `ChecksumMismatch` (the file exists with different content).
- **Fix:** Added `verify_rejects_manifest_naming_a_nonexistent_file`, an
  eleventh test alongside (not replacing) the ten the plan names, driving
  the same `minimal-manifest.json` fixture with its one artifact record but
  no corresponding file on disk, asserting `nr_manifest::validate`'s
  `ArtifactMissing` message ("does not exist") names the missing filename.
- **Files modified:** `crates/cli/tests/verify.rs`
- **Verification:** `cargo test -p nr-cli --test verify` selects and passes
  all 11 tests; full workspace suite re-run clean afterward
- **Committed in:** `5df4461`

---

**Total deviations:** 1 auto-fixed (missing test coverage this execution's
own criteria required, not present in the plan's fixed test-name list).
**Impact on plan:** Additive only - all ten of the plan's own named tests
are unchanged and still pass under their exact required names; the
eleventh test strengthens coverage without touching implementation
behavior. No scope creep: the fix stayed inside `crates/cli/tests/verify.rs`.

## Issues Encountered

None blocking. One out-of-scope, pre-existing item noted for transparency
per prior plans' own convention:
`.planning/phases/01-trustworthy-measurement/01-LEDGER.jsonl` remains
untracked in git (a donny-tools phase-tracking artifact predating this
session); left untouched, since only this plan's own task-related files
are staged per the scope boundary.

## User Setup Required

None for the code delivered by this plan. One repository-setting item is
explicitly out of this agent's hands, per the plan's own task 3 action
text: **a human must mark the `provenance` check as required in this
repository's GitHub branch protection settings** before D-13's "the commit
cannot merge" is actually enforced by GitHub itself; the workflow file
alone only causes the check to run and report, not to block a merge.

## Next Phase Readiness

- `nrmeasure verify` and `nrmeasure reconstruct` are both complete for this
  plan's scope, covered by 19 new tests (141 total in the workspace,
  `fmt`/`clippy --all-targets -D warnings` clean), and independently
  confirmed against the real repository tree, not just synthetic fixtures.
- **The provenance gate is currently red on `main`, by design.** Plan
  01-10 must run `nrmeasure reconstruct` against the real
  `measurements/2026-08-28-precision3591/` directory (with `--verdict
  contaminated`, matching the README's own documented caveat) and commit
  the resulting `manifest.json`, plus run `nrmeasure verify --write-index`
  to generate the first `measurements/INDEX.md`, before the gate can turn
  green. No allowlist or exemption was added in its place, per the plan's
  explicit instruction.
- Plan 01-09 (protocol doc) can reference the two exempt trees
  (`crates/*/tests/fixtures/`, `docs/rig/recon-*/` with `probe-` prefix) as
  a settled, tested contract in `docs/publication-layout.md`.
- A human still needs to mark the `provenance` GitHub Actions check as
  required in branch protection (see User Setup Required above); this is a
  one-time repository setting, not a code change.
- `BENCH-04` remains open at the requirements level (shared with 01-01,
  01-03, 01-07, 01-10); no action taken here, flagged for the end-of-phase
  verifier, per explicit instruction not to touch `.planning/REQUIREMENTS.md`.

---
*Phase: 01-trustworthy-measurement*
*Completed: 2026-08-31*

## Self-Check: PASSED

- All 5 created/modified files confirmed present and non-empty on disk
  with `[ -f ]` / `[ -s ]`.
- All 4 commits (`53fd636`, `80be097`, `7095661`, `5df4461`) confirmed
  present in `git log --oneline --all`.
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D
  warnings`, and `cargo test --workspace` (141 tests, 0 failed) all re-ran
  clean immediately before this SUMMARY was finalized.
- Every literal acceptance-criteria grep from all three tasks was re-run
  directly and passes: `ProvenanceTier::Reconstructed` present,
  `ProvenanceTier::HarnessGenerated` count 0, `unmapped RIG.txt key`
  present, `crates/\*/tests/fixtures`/`docs/rig/recon-`/`probe-` all
  present in `verify.rs`, `write-index` count 0 in `provenance.yml`, no
  allowlist/skip-list/`continue-on-error` in `provenance.yml`, and `git
  diff --stat crates/cli/Cargo.toml` shows no change across all three
  tasks.
- `cargo run -p nr-cli -- verify --help` and `-- reconstruct --help` both
  exit 0; `verify --help` lists `--strict`, `--write-index`, and
  `--check-index`; `reconstruct --help` shows `--verdict` as a required
  (non-bracketed) argument in its `Usage:` line.
- `actionlint .github/workflows/provenance.yml .github/workflows/ci.yml`
  exits 0.
- Re-ran the gate against the real, unmodified repository tree
  immediately before writing this summary:
  `cargo build -p nr-cli --release && ./target/release/nrmeasure verify
  --strict --check-index` exits 1, naming
  `measurements/2026-08-28-precision3591` as lacking a manifest and
  `measurements/INDEX.md` as not yet generated - exactly the documented,
  intended failure, with no allowlist anywhere in the source.
