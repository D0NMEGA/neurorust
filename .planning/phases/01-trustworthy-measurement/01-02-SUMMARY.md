---
status: PASS
agent: donny-executor
phase: 01-trustworthy-measurement
plan: 02
subsystem: measurement-recon
tags: [cyclictest, rtla, ftrace, timerlat, osnoise, systemd, power-profiles-daemon, ssh, fixtures]

# Dependency graph
requires:
  - phase: 01-trustworthy-measurement (plan 01)
    provides: cargo workspace, crates/histogram/tests/fixtures/ and its README convention
  - phase: 01-trustworthy-measurement (plan 03)
    provides: KernelInfo::redact_cmdline transform and convention, reused by hand here
provides:
  - Six verbatim rig-recon text files (tracers, tracer config, package versions, kernel cmdline,
    cpu isolation, rt-tuning.service state) captured live from the installed system over SSH
  - rtla-build-log.txt settling the PLAT-01 attribution instrument as rtla timerlat (already
    installed via linux-tools-common, no source build needed, correcting RESEARCH.md)
  - Five real rig-produced fixtures for the histogram and capture crate parsers, replacing
    guesswork with genuine cyclictest --json/-h/-H output, /proc/interrupts, and a sysfs tuning
    snapshot
  - FINDINGS.md answering RESEARCH.md open questions 2-4, resolving pitfall 4, and documenting
    the full root cause of the rt-tuning.service/power-profiles-daemon governor race
  - Downstream implication for plan 01-05's deep-cstates-disabled precondition (state-absent
    must count as a pass, not a failure)
affects: [01-04, 01-05, 01-06, 01-12]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Visible redaction extended beyond kernel.cmdline: the same [redacted] token convention
      from KernelInfo::redact_cmdline is applied by hand to any raw-text capture (uname -a,
      journalctl, cyclictest --json sysinfo.nodename) that turns out to carry the machine
      hostname, per the T-1-06 threat mitigation's general instruction rather than its narrower
      file list"
    - "Probe fixtures document their own non-measurement status in-file: both fixtures READMEs
      (crates/histogram and the new crates/capture one) state plainly that no number in a
      probe-prefixed fixture may be quoted or published"

key-files:
  created:
    - docs/rig/recon-2026-08-31/available-tracers.txt
    - docs/rig/recon-2026-08-31/tracer-config.txt
    - docs/rig/recon-2026-08-31/package-versions.txt
    - docs/rig/recon-2026-08-31/kernel-cmdline.txt
    - docs/rig/recon-2026-08-31/cpu-isolation.txt
    - docs/rig/recon-2026-08-31/rt-tuning-state.txt
    - docs/rig/recon-2026-08-31/rtla-build-log.txt
    - docs/rig/recon-2026-08-31/FINDINGS.md
    - crates/histogram/tests/fixtures/probe-cyclictest-h-60s.hist
    - crates/histogram/tests/fixtures/probe-cyclictest-histofall-60s.hist
    - crates/histogram/tests/fixtures/probe-cyclictest-60s.json
    - crates/capture/tests/fixtures/probe-proc-interrupts.txt
    - crates/capture/tests/fixtures/probe-sysfs-tuning.txt
    - crates/capture/tests/fixtures/README.md
  modified:
    - crates/histogram/tests/fixtures/README.md
    - .planning/phases/01-trustworthy-measurement/deferred-items.md

key-decisions:
  - "All three tasks executed directly over SSH rather than stopped at checkpoint:human-action as PLAN.md's literal task type says, because passwordless SSH plus two fixed, argument-free sudo scripts (nr-recon, nr-probe) were set up after the plan was written, making the executor capable of doing what the plan originally required a human at the console for"
  - "Root cause of the rt-tuning.service/powersave contradiction found and recorded rather than fixed: GDM's own greeter session D-Bus-activates power-profiles-daemon within the same boot second as rt-tuning.service, and ppd's performance profile never sets scaling_governor=performance on this Meteor Lake/HWP backend, so the governor always lands on powersave regardless of rt-tuning.service's own successful, error-free write"
  - "T-1-06 redaction applied beyond the threat model's named file list: cyclictest --json's sysinfo.nodename, and uname -a/journalctl lines in two recon .txt files, all carried the real hostname and were not anticipated by the table's three-file scope; redacted per the mitigation's own general instruction and the human's public-repo privacy directive"
  - "README.md updated in both fixture directories (crates/histogram existing file, crates/capture new file) to state explicitly that probe-prefixed fixtures are format probes and no number in them may ever be quoted, per this execution's explicit success criteria"
  - "rtla requires no source build on this rig: it ships inside the already-installed linux-tools-common package (version 7.0.12), correcting RESEARCH.md's assumption that a kernel-tree build would be needed"

requirements-completed: []

# Metrics
duration: ~25min
completed: 2026-08-31
---

# Phase 1 Plan 2: Rig recon (tracers, tooling, isolation, tuning state) Summary

**Live SSH recon of the Dell Precision 3591 replaces every RESEARCH.md guess with a verified fact: all three ftrace tracers are present, rtla needs no build, cores 6-11 are genuinely nohz_full, and the rig silently boots untuned because power-profiles-daemon wins a boot-time race against rt-tuning.service.**

## Performance

- **Duration:** ~25 min
- **Started:** approximately 2026-08-31T05:20:00Z
- **Completed:** 2026-08-31T05:44:03Z
- **Tasks:** 3
- **Files changed:** 15 (14 created, 1 modified)

## Accomplishments

- Confirmed `timerlat`, `osnoise` and `hwlat` are all available (`CONFIG_TIMERLAT_TRACER=y`,
  `CONFIG_OSNOISE_TRACER=y`, `CONFIG_HWLAT_TRACER=y`), resolving RESEARCH.md pitfall 4 in the good
  direction: no cyclictest-only fallback needed for PLAT-01
- Settled the PLAT-01 method: `rtla` 7.0.12 is already installed (ships inside
  `linux-tools-common`, no source build required), correcting RESEARCH.md's assumption that a
  kernel-tree build would be needed; `rtla timerlat top --cpus 6-11 --auto <us>` is the
  attribution instrument, alongside the literal `cyclictest --breaktrace --tracemark` capture the
  roadmap names
- Captured the real `cyclictest --json` schema (per-thread `min`/`avg`/`max`/`cycles` present, no
  overflow count anywhere in JSON) and the real `-h`/`-H` `.hist` format, including a footer
  column-count refinement the existing fixture README did not cover: `--histofall` adds its
  summary column to the histogram body, `# Max Latencies:` and `# Histogram Overflows:`, but not
  to `# Min Latencies:` or `# Avg Latencies:`
- Confirmed cores 6-11 are genuinely `nohz_full`, not only `isolcpus`, narrowing RESEARCH.md open
  question 4 (a stopped tick is not the missing piece for the ~100 Hz burst investigation, though
  it remains a hypothesis for plan 01-12 to test, not a finding)
- Found and fully root-caused the plan's headline target: `rt-tuning.service` reports
  `active`/`enabled` while every CPU governor reads `powersave`. Root cause: GDM's greeter session
  D-Bus-activates `power-profiles-daemon` within the same boot second as `rt-tuning.service`, and
  `power-profiles-daemon`'s "performance" profile is expressed on this HWP backend as
  `scaling_governor=powersave` plus `energy_performance_preference=performance`, never as the
  legacy governor override `rt-tuning.sh` writes; whichever runs last wins, and it never favors
  `rt-tuning.sh`. The rig has been silently booting untuned. Recorded, not fixed, and logged to
  `deferred-items.md` for whichever plan owns the harness's tuning story.
- Delivered five real rig-produced fixtures (three in `crates/histogram`, two in
  `crates/capture`) so plans 01-04/01-05/01-06 design their parsers against genuine tool output

## Task Commits

Each task was committed atomically:

1. **Task 1: Inventory tracers, tooling, and isolation state** - `5b58817` (feat)
2. **Task 2: Resolve rtla availability** - `f81a864` (docs)
3. **Task 3: Capture schema probes, precondition fixtures, and write FINDINGS.md** - `33abd86` (test)

**Plan metadata:** committed separately after this SUMMARY (see final commit).

## Files Created/Modified

- `docs/rig/recon-2026-08-31/available-tracers.txt` - verbatim `available_tracers` plus the
  `/sys/kernel/tracing/` directory listing
- `docs/rig/recon-2026-08-31/tracer-config.txt` - the six `CONFIG_*` symbols the plan asked for
- `docs/rig/recon-2026-08-31/package-versions.txt` - `dpkg -l`, cyclictest/hwlatdetect/rtla
  presence, cyclictest flag support
- `docs/rig/recon-2026-08-31/kernel-cmdline.txt` - `/proc/cmdline` with `root=` redacted (no
  `resume=` present on this cmdline)
- `docs/rig/recon-2026-08-31/cpu-isolation.txt` - isolated/nohz_full/realtime/uname, hostname
  redacted
- `docs/rig/recon-2026-08-31/rt-tuning-state.txt` - the plan's base capture plus a full
  read-only forensic appendix (unit file, ExecStart script, journalctl, power-profiles-daemon
  timing, cpufreq/EPP/ACPI state), hostname redacted throughout
- `docs/rig/recon-2026-08-31/rtla-build-log.txt` - `OUTCOME: rtla-available` plus the version
  and `timerlat top --help` evidence
- `docs/rig/recon-2026-08-31/FINDINGS.md` - the six required question headings plus the
  redaction note, the rt-tuning.service root cause, and the plan 01-05 downstream implication
- `crates/histogram/tests/fixtures/probe-cyclictest-h-60s.hist` - real `-h 400` capture
- `crates/histogram/tests/fixtures/probe-cyclictest-histofall-60s.hist` - real `-H 400` capture
- `crates/histogram/tests/fixtures/probe-cyclictest-60s.json` - real `--json` output, hostname
  redacted
- `crates/histogram/tests/fixtures/README.md` - new section documenting the three probes and
  the footer column-count refinement
- `crates/capture/tests/fixtures/probe-proc-interrupts.txt` - real `/proc/interrupts`, all 22 CPUs
- `crates/capture/tests/fixtures/probe-sysfs-tuning.txt` - real D-06/D-14 tuning snapshot
- `crates/capture/tests/fixtures/README.md` - new file documenting both capture-crate probes
- `.planning/phases/01-trustworthy-measurement/deferred-items.md` - two new entries: the
  governor race (fix deferred to whichever plan owns tuning) and the wrong `no_turbo` path in
  `nr-recon`/`nr-probe` (fix deferred to rig-side script maintenance)

## Decisions Made

**Execute directly instead of stopping at checkpoint:human-action.** PLAN.md's three tasks are
typed `checkpoint:human-action` on the premise that only a human at the rig's console could run
them. That premise no longer holds: passwordless SSH (`d0nmega@192.168.0.100`, verified working,
no password) plus two fixed, argument-free sudo scripts (`nr-recon` read-only, `nr-probe` the two
60 second probes) were set up after the plan was written. All commands the plan's action blocks
specify were either run unprivileged directly, or covered by one of the two scripts; nothing
required root beyond what those two scripts expose. No architectural decision was involved, so
this did not need a Rule 4 stop; it is a capability change in the execution environment, not a
scope change in the work.

**T-1-06 redaction applied beyond the threat model's three-file list.** The threat register names
`probe-sysfs-tuning.txt`, `probe-proc-interrupts.txt` and `FINDINGS.md` as the files to grep for
host identifiers, but its own mitigation text says to grep "the probe files" generally. Direct
inspection found the real hostname in `cyclictest --json`'s `sysinfo.nodename` (not on the named
list, and not anticipated because nobody expected `--json` to embed `uname()` output) and in raw
`uname -a`/`journalctl` lines inside `cpu-isolation.txt` and `rt-tuning-state.txt`. All three were
redacted with the literal token `[redacted]`, matching `KernelInfo::redact_cmdline`'s convention:
visible, minimal, and documented in FINDINGS.md's "Redaction and host identifiers" section rather
than silently dropped. The plan's own verification command
(`grep -rniE "$(whoami)|$(hostname)|..."`) was re-run against every artifact after redaction and
reports clean.

**Root cause investigated and recorded, rig state left untouched.** The instruction to treat the
untuned governor as a finding, not a problem to fix, was followed literally: every command used to
diagnose the `power-profiles-daemon` race was read-only (`journalctl`, `systemctl show`,
`cat` on sysfs paths, `powerprofilesctl get`). No governor, EPP, or service state was changed.

**README updates beyond PLAN.md's file list.** PLAN.md's `files_modified` frontmatter does not
list either fixtures README, but this execution's own success criteria required "README.md
updated to say they are format probes whose numbers must never be quoted." Both were updated
(the existing `crates/histogram` one, and a new one for the just-created `crates/capture` fixtures
directory) to satisfy that explicitly, scoped narrowly to the probe-documentation purpose.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical] Hostname redaction extended to cyclictest --json and two other recon files**
- **Found during:** Task 3, assembling the probe fixtures and running the plan's own
  host-identifier verification command against them
- **Issue:** `cyclictest --json`'s `sysinfo.nodename` field, and raw `uname -a`/`journalctl`
  output captured into `cpu-isolation.txt` and `rt-tuning-state.txt`, all carried the rig's real
  hostname. None of these three files appear in the threat model's T-1-06 component list, which
  only names `probe-sysfs-tuning.txt`, `probe-proc-interrupts.txt` and `FINDINGS.md`.
- **Fix:** Replaced the hostname with the literal token `[redacted]` in all three locations,
  touching nothing else in any of them. Documented in FINDINGS.md, "Redaction and host
  identifiers", including why the T-1-06 table's file list undersold its own mitigation's scope.
- **Files modified:** `crates/histogram/tests/fixtures/probe-cyclictest-60s.json`,
  `docs/rig/recon-2026-08-31/cpu-isolation.txt`, `docs/rig/recon-2026-08-31/rt-tuning-state.txt`
- **Verification:** `grep -rniE "$(whoami)|$(hostname)|([0-9a-f]{2}:){5}[0-9a-f]{2}"
  docs/rig/recon-2026-08-31/ crates/*/tests/fixtures/probe-*` reports "no host identifiers found"
- **Committed in:** `33abd86` (json) and `5b58817` (the two recon txt files)

**2. [Rule 2 - Missing critical] FINDINGS.md's own redaction-note section initially re-leaked the search pattern it was describing**
- **Found during:** re-running the plan's verification command after writing FINDINGS.md
- **Issue:** The "Redaction and host identifiers" section quoted the literal grep pattern used
  for the sweep (`grep -rniE "d0nmega|Precision-3591|..."`), which meant the pattern text itself,
  not real leaked data, tripped the same host-identifier check the section was documenting.
- **Fix:** Rewrote the section to describe the check using `$(whoami)`/`$(hostname)` symbolically
  instead of spelling out the literal matched strings.
- **Files modified:** `docs/rig/recon-2026-08-31/FINDINGS.md`
- **Verification:** Re-ran the exact verification command; reports "no host identifiers found"
- **Committed in:** `33abd86`

**3. [Rule 1 - Bug] `roadmap update-plan-progress` silently no-ops on zero-padded phase numbers**
- **Found during:** state/roadmap updates after the task commits, verifying `ROADMAP.md`
  actually changed
- **Issue:** `node donny-tools.cjs roadmap update-plan-progress 01` reported `{"updated": true,
  "summary_count": 3, ...}` correctly, but `ROADMAP.md`'s progress table row stayed at `2/15`
  (the count from before this plan's SUMMARY existed) even after a second run. Root cause,
  confirmed by reading `~/.claude/donny/bin/lib/roadmap.cjs`: the table-row regex is anchored on
  `phaseEscaped` from the caller's zero-padded argument (`"01"`), requiring the row to start with
  `| 01.` or `| 01 `, but `ROADMAP.md`'s own table and phase-detail heading both use the unpadded
  form (`| 1. Trustworthy measurement |`, `### Phase 1: Trustworthy measurement`). `String.replace`
  finds no match and returns the content unchanged, but the function still unconditionally reports
  `updated: true`, masking the no-op. The same bug would silently prevent the (currently absent)
  `**Plans:**` phase-detail line from ever being written for this phase. This is a bug in the
  shared, cross-project `~/.claude/donny/bin/lib/roadmap.cjs` CLI, the same class of format
  mismatch plan 01-03 documented in `state.cjs`'s progress-bar regex, not something specific to
  this plan's content.
- **Fix:** Out of scope to patch the shared CLI from this plan (same boundary 01-03 drew for its
  own tooling find). Directly corrected the one cell this plan is responsible for keeping
  accurate: `| 1. Trustworthy measurement | 2/15 | In Progress | - |` to `| ... | 3/15 | ... |`.
  Also caught and fixed the same class of stale value in `STATE.md`'s body `Progress: [bar] X%`
  line (frontmatter was correctly `20%`; the body stayed at the prior plan's `13%`), which is
  exactly the `state.cjs` bug 01-03 already found and fixed once, recurring because the shared
  script itself was not patched.
- **Files modified:** `.planning/ROADMAP.md`, `.planning/STATE.md`
- **Verification:** `grep -n "Trustworthy measurement" .planning/ROADMAP.md` shows `3/15`;
  `grep -n "^Progress:" .planning/STATE.md` shows `20%`, matching the frontmatter's `percent: 20`.
- **Committed in:** final metadata commit (after this SUMMARY)

---

**Total deviations:** 3 auto-fixed (2x Rule 2 information disclosure, 1x Rule 1 bug in shared
tooling). All three are corrections to artifacts this plan is responsible for keeping accurate;
none change the plan's scope or add new functionality.

## Issues Encountered

- The plan's own `<verification>` block's histofall column-count one-liner
  (`awk 'NR==2{print FILENAME": "NF" fields"}' file1 file2`) has an `NR`-vs-`FNR` bug: `awk`'s
  `NR` counts lines cumulatively across all input files, so `NR==2` only ever matches inside the
  first file. Re-ran with `FNR==2` to get a real per-file answer (7 fields for `-h`, 8 for `-H`,
  confirming the intended rule). Not fixed in PLAN.md itself since editing a plan's own
  verification prose is outside an executor's scope; noted here for transparency. Does not affect
  any committed artifact, only the sanity-check command text.
- The pre-established finding shared with this execution's instructions ("`/sys/devices/system/
  intel_pstate/` is EMPTY and `no_turbo` does not exist even to root... both are absent here
  anyway") turned out to be a false negative from `nr-recon`/`nr-probe` querying the wrong sysfs
  path. Direct, unprivileged reads confirmed the standard path
  (`/sys/devices/system/cpu/intel_pstate/`) is present, populated, and `no_turbo` reads `1`.
  Corrected in FINDINGS.md rather than silently accepted; logged to `deferred-items.md` since
  fixing the two scripts' source requires touching files outside this repository.

## User Setup Required

None - no external service configuration required. (SSH access and the two sudoers scripts were
already in place before this plan started, per this execution's explicit instructions.)

## Next Phase Readiness

- Plan 01-04 (`nr-histogram` parser) has real `cyclictest --json`, `-h`, and `-H` output to design
  against, including the footer column-count nuance FINDINGS.md documents.
- Plan 01-05 (`nr-capture` preconditions) has a real `/proc/interrupts` fixture, a real sysfs
  tuning snapshot, the correct `intel_pstate/no_turbo` path, and an explicit instruction to treat
  a missing deep-C-state directory as a pass rather than a failure.
- Plan 01-12 (PLAT-01 investigation) has its method decided (`rtla timerlat`, already installed,
  no build step needed) and a resolved isolation precondition (`nohz_full` is engaged on 6-11).
- The rig itself is NOT currently in a measurement-ready state: every CPU governor reads
  `powersave`, not `performance`, despite `rt-tuning.service` reporting healthy. Any future plan
  that runs the actual measurement protocol (01-07 onward) must either fix the boot-time race
  documented here or rely on `nr-capture`'s own precondition check to refuse the run; it must not
  trust `rt-tuning.service`'s `ActiveState` as a proxy for "the rig is tuned."
- `PLAT-01` and `PLAT-03` remain open at the requirements level: this plan enables both (tracer
  availability, attribution instrument, isolation state) but completes neither on its own. Per
  explicit instruction, `requirements mark-complete` was not run for this plan.

---
*Phase: 01-trustworthy-measurement*
*Completed: 2026-08-31*

## Self-Check: PASSED

- All 14 created files and both modified files confirmed present on disk with `[ -s ]`
  (non-empty).
- All 3 task commits (`5b58817`, `f81a864`, `33abd86`) confirmed present in `git log --oneline
  --all`.
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and
  `cargo test --workspace` all exit 0 with these fixtures in the tree.
- The plan's own verification commands (histofall column diff, JSON parse, `FINDINGS.md` heading
  grep, em-dash count, host-identifier sweep) were re-run after every edit and pass.
