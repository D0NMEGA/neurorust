---
status: PASS
agent: donny-executor
phase: 01-trustworthy-measurement
plan: 22
subsystem: rig-provisioning
tags: [sudoers, systemd, rtla, msr-tools, hwnoise, rdmsr, provenance, shell]

# Dependency graph
requires:
  - phase: 01-trustworthy-measurement
    provides: "01-11's TimeoutStartSec fix and the D-27 instrument decision
      (rtla hwnoise / rdmsr 0x34 over a new hwlat-tracer driver), 01-18's
      widened TracersQuiescent (four controls, not one), 01-02's rig recon
      and its no_turbo path defect, and the stray-capture probe exemption
      crates/cli/src/cmd/verify.rs already implements"
provides:
  - "One version-controlled sudoers file (deploy/sudoers/nr-measurement)
    describing all four passwordless root grants on the rig, installed only
    after visudo -c validates it, byte-identical to the four scripts under
    scripts/"
  - "scripts/nr-recon and scripts/nr-probe brought into version control for
    the first time; scripts/nr-run-measurement hardened with a fixed PATH,
    absolute-path root calls, a group/world-writable-binary refusal, and a
    sha256+mode banner before every exec"
  - "The runaway guard (TimeoutStartSec, not RuntimeMaxSec) confirmed live on
    the rig: zero RuntimeMaxSec warnings in the journal for the launch window"
  - "msr-tools installed and the msr module loaded; a real rtla hwnoise and a
    real rdmsr 0x34 capture committed as docs/rig/recon-2026-09-05/probe-*.txt,
    described in FINDINGS.md for plan 01-20's parser"
affects: [01-20, 01-21, 01-23]

# Tech tracking
tech-stack:
  added: [msr-tools]
  patterns:
    - "Validate-then-activate for a sudoers file: visudo -c -f on a temporary
      copy before install -m 0440 to /etc/sudoers.d/, so a malformed rule can
      never lock the machine out of sudo"
    - "Exec pinning for a NOPASSWD-executable binary: refuse a group- or
      world-writable $BIN outright, and print its sha256 and mode before
      every exec, so a substitution is either refused or visible in the
      journal after the fact"
    - "Probe, not measurement: a raw tool capture committed outside
      measurements/ under docs/rig/recon-<date>/probe-<name>.txt, exempted
      from the stray-capture scan by filename rather than by a new allowlist"

key-files:
  created:
    - deploy/sudoers/nr-measurement
    - deploy/sudoers/install.sh
    - scripts/nr-recon
    - scripts/nr-probe
    - docs/rig/recon-2026-09-05/probe-rtla-hwnoise.txt
    - docs/rig/recon-2026-09-05/probe-rdmsr-smi-count.txt
    - docs/rig/recon-2026-09-05/FINDINGS.md
  modified:
    - scripts/nr-run-measurement
    - scripts/nr-measure-mode
    - .planning/phases/01-trustworthy-measurement/deferred-items.md
    - .planning/STATE.md

key-decisions:
  - "install.sh's sudoers-cleanup loop was generalized from the one file the
    plan anticipated (nr-run-measurement) to two (nr-recon, nr-run-measurement),
    because the rig's real grants were hand-installed across both; done only
    after the replacement file is validated and installed, so there is no
    window with no grant at all"
  - "nr-recon/nr-probe's no_turbo path was already corrected on the rig, out
    of band, sometime after the 01-02 recon filed the defect; both scripts
    were vendored as-is rather than re-editing a path that was already right,
    confirmed by grep before commit"
  - "requirements-completed left empty for PLAT-02, following 01-09/01-18
    precedent: PLAT-02 means a human followed the clean protocol on the rig
    end to end, which this plan's sudoers/probe infrastructure work enables
    but does not itself constitute"
  - "rtla hwnoise's per-CPU Runtime column, not the wall-clock duration
    header, is the real per-CPU exposure figure (confirmed empirically: 59
    periods x 750000 us = 44.25s of Runtime inside a 60s wall-clock probe);
    plan 01-20 must parse Runtime directly rather than derive exposure from
    the requested -d"

requirements-completed: []

# Metrics
duration: 46min (spans a human-action checkpoint; see Performance)
completed: 2026-09-05
---

# Phase 1 Plan 22: Version-controlled rig root access and the D-27 instrument probes Summary

**One sudoers file now describes all four passwordless root grants on the rig, byte-identical
to their version-controlled scripts and validated before every install; `rtla hwnoise` and
`rdmsr 0x34` each have a real, committed capture and a findings document describing exactly
what plan 01-20's parser will read, replacing three failed `hwlatdetect` arms' worth of
assumption.**

## Performance

- **Duration:** ~46 min total wall-clock span, across a `checkpoint:human-action` boundary.
  Started 2026-09-05T22:15:49Z (first task-1 commit, dev host), reached the checkpoint at
  2026-09-05T22:21:54Z (~6 min), then paused for the operator's own interactive sudo session
  on the physical rig. That session's install and both probes were committed at
  2026-09-05T22:51:57Z (~27 min of real rig time, not agent execution time). This continuation
  session read the resulting state, wrote and committed task 3, and finished at
  2026-09-05T23:01:47Z (~10 min).
- **Tasks:** 3 of 3 (tasks 1 and 2 are `checkpoint:human-action`; task 3 is `auto`)
- **Files modified:** 11 across the whole plan (10 named in the plan's own frontmatter, plus
  `.planning/STATE.md` checkpoint bookkeeping)

## Accomplishments

- `scripts/nr-recon` and `scripts/nr-probe`, previously rig-local only, are version controlled
  for the first time.
- `scripts/nr-run-measurement` sets an explicit `PATH`, calls `systemd-run`/`systemctl` by
  absolute path, refuses to exec a group- or world-writable binary, and prints the binary's
  sha256 and mode before every exec. The refusal fired for real on the rig: the first launch
  attempt was refused because the release binary was mode 775, exactly the guard working as
  designed; after `chmod go-w` the relaunch succeeded and printed the exec banner.
- `scripts/nr-measure-mode` now writes and reads all four tracing controls
  (`current_tracer`, `events/enable`, `set_event`, `tracing_on`) the widened `TracersQuiescent`
  (plan 01-18) requires, closing the deferred-items entry that plan filed.
- `deploy/sudoers/nr-measurement` and `deploy/sudoers/install.sh` put every passwordless root
  grant on the rig under version control for the first time: one rule file naming four fixed
  absolute paths, installed only after `visudo -c -f` validates a temporary copy, with every
  script installed `-o root -g root -m 0755` first.
- Confirmed live on the rig: `sudo -l` lists exactly the four NOPASSWD entries; every installed
  script diffs clean against `scripts/`; `journalctl -u nr-measurement` carries zero
  `RuntimeMaxSec` warnings for the launch window, closing the defect plan 01-11 found and could
  not fix in place.
- `msr-tools` installed and the `msr` module loaded; `rdmsr -p <cpu> 0x34` read `0xfa6` (4006
  decimal) uniformly on CPUs 0, 5, and every isolated core 6-11, establishing directly that
  SMIs reach the isolated cores, which three `hwlatdetect` arms could not establish.
- A real 60 second `rtla hwnoise -c 6-11 -H 0-5 -P f:99` capture is committed and described in
  `docs/rig/recon-2026-09-05/FINDINGS.md`: every CPU in `-c 6-11` produces its own row (the
  property that actually replaces `hwlatdetect`), and the per-CPU `Runtime` column, not the
  wall-clock duration, is the real exposure figure a parser must read.

## Task Commits

Task 1 and 2 are `checkpoint:human-action`; their work spans a checkpoint the orchestrator
handled directly (dev-host portions) and the operator handled interactively (rig portions).
Task 3 is this continuation session's own work.

1. **Task 1, steps 1-4 (dev host: vendor nr-recon/nr-probe, harden
   nr-run-measurement/nr-measure-mode, write the sudoers rule and install.sh)**
   - `07c93ee` feat(01-22): version-control the rig's root scripts and their sudoers grant
   - `fae23a7` fix(01-22): sync the rig via rsync from the dev host, not git pull
     (Rule 3: `git pull` as the plan's own step 5 specifies does not work; the rig has no git
     and `~/neurorust` there is an `rsync`-only mirror)
   - `a9c3790` docs(01-22): checkpoint bookkeeping (STATE.md)
2. **Task 1 step 5 + Task 2 (rig: interactive sudo install, verification, and both D-27
   instrument probes)**
   - `1c5051d` feat(01-22): install the sudoers grant on the rig and probe both D-27
     instruments (includes the Rule 1 install.sh fix below)
3. **Task 3: Commit the probes and record what they settled**
   - `39fb6d5` docs(01-22): record the rtla hwnoise and MSR_SMI_COUNT probe formats

**Plan metadata:** (this commit, made immediately after this SUMMARY) docs(01-22): complete plan

## Files Created/Modified

- `deploy/sudoers/nr-measurement` - the four-line NOPASSWD rule, one fixed absolute path per
  line, with a comment block naming what each grant authorizes
- `deploy/sudoers/install.sh` - idempotent installer: installs all four scripts
  `-o root -g root -m 0755`, validates the sudoers rule with `visudo -c -f` on a temporary copy
  before activating it `-m 0440`, then removes both superseded hand-installed grant files
- `scripts/nr-recon` - vendored from the rig, already carrying the correct `no_turbo` path
- `scripts/nr-probe` - vendored from the rig, already carrying the correct `no_turbo` path
- `scripts/nr-run-measurement` - fixed `PATH`, absolute-path root calls, group/world-writable
  refusal, sha256+mode exec banner
- `scripts/nr-measure-mode` - three added tracing writes/reads (`events/enable`, `set_event`,
  `tracing_on`) alongside the existing `current_tracer` write
- `docs/rig/recon-2026-09-05/probe-rtla-hwnoise.txt` - 60s `rtla hwnoise -c 6-11 -H 0-5 -P f:99`
  capture plus `rtla hwnoise --help`
- `docs/rig/recon-2026-09-05/probe-rdmsr-smi-count.txt` - `rdmsr -p <cpu> 0x34` (hex and `-d`
  decimal) on CPUs 0, 5, 6, 7, 8, 9, 10, 11
- `docs/rig/recon-2026-09-05/FINDINGS.md` - what both probes show and mean for plans 01-20,
  01-21, and 01-23
- `.planning/phases/01-trustworthy-measurement/deferred-items.md` - closed the `RuntimeMaxSec`,
  `no_turbo` path, and `msr-tools` entries; left the `rtla hwnoise` entry open, naming 01-20
- `.planning/STATE.md` - checkpoint and completion bookkeeping

## Decisions Made

- Generalized `install.sh`'s sudoers-file removal from the one file the plan's own text
  anticipated (`nr-run-measurement`) to two (`nr-recon`, `nr-run-measurement`), matching the
  rig's actual split-across-two-files state. See Deviations, item 1.
- Left `nr-recon`/`nr-probe`'s already-correct `no_turbo` path untouched rather than re-editing
  it, since the fix already existed on the rig out of band. Confirmed by `grep` before commit;
  no code change was needed for what the plan's task 1 step 1 asked for.
- `requirements-completed` lists nothing, not `PLAT-02` from the plan's own frontmatter,
  following the precedent 01-09 and 01-18 established: `PLAT-02` ("a clean measurement protocol
  is defined and followed") is not complete until a human runs the full protocol on the rig,
  which this plan's sudoers and probe infrastructure work enables but does not itself perform.
  `PLAT-02` stays `Pending` in `REQUIREMENTS.md`.
- Documented `rtla hwnoise`'s `Runtime`-vs-wall-clock-`duration` distinction and the
  terminal-reset artifact in the default redraw directly in `FINDINGS.md`, and pointed plan
  01-20 at `-q/--quiet` instead of parsing the repeating redraw, rather than leaving either for
  01-20 to rediscover.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `install.sh` removed only one of two hand-installed sudoers grant files**
- **Found during:** Task 1 step 5, on the rig, during the operator's interactive sudo session
- **Issue:** `install.sh` as first committed (`07c93ee`) removed only
  `/etc/sudoers.d/nr-run-measurement` (2026-09-04) after installing the new consolidated
  `deploy/sudoers/nr-measurement`. The rig's grants were actually split across two
  hand-installed files: `/etc/sudoers.d/nr-recon` (2026-09-02, 113 bytes) additionally granted
  `nr-recon`, `nr-probe`, and `nr-measure-mode`. The plan's own task 1 step 4 text anticipated
  only the second file. Leaving `nr-recon` in place would have produced two sudoers files with
  overlapping grants, failing the plan's own "exactly one sudoers file describes the grant"
  acceptance criterion.
- **Fix:** Generalized the removal loop in `deploy/sudoers/install.sh` to iterate both file
  names, removing each only if present, and only after the replacement file is validated
  (`visudo -c`) and installed, so the operator is never without a grant.
- **Files modified:** `deploy/sudoers/install.sh`
- **Verification:** On the rig, `sudo -l` lists exactly the four NOPASSWD entries;
  `/etc/sudoers.d/` afterward contains only `README` (Ubuntu default) and `nr-measurement`.
- **Committed in:** `1c5051d`

**2. [Rule 3 - Blocking] `git pull` as the plan's own step 5 specifies does not work on the rig**
- **Found during:** Task 1, before the checkpoint was returned
- **Issue:** The rig has no `git` installed (`dpkg -l git` reports `un`) and `~/neurorust`
  there is a plain `rsync`-maintained directory, not a git clone. The plan's task 1 step 5
  opens with `cd ~/neurorust && git pull`, which fails outright as written.
- **Fix:** Pushed the six new/changed files directly from the dev host with
  `rsync -av <paths> precision3591-rig:~/neurorust/<paths>`, verified byte-identical via
  `sha256sum` on both ends, so the operator's rig-side session could run
  `sudo ./deploy/sudoers/install.sh` immediately with no pull step.
- **Files modified:** none (workaround was procedural, not a code change)
- **Verification:** `sha256sum` matched on both ends before the rig session began
- **Committed in:** `fae23a7` (the deferred-items.md record of the workaround)

**Total deviations:** 2 auto-fixed (1 Rule 1 bug fix, 1 Rule 3 blocking workaround).
**Impact on plan:** both were necessary for task 1 to actually satisfy its own acceptance
criteria against the rig's real state; neither expands scope beyond task 1's own file list.

Separately, not counted as deviations because no code changed and nothing was fixed:
`nr-recon`/`nr-probe`'s `no_turbo` path was already correct when pulled back from the rig
(someone fixed it out of band after the 01-02 recon filed the defect), and `stress-ng` was
already installed on the rig from the 01-11 session-2 D-18 arms, so task 2 only needed to
install `msr-tools`. Both are corrected assumptions, recorded in Decisions Made and in
`deferred-items.md`, not fixes.

## Issues Encountered

The plan's own compound verification one-liner
(`test -s docs/rig/recon-*/FINDINGS.md && ...`) now glob-matches two directories,
`recon-2026-08-31` and `recon-2026-09-05`, since both carry a `FINDINGS.md`; run verbatim in
zsh this fails with `test: too many arguments` rather than a real verification failure. Worked
around by scoping each check to `docs/rig/recon-2026-09-05/` directly, which passes cleanly
(confirmed non-empty and ASCII-only for `FINDINGS.md`, non-empty for both probes). No code or
plan text was changed for this; it is a property of the plan's own shorthand once a second
recon directory exists, worth knowing for whoever next runs a similar one-liner verbatim after
a third.

## User Setup Required

None beyond what tasks 1 and 2 already required and the operator already performed
(the interactive sudo session on the physical rig, `precision3591-rig`).

## Next Phase Readiness

- `cargo build -p nr-cli --release` and `./target/release/nrmeasure verify --strict
  --check-index` both pass: 8 run directories, 7 re-derived, 1 not re-derivable (the
  2026-08-28 reconstructed run, a pre-existing, expected state), 0 problems in strict mode.
  `git status --short measurements/` is empty.
- Redaction re-checked independently in this session:
  `grep -rniE "$(whoami)|/home/|precision-3591|precision3591-rig"` against both new probe
  files and the new `FINDINGS.md` finds nothing; no T-1-06/T-1-75 redaction was needed.
- Plan 01-20 has a real, described `rtla hwnoise` and `rdmsr 0x34` output schema to parse
  against, including two format details (`-q` avoids the repeating redraw; `Runtime`, not
  wall-clock `duration`, is the real per-CPU exposure) that `FINDINGS.md` states explicitly so
  01-20 does not have to rediscover them.
- `deferred-items.md`'s "the instrument that was available all along" section now has its
  `msr-tools` half closed and its `rtla hwnoise` half open, naming plan 01-20 as owner and
  pointing at the new `FINDINGS.md`.
- No blockers for the next wave. STATE.md's "Plan 01-22 checkpoint" blocker is resolved; the
  rig now runs the current, version-controlled scripts and sudoers grant.

## Self-Check: PASSED

All 7 files created/modified by this plan and named above are confirmed present on disk with
the expected content (`deploy/sudoers/nr-measurement`, `deploy/sudoers/install.sh`,
`scripts/nr-recon`, `scripts/nr-probe`, `docs/rig/recon-2026-09-05/probe-rtla-hwnoise.txt`,
`docs/rig/recon-2026-09-05/probe-rdmsr-smi-count.txt`, `docs/rig/recon-2026-09-05/FINDINGS.md`,
plus modified `scripts/nr-run-measurement` and `scripts/nr-measure-mode`). All 5 commit hashes
(`07c93ee`, `a9c3790`, `fae23a7`, `1c5051d`, `39fb6d5`) confirmed present in `git log`.

*Phase: 01-trustworthy-measurement*
*Completed: 2026-09-05*
