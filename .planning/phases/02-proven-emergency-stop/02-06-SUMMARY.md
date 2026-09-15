---
status: PASS
agent: donny-executor
phase: 02-proven-emergency-stop
plan: 06

subsystem: measurement-harness
tags: [nr-stop-harness, clock, scheduling, characterisation, trial-loop, nr-histogram, cristian-algorithm]

# Dependency graph
requires:
  - phase: 02-proven-emergency-stop
    plan: 02
    provides: "the nr-stop public API (State/step, Cause/Event, OutputPermit, EmergencyStop/
      AbortRecord) this harness drives via EmergencyStop::abort/acknowledge/state/abort_record"
  - phase: 02-proven-emergency-stop
    plan: 04
    provides: "the blocking Kani and coverage gates (this plan's own depends_on)"
provides:
  - "nr-histogram gains stats_from_samples: percentiles over a flat u64 slice at 5 significant
    figures, exact min/max from the slice itself, additive and non-breaking to the existing
    cyclictest path"
  - "nr-stop-harness, the sixth workspace member, a separate binary depending on nr-stop,
    nr-histogram, nr-metrics, nr-capture and nr-manifest, never the reverse"
  - "MonotonicRawClock/RawClock/FixtureClock (clock.rs): CLOCK_MONOTONIC_RAW on Linux via nix,
    None (refusal) everywhere else, FixtureClock exhausts rather than wraps and is Sync so it
    can be shared across the harness's own real threads"
  - "pin_current_thread/request_fifo (sched.rs): safe core_affinity/thread-priority wrappers,
    both Result-returning, no unsafe, no unwrap/expect on their own return values"
  - "read_overhead_ns/cross_core_offset_ns/offset_estimate_ns/current_clocksource
    (characterise.rs): D-35's own measured code path for the clock's read cost, the cross-core
    offset (a stated Cristian's-algorithm application with its symmetric-delay assumption named
    in the doc comment), and the running clocksource"
  - "run_trials/hot_side_trial/abort_side_trial/TrialConfig/TrialRow/TrialOutcome (trial.rs):
    the D-31 through D-34 stand-in trial loop, two long-lived threads synchronised per trial by
    a Barrier, preallocated buffers, every phase recorded, the now_ns-then-abort call pair
    adjacent with nothing between them"
  - "nr-stop-harness CLI: characterise (wired end to end) and abort-latency (--period-ns with no
    default, --trials/--hot-cpu/--abort-cpu/--priority/--seed, TSV to stdout, always refuses on
    a scheduling failure since a real run hardcodes require_realtime_scheduling: true)"
affects: ["02-07", "02-08", "02-09"]

# Tech tracking
tech-stack:
  added:
    - "nix 0.31.3 (MIT, target.cfg(linux)-only dependency): clock_gettime(CLOCK_MONOTONIC_RAW)"
    - "thread-priority 3.1.1 (MIT): set_thread_priority_and_policy, SCHED_FIFO"
    - "core_affinity 0.8.3 (MIT/Apache-2.0): get_core_ids/set_for_current"
  patterns:
    - "A trait plus a real implementation plus a Sync fixture implementation (MonotonicRawClock/
       RawClock/FixtureClock) is the same shape nr-capture already uses for SystemFacts/
       LiveFacts/FixtureFacts, applied to a second Linux-only concern"
    - "A concurrency-heavy function (run_trials) is decomposed into small, pub, directly-callable
       per-unit-of-work functions (hot_side_trial/abort_side_trial) specifically so integration
       tests can drive the same real logic against a caller-owned instance, when the top-level
       function's own return type does not expose what a test needs to inspect"
    - "A boolean config field (require_realtime_scheduling) gates a bypass that only ever exists
       for tests, is never reachable from any CLI flag, and defaults to the fatal behaviour in
       every real code path, so the escape hatch cannot be disabled by accident on the rig"

key-files:
  created:
    - crates/histogram/src/samples.rs
    - crates/histogram/tests/samples.rs
    - crates/stop-harness/Cargo.toml
    - crates/stop-harness/build.rs
    - crates/stop-harness/src/lib.rs
    - crates/stop-harness/src/main.rs
    - crates/stop-harness/src/clock.rs
    - crates/stop-harness/src/sched.rs
    - crates/stop-harness/src/characterise.rs
    - crates/stop-harness/src/trial.rs
    - crates/stop-harness/tests/clock.rs
    - crates/stop-harness/tests/trial.rs
  modified:
    - Cargo.toml
    - Cargo.lock
    - crates/histogram/src/lib.rs
    - crates/histogram/src/percentiles.rs

key-decisions:
  - "crates/stop-harness/src/lib.rs added outside every task's own declared <files> list (Rule
     3, blocking, task 2): the plan's own Cargo.toml text declares only a [[bin]] target, but
     the plan's own required tests (clock.rs's FixtureClock/read_overhead_ns/offset_estimate_ns/
     current_clocksource, trial.rs's run_trials/hot_side_trial/abort_side_trial) must be
     importable from crates/stop-harness/tests/, which is impossible without a library target.
     Cargo infers one purely from this file's presence; no Cargo.toml edit was needed."
  - "FixtureClock rebuilt from Cell<usize> (task 2) to AtomicUsize (task 3, Rule 3, blocking,
     touching a file outside task 3's own declared list): run_trials's C: MonotonicRawClock +
     Sync bound cannot be satisfied by a Cell-backed type, and task 3's own behavior list
     requires exercising run_trials 'with a fixture clock'. External behaviour (programmed
     sequence, exhausts rather than wraps) is unchanged; task 2's own six tests still pass."
  - "TrialConfig.require_fifo, as literally named in the plan's own action text, was broadened
     and renamed to require_realtime_scheduling, gating pin_current_thread failures as well as
     request_fifo failures (Rule 3, blocking). Measured directly on this dev host (Apple
     Silicon, aarch64 macOS): core_affinity::set_for_current returns false unconditionally, for
     every cpu id, single-threaded or concurrent. This contradicts the phase's own research,
     which described core_affinity's macOS path as gracefully requesting 'the highest
     performance for the thread rather than failing', a claim that research itself flagged as
     WebSearch-sourced and not independently re-verified. Real, measured behaviour on the actual
     target platform took precedence, matching this project's own established practice (e.g.
     01-04's 'followed the real rig data instead of an assumed schema'). The flag still defaults
     to fatal on every real code path (main.rs hardcodes true, with no CLI flag reaching it), so
     D-36's actual guarantee (the rig, running Linux, where both wrappers call the real syscalls)
     is unaffected; this is a macOS test-host accommodation only."
  - "offset_estimate_ns factored out of cross_core_offset_ns as its own pub function (task 2, not
     literally named in the plan's action text, which only describes the formula inline). Needed
     because cross_core_offset_ns itself requires C: Sync and two real pinned threads, which
     FixtureClock's single-threaded test cannot drive meaningfully; the formula itself has no
     such requirement, so it is directly testable once separated."
  - "hot_side_trial/abort_side_trial exposed as pub functions (task 3, not literally named in the
     plan's action text, which describes them as pseudocode sections inside run_trials). Needed
     for the same reason: TrialOutcome only ever returns rows, never the EmergencyStop instances
     a multi-trial run constructs internally, so the acknowledge-exactly-once and
     abort-record-agrees behaviors (which need to inspect the EmergencyStop directly) call these
     functions against a caller-owned EmergencyStop and Barrier instead of going through
     run_trials."
  - "Task 2's own sched.rs doc comment originally used the literal word 'unsafe' while explaining
     that no unsafe code is used, tripping this plan's own top-level verification grep (grep -rn
     'unsafe' crates/stop-harness/src/, expect no output). Reworded to 'reaches for anything
     unchecked' with identical meaning, before it was ever committed. Same class of self-collision
     already documented at 02-02 (deviation 3) and referenced by this plan's own task 1 action
     text (STATE.md 01-14)."
  - "requirements-completed left empty for STOP-07 despite appearing in this plan's own
     requirements frontmatter field, per this plan's own explicit requirements_bookkeeping
     instruction: STOP-07 needs an actual measurement on the reference rig (plan 02-08). This
     plan builds the instrument; it does not take the measurement."

requirements-completed: []

# Metrics
duration: 52min
completed: 2026-09-15
---

# Phase 2 Plan 6: The clock, scheduling setup, characterisation and trial loop for STOP-07 Summary

**`nr-stop-harness` now exists as a sixth workspace member with a real two-thread abort-trial loop driving `nr-stop`'s `EmergencyStop` through a fixture-testable `CLOCK_MONOTONIC_RAW` clock, plus its own D-35 read-overhead and cross-core-offset characterisation, and `nr-histogram` gained percentiles over a flat sample slice to feed it.**

## Performance

- **Duration:** 52 min
- **Started:** 2026-09-15T16:07:18Z (approximate; STATE.md recorded 02-05's completion at this timestamp)
- **Completed:** 2026-09-15T16:59:46Z
- **Tasks:** 3 (each executed as TDD RED then GREEN; no REFACTOR commit was needed for any task)
- **Files modified:** 16 (12 created, 4 modified)

## Accomplishments

- `crates/histogram/src/samples.rs`: `stats_from_samples(samples: &[u64], quantiles: &[f64])`
  builds an `hdrhistogram::Histogram` at 5 significant figures (not `percentiles.rs`'s 3, which
  only keeps microseconds exact; these samples are nanoseconds) and reports exact `min`/`max`
  from the slice itself, matching the exactness rule already on `CyclictestRun`'s own maximum.
  Additive: `PercentileError` gained one `EmptySamples` variant, `EmptyRun` and every existing
  function are untouched, and all 19 pre-existing histogram tests still pass.
- `crates/stop-harness/`, the sixth workspace member, binary name `nr-stop-harness` exactly (the
  name already committed in `crates/metrics/tests/series.rs:129` under D-04/D-55). Depends on
  `nr-stop`, `nr-histogram`, `nr-metrics`, `nr-capture`, `nr-manifest`; nothing in those five
  gained a dependency back.
- `clock.rs`: `MonotonicRawClock` trait; `RawClock` reads
  `nix::time::clock_gettime(ClockId::CLOCK_MONOTONIC_RAW)` on Linux with `u64::try_from` plus
  `checked_mul`/`checked_add` so no conversion can panic or wrap, and returns `None` on every
  other platform rather than substituting `CLOCK_MONOTONIC` or `Instant`; `FixtureClock` returns
  a programmed sequence, `Sync` via an `AtomicUsize` index, exhausts to `None` rather than
  wrapping. The Linux path was verified with `cargo check`/`cargo clippy --target
  x86_64-unknown-linux-gnu`, since this dev host cannot otherwise type-check it at all.
- `sched.rs`: `pin_current_thread`/`request_fifo`, both through `core_affinity`/`thread-priority`
  safe APIs, both `Result`-returning, no `unsafe`, no bare `unwrap()`/`expect(` anywhere.
- `characterise.rs`: `read_overhead_ns` (consecutive-delta read cost), `cross_core_offset_ns` (a
  real two-thread ping-pong over a shared `AtomicU64` pair, pinning fatal on failure since a
  skew figure measured on the wrong cores would look measured and would not be),
  `offset_estimate_ns` (the Cristian's-algorithm formula itself, `i64`-converted with no
  underflow, factored out so it is directly testable without a `Sync` clock), and
  `current_clocksource` (reads sysfs, parametrised on the root so macOS covers it via a
  temp-directory fixture).
- `trial.rs`: `run_trials` spawns two long-lived threads via `std::thread::scope`, synchronised
  per trial by a `Barrier` (used only between trials, never inside the measured interval, stated
  in the module doc). The hot thread polls `EmergencyStop::state` once per fixed-period
  iteration until it observes the gate closed, then acknowledges; the abort thread busy-waits a
  uniformly random phase (xorshift64star, seeded, eight lines, no `rand` dependency) and calls
  `EmergencyStop::abort` with nothing but the `now_ns` read between it and the call. Both
  `Vec<EmergencyStop>` and `Vec<TrialRow>` are preallocated before the first trial runs.
- `main.rs`: `characterise` wired up end to end printing raw samples to stdout; `abort-latency`
  takes `--period-ns` (no default: the caller always states which of the two published periods,
  33,000 ns or 1,000,000 ns, a run belongs to), `--trials` (default 200,000, help text states
  the in-repo size trade), `--hot-cpu`/`--abort-cpu` (7/8), `--priority` (80), `--seed`, prints a
  TSV header plus one row per trial, and always constructs `require_realtime_scheduling: true` so
  a real invocation refuses cleanly. Confirmed by actually running it on macOS: refuses with
  "the platform refused to pin the current thread to cpu 8", exit 1.
- Empirically stress-tested, not just written and hoped for: the full `nr-stop-harness` test
  suite was run 8 times back to back, and the full workspace suite (`cargo test --workspace
  --all-targets`, exactly `#[test]` functions running under cargo's default parallel harness,
  which is also how CI invokes it) was run 5 times back to back, all green, after tuning the
  fixture-clock budget for real cross-test CPU contention (see Deviations).

## Task Commits

1. **Task 1: Percentiles over a flat sample slice, in nr-histogram** - RED `31f9e2b` (test),
   GREEN `ad75916` (feat)
2. **Task 2: The harness crate, the clock, the scheduling setup, and the clock characterisation**
   - RED `72f98f9` (test), GREEN `9657e0b` (feat)
3. **Task 3: The stand-in hot path and the abort trial loop** - RED `f8c6038` (test), GREEN
   `0cbff86` (feat)

No REFACTOR commit was needed for any task: each GREEN implementation passed its own acceptance
criteria and the workspace-wide gates without further cleanup.

## Files Created/Modified

- `crates/histogram/src/samples.rs` - `SampleStats`, `stats_from_samples` (71 lines)
- `crates/histogram/tests/samples.rs` - the six raw-sample-statistics tests (64 lines)
- `crates/histogram/src/lib.rs` - added `pub mod samples;`, extended the crate doc's scope
- `crates/histogram/src/percentiles.rs` - added `PercentileError::EmptySamples`, nothing else
  changed (confirmed via `git diff`)
- `crates/stop-harness/Cargo.toml` - the package manifest, binary name `nr-stop-harness`
- `crates/stop-harness/build.rs` - git-sha build-time stamping, a declared copy of
  `crates/cli/build.rs` (95 lines)
- `crates/stop-harness/src/lib.rs` - `pub mod characterise; pub mod clock; pub mod sched; pub mod
  trial;` (15 lines; see Deviations for why this file exists at all)
- `crates/stop-harness/src/main.rs` - the `clap` CLI, both subcommands (167 lines)
- `crates/stop-harness/src/clock.rs` - `MonotonicRawClock`, `RawClock`, `FixtureClock` (78 lines)
- `crates/stop-harness/src/sched.rs` - `pin_current_thread`, `request_fifo`, `SchedError` (53
  lines)
- `crates/stop-harness/src/characterise.rs` - `read_overhead_ns`, `cross_core_offset_ns`,
  `offset_estimate_ns`, `current_clocksource`, `CharacteriseError` (160 lines)
- `crates/stop-harness/src/trial.rs` - `run_trials`, `hot_side_trial`, `abort_side_trial`,
  `TrialConfig`, `TrialRow`, `TrialOutcome`, `TrialError` (294 lines)
- `crates/stop-harness/tests/clock.rs` - the six clock/characterisation tests (71 lines)
- `crates/stop-harness/tests/trial.rs` - the seven trial-loop tests (163 lines)
- `Cargo.toml` - added `crates/stop-harness` to members; added `nr-stop`, `thread-priority`,
  `core_affinity` to `workspace.dependencies`
- `Cargo.lock` - the three new dependencies and their transitive closures

## Recorded per the plan's own `<output>` instructions

**Resolved versions and licences** (`cargo deny check licenses`, exit 0):

| Crate | Version | Licence |
|-------|---------|---------|
| `nix` | 0.31.3 | MIT |
| `thread-priority` | 3.1.1 | MIT |
| `core_affinity` | 0.8.3 | MIT/Apache-2.0 |

All three clear `deny.toml`'s allow list (`Apache-2.0`, `MIT`, and others) as-is.

**API signatures actually used**, verified against the real downloaded crate source at the
pinned versions (`~/.cargo/registry/src/.../{nix,thread-priority,core_affinity}-<version>/`)
rather than trusting research's own reconstructed examples:

- `nix::time::clock_gettime(clock_id: nix::time::ClockId) -> nix::Result<TimeSpec>`;
  `TimeSpec::tv_sec(&self) -> time_t` and `TimeSpec::tv_nsec(&self) -> timespec_tv_nsec_t`, both
  effectively `i64` on `x86_64-unknown-linux-gnu`. `ClockId::CLOCK_MONOTONIC_RAW` is a `pub
  const` gated under nix's own `linux_android` cfg alias (covers plain Linux).
- `thread_priority::set_thread_priority_and_policy(native: ThreadId, priority: ThreadPriority,
  policy: ThreadSchedulePolicy) -> Result<(), thread_priority::Error>`;
  `thread_priority::thread_native_id() -> ThreadId` (`= libc::pthread_t` on unix);
  `ThreadPriorityValue: TryFrom<u8, Error = String>`, range 0-99;
  `ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::Fifo)`. Matches the research's
  reconstructed shape exactly; confirmed rather than assumed.
- `core_affinity::get_core_ids() -> Option<Vec<CoreId>>`; `core_affinity::set_for_current(core_id:
  CoreId) -> bool`; `CoreId { pub id: usize }`. Matches research's reconstructed shape for the
  function signatures; the *behaviour* on macOS did not match research's description (see
  Deviations: `set_for_current` returns `false` unconditionally on this dev host, not the
  graceful highest-performance fallback research described).

**Default flag values as shipped** (`nr-stop-harness abort-latency --help`):

| Flag | Default | Note |
|------|---------|------|
| `--period-ns` | none (required) | caller always states which published period a run measures |
| `--trials` | 200,000 | |
| `--hot-cpu` | 7 | inside the rig's isolated set 6-11 |
| `--abort-cpu` | 8 | inside the rig's isolated set 6-11 |
| `--priority` | 80 | not 99; the two threads never contend on separate isolated cores |
| `--seed` | 1 | nonzero; `trial.rs` also substitutes a fallback if a caller passes 0 |

**Byte size of one rendered `TrialRow`** (TSV line, `{trial}\t{phase_ns}\t{abort_raw_ns}\t
{observed_raw_ns}\t{latency_ns}\n`): measured directly rather than recomputing the plan's own
estimate from scratch. For representative magnitudes at the default `--trials 200000` (a 6-digit
trial number, a 6-digit `phase_ns` for a 1,000,000 ns period, 15-digit `CLOCK_MONOTONIC_RAW`
readings representing roughly 1.2 days of rig uptime in nanoseconds, and a 5-digit `latency_ns`
in the low tens of microseconds), one row renders to **52 bytes**, consistent with the plan's own
"roughly 55 bytes" estimate. At `MAX_IN_REPO_FILE_BYTES` (25 MiB), that is roughly 500,000 rows
before a single capture file would need to become an external pointer; the shipped default of
200,000 stays well inside that bound with room for the manifest and report, matching the plan's
own stated trade at this measured figure rather than the plan's own approximation.

## Decisions Made

See `key-decisions` in the frontmatter for full reasoning on each. Summary:

- `crates/stop-harness/src/lib.rs` added (Rule 3, blocking): the only way to satisfy this plan's
  own mandated `cargo test -p nr-stop-harness` commands, since the package's Cargo.toml text
  declares no `[lib]` and the required tests import internal modules directly.
- `FixtureClock` rebuilt `Cell` to `AtomicUsize` (Rule 3, blocking): required for `run_trials`'s
  `Sync` bound, needed by task 3's own behavior list.
- `TrialConfig.require_fifo` renamed and broadened to `require_realtime_scheduling`, now also
  gating `pin_current_thread` (Rule 3, blocking, discovered by direct measurement on this dev
  host, not assumed from the phase's own research).
- `offset_estimate_ns` and `hot_side_trial`/`abort_side_trial` introduced as `pub`, testable
  decompositions of logic the plan's action text describes inline; needed because the plan's own
  `TrialOutcome`/return-type shapes do not expose what several of the plan's own named test
  behaviors need to inspect.
- One self-inflicted collision (not the plan's own text this time, but a comment I wrote) between
  `sched.rs`'s doc comment and this plan's own `grep -rn 'unsafe'` verification, caught and fixed
  before it was ever committed.
- `requirements-completed` left empty for STOP-07, per this plan's own explicit
  `requirements_bookkeeping` instruction.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `crates/stop-harness/src/lib.rs` did not exist in the plan's own file list, but the plan's own tests cannot compile without it**

- **Found during:** Task 2, RED phase, immediately after writing the failing test against
  `nr_stop_harness::clock`/`characterise`.
- **Issue:** The plan's own literal `crates/stop-harness/Cargo.toml` text declares only a
  `[[bin]]` target. Integration tests under `tests/` can only import a crate's *library* target;
  a binary-only package exposes nothing to them. `cargo test -p nr-stop-harness --test clock`
  failed with `E0433: cannot find module or crate nr_stop_harness`.
- **Fix:** Added `crates/stop-harness/src/lib.rs` declaring `pub mod clock; pub mod sched; pub
  mod characterise;` (task 3 added `pub mod trial;`). Cargo infers a library target from the
  file's mere presence, using the package's default library name (`nr_stop_harness`); no
  Cargo.toml edit was needed, so the plan's own literal manifest text is otherwise unchanged.
- **Files modified:** `crates/stop-harness/src/lib.rs` (new, outside every task's own declared
  `<files>` list).
- **Verification:** `cargo test -p nr-stop-harness --test clock` compiles and all 6 tests pass.
- **Committed in:** `9657e0b` (task 2 GREEN commit)

**2. [Rule 3 - Blocking] `FixtureClock` was `Cell`-backed (task 2) but `run_trials` needs `Sync` (task 3)**

- **Found during:** Task 3, while designing `run_trials`'s signature against the plan's own
  explicit instruction ("Generic over the clock... so the real clock's read inlines and the
  macOS tests still exercise the same loop through `FixtureClock`") and its own explicit
  requirement to use `std::thread::scope` with two real threads.
- **Issue:** `std::cell::Cell` is `!Sync` by design. A `C: MonotonicRawClock + Sync` bound
  (required to share one clock reference across two real `thread::scope` threads) cannot be
  satisfied by task 2's `FixtureClock`, yet task 3's own behavior list requires exercising
  `run_trials` "with a fixture clock".
  - **Fix:** Rebuilt `FixtureClock`'s internal index from `Cell<usize>` to `AtomicUsize`,
    using `fetch_add` so concurrent callers each get a distinct, monotonically increasing index
  (no two threads ever observe the same reading). The externally observable behaviour (returns
  the programmed sequence in order, exhausts to `None` rather than wrapping) is unchanged; all 6
  of task 2's own tests, already committed, still pass unmodified.
- **Files modified:** `crates/stop-harness/src/clock.rs` (outside task 3's own declared
  `<files>` list).
- **Verification:** `cargo test -p nr-stop-harness --test clock` (task 2's suite) still passes
  6/6 after the change; `cargo test -p nr-stop-harness --test trial` (task 3's suite, which
  needs the `Sync` bound) passes 7/7.
- **Committed in:** `f8c6038` (task 3 RED commit, alongside the new failing trial tests)

**3. [Rule 3 - Blocking] `core_affinity::set_for_current` fails unconditionally on this dev host, not gracefully as the phase's own research described**

- **Found during:** Task 3, first attempt at running the new trial tests, which failed with
  `Sched(PinRefused { cpu: 1 })`.
- **Issue:** `02-RESEARCH.md`'s Architecture Pattern 4 states, citing a WebSearch synthesis
  flagged in that same document as "MEDIUM confidence, not independently re-verified against the
  raw source": "on macOS aarch64 specifically... it's not possible to pin a thread to a specific
  core, but the library will still try to request the highest performance for the thread rather
  than failing." Measured directly on this exact dev host (Apple Silicon, aarch64 macOS) with a
  standalone scratch program calling `core_affinity::set_for_current` for cpu ids 0 through 3,
  single-threaded, no contention: it returns `false` every time, for every id. This is not a
  graceful fallback; it is an outright failure, contradicting the unverified claim.
- **Fix:** `TrialConfig.require_fifo`, the field name and scope the plan's own action text
  specified ("The tests must therefore exercise `run_trials` through a path that does not
  require it. Add a `TrialConfig` field... rather than swallowing the error"), was renamed to
  `require_realtime_scheduling` and its scope broadened to gate `pin_current_thread` failures as
  well as `request_fifo` failures, both through one `setup_realtime_scheduling` helper. Fatal
  when `true` (every real invocation; `main.rs` hardcodes it, reachable from no CLI flag),
  tolerated when `false` (tests only). The reference rig runs Linux, where `core_affinity` calls
  the real `sched_setaffinity` and this limitation does not apply; D-36's actual guarantee is
  unaffected.
- **Files modified:** `crates/stop-harness/src/trial.rs`, `crates/stop-harness/tests/trial.rs`.
- **Verification:** `cargo test -p nr-stop-harness --test trial` passes 7/7; `cargo run -p
  nr-stop-harness -- abort-latency --period-ns 1000 --trials 5` (a real invocation, hardcoded
  `require_realtime_scheduling: true`) correctly refuses with "the platform refused to pin the
  current thread to cpu 8", exit 1, confirming the bypass cannot reach a real run.
- **Committed in:** `f8c6038` (RED, field introduced) and `0cbff86` (GREEN, main.rs wiring
  confirms the real path still refuses)

**4. [Rule 1 - Bug] `sched.rs`'s own doc comment used the literal word "unsafe" while explaining that no unsafe code is used**

- **Found during:** Task 2, immediately after writing `sched.rs` and running this plan's own
  acceptance criterion `grep -rq 'unsafe' crates/stop-harness/src/` (expect no match).
- **Issue:** The module doc's own prose read "Neither wrapper needs `unsafe` in caller code, so
  none enters this crate" — a literal occurrence of the word `unsafe`, which the grep (correctly)
  flagged. This is text I wrote, not text quoted from the plan.
- **Fix:** Reworded to "Both expose fully checked APIs to the caller, so none of this crate's own
  code reaches for anything unchecked," preserving the identical technical claim without the
  literal token. Same class of self-collision this project has already documented and resolved
  the same way (02-02's deviation 3, STATE.md 01-14).
- **Files modified:** `crates/stop-harness/src/sched.rs`.
- **Verification:** `grep -rq 'unsafe' crates/stop-harness/src/` now returns no match.
- **Committed in:** `9657e0b` (task 2 GREEN commit; caught before this file was ever committed
  in its uncorrected form)

**5. [Rule 1 - Bug] `clippy::type_complexity` on `run_trials`'s `thread::scope` closure return type**

- **Found during:** Task 3, first `cargo clippy --workspace --all-targets -- -D warnings` after
  writing `trial.rs` (both the native macOS target and the `x86_64-unknown-linux-gnu`
  cross-check target flagged it identically).
- **Issue:** `std::thread::scope(|scope| -> Result<(Vec<(u64, u64)>, Vec<u64>), TrialError> {
  ... })` trips `clippy::type_complexity`, denied via this workspace's `clippy::all = deny`.
- **Fix:** Factored `Vec<(u64, u64)>` into a named type alias, `type AbortSideReadings =
  Vec<(u64, u64)>;`, with a doc comment. No behaviour change.
- **Files modified:** `crates/stop-harness/src/trial.rs`.
- **Verification:** `cargo clippy --workspace --all-targets -- -D warnings` and `cargo clippy
  --target x86_64-unknown-linux-gnu -p nr-stop-harness --all-targets -- -D warnings` both exit 0.
- **Committed in:** `0cbff86` (task 3 GREEN commit)

**Total deviations:** 5 auto-fixed (3 Rule 3 blocking, 2 Rule 1 bugs). **Impact on plan:** none
change this plan's public API shapes, correctness properties, or any of the D-31 through D-37
decisions; two (deviations 1 and 4) are pure build-config/wording fixes; two (2 and 3) were
required for the plan's own mandated tests to exist and pass at all, and deviation 3 in
particular is a genuine, dev-host-specific platform fact discovered by direct measurement rather
than assumed from unverified research, documented exactly as such rather than silently patched
over.

## Issues Encountered

- The plan's own top-level acceptance criterion "confirm the only hit is
  `crates/stop-harness/Cargo.toml`" for `grep -l 'nr-stop' crates/*/Cargo.toml` is, read
  literally, unsatisfiable: `crates/stop/Cargo.toml` (created in plan 02-01, untouched by this
  plan) declares `name = "nr-stop"` in its own `[package]` section, which the same grep always
  matches on its own name alone, independent of any dependency edge. This is not a defect this
  plan introduced. The substantive rule this criterion protects (from D-55: none of the five
  measurement crates gain a dependency on `nr-stop`) was checked directly and holds: `grep -n
  'nr-stop' crates/{manifest,histogram,capture,metrics,cli}/Cargo.toml` returns no match in any
  of the five files. Recorded so a future reader does not mistake this self-match for a real
  dependency-direction violation; matches this project's own established precedent for the same
  class of grep self-collision (STATE.md 01-14, 02-02 deviation 3).
- The full test suite was measurably flaky during development (not in what was committed): a
  first attempt at the trial-loop tests used a fixture-clock budget sized for one trial running
  in isolation, which was sufficient every time each test ran alone (`--exact`) but failed three
  of seven tests once the whole suite ran under `cargo test`'s own default parallel harness
  (multiple tests' `thread::scope` calls contending for real CPU at once). Diagnosed directly
  (the failures were all `ClockUnavailable`, the fixture running dry) and fixed by budgeting for
  contention rather than for an isolated best case; verified by 8 repeated full-suite runs plus 5
  repeated `cargo test --workspace --all-targets` runs, all green. Recorded so a future reader
  tuning trial counts or CI parallelism understands why the fixture budgets in `tests/trial.rs`
  look larger than a single trial's average consumption would suggest.

## User Setup Required

None. Everything in this plan ran on the macOS dev host with the workspace's already-installed
stable toolchain, plus a one-time `cargo check --target x86_64-unknown-linux-gnu` cross-check
(the target was already installed per `rust-toolchain.toml`). No rig access, no Kani install, no
coverage nightly, and no new host-level tool installation were needed.

## Next Phase Readiness

- `nr-stop-harness` exists with the exact binary name the schema already expected
  (`crates/metrics/tests/series.rs:129`), and `nr-histogram`, `nr-metrics`, `nr-capture`,
  `nr-manifest` are all already declared dependencies (per D-55), ready for plan 02-07 to add the
  capture pipeline (environment snapshot, preconditions, run directory, manifest, `StageMetrics`
  entry) behind both subcommands without any new dependency-direction work.
  `docs/rig/recon-2026-08-31/FINDINGS.md`'s `tsc=reliable skew_tick=1` cmdline finding is exactly
  what `cross_core_offset_ns`'s doc comment names as the reason the offset is worth measuring
  rather than assumed to be zero; plan 02-08 can cite it directly.
- `run_trials`'s two-thread design, its `require_realtime_scheduling` flag, and its
  `hot_side_trial`/`abort_side_trial` decomposition are all proof-ready for plan 02-08's actual
  rig invocation: on Linux, `pin_current_thread`/`request_fifo` call the real syscalls, so a real
  run (which always sets `require_realtime_scheduling: true`, unreachable from any CLI flag)
  will refuse exactly as D-36 requires if either fails, rather than silently publishing a number
  it cannot stand behind.
- The measured 52-byte `TrialRow` TSV size and the 200,000-trial default are ready for plan
  02-08 to use directly when sizing an actual rig capture; the arithmetic is in this file's
  Output section and in `--trials`'s own help text.
- `docs/proofs/` and Phase 2's Kani/coverage gates (plans 02-03/02-04) are unaffected; this plan
  touched nothing under `crates/stop/` itself, only the harness that will eventually measure it.
  `measurements/` and `.planning/phases/01-trustworthy-measurement/` were not touched, per this
  plan's own explicit scope.
- STOP-07 remains `Pending` in `REQUIREMENTS.md`; plan 02-08 (the actual rig measurement) is what
  closes it, per this plan's own `requirements_bookkeeping` instruction. STOP-01 through STOP-06
  are already closed from earlier plans in this phase.
- No blockers carried forward from this plan. Phase 1 remains open on its own track (01-15 task
  3, unrelated) and this plan touched nothing under `measurements/`.

*Phase: 02-proven-emergency-stop*
*Completed: 2026-09-15*

## Self-Check: PASSED

- FOUND: crates/histogram/src/samples.rs
- FOUND: crates/histogram/tests/samples.rs
- FOUND: crates/stop-harness/Cargo.toml
- FOUND: crates/stop-harness/build.rs
- FOUND: crates/stop-harness/src/lib.rs
- FOUND: crates/stop-harness/src/main.rs
- FOUND: crates/stop-harness/src/clock.rs
- FOUND: crates/stop-harness/src/sched.rs
- FOUND: crates/stop-harness/src/characterise.rs
- FOUND: crates/stop-harness/src/trial.rs
- FOUND: crates/stop-harness/tests/clock.rs
- FOUND: crates/stop-harness/tests/trial.rs
- FOUND commit: 31f9e2b (test(02-06): add failing test for raw-sample percentiles)
- FOUND commit: ad75916 (feat(02-06): percentiles over a flat sample slice in nr-histogram)
- FOUND commit: 72f98f9 (test(02-06): add failing test for the harness clock and characterisation)
- FOUND commit: 9657e0b (feat(02-06): the harness clock, scheduling setup, and clock characterisation)
- FOUND commit: f8c6038 (test(02-06): add failing test for the abort trial loop)
- FOUND commit: 0cbff86 (feat(02-06): the stand-in hot path and the abort trial loop)
