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
