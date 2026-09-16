# Deferred items

Out-of-scope discoveries logged during plan execution. Not fixed as part of the
originating plan; noted here for a future plan or maintenance pass to pick up.

## From 02-08 (rig capture, task 1 checkpoint prep)

- **02-08-PLAN.md's own inline `nr-stop-harness` commands use flags that do not exist
  on the real CLI.** Task 1's deliberate refusal-check command, and the shared
  `<interfaces>` block's "Shape" example, both include `--class`, `--instrument` and
  `--note` on `nr-run-measurement stop-harness characterise`/`abort-latency`. Confirmed
  by reading `crates/stop-harness/src/main.rs` directly: `Characterise` accepts only
  `--iterations`, `--cpu-a`, `--cpu-b`, `--rounds`, `--sys-root`, `--rig-slug` (required,
  no default) and `--measurements-root` (pinned by the wrapper script); `AbortLatency`
  accepts only `--period-ns` (required), `--trials`, `--hot-cpu`, `--abort-cpu`,
  `--priority`, `--seed`, `--rig-slug` (required), `--measurements-root` (pinned) and
  `--characterisation-tsv`. Neither subcommand has a `--class`, `--instrument` or
  `--note` flag at all: `run_class`/`instrument_class` are hardcoded internally per
  subcommand (`Characterise` always builds `RunClass::Recon` /
  `InstrumentClass::HeadlineSeries`; `AbortLatency` always builds `RunClass::Headline` /
  the same instrument class), and there is no operator-note field anywhere in the
  harness. Passing the plan's literal stale flags would make `clap` reject the
  invocation before `main()` ever reaches `admit()`, i.e. before any run directory is
  created or any precondition is evaluated, which would produce a confusing
  argument-parse failure in place of the intended `NoActiveSshSessions` refusal (task 1)
  or the intended capture (tasks 2 and 3). The plan's own `<interfaces>` comment already
  says "Plan 02-07's SUMMARY records the exact final flag set; that SUMMARY wins over
  this block if they differ" (and 02-07-SUMMARY.md's "Plan-mandated output" section and
  `deploy/systemd/README.md`'s "stop-harness subcommand" section both agree with
  `main.rs` and each other, omitting `--class`/`--instrument`/`--note` throughout); this
  entry extends that same resolution to task 1/2/3's own inline action-text commands,
  not just the shared block. Not fixed here (02-08's task 1 file scope is "none in the
  repository; this task changes the rig only", and rewriting a plan's own prose is a
  bigger edit than a checkpoint-prep pass should make unilaterally); whoever executes
  tasks 2 and 3 should build every `nr-stop-harness`/`nr-run-measurement stop-harness`
  invocation from `main.rs`'s real clap surface (or the two confirmed-real sources
  above), never from this plan's own inline text.

## From 02-08 task 1 (found during the first rig checkpoint, 2026-09-15)

- **Cargo is installed on the rig but unreachable from any shell.** `~/.cargo/bin/cargo` is a
  working rustup shim (cargo 1.98.0, rustc 1.98.0, toolchain
  `stable-x86_64-unknown-linux-gnu`), and `~/.cargo/env` exists, but no rc file sources it:
  `grep -n cargo ~/.bashrc ~/.profile ~/.bash_profile` returns nothing, and the login `PATH` is
  the stock one with no `~/.cargo/bin`. So a plain `cargo build` on the rig fails with
  "Command 'cargo' not found" and suggests installing it, which would be the wrong fix.
  The workaround is `. "$HOME/.cargo/env"` first, or an absolute `~/.cargo/bin/cargo`.

  Whoever built `target/release/nrmeasure` there on 2026-09-10 must have done one of those and
  it was never recorded, so the next person hit it cold. This is a reproducibility gap of the
  same kind `docs/measurement-protocol.md` exists to close: a third party following the written
  procedure cannot build the harness. Whichever plan next owns rig documentation should either
  add the source line to the runbook, or wire `~/.cargo/env` into the rig's shell rc so the
  documented command works as written. Not fixed here: changing the rig's shell environment is
  outside a capture plan's scope, and the workaround is one line.

## From 02-08 task 2 (found while driving the D-35 capture, 2026-09-16)

- **`--collect` never reaps `nr-measurement.service` on this rig, so every capture wedges the unit
  name for the next one.** Any run through `nr-run-measurement` leaves
  `/run/systemd/transient/nr-measurement.service` on disk and the unit at `LoadState=loaded`. The
  next launch then dies before reaching a single precondition:
  `Failed to start transient service unit: Unit nr-measurement.service was already loaded or has
  a fragment file.`

  This was first read as something a *refused* run does. That was too narrow, and the narrower
  version is wrong. It was isolated by launching a deliberately trivial unit under the same name:

      sudo systemd-run --unit=nr-measurement --collect --property=Type=oneshot /bin/true

  That succeeded ("Running as unit: nr-measurement.service"), `/bin/true` exited 0, and 25 seconds
  later the fragment file was still present with `LoadState=loaded`, `ActiveState=inactive`,
  `Result=success`, `CollectMode=inactive-or-failed`. Nothing had failed and nothing had refused,
  and the unit still was not collected. So the trigger is running at all, not failing.

  What does and does not clear it, all confirmed on the machine:

  - `systemctl reset-failed` does NOT remove the fragment file. On a unit that is merely loaded
    and dead (`Result=success`) it is a no-op, which is why it appears to do nothing; on a genuinely
    failed unit it clears the failed state and still leaves the file, so the unit reloads from it.
  - `systemctl stop` and `systemctl daemon-reload` alone change nothing.
  - Removing the fragment file and then reloading does clear it, to `LoadState=not-found`, after
    which `systemd-run` reuses the name normally. This is the only working recovery found.
  - Removing the file while the unit sits at `LoadState=error` leaves systemd in a state where the
    next `systemd-run` fails with `Device or resource busy` instead. Recover by removing the file
    and reloading again from a clean (non-error) state. No lingering cgroup was involved:
    `/sys/fs/cgroup/system.slice/nr-measurement.service` was absent throughout.

  The underlying reason systemd declines to garbage-collect a unit whose `CollectMode` permits it
  is NOT established here and is not claimed. What is established is the behaviour and the
  recovery, both reproduced several times in one sitting.

  Why it matters beyond one run. `scripts/nr-run-measurement` pins a single unit name on purpose,
  so two captures cannot contend for the isolated cores invisibly, and that is right. The cost is
  that the name wedges after every use and the operator account cannot unwedge it: `systemctl` is
  not one of the four fixed paths in `deploy/sudoers/nr-measurement`, by the same deliberate
  asymmetry that made the 2026-09-04 runaway stoppable only by the operator. A rig in this state
  looks exactly like a rig refusing a run, which is the one distinction this project most needs to
  keep sharp. Every capture in this plan needed a root cleanup between attempts. The weekly timer
  is NOT affected: it drives `neurorust-measure.service`, a different unit.

  Possible fix, for whichever plan next owns the wrapper, not applied here: `nr-run-measurement`
  already runs as root, so immediately after its existing `systemctl is-active` check proves
  nothing is actually running, it can clear a stale same-named unit itself. That closes the wedge
  without widening the sudoers grant by one entry and without giving up the fixed name. Not done
  in 02-08, whose file scope is the rig and `measurements/` only, and whose task 2 is a capture
  rather than a change to the root entry point.

- **Plan 02-08 task 2's acceptance criteria require `metrics-entry.json` in the characterise run
  directory, which no characterise run has ever written.** `metrics-entry.json` is written inside
  `run_abort_latency` (crates/stop-harness/src/main.rs:591-608); `run_characterise`
  (main.rs:256-455) does not write one, and 02-07-SUMMARY.md already said so in as many words:
  "characterise and abort-latency each leave a validating run directory (raw capture,
  manifest.json, REPORT.md; abort-latency also metrics-entry.json)". The 2026-09-16 run is
  therefore correct with three files and not missing a fourth.

  Same class as the stale CLI flags logged above, and the same resolution: where 02-08's own text
  and `main.rs` disagree, `main.rs` and 02-07-SUMMARY.md win. Recorded rather than silently
  treated as met, because "the artifact list was wrong" and "the artifact is missing" are the two
  readings a later reader has to choose between, and only one of them is true here.

- **The rig now depends on a launcher script that is not in this repository.**
  `/home/d0nmega/nr-capture-launch.sh`, written during 02-08 task 2. It exists because
  `NoActiveSshSessions` expects zero established connections to port 22, while
  `nr-run-measurement` checks preconditions within milliseconds of being launched, which is
  necessarily from a session that is still established. The gap between "launched" and "safe to
  check" is the whole problem, and Phase 1 closed it with a delayed `systemd-run` trigger that
  needs root beyond the four fixed paths in the NOPASSWD grant. This closes it from inside the
  grant: it polls the same `ss -Htn state established '( sport = :22 )'` query
  `LiveFacts::active_ssh_sessions` itself runs, waits for zero, settles 45 seconds, re-checks, and
  only then launches. Waiting on the condition rather than sleeping a guessed duration is the
  point: a fixed delay is a bet about how long an operator takes to close a terminal.

  It worked, and every capture in 02-08 went through it. The problem is that it is rig-only, so
  the documented procedure in `docs/measurement-protocol.md` still cannot be followed by a third
  party, and a rig rebuild loses it. Exactly the reproducibility gap the `cargo`-not-on-PATH entry
  above describes, created rather than merely found.

  Whichever plan next owns rig deployment should take it into `deploy/` alongside
  `neurorust-weekly-run.sh`, with the settle window and the 30 minute deadline as named constants
  and the `ss` query cross-referenced to `sources.rs` so the two cannot drift. Not done in 02-08,
  whose file scope is `measurements/`, `INDEX.md` and `deploy/systemd/README.md`, and adding a new
  deployable script is a larger change than a capture plan should make unilaterally.

  One sharp edge to carry with it: there is a roughly one second window between the launcher's
  final zero-session check and the harness gathering its own facts. A session opened inside that
  window refuses the run. That happened once during 02-08, from reconnecting to collect a result
  at the moment the previous capture fired.
