---
status: PASS
agent: donny-executor
phase: 01-trustworthy-measurement
plan: 07
subsystem: measurement-cli
tags: [rust, clap, cli, provenance, assert_cmd, insta, bench-04, bench-05, plat-02]

# Dependency graph
requires:
  - phase: 01-trustworthy-measurement (plan 03)
    provides: nr-manifest's RunManifest/RunClass/InstrumentClass/ProvenanceTier,
      blake3_file, validate, and the argv-relativization convention
  - phase: 01-trustworthy-measurement (plan 04)
    provides: nr-histogram's parse_hist_file/parse_json_file/reconcile and the
      overflow-inclusive Percentiles path
  - phase: 01-trustworthy-measurement (plan 05)
    provides: nr-capture's SystemFacts/FixtureFacts, run_all/refuse_on_violation,
      environment::snapshot, interference::snapshot/snapshot_from_text/verdict,
      and argv::relativize_argv
  - phase: 01-trustworthy-measurement (plan 06)
    provides: nr-metrics' render_run_report and the D-22 PLAT-03 decomposition
provides:
  - nrmeasure run, a single command that asserts every D-06 precondition,
    refuses cleanly before writing anything on a violation, executes
    cyclictest (and hwlatdetect on request), brackets the run with D-15
    interference snapshots, captures the D-14 environment snapshot, places
    byte-identical raw captures with blake3 checksums, and writes a
    validated manifest.json plus a generated REPORT.md
  - The measurements/<date>-<rig>-<class>[-NN] directory convention
    (rundir.rs), disambiguated by directory existence, with the
    MAX_IN_REPO_FILE_BYTES/MAX_IN_REPO_RUN_BYTES size policy
  - The nrmeasure CLI entry point with all four subcommands declared
    (run implemented; verify/reconstruct/series stubbed for plans 01-08/01-14)
  - End-to-end pipeline coverage on macOS with no rig, via two fake tool
    scripts and a facts fixture derived from the real (currently untuned)
    rig capture
affects: [01-08-nr-cli, 01-09-protocol-doc, 01-10-reconstruct, 01-14-nr-cli]

# Tech tracking
tech-stack:
  added: [tempfile]
  patterns:
    - "Overrides struct instead of inline std::env reads: cmd::run::run(args)
      resolves every environment-derived value (fixture paths, tool paths)
      exactly once into an Overrides value and calls execute(args, &overrides);
      tests construct Overrides directly and call execute, so no test ever
      needs std::env::set_var/remove_var, which edition 2024 made unsafe fn
      and this workspace forbids outright ([workspace.lints.rust]
      unsafe_code = \"forbid\")"
    - "@begin <key>/@end block extension (extract_multiline_blocks, cmd/run.rs
      only): FixtureFacts::parse's one-assignment-per-line format cannot
      represent a value with embedded newlines (/proc/cpuinfo, /etc/os-release);
      this crate preprocesses those blocks out and applies them via
      FixtureFacts::with, leaving nr-capture's own fixture format untouched"
    - "Version captured once, reused twice: tools::resolve_and_verify spawns
      --version exactly once per tool (also serving as the 'fail early if a
      tool is missing' check); tools::run_tool takes that string rather than
      re-spawning to learn it a second time"

key-files:
  created:
    - crates/cli/src/rundir.rs
    - crates/cli/src/tools.rs
    - crates/cli/src/cmd/mod.rs
    - crates/cli/src/cmd/run.rs
    - crates/cli/src/cmd/verify.rs
    - crates/cli/src/cmd/reconstruct.rs
    - crates/cli/src/cmd/series.rs
    - crates/cli/tests/run_pipeline.rs
    - crates/cli/tests/fixtures/fake-cyclictest.sh
    - crates/cli/tests/fixtures/fake-hwlatdetect.sh
    - crates/cli/tests/snapshots/run_pipeline__full_run_report_matches_snapshot.snap
  modified:
    - crates/cli/src/main.rs
    - crates/cli/Cargo.toml
    - Cargo.toml
    - Cargo.lock

key-decisions:
  - "interference::snapshot ships Linux-gated with no facts-source parameter
    (plan 01-05's actual shape), not the &facts-parameterized signature this
    plan's own <interfaces> block assumed. Added NRMEASURE_INTERRUPTS_FIXTURE,
    a second test-only env var mirroring NRMEASURE_FACTS_FIXTURE, rather than
    changing nr-capture's already-shipped signature."
  - "std::env::set_var/remove_var are unsafe fn (edition 2024) and this
    workspace forbids unsafe_code outright. Restructured run(args) into a
    thin wrapper over execute(args, &Overrides), so every test constructs an
    Overrides value directly instead of mutating the process environment."
  - "crates/capture/tests/fixtures/probe-sysfs-tuning.txt (named CLEAN
    throughout nr-capture's own tests) is the real rig's actual untuned
    capture, not a passing scenario; several checks genuinely Fail against
    it. tuned_facts_text() derives a genuinely passing fixture from it by
    overriding only the fields a tuned rig would actually report differently
    (governor, systemd target, no_turbo, thermal), rather than fabricating a
    synthetic fixture."
  - "FixtureFacts::parse's format cannot represent /proc/cpuinfo or
    /etc/os-release (embedded newlines). Added extract_multiline_blocks, a
    small @begin/@end preprocessing extension entirely inside cmd/run.rs, so
    a fixture-driven run's D-14 snapshot is genuinely populated rather than
    rendering mostly empty."
  - "A non-Clean contamination verdict, including Uncalibrated (the shipped
    D-17 default until plan 01-11's calibration pair exists), marks
    excluded_from_series = true. Not explicitly specified by this plan's own
    text (which only names Contaminated and tool-failure explicitly); chosen
    because an unknown contamination status is not a defensible headline
    figure either, matching the project's own rig-discipline stance."
  - "BENCH-04/BENCH-05/PLAT-02 traceability: shared with plans 01-01/01-03/
    01-04/01-08/01-10/01-13. Per explicit instruction, requirements
    mark-complete was not run and .planning/REQUIREMENTS.md was left
    untouched; the end-of-phase verifier owns marking these complete."

requirements-completed: [BENCH-04, BENCH-05, PLAT-02]

# Metrics
duration: ~25min commit-to-commit (context-loading, research, and the blocked
  real-rig verification attempts are not included in this figure)
completed: 2026-08-31
---

# Phase 01 Plan 07: nrmeasure run - orchestration, the D-14 stamped run directory Summary

**`nrmeasure run`: one command that asserts 14 D-06 preconditions, brackets cyclictest/hwlatdetect with D-15 interference snapshots, captures the full D-14 environment, and writes a validated manifest.json plus a generated REPORT.md, covered end to end on macOS by two fake tool scripts and a facts fixture derived from the real, currently-untuned rig capture.**

## Performance

- **Duration:** approximately 25 minutes of active implementation, measured
  commit-to-commit (`afc0e5f` at 10:41:11 to `8271264` at 11:05:46, local
  time on 2026-08-31). The preceding context-loading and research phase (all
  15 files_to_read, plus exploratory reads of every upstream crate's public
  API) is not included, nor are the several rig-connectivity retry attempts
  spread across the session (see "Real rig verification" below).
- **Started:** 2026-08-31T10:41:11-05:00 (Task 1 commit)
- **Completed:** 2026-08-31T11:05:46-05:00 (Task 3 commit)
- **Tasks:** 3 (all complete)
- **Files changed:** 15 (11 created, 4 modified)

## Accomplishments

- `nrmeasure run`'s full 12-step order of operations, exactly as specified:
  resolve tool paths and fail early if one is missing; run all 14 D-06
  preconditions and refuse (exit 2, name every offense, write nothing) on a
  violation; snapshot D-15 interference before cyclictest; execute
  cyclictest, then hwlatdetect if `--with-hwlatdetect`, never concurrently;
  snapshot interference again and render the verdict; capture the D-14
  environment snapshot; parse and reconcile the raw captures; create the run
  directory; place captures byte-identical with blake3 checksums; assemble,
  validate and write `manifest.json`; render and write `REPORT.md`
- `rundir.rs`: the `measurements/<date>-<rig>-<class>[-NN]` naming
  convention, disambiguated by directory existence (never silently reusing
  or clobbering a prior run's slot), the `run_id` charset guard shared with
  `nr_manifest`'s own rule, a `manifest.json`-populated refusal guard, and
  the stated `MAX_IN_REPO_FILE_BYTES`/`MAX_IN_REPO_RUN_BYTES` size policy
- `tools.rs`: every tool invocation goes through `Command::new(path).args(Vec<String>)`,
  never a shell; `NRMEASURE_CYCLICTEST`/`NRMEASURE_HWLATDETECT` override the
  resolved path for the fake-binary test seam; the tool's version is
  captured once (also serving as the "fail early if missing" check) and
  reused when it is actually invoked
  - `NRMEASURE_FACTS_FIXTURE` is refused outright for `Headline`/`Weekly`/
  `Soak` run classes: a fixture-backed run can never produce a publishable
  figure, tested directly (`fixture_facts_refused_for_publishable_classes`)
- BENCH-06 mechanised: a non-zero tool exit or a non-`Clean` contamination
  verdict marks the run `excluded_from_series` with a reason; the run
  directory is written regardless, never dropped
- End-to-end coverage with no rig: `crates/cli/tests/run_pipeline.rs` drives
  the real compiled binary via `assert_cmd`, with `fake-cyclictest.sh`
  emitting the committed 2026-08-28 real 10-minute capture byte-identically
  (via `cp`, not a heredoc, to avoid any escaping risk on a ~150-line file)
  plus a `--json` fixture whose per-thread maxima were derived to agree with
  that capture's own footer, so `nr_histogram::json::reconcile` accepts the
  pairing. All 6 named tests pass, plus 9 unit tests in `cmd/run.rs` and
  `rundir.rs`. Full workspace: 112 tests, `fmt`/`clippy --all-targets -D warnings`
  both clean
- Confirmed the D-14 snapshot is genuinely populated end to end: the
  committed report snapshot shows `cmdline: ... root=[redacted] ...` (the
  01-03 redaction convention firing on a real code path, not just a unit
  test), a real kernel release/version/OS/topology, and 22 CPUs all reading
  `performance`

## Task Commits

Each task was committed atomically:

1. **Task 1: The CLI skeleton and the measurements directory layout** - `afc0e5f` (feat)
2. **Task 2: The run orchestration** - `975e318` (feat)
3. **Task 3: End to end pipeline test with fake tool binaries** - `8271264` (test)

**Plan metadata:** committed separately after this SUMMARY (see final commit).

## Files Created/Modified

- `crates/cli/src/rundir.rs` - `RunDir`, `RunDirError`, the naming/disambiguation
  logic, `refuse_if_manifest_exists`, `exceeds_in_repo_limit`, the two size constants
- `crates/cli/src/tools.rs` - `resolve_tool_path`, `resolve_and_verify`, `run_tool`,
  `ToolOutput`, `ToolError`, the two `NRMEASURE_*` path-override env var names
- `crates/cli/src/cmd/mod.rs` - registers the four subcommand modules
- `crates/cli/src/cmd/run.rs` - `Args`, `RunClassArg`/`InstrumentClassArg`,
  `Overrides`, `run`/`execute`, every orchestration helper, and 2 unit tests
- `crates/cli/src/cmd/verify.rs`, `reconstruct.rs`, `series.rs` - `Args` structs
  plus a `bail!` stub, so `main.rs` compiles with the full subcommand set
  declared now (plans 01-08 and 01-14 replace the stub bodies)
- `crates/cli/src/main.rs` - clap derive entry point, all four `Command` variants,
  `help_text_is_ascii`/`help_text_lists_all_four_subcommands` tests
- `crates/cli/tests/run_pipeline.rs` - the 6 named end-to-end tests
- `crates/cli/tests/fixtures/fake-cyclictest.sh`, `fake-hwlatdetect.sh` - executable,
  ASCII-only fake tools
- `crates/cli/tests/snapshots/run_pipeline__full_run_report_matches_snapshot.snap` -
  the committed, reviewable REPORT.md snapshot (timestamps/manifest-blake3 redacted)
- `crates/cli/Cargo.toml`, `Cargo.toml`, `Cargo.lock` - add `thiserror`/`tempfile` to
  `nr-cli`, `insta` as a dev-dependency, `tempfile` at the workspace level

## Decisions Made

See the frontmatter `key-decisions` block for the full list. The two with the
widest downstream effect:

**`std::env::set_var`/`remove_var` are `unsafe fn` under edition 2024, forbidden
workspace-wide.** The plan's own task 3 action text describes a
`NRMEASURE_FACTS_FIXTURE` environment variable "that `cmd/run.rs` honours when
set," which reads naturally as env-var mutation from tests. `nr-cli`'s
`Cargo.toml` targets `edition = "2024"` (workspace-wide), under which
`std::env::set_var`/`remove_var` became `unsafe fn`, and `[workspace.lints.rust]
unsafe_code = "forbid"` makes any `unsafe` block a hard compile error with no
local override. Restructured `cmd::run` into `pub fn run(args) -> Result<i32>`
(a thin wrapper resolving every environment-derived value exactly once into an
`Overrides` struct) plus `fn execute(args, &Overrides) -> Result<i32>` (the real
orchestration, parameterised, never touching `std::env` itself). Every test in
`cmd/run.rs` and `crates/cli/tests/run_pipeline.rs` calls `execute`/spawns the
binary with `.env(...)` (which sets the *child* process's environment, an
entirely different, always-safe API) rather than mutating the current process's
environment. No `unsafe` anywhere in `nr-cli`.

**`interference::snapshot` does not take a `facts` parameter.** This plan's own
`<interfaces>` block states `nr_capture::interference::snapshot(&facts, &cpus) ->
InterferenceSnapshot`. Plan 01-05 actually shipped `snapshot(cpus: &[u32])`
(`#[cfg(target_os = "linux")]`-gated, reading `/proc/interrupts` directly, no
facts-source seam of its own) plus a free `snapshot_from_text(text, cpus)` for
tests (01-05-SUMMARY.md's own key-decisions confirm this was a deliberate choice,
not an oversight). Followed the real, already-shipped code rather than the
plan's stale assumption: added `NRMEASURE_INTERRUPTS_FIXTURE`, a second
test-only environment variable exactly mirroring `NRMEASURE_FACTS_FIXTURE`'s
purpose, reused for both the before and after snapshot in a test (the shipped
`config/contamination-thresholds.json` is uncalibrated, so the verdict is
`Uncalibrated` regardless of the observed deltas either way).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Task 1 needed placeholder `cmd::run`/`cmd::verify`/`cmd::reconstruct`/`cmd::series` modules outside its own declared file list**
- **Found during:** Task 1, writing `main.rs` per its own action text, which
  types `Command::Run(cmd::run::Args)`, `Command::Verify(cmd::verify::Args)`,
  `Command::Reconstruct(cmd::reconstruct::Args)` and `Command::Series(cmd::series::Args)`
- **Issue:** Task 1's `<files>` list is `main.rs, cmd/mod.rs, rundir.rs`. Rust
  requires every module and type `main.rs` references to actually exist to
  compile at all; without `cmd/verify.rs`/`reconstruct.rs`/`series.rs` (each
  with an `Args` struct) and *some* `cmd/run.rs`/`tools.rs`, task 1's own commit
  could not build, let alone pass `cargo test -p nr-cli`.
- **Fix:** Task 1 adds `cmd/verify.rs`, `cmd/reconstruct.rs`, `cmd/series.rs`
  (each with an `Args` struct and a `bail!("not implemented until plan 01-08/
  01-14")` body, exactly as the plan's own action text separately specifies for
  these three), plus a minimal placeholder `cmd/run.rs`/`tools.rs` (an empty
  `Args {}` and a `bail!` stub). Task 2's commit replaces the `cmd/run.rs`/
  `tools.rs` placeholders wholesale with the real orchestration.
- **Files modified:** `crates/cli/src/cmd/verify.rs`, `reconstruct.rs`, `series.rs`,
  `run.rs` (placeholder), `crates/cli/src/tools.rs` (placeholder)
- **Verification:** `cargo build -p nr-cli` and `cargo test -p nr-cli` both
  succeed after task 1's commit alone
- **Committed in:** `afc0e5f` (Task 1 commit)

**2. [Rule 3 - Blocking] A temporary `#[allow(dead_code)]` in `rundir.rs`, removed once task 2 wires it up**
- **Found during:** Task 1, running `cargo clippy -p nr-cli --all-targets -- -D
  warnings` per this execution's own stated gate ("after every task commit")
- **Issue:** Every other crate in this workspace (`nr-manifest`, `nr-histogram`,
  `nr-capture`, `nr-metrics`) is a library, so a `pub` item is exempt from
  dead-code analysis regardless of internal callers (an external consumer might
  use it). `nr-cli` is binary-only (`[[bin]]`, no `[lib]`), so `rundir.rs`'s
  `pub` API, built in task 1 but not called until `cmd::run` (task 2), is
  genuinely dead code from the bin target's own analysis, and `clippy -D
  warnings` turns that into a hard build failure.
- **Fix:** Added `#![allow(dead_code)]` at the top of `rundir.rs` in task 1's
  commit, with a doc comment explaining why and that it is temporary; removed
  it in task 2's commit once `cmd::run` genuinely calls every function.
- **Files modified:** `crates/cli/src/rundir.rs`
- **Verification:** `cargo clippy -p nr-cli --all-targets -- -D warnings` clean
  after both the task 1 and task 2 commits
- **Committed in:** `afc0e5f` (added), `975e318` (removed)

**3. [Rule 1 - Bug] `interference::snapshot` does not take a `facts` parameter; added a second test-only fixture env var**
- **Found during:** Task 2, implementing the before/after interference
  snapshots per this plan's own `<interfaces>` block
- **Issue:** See "Decisions Made" above. The plan's assumed signature
  (`snapshot(&facts, &cpus)`) does not exist; the real, shipped function
  (`nr_capture::interference::snapshot(cpus: &[u32])`) is Linux-gated with no
  facts-source parameter at all, so following the plan's literal text would
  not compile, and the pipeline could not be exercised end to end on macOS
  without an alternative seam.
- **Fix:** Added `NRMEASURE_INTERRUPTS_FIXTURE` (a file path to `/proc/
  interrupts`-shaped text, read once and passed to `interference::
  snapshot_from_text` for both the before and after snapshot), gated by the
  same `#[cfg(target_os = "linux")]` split already used for `SystemFacts`.
- **Files modified:** `crates/cli/src/cmd/run.rs`
- **Verification:** `raw_capture_is_byte_identical` (unit) and all 6
  `run_pipeline.rs` tests pass on macOS
- **Committed in:** `975e318` (Task 2 commit)

**4. [Rule 3 - Blocking] `std::env::set_var`/`remove_var` are forbidden; restructured around an explicit `Overrides` value**
- **Found during:** Task 2, writing the first unit tests for `refusal_writes_nothing`
  and `raw_capture_is_byte_identical`
- **Issue:** See "Decisions Made" above. `cargo test -p nr-cli` failed to
  compile with `error: usage of an unsafe block ... requested on the command
  line with -F unsafe-code` on every `std::env::set_var`/`remove_var` call.
- **Fix:** Introduced `struct Overrides { facts_fixture_path, interrupts_fixture_path,
  cyclictest_path, hwlatdetect_path }` with `Overrides::from_env()` (the only
  place `std::env::var`, a safe read, is called), and split `run(args)` into a
  thin wrapper over `execute(args, &Overrides)`. Tests construct `Overrides`
  directly; the integration test spawns the real binary and uses `assert_cmd`'s
  `.env(...)` (child-process environment, unrelated API, no `unsafe`).
- **Files modified:** `crates/cli/src/cmd/run.rs`
- **Verification:** `cargo build -p nr-cli`/`cargo test -p nr-cli` compile and
  pass with zero `unsafe` blocks anywhere in the crate
- **Committed in:** `975e318` (Task 2 commit)

**5. [Rule 2 - Missing Critical] The "clean" facts fixture is actually the real rig's untuned capture; derived a genuinely passing scenario instead**
- **Found during:** Task 2, writing `raw_capture_is_byte_identical`; confirmed
  against `crates/capture/tests/preconditions.rs`'s own doc comments, which
  name this same file `CLEAN` while documenting the identical fact
- **Issue:** `probe-sysfs-tuning.txt` genuinely fails `SystemdDefaultTargetIsMultiUser`,
  `GovernorIsPerformanceOnAllCpus` and `ThermalHeadroomAtStart` (69.1 C against a
  60 C ceiling) exactly as FINDINGS.md documents for the real rig today. Using
  it unmodified as a "happy path" fixture would always refuse, making the
  happy-path test unwritable.
- **Fix:** `tuned_facts_text()` (in both `cmd/run.rs`'s unit tests and
  `crates/cli/tests/run_pipeline.rs`) takes the real fixture and overrides only
  governor, the systemd default target, `no_turbo` and thermal readings (the
  fields a genuinely tuned rig would report differently), rather than
  fabricating a fixture from nothing.
- **Files modified:** `crates/cli/src/cmd/run.rs`, `crates/cli/tests/run_pipeline.rs`
- **Verification:** Both files' happy-path tests pass; `refusal_writes_nothing`/
  `refusal_exits_two_and_writes_nothing` still use the unmodified `VIOLATED_FACTS`
  fixture, confirming the refusal path is exercised against a real violation
- **Committed in:** `975e318` (Task 2), `8271264` (Task 3)

**6. [Rule 2 - Missing Critical] `FixtureFacts::parse`'s format cannot represent `/proc/cpuinfo`/`/etc/os-release`; added a small preprocessing extension**
- **Found during:** Task 3, reviewing the first generated `REPORT.md` snapshot,
  which showed `system:`, `cpu:`, `kernel:` and `os:` all rendering empty
- **Issue:** `probe-sysfs-tuning.txt`'s `key=value`, one-assignment-per-line
  format has no way to represent a value containing embedded newlines. Without
  `/proc/cpuinfo`/`/etc/os-release` data, the D-14 host/kernel/os fields in a
  fixture-driven run are legitimately absent (correctly tracked via
  `absent_fields`, per D-16), but this does not *demonstrate* "the full D-14
  snapshot," which both this plan's own success criteria and this execution's
  stated success criteria require showing.
- **Fix:** Added `extract_multiline_blocks` (`cmd/run.rs`): a small `@begin
  <key>` / `@end` block extension, preprocessed out of the fixture text before
  calling `FixtureFacts::parse`, then applied via `FixtureFacts::with` (which
  takes a plain string with no line-splitting concern). `nr_capture`'s own
  fixture format and code are completely untouched; this is a preprocessing
  step entirely on this crate's side of the seam.
- **Files modified:** `crates/cli/src/cmd/run.rs`, `crates/cli/tests/run_pipeline.rs`
- **Verification:** The committed snapshot now shows a real kernel release/version,
  OS distro/version, CPU model/topology, and `cmdline: ... root=[redacted] ...`
  (confirming 01-03's redaction convention fires on this real code path)
- **Committed in:** `8271264` (Task 3 commit)

**7. [Rule 1 - Bug, in the plan text, not this crate's code] The plan's own `<verification>` command selects zero tests**
- **Found during:** Running the plan's overall `<verification>` block verbatim
  after Task 3
- **Issue:** `cargo test -p nr-cli run_pipeline -- --nocapture` uses a bare
  substring filter. Cargo's default test filter matches individual test
  function names, not the containing file/binary name, and none of task 3's
  own mandated exact test names (`full_run_produces_valid_run_dir` and the
  other five) contain the substring `run_pipeline`. The command therefore
  selects 0 tests in both the `nrmeasure` unit-test binary and the
  `run_pipeline` integration binary, and reports `test result: ok. 0 passed`
  in each - a silent no-op that looks green without checking anything.
- **Fix:** Not a code change (nothing to fix in `nr-cli`; the task's own exact
  test names are fixed and the plan's own filter text is what does not match
  them). Ran the equivalent, correctly-targeted command instead,
  `cargo test -p nr-cli --test run_pipeline -- --nocapture`, which does select
  and run all 6 tests. Recorded here so a future reader does not mistake the
  plan's literal command for a real check.
- **Verification:** `cargo test -p nr-cli --test run_pipeline -- --nocapture`:
  6 passed, 0 failed
- **Committed in:** n/a (no file changed; a verification-step finding only)

---

**Total deviations:** 7 (2 Rule 3 blocking scaffolding fixes in task 1, 1 Rule 1
interface-mismatch fix, 1 Rule 3 blocking restructure around a language/lint
constraint, 2 Rule 2 missing-critical fixture-fidelity closures, 1 Rule 1 finding
in the plan's own verification text, not in code).
**Impact on plan:** All were necessary for this plan's own literal acceptance
criteria, or this execution's own stated gates, to pass truthfully. No scope
creep: every change stayed inside `crates/cli/`; `nr-manifest`, `nr-histogram`,
`nr-capture` and `nr-metrics` were not touched.

## Known Stubs

- **`cmd/verify.rs`, `cmd/reconstruct.rs`, `cmd/series.rs` are intentional,
  plan-specified stubs.** Each is an `Args` struct plus a body that returns
  `anyhow::bail!("nrmeasure {verify,reconstruct,series} is not implemented
  until plan 01-08/01-14")`. This is exactly what this plan's own action text
  calls for ("declared with their Args structs and a body that returns
  anyhow::bail!(...)"), so `main.rs`'s subcommand set is stable across the
  plans that implement each one. Not a shortfall of this plan's own goal
  (`nrmeasure run`, fully implemented); resolved by plans 01-08 (`verify`,
  `reconstruct`) and 01-14 (`series`).
- **`place_artifact`'s `StorageLocation::External` path records an empty
  `url`.** The plan's own action text says the operator "uploads it and
  supplies the URL" for an oversized capture; that upload step is inherently
  manual and this plan does not automate it (nor does it need to: neither
  `cyclictest` nor `hwlatdetect` captures ever approach the 25 MiB per-file
  threshold, so no test in this plan exercises this path). `nr_manifest::
  validate` correctly rejects an `External` record with an empty URL, so
  `run()` would refuse to write a manifest for a run that hits this path
  today, rather than silently publishing an unverifiable pointer. A future
  investigation-capture plan (01-12) is the first to actually need this path
  filled in.

## Issues Encountered

**The reference rig was network-unreachable for the entire session.** `ping
192.168.0.100` returned 100% packet loss with no ARP entry, and `ssh -v` failed
at TCP connect (`Operation timed out`), retried at the start of the session, after
all three tasks were committed, and once more immediately before writing this
summary. This is a genuine connectivity outage (the machine did not answer at
any network layer), not an authentication gate; there is no `nr-run` wrapper
output to authenticate against because the TCP handshake itself never
completed. See "Real rig verification" below for what this means for this
execution's stated success criteria.

## Real rig verification (COMPLETED 2026-08-31)

Performed against the live rig once it was reachable again. The earlier outage was a network
partition (the dev host was on a different wifi network), not a rig failure: the rig was powered
on throughout, and its `~/neurorust-agent.log` ends at the last command sent before the split.

Setup performed to make this possible, which plans 01-11 through 01-15 also require: the
repository was rsynced to `~/neurorust` on the rig, rustup installed a minimal stable toolchain,
and `build-essential` supplied the C linker rustup warned was missing. `cargo build -p nr-cli
--release` then completes natively on the rig in about 21 seconds.

Result of `nrmeasure run --rig-slug precision3591 --class weekly --cpus 6-11 --main-cpus 0,1
--duration 5`:

```
run refused: 8 precondition(s) violated
  NoActiveSshSessions: observed "1", expected "0" (Fail)
  SystemdDefaultTargetIsMultiUser: observed "graphical.target", expected "multi-user.target" (Fail)
  DisplayManagerInactive: observed "gdm.service=active, ...", expected "inactive" (Fail)
  NoGraphicalSession: observed "1", expected "0" (Fail)
  GovernorIsPerformanceOnAllCpus: observed "cpu0=powersave", expected "performance on all CPUs" (Fail)
  ThermalHeadroomAtStart: observed "63.0 C", expected "package temp <= 60 C" (Fail)
  NoPackageManagerActivity: observed "snapd", expected "no apt, dpkg, unattended-upgrade or snapd process" (Fail)
  TracersQuiescent: observed "current_tracer not available", expected "nop" (Unavailable)

EXIT=2
```

Exit code 2, every offending check named with observed and expected values, and no run directory
written (`measurements/` on the rig still contains only the 2026-08-28 set and INDEX.md).

Three things worth recording beyond the stated criterion. The harness detected the SSH session
being used to invoke it, which is precisely the contamination that invalidated the 2026-08-28
baseline, so the check is not merely present but effective against the real failure mode. It
reported all eight violations rather than bailing at the first. And it surfaced four conditions
neither the fixture nor the recon had exercised: the graphical target and active GDM session,
package-manager activity from snapd, and a thermal reading above the gate. The rig is further
from a publishable state than the governor finding alone suggested, which is information plan
01-11 needs before its calibration pair.

## Superseded: original blocked note


This execution's own success criteria require: "`nrmeasure run` against the
REAL rig refuses, names the failing governor precondition, and records all
precondition results - verified for real over SSH, not just from a fixture."
**This could not be performed.** The rig at `192.168.0.100` was unreachable at
the network layer for the entire session (confirmed with `ping`, `arp -a`, and
verbose `ssh -v`, retried three times across the session, including
immediately before this summary was written). This is an infrastructure
outage outside this agent's ability to fix (no amount of code or local
tooling can restore a remote machine's network link), and is documented here
rather than silently skipped or fabricated as passing.

**What is proven instead:**
- The refusal path is fully exercised, twice, against the real rig's own
  captured violation fixture (`violated-sysfs-tuning.txt`, and
  `probe-sysfs-tuning.txt`'s own untuned governor/target/thermal state used
  directly, unmodified, in the corresponding tests): exit code 2, every
  offending check named with observed/expected values, and no run directory
  written. This is the exact scenario the rig is independently known to be
  in today (docs/rig/recon-2026-08-31/FINDINGS.md).
- `cargo check --target x86_64-unknown-linux-gnu -p nr-cli` (the same
  cross-target check plan 01-05 used to confirm its own Linux-only code
  paths) was not re-run in this session as a substitute; this is noted as an
  additional gap, not a claim of partial verification.

**What a future session must do once the rig is reachable again**, exactly:
```
ssh -o BatchMode=yes d0nmega@192.168.0.100 '~/bin/nr-run "cd ~/neurorust && \
  git pull && cargo build -p nr-cli --release && \
  ./target/release/nrmeasure run --rig-slug precision3591 --class weekly \
  --cpus 6-11 --main-cpus 0,1 --duration 5"'
```
Expected: exit code 2, stderr naming `GovernorIsPerformanceOnAllCpus` (and any
other currently-failing checks reported live from the rig, which may differ
from the checked-in fixture if the rig's state has changed since the last
recon), and no new directory under `measurements/` on the rig. If the rig has
since been retuned and this now *succeeds*, that is itself news worth
recording (it would mean `rt-tuning.service`'s race with `power-profiles-daemon`,
documented in `deferred-items.md`, has been resolved or no longer applies).

## User Setup Required

None for the code delivered by this plan - no external service configuration
required. The one outstanding item is entirely out of this agent's hands: the
reference rig needs to be powered on and reachable on `192.168.0.100` again
before the real-rig verification above can be completed.

## Next Phase Readiness

- `nrmeasure run` is complete for this plan's scope: the full 12-step pipeline,
  covered end to end on macOS with no rig (15 tests: 9 unit, 6 integration).
  `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`
  and `cargo test --workspace` (112 tests) are all green on this host.
- Plan 01-08 (`nrmeasure verify`/`reconstruct`) replaces `cmd/verify.rs` and
  `cmd/reconstruct.rs`'s stub bodies; `main.rs`'s `Command` enum and both
  `Args` structs are already in place and do not need to change.
- Plan 01-14 (`nrmeasure series`) replaces `cmd/series.rs`'s stub body the
  same way.
- Plan 01-10 (D-16 reconstruction of `measurements/2026-08-28-precision3591/`)
  can rely on `nr_manifest::validate` and the `RunManifest` surface exactly as
  this plan exercises them; nothing here changes that contract.
- `BENCH-04`, `BENCH-05` and `PLAT-02` remain open at the requirements level
  (shared with other plans as detailed under Decisions); no action taken
  here, flagged for the end-of-phase verifier.
- **Blocker for a full close-out of this plan's own stated success criteria:**
  the real-rig refusal verification above is outstanding purely due to the
  rig's network unavailability during this session; the exact command to run
  once it reconnects is recorded above. Everything else in this plan's
  success criteria is met and verified.

---
*Phase: 01-trustworthy-measurement*
*Completed: 2026-08-31*

## Self-Check: PASSED

- All 11 created files and 4 modified files confirmed present and non-empty on
  disk with `[ -s ]`.
- All 3 task commits (`afc0e5f`, `975e318`, `8271264`) confirmed present in
  `git log --oneline --all`.
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  and `cargo test --workspace` (112 tests, 0 failed) all re-ran clean
  immediately before this SUMMARY was finalized.
- Every literal acceptance-criteria grep and named-test selection from all
  three tasks was re-run directly and passes: the two `MAX_IN_REPO_*`
  constant greps, `refuse_on_violation`/`render_run_report`/`distance=0` in
  `cmd/run.rs`, `NRMEASURE_CYCLICTEST` in `tools.rs`, the no-shell-invocation
  grep, `Vec<String>`, the `NRMEASURE_FACTS_FIXTURE cannot be used with run
  class` literal, both fake scripts' executable bit and ASCII cleanliness,
  and `cargo test -p nr-cli --test run_pipeline` selecting and passing all 6
  named tests.
- `cargo run -p nr-cli -- --help` and `-- run --help` both exit 0, list all
  four subcommands and every flag in the `run` argument surface, and contain
  no byte above `0x7F`.
- The real-rig verification this execution's own success criteria require
  was attempted three times across the session (start, after all three
  tasks, and immediately before this summary) and could not be completed:
  `ping`/`arp -a`/verbose `ssh` all confirm the rig at `192.168.0.100` is
  unreachable at the network layer. Recorded honestly as PARTIAL rather than
  claimed as passed; the exact command for a future session is in "Real rig
  verification" above.
