---
status: PASS
agent: donny-executor
phase: 01-trustworthy-measurement
plan: 27

subsystem: provenance
tags: [rust, build-script, rsync, rig, git-sha, pushed-stamp, series-admission, D-29]

# Dependency graph
requires:
  - phase: 01-trustworthy-measurement
    provides: "GitShaSource and the build-time revision embed (plan 01-16), the
      SeriesAdmission record and determine_admission (plan 01-24), the widened
      TracersQuiescent signals (plan 01-25), check_firmware_figures and the strict
      firmware re-derivation (plan 01-26), the sudoers grant and the root scripts
      (plan 01-22)"
provides:
  - "GitShaSource::PushedStamp: a revision asserted by the push, recorded as its own
    source and never presented as a build-time observation, closing C2"
  - "GitShaSource::from_build_env, one mapping of the embedded literal shared by
    cmd::run and cmd::reconstruct, falling through to Unavailable on anything
    unrecognised"
  - "crates/cli/build.rs reads ../../.git-sha when git cannot answer, validating the
    sha as 40 hex characters and dirty as an explicit true or false"
  - "scripts/nr-push-to-rig.sh: the one supported way source reaches the rig. Explicit
    include list, no removal flag, refuses a dirty tree without --allow-dirty, sends
    .git-sha last in its own transfer, verifies it by sha256 before reporting success"
  - "measurements/2026-09-07-precision3591-recon: the first capture admitted to the
    series, and the first that names the revision it was built from"
affects: [01-12, 01-13, 01-14, 01-15]

# Tech tracking
tech-stack:
  added: []
  changed: []
---

# Phase 1 Plan 27: Source identity on the rig, proven by one short capture

## What this closes

Finding C2 of `01-REVIEW-2026-09-06.md`. Every D-18 manifest recorded
`git_sha: unavailable-at-build-time` because the rig has no `git` and its `~/neurorust`
is an rsync mirror with no `.git`. The captures identified the executable by blake3
without establishing which committed revision produced it, which was most of what plan
01-16 built the field for.

## The result that matters

`measurements/2026-09-07-precision3591-recon` records `excluded_from_series: false`,
with `series_admission.admitted: true` and no exclusions, while
`config/contamination-thresholds.json` is still `provisional` and the shape verdict is
still recorded beside it.

Every previous run in the rig's journal reads `contamination verdict: Clean,
excluded_from_series: true`. That is finding A1 in the field: a clean measurement thrown
out of the series that exists to protect it. Plans 01-13 and 01-14 were blocked on this
and are not any more, established on the machine rather than in a test.

Observed values from the committed manifest:

| Check | Observed |
|---|---|
| `series_admission.admitted` | `true`, `exclusions: []` |
| `excluded_from_series` | `false`, no reason |
| `harness.git_sha_source` | `pushed-stamp` |
| `harness.git_sha` | `09075d63ff9e26d7a8cf4da7e340c13c3284cddf`, equal to the dev host's `.git-sha` |
| `harness.git_dirty` | `false`, matching the stamp |
| Preconditions | 15, `tracers-quiescent: pass` |
| Its `observed` string | `current_tracer=nop events/enable=0 set_event=(empty) tracing_on=0 instances=none samplers=none` |
| `firmware_screens[0]` | `rtla-hwnoise`, `observed_cpus [6,7,8,9,10,11]`, none missing |
| Shape verdict | `clean`, `thresholds_provisional: true` |
| Gate | 14 run directories, 12 re-derived, 5 firmware re-derived, 0 problems (strict) |

Figures: p50 2 us, p95 4 us, p99 8 us, max 42 us over 3,599,990 samples, no overflow,
SMI delta zero on every isolated CPU. Recorded as `recon` class with a note saying it is
an instrument check, because it is not a published latency figure.

One limit worth stating. This run had a clean shape, so it does not demonstrate on real
hardware that a bad-looking shape is still admitted. That case is pinned by
`a_bad_looking_shape_alone_does_not_exclude`, which runs the real committed 2026-08-28
capture against the real shipped thresholds. What this run establishes is that the
provisional-thresholds blocker is gone.

## Deviations

The executor was killed by a session limit part-way through task 1's verification. Task
1 was finished and committed from the main thread after re-running the full gate set
from scratch: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D
warnings`, `cargo test --workspace`, both schema-currency tests, and
`nrmeasure verify --strict --check-index`. Nothing was left half-applied.

`schemas/attempt.schema.json` changed although the plan's `files_modified` names only
`schemas/manifest.schema.json`. `GitShaSource` is shared between both generated schemas,
so adding a variant moves both. Expected, not a scope change.

## Findings recorded rather than fixed

The rig session turned up four defects before it produced a number. All are in
`deferred-items.md` with an owner; none is fixed here, because each belongs to
`docs/measurement-protocol.md` and plan 01-15, which owns the rig install runbook.

1. **Every rebuild on the rig trips the exec-mode guard.** `cargo` inherits the rig's
   002 umask, so the binary lands at 775 and `nr-run-measurement` refuses to exec it.
   The guard is right and stays; it runs as root under NOPASSWD. What is missing is that
   no document warns of it, so a third party following the published protocol meets a
   hard refusal on their first capture. Commit `1cdf6a2`.
2. **A dropped SSH link blocks the rig for about two hours.** A connection died from the
   dev host with a broken pipe; the rig never saw the peer leave, so the socket stayed
   ESTABLISHED and `ss` kept counting two sessions. `NoActiveSshSessions` counts exactly
   those sockets, so measurement is blocked until keepalive reaps the corpse at 7200
   seconds, with no diagnostic saying so. Cleared with `ss -K`. Commit `b5322ff`.
3. **The documented rig pull overwrites committed evidence.** This task's own step 5
   specifies an unfiltered `rsync` of the whole tree, which replaced eleven published
   `REPORT.md` files with the rig's older copies, wrote `/home/d0nmega` back into
   manifests published with `[redacted]`, and reverted the osnoise-starved capture's
   `ATTEMPT.json` from `failed` to `in-progress`, dropping its `preserved` block. The
   strict gate caught the attempt record; it could not catch the others. Recovered with
   `git checkout -- measurements/`. Commit `ed1c1db`.
4. **The published `manifest blake3:` line is not the digest of `manifest.json`.** Found
   earlier the same day while re-checking plan 01-26's rewrite; recorded then, listed
   here because it is still open. Commit `b06194c`.

Findings 1 through 3 are all defects against success criterion 2, which is that a third
party can follow the documented protocol and reproduce a run rather than guessing at it.
Each of them cost a round trip on a two-minute capture. Any of them would have cost a
four-hour slot inside plan 01-12 or 01-13, which is what this task existed to prevent.

## Commits

- `d4d8883` feat(01-27): record the revision a rig build was pushed from
- `d8ec9bf` feat(01-27): commit the harness check that proves admission on the rig
- `1cdf6a2`, `b5322ff`, `ed1c1db` docs: the three findings above

## Self-Check: PASSED

- Exactly one new run directory, `measurements/2026-09-07-precision3591-recon`, with a
  manifest, `REPORT.md`, `cyclictest.hist` and `rtla-hwnoise.txt`
- `harness.git_sha_source` is `pushed-stamp`; `harness.git_sha` is 40 lowercase hex
  characters equal to the dev host's `.git-sha`; `harness.git_dirty` matches
- `series_admission` present, `admitted == !excluded_from_series`, both favourable
- 15 precondition results; `tracers-quiescent` passes carrying `instances=` and
  `samplers=`
- `firmware_screens[0].instrument` is `rtla-hwnoise` with no CPU in 6 to 11 unobserved
- `verify --strict --check-index` exits 0 with 5 firmware figures re-derived
- `measurements/INDEX.md` lists the new run
- No file under any pre-existing `measurements/*` directory differs from its committed
  state
