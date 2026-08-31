---
status: PASS
agent: donny-executor
phase: 01-trustworthy-measurement
plan: 05
subsystem: measurement-capture
tags: [rust, sysfs, procfs, systemd, preconditions, contamination, d-06, d-14, d-15]

# Dependency graph
requires:
  - phase: 01-trustworthy-measurement (plan 02)
    provides: real rig fixtures (probe-sysfs-tuning.txt, probe-proc-interrupts.txt) and
      FINDINGS.md's rig-state corrections (governor race, cpuidle C6/C10 absence,
      the correct no_turbo sysfs path)
  - phase: 01-trustworthy-measurement (plan 03)
    provides: nr_manifest's PreconditionCheck/Result/Status, InterferenceSnapshot
      family, ContaminationVerdict, HostInfo/KernelInfo/OsInfo/TuningInfo/PowerInfo/
      NetworkInfo, KernelInfo::redact_cmdline, and the argv-relativization convention
      documented on ToolInvocation.argv
provides:
  - nr-capture crate with the 14 D-06 precondition checks (assert and record, never
    enforce), the D-14 environment snapshot readers, and the D-15 interference diff
    and automatic contamination verdict
  - SystemFacts trait (sources.rs) so every check and every snapshot field is
    exercised by real logic on macOS via FixtureFacts, not skipped
  - A standalone, tested argv-relativization utility (argv.rs) closing plan 01-03's
    carried-forward Decision B, ready for nr-cli to call
  - config/contamination-thresholds.json shipped uncalibrated with the D-17
    provenance contract enforced at load time
affects: [01-06, 01-07, 01-11, 01-12]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "SystemFacts trait behind every read: LiveFacts (Linux-gated) and FixtureFacts
      (any platform) both answer read_text/path_exists/list_dir/systemctl_show/...,
      so the same check logic is exercised for real on macOS CI, not decorated
      around a Linux-only stub (RESEARCH.md pitfall 5)"
    - "Index-probing for paired sysfs data: cpuidle states and thermal zones are
      enumerated by trying .../state{N}/{name,disable} and
      .../thermal_zone{N}/{type,temp} for increasing N until the first absent index,
      identically in LiveFacts and FixtureFacts, rather than adding a
      check-specific trait method for each"
    - "Absence is a value, not a crash: every HostInfo/KernelInfo/OsInfo field that
      cannot be read gets an explicit AbsentField plus an empty-string/0/false
      placeholder, since those structs have no Option fields to hold a genuine
      absence (D-16's 'never guess' extended to live capture, not just
      reconstruction)"

key-files:
  created:
    - crates/capture/src/sources.rs
    - crates/capture/src/preconditions.rs
    - crates/capture/src/environment.rs
    - crates/capture/src/interference.rs
    - crates/capture/src/argv.rs
    - crates/capture/tests/preconditions.rs
    - crates/capture/tests/interference.rs
    - crates/capture/tests/fixtures/violated-sysfs-tuning.txt
    - crates/capture/tests/fixtures/proc-interrupts-after-clean.txt
    - crates/capture/tests/fixtures/proc-interrupts-after-contaminated.txt
    - config/contamination-thresholds.json
  modified:
    - crates/capture/src/lib.rs

key-decisions:
  - "procfs 0.18.0 has no /proc/interrupts parser at all (verified against the
    published procfs-0.18.0 and procfs-core-0.18.0 crate sources), contrary to
    RESEARCH.md's and this plan's own assumption. Hand-rolled a small, scoped
    parser (parse_proc_interrupts) shared between the live path and every test,
    rather than the procfs::interrupts() call the plan's action text names."
  - "GovernorIsPerformanceOnAllCpus and TracersQuiescent correctly FAIL/refuse
    against the real rig's current state (every governor reads powersave; GDM is
    active) per this execution's explicit instructions. Not weakened."
  - "DeepCstatesDisabled treats an absent C6/C10 cpuidle state directory as a pass,
    per this execution's explicit instructions and confirmed live on the rig
    (cpu0 has exactly POLL and C1E; state2 does not exist)."
  - "nr_manifest::InterferenceSnapshot.context_switches (typed Vec<CpuCounter>, one
    entry per isolated CPU) is populated from /proc/interrupts' RES row rather than
    /proc/stat's ctxt, since ctxt is a single machine-wide counter with no per-CPU
    breakdown and cannot fill that field's type at all."
  - "D-15's contamination reason lives in a local VerdictOutcome wrapper around
    nr_manifest::InterferenceSnapshotPair, not as a new field on that type: none of
    this plan's files include crates/manifest/, and adding a required field to an
    already-shipped, deny-unknown-fields, cross-phase-contract schema (plan 01-03)
    would ripple into schemas/manifest.schema.json and the committed
    minimal-manifest.json fixture, none of which this plan touches."
  - "Argv relativization (plan 01-03's deferred-items.md Decision B) implemented as
    a standalone module (argv.rs) rather than inline in a task, because none of
    this plan's three tasks shell out to a tool or build a ToolInvocation
    themselves; nr-cli (plan 01-07) does that, per this plan's own <interfaces>
    block. relativize_argv is the tested utility it will call."
  - "SystemFacts gained one method beyond the plan's seven-method sketch, list_dir,
    because network interface names (enp0s31f6, wlp0s20f3) are not a probeable
    contiguous numeric index the way cpuidle states and thermal zones are."
  - "FixtureFacts::parse and Thresholds::parse are named parse, not from_str: naming
    either from_str (as the plan's own action-text example does for FixtureFacts)
    trips clippy::should_implement_trait, which is denied via clippy::all at the
    workspace level, even with a real std::str::FromStr impl also present."

requirements-completed: [PLAT-02, BENCH-04, BENCH-06]

# Metrics
duration: ~50min
completed: 2026-08-31
---

# Phase 01 Plan 05: nr-capture (D-06 preconditions, D-14 environment snapshot, D-15 contamination verdict) Summary

**nr-capture crate: 14 assert-and-record preconditions, a fixture-testable SystemFacts trait spanning macOS and Linux, D-14 host/kernel/tuning/power/network readers with explicit absence tracking, and a D-15 contamination verdict that hand-parses /proc/interrupts after discovering procfs 0.18.0 has no support for it at all.**

## Performance

- **Duration:** ~50 min
- **Started:** approximately 2026-08-31T06:22:00Z
- **Completed:** 2026-08-31T07:09:28Z
- **Tasks:** 3 (plus one carried-forward addition, argv relativization)
- **Files changed:** 12 (11 created, 1 modified)

## Accomplishments

- 14 typed D-06 precondition checks, each returning a `PreconditionResult` whether
  it passed or failed; `run_all` always returns exactly 14 in stable order, and
  `refuse_on_violation` refuses on any `Fail` (and on `Unavailable` for a
  headline-class run), naming every offense with its observed and expected values
- `SystemFacts` trait (`sources.rs`) abstracts every sysfs/procfs/systemd read
  behind one interface; `LiveFacts` (Linux-only) and `FixtureFacts` (any platform)
  both implement it, so all 14 checks and every D-14 field are exercised by real
  logic on the macOS dev host, not skipped (RESEARCH.md pitfall 5)
- `GovernorIsPerformanceOnAllCpus` reads `cpufreq/scaling_governor` directly and
  correctly fails against the real rig fixture (every CPU reads `powersave` despite
  `rt-tuning.service` reporting healthy); `DeepCstatesDisabled` treats an absent
  `C6`/`C10` cpuidle directory as satisfied rather than unreadable
- D-14 environment snapshot (`environment.rs`) fills `HostInfo`/`KernelInfo`/
  `OsInfo`/`TuningInfo`/`PowerInfo`/`NetworkInfo` from real sources; every field
  that cannot be read gets an explicit `AbsentField` rather than a guessed value.
  `kernel.cmdline` is redacted via `nr_manifest::KernelInfo::redact_cmdline` before
  it is stored; the module reads no hostname, machine-id, hardware address, or
  wireless network name
- D-15 interference diff and verdict (`interference.rs`): discovered `procfs`
  0.18.0 has no `/proc/interrupts` parser at all (RESEARCH.md's assumption did not
  hold), so this module hand-parses the text format once and shares it between the
  live path and every test. `Thresholds::parse` enforces D-17's calibration
  provenance requirement; the shipped `config/contamination-thresholds.json` is
  uncalibrated and `verdict()` never renders `Clean`/`Contaminated` for it. A
  `Contaminated` verdict is never dropped (BENCH-06)
- Closed plan 01-03's carried-forward argv-relativization obligation (`argv.rs`),
  a standalone tested utility, since none of this plan's three tasks build a
  `ToolInvocation` themselves
- Verified `LiveFacts`'s actual commands read-only against the real rig over SSH
  (`ss`, `loginctl show-session` types, `systemctl show` property format, cpuidle
  state0/state1/absent-state2, network wireless-directory classification, thermal
  zone type/temp, `/proc/cpuinfo` field layout) since cross-compilation
  type-checking alone cannot prove runtime behavior

## Task Commits

Each task was committed atomically:

1. **Task 1: The 14 preconditions, read through a trait so both CI legs test them for real** - `2a15f35` (feat)
2. **Task 2: The D-14 environment snapshot readers** - `6d3e7dc` (feat)
3. **Task 3: The interference diff and the contamination verdict, with an uncalibrated state** - `d8e15a8` (feat)
4. **Addition: relativize tools[].argv output-file paths (plan 01-03 carry-forward)** - `58ea50f` (feat)

**Plan metadata:** committed separately after this SUMMARY (see final commit).

## Files Created/Modified

- `crates/capture/src/sources.rs` - `SystemFacts` trait, `LiveFacts` (Linux-gated),
  `FixtureFacts`, and shared discovery helpers (governor count, cpuidle states,
  thermal zones, AC/battery candidates, CPU-list parsing)
- `crates/capture/src/preconditions.rs` - the 14 checks, `PreconditionSpec`,
  `run_all`, `refuse_on_violation`, `RefusalError`, `THERMAL_HEADROOM_CEILING_C`
- `crates/capture/src/environment.rs` - `EnvironmentSnapshot`, `snapshot()`, and the
  per-struct field builders with absence tracking
- `crates/capture/src/interference.rs` - `parse_proc_interrupts`,
  `snapshot_from_text`/`snapshot`, `Thresholds`, `verdict()`, `VerdictOutcome`
- `crates/capture/src/argv.rs` - `relativize_argv`, closing the 01-03 carry-forward
- `crates/capture/tests/preconditions.rs` - the 7 named behavior tests
- `crates/capture/tests/interference.rs` - the 7 named behavior tests
- `crates/capture/tests/fixtures/violated-sysfs-tuning.txt` - derived from the real
  probe with `no_turbo`, `current_tracer` and `ac_online` deliberately flipped
- `crates/capture/tests/fixtures/proc-interrupts-after-{clean,contaminated}.txt` -
  derived from the real probe with per-CPU CAL/TLB/RES drift (clean) or an
  additional ~137000 CAL / ~5000 TLB spike on cpus 6-11 (contaminated)
- `config/contamination-thresholds.json` - shipped uncalibrated per D-17
- `crates/capture/src/lib.rs` - registers all four new modules

## Decisions Made

See the frontmatter `key-decisions` block for the full list. The two with the
widest downstream effect:

**procfs 0.18.0 cannot parse `/proc/interrupts`.** Verified by downloading and
grepping the published `procfs-0.18.0` and `procfs-core-0.18.0` crate sources
directly (no `interrupt`-related symbol anywhere in either). RESEARCH.md's
`## Code examples` section and this plan's own Task 3 action text both assume
`procfs::interrupts()` exists; it does not, at this pinned version. Rather than
silently working around this, `interference.rs`'s module doc comment states the
finding plainly, and the crate hand-rolls a small, scoped `/proc/interrupts` text
parser used identically by the live path and every test. This is arguably a
stronger outcome for RESEARCH.md pitfall 5 than the original plan: the same parser
that would run on the rig is the one every test exercises, rather than "the same
format, reimplemented separately for tests."

**The contamination reason lives outside nr_manifest's schema.** D-15 requires the
verdict to name the offending counter and CPU, but `nr_manifest::
InterferenceSnapshotPair` (already shipped by plan 01-03, with a generated JSON
Schema and a drift test) has no field for one. Adding one would touch
`crates/manifest/`, `schemas/manifest.schema.json`, and the committed
`minimal-manifest.json` fixture, none of which are in this plan's file list, and
none of which a single-crate plan should change unreviewed. `interference::verdict`
returns a local `VerdictOutcome { pair, reason }` instead, leaving nr_manifest's
schema untouched.

**BENCH-04/BENCH-06/PLAT-02 traceability.** This plan carries these three jointly
with several other plans (BENCH-04 with 01-01/01-03/01-07/01-08/01-10; BENCH-06
with 01-06; PLAT-02 with the eventual protocol document and 01-07/01-08). Per
explicit instruction, `requirements mark-complete` was not run and
`.planning/REQUIREMENTS.md` was left untouched; the end-of-phase verifier owns
marking these complete once every contributing plan has landed.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] procfs 0.18.0 has no `/proc/interrupts` support; hand-rolled a scoped parser instead**
- **Found during:** Task 3, before writing any interference-diff code, per this
  task's own `<read_first>` instruction to check the real crate behavior
- **Issue:** RESEARCH.md's "Code examples" section and this plan's Task 3 action
  text both instruct using `procfs::interrupts()` for the live `/proc/interrupts`
  parse. Downloaded and grepped the actual published `procfs-0.18.0` and its
  `procfs-core-0.18.0` dependency: neither source tree contains any
  `/proc/interrupts` or per-IRQ-row parsing at all; the only interrupt-adjacent
  fields are CPU-time-in-irq counters in `/proc/stat`'s `KernelStats`, unrelated to
  the per-CPU, per-label (`CAL`, `TLB`, `RES`, ...) table this module needs.
- **Fix:** Wrote `parse_proc_interrupts`, a small parser scoped to exactly the rows
  D-15 needs (greedily consuming up to `num_cpus` integer tokens per row, which
  correctly separates counts from trailing driver/device text without parsing the
  descriptor). Used identically by `snapshot_from_text` (every test) and the
  `#[cfg(target_os = "linux")]` `snapshot()` wrapper (a two-line pass-through with
  no untested logic of its own).
- **Files modified:** `crates/capture/src/interference.rs`
- **Verification:** `cargo test -p nr-capture` green on macOS and cross-compiled
  Linux type-checking (`cargo check --target x86_64-unknown-linux-gnu`); all 7
  named interference behaviors pass against the real and derived fixtures
- **Committed in:** `d8e15a8` (Task 3 commit)

**2. [Rule 2 - Missing Critical] Contamination reason carried in a local wrapper, not a new nr_manifest field**
- **Found during:** Task 3, implementing `verdict()`'s described behavior ("the
  pair carries the verdict plus a reason string")
- **Issue:** `nr_manifest::InterferenceSnapshotPair` (plan 01-03's already-shipped,
  schema-generated, `deny_unknown_fields` type) has no field for a reason string.
  D-15's own behavior requirement (`verdict_contaminated`: "the reason names CAL
  and the CPU") cannot be satisfied by returning that type alone.
- **Fix:** Added a local `VerdictOutcome { pair: InterferenceSnapshotPair, reason:
  Option<String> }` return type in `interference.rs`, leaving nr_manifest's schema,
  its generated `schemas/manifest.schema.json`, and its committed
  `minimal-manifest.json` fixture completely untouched. A future caller (nr-cli)
  folds `reason` into `RunManifest.exclusion_reason` when publishing.
- **Files modified:** `crates/capture/src/interference.rs`
- **Verification:** `verdict_contaminated` test asserts the reason names `CAL` and
  a `cpu` token; `cargo test -p nr-manifest` (unaffected by this plan) remains
  unchanged and green
- **Committed in:** `d8e15a8` (Task 3 commit)

**3. [Rule 2 - Missing Critical] context_switches populated from RES, not /proc/stat's ctxt**
- **Found during:** Task 3, mapping D-15's counters onto
  `nr_manifest::InterferenceSnapshot.context_switches: Vec<CpuCounter>`
- **Issue:** That field's type requires one count per isolated CPU.
  `/proc/stat`'s `ctxt` line is a single machine-wide total with no per-CPU
  breakdown, so it cannot populate this field at all regardless of implementation
  effort.
- **Fix:** Populated `context_switches` from `/proc/interrupts`' `RES`
  (rescheduling interrupt) row instead, the closest true per-CPU scheduling-
  interference signal available without a heavier tracer (RESEARCH.md pattern 2).
  Documented directly on the field's population code and in the module doc
  comment. `CalibratedThresholds.res_delta_max` gates it; `context_switch_delta_max`
  is still loaded and validated for presence (a `calibrated` file must set it) but
  not yet independently compared, since there is no second per-CPU source for it.
- **Files modified:** `crates/capture/src/interference.rs`
- **Verification:** `contaminated_run_is_not_dropped` and `verdict_clean`/
  `verdict_contaminated` all pass against the real and derived fixtures
- **Committed in:** `d8e15a8` (Task 3 commit)

**4. [Rule 2 - Missing Critical] Argv relativization added as a standalone module**
- **Found during:** Reviewing this execution's explicit success criteria and
  `.planning/phases/01-trustworthy-measurement/deferred-items.md`'s "From 01-03"
  entry, both of which state this crate owns the `tools[].argv` relativization
  rewrite plan 01-03 documented but could not implement
- **Issue:** None of this plan's three tasks build a `ToolInvocation` or shell out
  to any tool (confirmed against the plan's own `<interfaces>` block: the
  measurement run happens between two `interference::snapshot` calls, outside
  `nr-capture`, in the future nr-cli). Without action, the explicitly-flagged
  obligation would go unaddressed by this plan with no note explaining why.
- **Fix:** Added `crates/capture/src/argv.rs`: `relativize_argv(run_dir, argv)`
  rewrites any absolute-path token (bare or `--flag=value`) under `run_dir` to a
  relative path, leaving everything else untouched. A small, self-contained,
  fully tested utility ready for nr-cli (plan 01-07) to call at the point it
  actually captures an argv.
- **Files modified:** `crates/capture/src/argv.rs`, `crates/capture/src/lib.rs`
- **Verification:** 3 new tests (relativizes an output-file path under the run
  directory, leaves a path outside it untouched, leaves bare flags/relative paths
  untouched) all pass
- **Committed in:** `58ea50f` (separate commit, since it addresses a cross-plan
  carry-forward rather than any of the plan's own three tasks)

**5. [Rule 1 - Bug] `from_str`-named methods renamed to `parse` to satisfy clippy**
- **Found during:** Running `cargo clippy --workspace --all-targets -- -D
  warnings` after Task 3
- **Issue:** `FixtureFacts::from_str` (named exactly as Task 1's own action-text
  example shows, `FixtureFacts::from_str(text)?.with(...)`) and
  `Thresholds::from_str` both trip `clippy::should_implement_trait`, denied via
  `clippy::all` at the workspace level (`Cargo.toml`
  `[workspace.lints.clippy] all = { level = "deny", ... }`). Implementing the real
  `std::str::FromStr` trait alongside an inherent method of the same name does not
  suppress the lint; it still fires on the inherent method.
- **Fix:** Renamed both to `parse` (not a clippy-recognised trait-method
  look-alike), updating every call site. `FixtureFacts::parse` remains infallible
  (`-> Self`); `Thresholds::parse` remains fallible (`-> Result<Self,
  InterferenceError>`), matching each type's actual failure mode rather than
  forcing a uniform signature.
- **Files modified:** `crates/capture/src/sources.rs`, `crates/capture/src/
  interference.rs`, `crates/capture/tests/preconditions.rs`, `crates/capture/tests/
  interference.rs`, `crates/capture/src/environment.rs`
- **Verification:** `cargo clippy --workspace --all-targets -- -D warnings` clean
  on both the macOS host target and `--target x86_64-unknown-linux-gnu`
- **Committed in:** `2a15f35` (Task 1), `d8e15a8` (Task 3) - each commit contains
  the renamed form only, since the rename was applied before either was committed

**6. [Rule 1 - Bug] `names.iter().any()` replaced with `names.contains()` on the Linux-only code path**
- **Found during:** Running `cargo clippy --target x86_64-unknown-linux-gnu
  --workspace --all-targets -- -D warnings`, which this plan's environment notes
  do not list as a required gate but which exercises `LiveFacts`'s
  `#[cfg(target_os = "linux")]` code for the first time
- **Issue:** `clippy::manual_contains` (also denied via `clippy::all`) flagged
  `running_processes_matching`'s `names.iter().any(|name| *name == comm)`. This
  code path never compiles on macOS, so macOS-only clippy runs cannot catch it.
- **Fix:** `names.contains(&comm)`, equivalent and idiomatic.
- **Files modified:** `crates/capture/src/sources.rs`
- **Verification:** Linux-target clippy clean; re-ran the full macOS gate
  (`fmt --check`, `clippy --workspace --all-targets -- -D warnings`,
  `test --workspace`) afterward, still green
- **Committed in:** `2a15f35` (Task 1 commit, before the reformat/re-verify pass)

---

**Total deviations:** 6 auto-fixed (1 bug correcting a RESEARCH.md/plan tooling
assumption against the real crate, 3 missing-critical closures of gaps between the
plan's prose and nr_manifest's already-shipped schema or another plan's carried-
forward obligation, 2 bugs surfaced only by running clippy against the actual
Linux target rather than macOS alone).
**Impact on plan:** All six were necessary for the plan's own literal verification
commands and success criteria to pass, or to close an obligation this plan was
explicitly told it owns. No scope creep beyond `crates/capture/` and its own
`config/contamination-thresholds.json`; nr_manifest's schema, its generated JSON
Schema, and its committed fixture were left untouched throughout.

## Issues Encountered

**Self-inflicted file collision during the staged-commit process, caught before
any commit.** While constructing atomic per-task commits by temporarily relocating
not-yet-committed files to a scratch directory (so each commit's tree state is
independently buildable and testable, matching the sampling requirement to run the
full suite after every task commit), `src/interference.rs` and
`tests/interference.rs` were both moved to the same scratch filename in one batched
command, and the second `mv` silently overwrote the first. This was caught
immediately afterward (`head -5` on the restored file showed the test file's
content, not the module's), before any commit referenced the corrupted state.
`src/interference.rs` was reconstructed from this session's own prior tool-call
context (the file had been written once already, with two subsequent fixes: a
`#[derive(Debug)]` addition and the `from_str` -> `parse` rename described in
deviation 5 above); `cargo fmt`, `cargo clippy --workspace --all-targets -- -D
warnings` (both macOS and Linux targets), and `cargo test -p nr-capture` all
reproduced the exact same passing state as immediately before the mistake,
including the same test counts, giving confidence the reconstruction is
functionally identical to what was lost. No broken or partial state was ever
committed.

## User Setup Required

None - no external service configuration required. SSH access and the rig's
passwordless read-only script wrapper (`~/bin/nr-run`) were already in place from
plan 01-02; used here only for read-only verification (`ss`, `loginctl`,
`systemctl show`, `cat` on sysfs paths), never to change rig state.

## Next Phase Readiness

- `nr-capture` is complete for this plan's scope: 14 D-06 preconditions, the D-14
  environment snapshot, and the D-15 interference diff and verdict, all tested
  through `SystemFacts` on macOS and cross-compilation-checked plus read-only
  SSH-verified against the real rig for the Linux-only `LiveFacts` path. `cargo
  fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` (macOS and
  `x86_64-unknown-linux-gnu`), and `cargo test --workspace` (74 tests) are all
  green.
- Plan 01-07 (`nr-cli`) can call `capture::sources::live()`,
  `capture::preconditions::{run_all, refuse_on_violation}`,
  `capture::environment::snapshot()`, `capture::interference::{snapshot,
  verdict}`, and `capture::argv::relativize_argv()` directly, exactly as this
  plan's `<interfaces>` block anticipated (with `refuse_on_violation` and
  `interference::verdict` each taking one additional parameter beyond that
  block's illustrative signatures: `instrument_class` and `run_duration`
  respectively, both required by their own documented behavior).
- Plan 01-11 (the D-17 calibration pair) has a `Thresholds::load`/`parse` ready to
  consume its output; the shipped `config/contamination-thresholds.json` will need
  its `status`, `calibration.derived_from` (at least 2 run ids), and every
  `per_run_hour` value filled in once that pair exists.
- Plan 01-12 (PLAT-01 investigation) can rely on `IsolcpusCoversTargetCpus` and
  `TracersQuiescent` as already-verified-correct against the real rig's current
  `isolcpus=6-11` and tracer state.
- The rig's live state (verified read-only via SSH during this execution) still
  shows `systemd.default_target=graphical.target`, GDM active with a real Wayland
  session, and every CPU governor at `powersave`. Any plan that runs the actual
  measurement protocol (01-07 onward) will see `refuse_on_violation` correctly
  refuse a headline-class run against this state until it is retuned; this is the
  D-06 assertion list working as intended, not a bug to route around.
- `PLAT-02`, `BENCH-04` and `BENCH-06` remain open at the requirements level
  (shared with other plans as detailed under Decisions); no action needed here,
  flagged for the end-of-phase verifier.

---
*Phase: 01-trustworthy-measurement*
*Completed: 2026-08-31*

## Self-Check: PASSED

- All 11 created files and 1 modified file (`crates/capture/src/lib.rs`) confirmed
  present and non-empty on disk with `[ -s ]`.
- All 4 commits (`2a15f35`, `6d3e7dc`, `d8e15a8`, `58ea50f`) confirmed present in
  `git log --oneline --all`.
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`
  (both the macOS host target and `--target x86_64-unknown-linux-gnu`), and
  `cargo test --workspace` (74 tests: 21 nr-capture unit, 7
  `tests/preconditions.rs`, 7 `tests/interference.rs`, plus the unaffected
  nr-histogram/nr-manifest/nr-metrics/nr-cli suites) all re-ran clean immediately
  before this SUMMARY was finalized.
- Every literal acceptance-criteria command from the plan's three tasks (the
  `cfg(target_os = "linux")` greps, the no-mutation grep, `THERMAL_HEADROOM_CEILING_C`,
  `rig_slug: &str`, the `AbsentField` count, the host-identifier grep, the
  `config/contamination-thresholds.json` shape checks, `ContaminationVerdict`,
  `procfs`, and the no-hardcoded-threshold grep) were re-run directly and pass.
- Every field of `HostInfo`, `KernelInfo`, `OsInfo`, `TuningInfo`, `PowerInfo` and
  `NetworkInfo` confirmed present as an assignment target in `environment.rs` via
  a script comparing field names extracted from `crates/manifest/src/fields.rs`.
- `LiveFacts`'s actual commands (not just its types) verified read-only against
  the real rig over SSH: `ss`, `loginctl` (including real session `Type` values),
  `systemctl show` property-line format, cpuidle state0/state1/absent-state2,
  `/sys/devices/system/cpu/isolated`, power-supply and net interface directory
  names, wireless-directory classification, `uevent` `DRIVER=` format, thermal
  zone `type`/`temp`, and `/proc/cpuinfo` field layout all matched this
  implementation's assumptions exactly.
