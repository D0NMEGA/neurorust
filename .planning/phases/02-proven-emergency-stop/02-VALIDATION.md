---
phase: 2
slug: proven-emergency-stop
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-09-14
---

# Phase 2 - Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from `02-RESEARCH.md` "Validation Architecture", with STOP-06 resolved by D-57.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | built-in `cargo test`, plus `cargo kani` as a separate verification surface and `cargo llvm-cov` as a coverage surface |
| **Config file** | none dedicated; workspace `Cargo.toml` lints apply. A `[package.metadata.kani]` table in `crates/stop/Cargo.toml` only if Wave 0 finds one is needed |
| **Quick run command** | `cargo test -p nr-stop && cargo kani -p nr-stop` |
| **Full suite command** | `cargo test --workspace --all-targets && cargo kani -p nr-stop && cargo +nightly-<pinned> llvm-cov -p nr-stop --branch --fail-under-branches 100` |
| **Estimated runtime** | ~60 seconds (three-state, four-event machine; Kani solve expected in seconds) |

Kani proofs are gated behind `#[cfg(kani)]` so they never run under `cargo test` and never
affect the existing fmt, clippy, test or deny jobs.

---

## Sampling Rate

- **After every task commit:** `cargo test -p nr-stop && cargo kani -p nr-stop`
- **After every plan wave:** full suite command above, plus a coverage report read by hand
- **Before `/donny-verify-work`:** full suite green, `docs/proofs/` report committed and
  reviewed (D-54), one headline-class STOP-07 rig capture published with its raw capture
- **Max feedback latency:** 60 seconds

STOP-06 closes on the FSM before STOP-07's rig work begins. STOP-07 depends on nothing in
STOP-06, but running them the other way round lets coverage gaps hide behind rig work.

---

## Per-Task Verification Map

| Req ID | Behavior | Test Type | Automated Command | File Exists | Status |
|--------|----------|-----------|-------------------|-------------|--------|
| STOP-01 | `step()` is total over the enumerated state space; no reachable state is unreachable by the model | Kani proof | `cargo kani -p nr-stop --harness step_is_total` | ❌ W0 | ⬜ pending |
| STOP-02 | From any reachable state an `Abort` reaches `Stopping` or `Stopped`, and no path returns to `Running` | Kani proof | `cargo kani -p nr-stop --harness abort_never_returns_to_running` | ❌ W0 | ⬜ pending |
| STOP-03 | No permit is issued from `Stopping` or `Stopped`; the modelled consumer (D-45) emits nothing after a stop | Kani proof + unit test | `cargo kani -p nr-stop --harness no_permit_when_not_running`; `cargo test -p nr-stop modelled_consumer` | ❌ W0 | ⬜ pending |
| STOP-04 | Totality, latch and gate invariants, panic and overflow freedom, plus the published scoping note stating what the proof does and does not establish | Kani proof (4 families) + committed doc | `cargo kani -p nr-stop`; `test -f docs/proofs/emergency-stop-proof-scope.md` | ❌ W0 | ⬜ pending |
| STOP-05 | `cargo kani` blocks CI on every push and pull request (D-52) | CI job | the `kani` job in `.github/workflows/ci.yml`, non-conditional | ❌ W0 | ⬜ pending |
| STOP-06 | 100 percent branch coverage of `crates/stop` on a pinned dated nightly (D-57) | Coverage run | `cargo +nightly-<pinned> llvm-cov -p nr-stop --branch --fail-under-branches 100` | ❌ W0 | ⬜ pending |
| STOP-07 | Abort latency measured on the reference rig, published as total plus decomposition, never subtracted (D-34) | Rig capture, manual-only | `nr-stop-harness` on the Precision 3591 under the Phase 1 protocol | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/stop/` package scaffold: `[lints] workspace = true`, `#![forbid(unsafe_code)]`
      restated at the crate root (D-49), added to the workspace members list
- [ ] A trivial one-file `#[kani::proof]` spike, run once, confirming Kani 0.67.0 drives this
      workspace's `edition = "2024"` and `resolver = "3"`. Confirmatory rather than exploratory:
      Kani 0.67.0 bundles `nightly-2025-11-21`, which is well past edition 2024 (Rust 1.85) and
      resolver 3 (Rust 1.84), both verified on the main thread. The spike also gives an early
      signal on the CI job shape before it is load-bearing.
- [ ] A trivial `cargo llvm-cov --branch` run on the pinned nightly, confirming the flag is
      accepted and that `--fail-under-branches` gates as expected, before the threshold becomes
      a blocking check
- [ ] `crates/stop/` proof module gated behind `#[cfg(kani)]`, covering STOP-01 through STOP-04
- [ ] `crates/stop-harness/` package scaffold, covering STOP-07
- [ ] `kani` job in `.github/workflows/ci.yml`, covering STOP-05
- [ ] Coverage job or step on the pinned nightly, covering STOP-06
- [ ] `docs/proofs/` directory and the scoping note, covering STOP-04's published-claim half

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Abort latency on the reference rig | STOP-07 | Needs the physical Precision 3591, isolated cores 6-11, `SCHED_FIFO`, and the Phase 1 clean-run protocol with no login session. Not reproducible in CI by construction. | Run `nr-stop-harness` on the rig under the D-05 systemd path, with all fifteen D-06 preconditions passing. Publish the total and the decomposition side by side per D-34, plus the cross-core skew and clock-read-overhead capture per D-35. |
| The published proof-scope claim reads honestly | STOP-04 | A claim about what a proof does and does not establish cannot be checked by grep. | Human review of `docs/proofs/` before the commit that publishes it, per D-07's reviewed-commit rule. |
| The Phase 6 handoff contract is complete | STOP-03, D-45 | The contract constrains a future phase; correctness is a judgment about whether the rules are sufficient. | Human review that the contract names permit caching and emit-without-permit as prohibited. |

---

## Validation Sign-Off

- [ ] All tasks have an automated verify or a Wave 0 dependency
- [ ] Sampling continuity: no 3 consecutive tasks without an automated verify
- [ ] Wave 0 covers every MISSING reference above
- [ ] No watch-mode flags
- [ ] Feedback latency < 60s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
