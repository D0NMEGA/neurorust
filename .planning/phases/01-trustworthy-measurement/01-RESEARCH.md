# Phase 1: Trustworthy measurement - Research

**Researched:** 2026-08-30
**Domain:** PREEMPT_RT latency attribution, measurement protocol, benchmark provenance and regression CI
**Confidence:** HIGH on tooling and method (primary kernel docs plus a worked third-party example); MEDIUM on rig-specific availability (not verifiable from the dev host); LOW on the specific root cause of the 3.8 ms stall (hypotheses only, listed as such)
**Method:** Main-thread browser-harness pass (per the global operating instruction that browser-harness cannot run in a subagent). Sources are primary kernel documentation, distro package indexes, crates.io, and one worked vendor case study, all with URLs in `## Sources`.

## User Constraints

Copied verbatim from `01-CONTEXT.md`. The planner MUST honor these. Research below investigates
how to implement these decisions, not whether to.

### Locked decisions

- **D-01:** The capture harness is a Rust binary in a new cargo workspace, not a set of shell
  scripts. It shells out to `cyclictest` and `hwlatdetect` but owns run-manifest generation,
  histogram parsing, percentile computation and JSON emission.
- **D-02:** CI is GitHub Actions targeting `x86_64-unknown-linux-gnu` plus the macOS dev host.
  Phase 1 gates are fmt, clippy, test, the provenance check (D-13) and the regression check (D-11).
- **D-03:** Order of work: workspace and CI skeleton first, then manifest schema, histogram
  parser and metrics JSON, then the written protocol (PLAT-02), then the first clean run,
  then PLAT-01, then the PLAT-03 verdict, then the weekly job last.
- **D-04:** The manifest and metrics schema must generalise beyond cyclictest now, not later.
- **D-05:** Measurement runs execute on the rig from a systemd oneshot service plus timer,
  with no login session. GitHub Actions is reduced to validating what gets pushed.
- **D-06:** The harness asserts preconditions and refuses to run on violation. It changes
  nothing about system state.
- **D-07:** Publication path is split. The weekly p50/p95/p99 JSON auto-commits. Anything that
  becomes a published claim reaches main only through a reviewed commit.
- **D-08:** A missed week is recorded as a gap in the coverage record. No backfill.
- **D-09:** The weekly watch covers both `cyclictest` and `hwlatdetect`.
- **D-10:** Roughly 1 hour weekly run plus a periodic longer soak, tagged distinctly.
- **D-11:** The rig only measures and pushes. A GitHub Actions workflow compares against a
  committed baseline and fails when p99 or max regresses beyond a stated threshold.
- **D-12:** A sidecar `manifest.json` per run directory is the source of truth, carrying
  checksums over each raw capture. Raw captures stay byte-identical.
- **D-13:** CI enforcement is blocking. Missing manifest or mismatched checksum fails the check.
- **D-14:** The required field set is a full environment snapshot (see CONTEXT.md for the list).
- **D-15:** The harness renders an automatic contamination verdict from interference counters.
- **D-16:** Existing captures get reconstructed manifests, with a `reconstructed` vs
  `harness-generated` provenance tier. Never guess an unrecorded field.
- **D-17:** Contamination thresholds derived from a deliberate calibration pair (clean arm plus
  a deliberately contaminated arm reproducing the 2026-08-28 conditions).
- **D-18:** The `hwlatdetect` firmware baseline is re-run on the installed PREEMPT_RT system
  and the delta against the live-USB stock-kernel figures is published.
- **D-19:** "Named" means a specific kernel path identified from the ftrace and
  `cyclictest --tracemark` capture, committed next to the claim, one path per phenomenon.
- **D-20:** The investigation is budgeted at three capture-and-analyse cycles.
- **D-21:** If the stall does not reproduce on a clean run, that is the result, not a failure.
- **D-22:** PLAT-03 reports both total observed max against the 30 us gate and, separately, the
  kernel contribution above the independently measured firmware floor.
- **D-23:** `measurements/2026-08-28-precision3591/README.md` is corrected in place.

### Claude's discretion

Publication surface and run matrix: `measurements/` layout, raw captures in-repo vs pointers,
histograms committed vs generated, exact run matrix composition, how BENCH-06 is operationalised,
metrics JSON naming and layout and D-11 threshold values, exact systemd unit form.

### Deferred

None.

## Project constraints (from CLAUDE.md)

The planner must verify compliance with these. They are binding.

| Constraint | Effect on Phase 1 |
|------------|-------------------|
| Team is one person plus Claude, no parallel workstreams | Sequence plans for a single operator. Waves express dependency, not staffing. |
| Hot path: no allocation, locks, logging, blocking syscalls, unbounded queue | Not directly applicable. Phase 1 ships no runtime code. The harness is an offline tool and MAY allocate freely. Do not import hot-path rules into harness design. |
| Linux only for deployment; PREEMPT_RT required for any published RT number | Harness must build and unit-test on macOS (dev host) but only produce published figures on the rig. Split platform-dependent code behind `#[cfg(target_os = "linux")]` so the macOS CI leg stays green. |
| Rig discipline: every quoted figure names its machine | This is BENCH-04, mechanised by D-13. |
| Timestamps: PTP hardware clock confirmed on the wired `e1000e` NIC | Not exercised in Phase 1. Do not add PTP work here. |
| Budget 0 to 55 USD | All tooling recommended below is free and open source. |
| Licence Apache 2.0 and MIT dual | Set `license = "Apache-2.0 OR MIT"` in workspace `Cargo.toml` and add both LICENSE files. Verify every added dependency is compatible (all recommended crates are MIT/Apache-2.0). |
| Provenance: no public artifact may imply a spec obtained via an advisor | Keep the 1024-channel sizing target out of Phase 1 artifacts entirely. Phase 1 publishes latency figures, not channel counts. |
| Writing style: no em dashes, ASCII only, no emoji, sentence case headings | Applies to every committed markdown file, the README correction (D-23), and CLI help text. |

## Summary

The single most consequential finding is that the Linux kernel now ships a purpose-built
toolchain for exactly the PLAT-01 problem, and it is materially better than the
`cyclictest --breaktrace --tracemark` plus manual ftrace workflow that D-19 and the roadmap
success criterion describe. The `timerlat` tracer and its `rtla` front end arm a periodic timer
like cyclictest does, but on a threshold breach they stop tracing and emit a decomposed
attribution: IRQ handler delay, IRQ latency, the blocking thread with a full kernel stack trace,
IRQ interference and thread interference, each with a duration and a percentage of the total.
That decomposed stack trace is precisely the artifact D-19 defines as "named". The companion
`osnoise` tracer counts every interference source (hardware, NMI, IRQ, softirq, thread) per CPU
with per-source durations, which is a stronger and kernel-attributed version of the interference
signal D-15 proposes to derive from `/proc/interrupts` diffs.

This does not make D-19 wrong, and the planner should not quietly substitute one for the other.
The roadmap success criterion names the `cyclictest --tracemark` capture specifically, and there
is a real methodological reason to keep both: `rtla`/`timerlat` runs its own measurement thread
and perturbs the system, so it belongs in the PLAT-01 investigation runs, not in the headline
series that PLAT-03 and BENCH-08 report. The recommended shape is to use `rtla timerlat` as the
attribution instrument during the three D-20 investigation cycles, and to keep
`cyclictest --breaktrace --tracemark` as the artifact-producing capture that satisfies the
criterion literally. One methodological requirement is easy to miss and must be a planned task:
the break-trace threshold has to be calibrated against the maximum latency observed with tracing
already enabled, not against the untraced maximum, because tracing inflates the baseline.

For the harness itself the ecosystem is settled and boring, which is the right outcome. `cyclictest`
already emits machine-readable output via `--json`, so D-01's parser has less to invent than the
context assumed, though the histogram file is still the source for full distribution and
percentiles. `hdrhistogram` is the standard Rust histogram and percentile crate at 109M downloads,
`procfs` reads `/proc/interrupts` for D-14 and D-15, `schemars` generates the JSON Schema that
makes D-04's cross-phase contract enforceable, and `blake3` covers D-12's checksums. The one
environment risk worth planning around is that `rtla` is not packaged for Ubuntu at all (only
`rt-tests` is), so it has to be built from the kernel source tree, and the tracers it drives
require kernel config options that must be verified on the rig before the PLAT-01 plan can run.

**Primary recommendation:** Build the harness on `cyclictest --json` plus histogram parsing with
`hdrhistogram`, keep `/proc/interrupts` diffing as the zero-overhead always-on contamination
signal for the headline series, and add `rtla timerlat --auto` as the separate attribution
instrument for the PLAT-01 investigation runs only. Verify `CONFIG_TIMERLAT_TRACER` and build
`rtla` on the rig as an early enabling task, because PLAT-01's method depends on it.

## Standard stack

All versions verified against the crates.io API on 2026-08-30. All are MIT or Apache-2.0, which
matches the project's dual-licence constraint.

### Core

| Library | Version | Purpose | Why standard |
|---------|---------|---------|--------------|
| `hdrhistogram` | 7.6.0 | Histogram storage, percentile computation (p50/p95/p99), histogram merge | 109M downloads. The Rust port of HdrHistogram, the reference implementation for latency percentiles. Handles the recording and quantile maths that BENCH-08 needs, and its merge support is what lets per-thread cyclictest histograms be combined into an all-thread figure correctly. |
| `serde` + `serde_json` | 1.0.229 / 1.0.151 | Manifest and metrics serialisation | Universal. D-12's `manifest.json` and D-11's metrics JSON are both serde structs. |
| `schemars` | 1.2.2 | Generate JSON Schema from the manifest and metrics Rust types | 427M downloads. This is what turns D-04 ("must generalise beyond cyclictest now") into something enforceable: the schema is generated from the types, committed, and the D-13 CI gate validates every `manifest.json` against it. Phase 2 STOP-07 and Phase 3 SUBS-06 then write against a published contract rather than a convention. |
| `procfs` | 0.18.0 | Read `/proc/interrupts`, `/proc/stat`, cmdline, per-CPU data | 75M downloads. Typed access to exactly the D-14 fields and the D-15 interference counters. Linux-only, so gate behind `#[cfg(target_os = "linux")]`. |
| `blake3` | 1.8.7 | Per-file checksums in the manifest (D-12) | 172M downloads. Fast enough that checksumming multi-hundred-MB raw captures is not a bottleneck. `sha2` 0.11.0 is the alternative if a more conventional hash name matters for reviewers. |
| `clap` | 4.6.6 | Harness CLI | Universal. Derive API keeps subcommands (`run`, `verify`, `reconstruct`, `report`) declarative. |

### Supporting

| Library | Version | Purpose | When to use |
|---------|---------|---------|-------------|
| `thiserror` | 2.0.20 | Typed errors in library code | Harness core. Precondition failures (D-06) are a typed enum so each check result lands in the manifest distinctly rather than as a string. |
| `anyhow` | 1.0.104 | Error context in the binary | The CLI entry point only. |
| `time` | 0.3.55 | UTC start and end timestamps (D-14) | Prefer over `chrono` for a new project; no unmaintained transitive deps and the RFC3339 formatting is what the manifest wants. |
| `sysinfo` | 0.39.6 | Cross-platform host facts where procfs is Linux-only | Optional. Most D-14 fields come from procfs and sysfs directly; use sparingly rather than as the primary source. |
| `insta` | 1.48.0 | Snapshot tests for manifest and metrics JSON output | High value here. The schema is a cross-phase contract, so snapshot tests make an accidental breaking change visible in review as a diff. |
| `assert_cmd` + `predicates` | 2.2.2 / 3.1.4 | CLI integration tests | Tests that the harness refuses to run on a violated precondition (D-06) without needing a rig. |
| `toml` | 1.1.4 | Config parsing if the run matrix is declared in a file | Only if the run matrix becomes config-driven. Hardcode first per YAGNI. |

### Alternatives considered

| Instead of | Could use | Tradeoff |
|------------|-----------|----------|
| `hdrhistogram` | `histogram` 1.5.0 | Simpler API, 25M downloads, but no established percentile-reporting conventions and weaker merge story. `hdrhistogram` is the better default for a project whose credibility rests on published percentiles. |
| Parsing the `.hist` file | `cyclictest --json` only | The JSON gives final summary results; the `.hist` file gives the full per-bin distribution that BENCH-05 requires for histograms. Use both, and treat the `.hist` as the archival raw capture. |
| `/proc/interrupts` diffing for D-15 | `osnoise` tracer interference counters | The osnoise counters are strictly more precise and kernel-attributed, but osnoise runs its own workload thread and perturbs the measurement. Keep procfs diffing for the always-on headline series; use osnoise only in investigation runs. This is a real design split the planner must preserve. |
| `blake3` | `sha2` 0.11.0 | SHA-256 is more conventional in provenance contexts and may read as more credible to an external reviewer. Either is defensible; this is a genuine discretion call. |
| Hand-rolled percentile maths | `hdrhistogram` | Listed under "don't hand-roll" below. Percentile computation from binned data has real edge cases (bin midpoint vs upper bound, overflow handling) that the 2026-08-28 README already got wrong once. |

**Installation:**

```bash
cargo add hdrhistogram serde serde_json schemars blake3 clap thiserror anyhow time
cargo add --target 'cfg(target_os = "linux")' procfs
cargo add --dev insta assert_cmd predicates
```

## Architecture patterns

### Recommended project structure

```
Cargo.toml                     # workspace root, license = "Apache-2.0 OR MIT"
crates/
  manifest/                    # D-12/D-14 schema types, schemars derive, checksums
    src/lib.rs
    src/fields.rs              # the D-14 field set as typed structs
    src/checksum.rs
  capture/                     # environment snapshot + precondition assertions (D-06)
    src/preconditions.rs       # each check is a typed variant, result recorded not enforced
    src/environment.rs         # procfs/sysfs readers, Linux-gated
    src/interference.rs        # D-15 counter diff and contamination verdict
  histogram/                   # cyclictest .hist and --json parsing, percentiles
    src/hist.rs
    src/percentiles.rs         # thin wrapper over hdrhistogram
  metrics/                     # D-11 metrics JSON, baseline comparison
    src/series.rs
  cli/                         # the `nrmeasure` binary: run, verify, reconstruct, report
    src/main.rs
schemas/                       # generated JSON Schema, committed, CI-validated
  manifest.schema.json
  metrics.schema.json
deploy/systemd/                # D-05 unit files, version-controlled
  neurorust-measure.service
  neurorust-measure.timer
measurements/                  # existing convention kept: <ISO-date>-<rig-slug>/
docs/
  measurement-protocol.md      # PLAT-02, the third-party reproduction contract
.github/workflows/
  ci.yml                       # fmt, clippy, test (Linux + macOS)
  provenance.yml               # D-13 blocking gate
  regression.yml               # D-11, triggered on push of a new run
```

The crate split follows the project's own file-organisation rule (many small focused crates,
200-400 lines typical). It also matters for D-02: the macOS CI leg can build and test
`manifest`, `histogram` and `metrics` without any Linux-only code, so a genuine cross-platform
gate exists rather than a token one.

### Pattern 1: assert-and-record, never enforce

D-06 fixes this and the research supports it. Every precondition check returns a typed result
that is written into the manifest whether it passed or failed, and a violation aborts the run
rather than mutating system state. The payoff the context identifies is real: the assertion list
becomes the third-party reproduction checklist for PLAT-02, which an enforcing harness needing
root could not provide. Implement each check as an enum variant with a `status` and an observed
value, so the manifest records what was seen and not merely that something failed.

### Pattern 2: the two-instrument split

Headline-series runs (weekly regression, PLAT-03 verdict, published histograms) use `cyclictest`
alone plus zero-overhead `/proc/interrupts` diffing. Investigation runs (the three D-20 PLAT-01
cycles) additionally use `rtla timerlat` or raw `timerlat`/`osnoise` tracers, are tagged
distinctly in the manifest, and never feed the regression series. This mirrors D-10's rule that
run classes are tagged so they are never averaged together, and extends it to instrumentation
class. Getting this wrong contaminates the very series the phase exists to make trustworthy.

### Pattern 3: schema-generated, not schema-written

The JSON Schema files under `schemas/` are generated from the Rust types by a test or an xtask,
and CI fails if the committed schema differs from the generated one. This is the standard way to
keep D-04's cross-phase contract from drifting, and it means Phase 2 and Phase 3 authors read a
published schema rather than reverse-engineering a struct.

### Anti-patterns to avoid

- Running `osnoise` or `timerlat` concurrently with a headline cyclictest run. Both spawn their
  own per-CPU threads and inflate the numbers you are trying to publish.
- Setting a `--breaktrace` threshold from the untraced maximum. See the pitfall section; this
  produces either a trace that never fires or one that fires constantly on tracing overhead.
- Computing percentiles from the histogram while ignoring cyclictest's separate overflow count.
  This is the exact error already present in the 2026-08-28 README and corrected under D-23.
- Rewriting raw captures to embed provenance headers. D-12 already rejects this; the sidecar
  manifest exists so raw files stay byte-identical to what the tool emitted.

## Don't hand-roll

| Problem | Use instead | Why |
|---------|-------------|-----|
| Percentile computation from binned latency data | `hdrhistogram` | Bin-boundary and overflow handling has already caused one published error in this repo. |
| JSON Schema authoring | `schemars` derive | Hand-written schema drifts from the types silently. |
| `/proc/interrupts` parsing | `procfs` | The format has per-CPU columns, variable IRQ naming, and architecture-specific rows (CAL, TLB, NMI) that are tedious to parse correctly. |
| Latency attribution from raw ftrace | `rtla timerlat` | It already decomposes IRQ delay vs blocking thread vs interference and prints the stack trace. Reimplementing this analysis over raw trace text is weeks of work for a worse result. |
| Timer-latency measurement itself | `cyclictest` / `timerlat` | D-01 already fixes this: shell out, do not reimplement. |
| Content hashing | `blake3` or `sha2` | Never hand-roll. |

## Runtime state inventory

| State | Owner | Where it lives | Notes |
|-------|-------|----------------|-------|
| Run manifest | harness | `measurements/<date>-<rig>/manifest.json` | D-12 source of truth. Immutable once written. |
| Raw captures | cyclictest / hwlatdetect | same run directory | Byte-identical to tool output. Checksummed by the manifest. |
| Metrics series | harness | committed JSON, auto-committed per D-07 | Regression input for D-11. |
| Baseline | human | committed, reviewed | D-11 compares against this. Changing it is a reviewed act. |
| Coverage record | harness | committed | D-08 gap records for missed or refused weeks. |
| Tuning state | `rt-tuning.service` on the rig | queried, never set, by the harness | D-06 and D-14. The service is authoritative; the harness reads it. |

## Common pitfalls

### Pitfall 1: break-trace threshold set from the untraced maximum

Enabling ftrace inflates system latency, so a threshold derived from an untraced run will either
never fire or fire on tracing overhead rather than on the phenomenon. The Linux Foundation RT
wiki prescribes a two-step calibration: first run with tracing enabled and a deliberately huge
`--breaktrace` limit to observe the inflated maximum, then set the real limit slightly below that
inflated maximum. Their worked example goes from a 130 us untraced max to a 190 us traced max, so
the real break limit becomes 180 us, not 130. **This calibration must be an explicit planned task
inside the PLAT-01 investigation, and it consumes part of the D-20 three-cycle budget.**

### Pitfall 2: function tracing as the first instrument

The same source is explicit that jumping straight to full function tracing is a mistake: the
overhead is high enough that the target latency often stops reproducing, and the output volume
makes analysis slow. Start with the least detailed instrumentation that can distinguish the
candidates, and escalate. With `timerlat` available this is largely solved, because its default
output is already an attribution rather than a raw function stream.

### Pitfall 3: treating the two phenomena as one

The context's own structural finding stands and the research reinforces it. A sustained ~100 Hz
burst across all isolated cores and a rare isolated global spike have different candidate causes
and will produce different stack traces. D-19's "one named path per distinct phenomenon" is the
right bar, and the `--breaktrace` threshold that catches one will not catch the other: a limit
tuned to the 3.8 ms spike will never fire on the 400 us burst.

### Pitfall 4: assuming the tracers are available

`timerlat`, `osnoise` and `hwlat` require `CONFIG_TIMERLAT_TRACER`, `CONFIG_OSNOISE_TRACER` and
`CONFIG_HWLAT_TRACER` respectively, plus `CONFIG_TRACING` and `CONFIG_FTRACE`. On the rig this is
checkable with `cat /sys/kernel/tracing/available_tracers`. If the tracers are absent the PLAT-01
method changes materially and the plan needs a fallback, so this check belongs early, not inside
the investigation plan.

### Pitfall 5: the macOS CI leg silently testing nothing

D-02 puts macOS in CI. Without a deliberate crate split, every meaningful test is Linux-gated and
the macOS leg becomes decorative. Keep manifest, histogram and metrics logic platform-independent
and give them real tests, using committed fixture captures as input.

### Pitfall 6: publishing a reconstructed manifest that looks harness-generated

D-16 already anticipates this with the provenance tier. The trap in implementation is defaulting
the tier field, which makes a reconstructed manifest indistinguishable from a real one when the
field is omitted. Make the tier non-optional in the type so it cannot be forgotten.

## Code examples

### Reading the interference counters for D-15

```rust
use procfs::Current;

// Snapshot before and after the run; the delta over isolated CPUs is the D-15 signal.
let before = procfs::interrupts()?;   // maps IRQ label -> per-CPU counts
// ... run cyclictest ...
let after = procfs::interrupts()?;

// CAL (function call IPIs) and TLB (shootdowns) are the counters the 2026-08-28
// post-mortem found diagnostic. Diff them per isolated CPU, not machine-wide.
```

`procfs` also exposes `/proc/stat` for context switches and `KernelConfig` for reading the
running kernel's config, which covers several D-14 fields directly.

### Percentiles that account for the overflow count

```rust
use hdrhistogram::Histogram;

let mut h: Histogram<u64> = Histogram::new(3)?;      // 3 significant figures
for (bin_us, count) in parsed_bins {
    h.record_n(bin_us, count)?;
}
// cyclictest reports overflows (>= --histofall max) SEPARATELY from the bins.
// They must be recorded too, or every percentile and every over-gate count is wrong.
// This is exactly the error corrected under D-23.
for _ in 0..overflow_count {
    h.record(histogram_max_us)?;                     // lower bound; document the convention
}
let p99 = h.value_at_quantile(0.99);
```

The overflow handling convention (recording at the histogram maximum is a lower bound on the true
value) must be stated in the metrics schema documentation, because it makes p99 slightly
conservative and a reader deserves to know.

### The PLAT-01 attribution run

```bash
# Investigation run only. Never combined with a headline series run.
rtla timerlat top --cpus 2-7 --auto 3000        # threshold in us, below the 3.8 ms spike

# Fallback if rtla or CONFIG_TIMERLAT_TRACER is unavailable, and the capture the
# roadmap success criterion names literally:
echo 1 > /sys/kernel/tracing/events/osnoise/enable
cyclictest --mainaffinity=0,1 --affinity=2-7 --threads --mlockall --priority=99 \
           --interval=200 --histofall=400 --json=results.json --histfile=hist.txt \
           --breaktrace=<calibrated> --tracemark
```

## State of the art

The PREEMPT_RT patchset is largely merged into mainline as of Linux 6.12, so `CONFIG_PREEMPT_RT`
is selectable on most architectures without out-of-tree patches. The analysis tooling has moved
in one clear direction over the last few years: away from "run cyclictest, then hand-analyse an
ftrace dump" and toward purpose-built tracers that do the attribution in-kernel. `osnoise` and
`timerlat` (both by Daniel Bristot de Oliveira) plus the `rtla` front end are the current state of
the art, and there is a peer-reviewed treatment of timerlat in IEEE Transactions on Computers
(2025), which is unusually strong provenance for a kernel tool.

For measurement protocol, OSADL's QA farm is the reference public practice and is worth reading
closely for PLAT-02 because it is an existing, long-running, publicly documented version of what
this phase is trying to build. Their protocol runs cyclictest for 5 hours 33 minutes twice daily,
alternating a near-idle period with a defined mid-range load (about 1 Mbit/s network, 1 MB/s I/O,
1 MB/s memory allocation), publishes 1 us resolution histograms on a logarithmic y axis, and
treats worst-case latency as the headline result. Their stated rationale for keeping data since
2011 is directly relevant to BENCH-08 and D-10: worst-case latency cannot be established from a
single run, and a long series is what makes the claim credible. That is the strongest available
external argument for the weekly cadence this phase is building.

For benchmark regression in CI, the two live projects are `benchmark-action/github-action-benchmark`
(1,251 stars, active) and `bencherdev/bencher` (892 stars, active). Both are designed for
microbenchmark suites rather than for a single long RT capture pushed from an unattended rig, so
neither is a drop-in for D-11. They are worth reading for their threshold and alert conventions,
but D-11's comparison logic is simple enough (compare p99 and max against a committed baseline,
fail beyond a stated threshold) that a small purpose-built workflow step is the lower-risk choice
and keeps the judgement in reviewable CI config as D-11 requires.

## Assumptions log

| Assumption | Basis | Confidence | How to verify |
|------------|-------|-----------|---------------|
| The rig's kernel has `CONFIG_TIMERLAT_TRACER`, `CONFIG_OSNOISE_TRACER`, `CONFIG_HWLAT_TRACER` | Common in distro RT kernels but not guaranteed | LOW | `cat /sys/kernel/tracing/available_tracers` on the rig. Blocking for the PLAT-01 method. |
| `cyclictest --json` output contains enough for the metrics series | Confirmed the flag exists in the rt-tests man page; the exact schema was not inspected | MEDIUM | Run `cyclictest --json=/tmp/x.json -l 1000` on the rig and read the file before designing the parser. |
| rt-tests on the rig is 2.5-1 (noble) or similar | Ubuntu package index shows 2.2-1 jammy, 2.5-1 noble | MEDIUM | `dpkg -l rt-tests` on the rig. Older versions may lack `--json`. |
| The ~100 Hz burst and the isolated 3.8 ms spikes have different causes | Structural analysis of the existing histogram in CONTEXT.md, not yet traced | LOW | This is what PLAT-01 investigates. Stated as a hypothesis, not a finding. |
| SMI behaviour is kernel-independent, so live-USB stock-kernel hwlatdetect figures carry over | Kernel docs confirm SMIs are serviced by BIOS and the kernel is unaware of them, which supports the mechanism | MEDIUM | This is D-18. See the note below; the mechanism is kernel-independent but the measurement conditions are not. |

### Note on D-18 and the SMI carry-over assumption

The existing README's assumption is half right, and the distinction matters for how the D-18
result is written up. The kernel documentation is explicit that SMIs are set up and serviced by
BIOS code and that Linux "does not even know that they are occurring", so the *source* of that
latency genuinely is kernel-independent. What is not kernel-independent is the *measurement
condition*: C-state residency, turbo behaviour, thermal state and whether NMIs are being counted
all differ between a live-USB stock kernel and a tuned PREEMPT_RT install, and all of them change
how often and how long an SMI window is observed. So D-18's re-run is justified, but the expected
finding is a difference in observed distribution rather than a refutation of the mechanism, and
the write-up should say so rather than framing any delta as "the assumption was wrong".

## Open questions

1. **Does the roadmap criterion require a literal `cyclictest --tracemark` capture, or is an
   `rtla timerlat` trace an acceptable substitute?** The criterion names the former. The
   recommendation above is to produce both, but if the planner judges that wasteful, this is a
   user decision, not a planner decision. Flagged rather than silently resolved.
2. **Is `rtla` obtainable on the rig?** It is not in the Ubuntu package archive (searching `rtla`
   returns only reportlab false positives). It is packaged for Debian and it builds from the
   kernel source tree at `tools/tracing/rtla`. The plan needs an explicit enabling task, and if
   the build fails the fallback is driving the tracers through raw tracefs, which works but is
   more manual.
3. **What exactly is in `cyclictest --json`?** Needs one command on the rig before the parser is
   designed. Cheap to answer, and it changes how much of D-01's parser is actually needed.
4. **Are the isolated cores actually `nohz_full`, or only `isolcpus`?** `isolcpus` alone keeps the
   scheduler tick running on those cores. Given the ~10 ms burst spacing matches a 100 Hz tick,
   this is worth checking first in the PLAT-01 investigation. The kernel cmdline is already a D-14
   field so the answer will be recorded regardless.

## Environment availability

| Dependency | Required by | Available | Version | Fallback |
|------------|-------------|-----------|---------|----------|
| `rt-tests` (cyclictest, hwlatdetect) | PLAT-01, PLAT-03, BENCH-05, BENCH-08 | yes, packaged | 2.5-1 (noble), 2.2-1 (jammy) | none needed |
| `rtla` | PLAT-01 attribution | **not in Ubuntu archive** | Debian only | Build from kernel tree `tools/tracing/rtla`, or drive `timerlat`/`osnoise` via raw tracefs |
| `CONFIG_TIMERLAT_TRACER` / `OSNOISE` / `HWLAT` | PLAT-01, D-15 diagnostics | unverified on rig | - | `cyclictest --breaktrace --tracemark` plus function_graph tracing |
| `trace-cmd` | trace capture and archival | likely packaged | - | raw tracefs reads |
| Rust toolchain | everything | assumed present on dev host and rig | stable | - |
| GitHub Actions Linux + macOS runners | D-02 | yes | - | - |
| systemd | D-05 | yes on the rig | - | none acceptable; D-05 depends on it |

**Missing dependencies with no fallback:** none identified.

**Missing dependencies with fallback:** `rtla` (build from source, or raw tracefs). Tracer kernel
config (fall back to the classic breaktrace workflow, which is what D-19 already describes).

## Validation Architecture

### Test framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` plus `insta` 1.48.0 for snapshots and `assert_cmd` 2.2.2 for CLI |
| Config file | none needed; `cargo test` is the entry point. Wave 0 creates the workspace. |
| Quick run command | `cargo test --workspace` |
| Full suite command | `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` |

The important constraint on this phase's validation: most requirements are about artifacts and
provenance, not runtime behaviour, and several cannot be verified on the dev host at all because
they need the rig. The test map below separates those honestly rather than pretending a unit test
covers a rig measurement.

### Phase requirements to test map

| Req ID | Behavior | Test type | Automated command | File exists? |
|--------|----------|-----------|-------------------|--------------|
| BENCH-04 | Manifest carries every required D-14 field; a manifest missing one is rejected | unit | `cargo test -p manifest required_fields` | no, Wave 0 |
| BENCH-04 | Provenance CI gate fails on a capture with no manifest | integration | `cargo test -p cli provenance_gate_rejects_orphan_capture` | no, Wave 0 |
| BENCH-05 | Histogram parser reproduces known percentiles from a committed fixture capture | unit | `cargo test -p histogram percentiles_from_fixture` | no, Wave 0 |
| BENCH-05 | Overflow samples are included in percentile and over-gate counts | unit | `cargo test -p histogram overflow_counted` | no, Wave 0 |
| BENCH-06 | A run marked `contaminated` is retained and rendered, not dropped | unit | `cargo test -p metrics contaminated_run_retained` | no, Wave 0 |
| BENCH-08 | Metrics JSON carries p50, p95, p99 per stage and validates against the schema | unit | `cargo test -p metrics schema_roundtrip` | no, Wave 0 |
| BENCH-08 | Regression check fails when p99 exceeds the baseline threshold | integration | `cargo test -p metrics regression_gate` | no, Wave 0 |
| PLAT-02 | Precondition assertions detect a violated state and refuse the run | integration | `cargo test -p capture preconditions_refuse_on_violation` (fixture-driven) | no, Wave 0 |
| PLAT-02 | Documented protocol exists and lists every assertion the harness makes | manual | doc review against `capture::preconditions` enum | no |
| PLAT-01 | Named kernel path with committed trace | **manual, rig-only** | not automatable | n/a |
| PLAT-03 | Verdict reports total max and kernel contribution above firmware floor | unit (report rendering) + manual (the measurement) | `cargo test -p metrics plat03_report_decomposition` | no, Wave 0 |

### Sampling rate

- **Per task commit:** `cargo test --workspace`
- **Per wave merge:** `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
- **Phase gate:** full suite green, plus the provenance gate green against the real `measurements/`
  tree including the D-16 reconstructed manifests, before `/donny-verify-work`

### Wave 0 gaps

- [ ] Cargo workspace with the crate split above, `license = "Apache-2.0 OR MIT"`
- [ ] `crates/histogram/tests/fixtures/` seeded with the existing `cyclictest-rt-isolated-idle-10m.hist`
      so the parser has a real input with a known-hard case (888 overflows) from the first commit
- [ ] `.github/workflows/ci.yml` with the Linux and macOS legs
- [ ] `cargo test --workspace` green on an empty workspace before any feature work

### Manual-only verifications

| Behavior | Requirement | Why manual | Test instructions |
|----------|-------------|------------|-------------------|
| The named kernel path is correct | PLAT-01 | Requires the rig, a reproducing capture, and human trace reading | Follow the investigation runbook; commit the trace next to the claim |
| Worst-case latency under 30 us, or an attributed residual | PLAT-03 | Requires a clean rig run of the stated duration | Run the headline protocol; read the verdict from the generated report |
| Third party can reproduce from the protocol document | PLAT-02 | Requires a human following prose | Have a reader work through `docs/measurement-protocol.md` against a clean rig |
| Firmware floor delta on RT vs stock kernel | D-18 | Requires the rig and a comparison against archived live-USB figures | Run hwlatdetect under the harness; publish the delta |

## Security domain

ASVS L1. This phase ships no network service, no authentication, no user input from untrusted
sources, and no data store. The applicable surface is narrow and mostly supply-chain and CI.

### Applicable ASVS categories

| ASVS category | Applies | Standard control |
|---------------|---------|------------------|
| V2 Authentication | no | No auth surface in this phase |
| V3 Session management | no | No sessions |
| V4 Access control | partly | The rig pushes to the repo. Use a deploy key or fine-grained token scoped to a single repo with contents write only, never a personal access token with broad scope. |
| V5 Input validation | yes | The harness parses tool output and manifest JSON. Validate against the generated schema; reject rather than coerce malformed captures. |
| V6 Cryptography | yes | `blake3` or `sha2` for checksums. Never hand-roll. Note the checksums are integrity, not authenticity: they detect accidental corruption and prove a capture matches its manifest, they do not prove who produced it. State that limitation in the provenance documentation rather than overclaiming. |
| V12 Files and resources | yes | The harness writes into `measurements/`. Reject path traversal in run identifiers; do not build paths from unvalidated strings. |
| V14 Configuration | yes | CI secrets never echoed. The manifest records hostnames and hardware serial-adjacent fields, so review what D-14's full environment snapshot exposes publicly before the first push. |

### Known threat patterns

| Pattern | STRIDE | Standard mitigation |
|---------|--------|---------------------|
| Rig push token over-scoped, compromising the repo if the laptop is lost | Elevation of privilege | Fine-grained token, single repo, contents write only, expiring |
| Manifest checksums presented as proof of authenticity | Spoofing | Document explicitly that checksums prove integrity only; a signing story is out of scope for Phase 1 |
| Environment snapshot leaks host identifiers into a public repo | Information disclosure | Review the D-14 field set for anything host-identifying beyond what the rig-discipline constraint actually requires, before the first published manifest |
| Supply chain: a compromised crate in the harness dependency tree | Tampering | `cargo deny` or `cargo audit` as a CI gate; the dependency set above is small and all crates are high-download and widely vendored |
| CI provenance gate bypassed by pushing a figure outside `measurements/` | Tampering | Scope the D-13 gate by file type and content, not only by directory, or state the directory convention as a reviewed rule |

## Main-thread-gated research

None outstanding. This research pass ran on the main thread with browser-harness, so the
login-gated and JS-challenged sources were reachable directly. One source, the Linux Foundation
RT wiki, was behind a Cloudflare interstitial that did not clear; it was retrieved from the
Wayback Machine instead and the snapshot date is recorded in the sources below.

## Sources

### Primary (HIGH confidence)

- https://docs.kernel.org/trace/timerlat-tracer.html - timerlat mechanism, `stop_tracing_us`,
  `print_stack`, IRQ vs thread latency split, the osnoise event integration, user-space interface
- https://docs.kernel.org/trace/osnoise-tracer.html - interference counters (HW/NMI/IRQ/SIRQ/THREAD),
  tracer options, `OSNOISE_PREEMPT_DISABLE` and `OSNOISE_IRQ_DISABLE`, the osnoise tracepoints
- https://docs.kernel.org/trace/hwlat_detector.html - SMI detection mechanism and the statement
  that the kernel does not service or observe SMIs (basis for the D-18 note)
- https://docs.kernel.org/admin-guide/kernel-per-CPU-kthreads.html - the canonical per-CPU kthread
  noise catalog (ksoftirqd, kworker, rcuc, rcuo*, irq/*), the per-softirq mitigations, and the
  CPU offline/online trick for migrating recurring timers off an isolated core
- https://docs.kernel.org/timers/no_hz.html - nohz_full requirements, RCU callback offload
  (`rcu_nocbs`), and the caveat that at least one housekeeping CPU must keep its tick
- https://manpages.debian.org/unstable/rt-tests/cyclictest.8.en.html - confirms `--json=FILENAME`,
  `--histofall`, `--histfile`, `--breaktrace`, `--tracemark`, `--mainaffinity`, `--policy`
- https://manpages.debian.org/unstable/rtla/rtla-timerlat-top.1.en.html - rtla timerlat top options
- https://packages.ubuntu.com/search?keywords=rt-tests - rt-tests 2.2-1 (jammy), 2.5-1 (noble)
- https://packages.ubuntu.com/search?keywords=rtla - confirms rtla is NOT packaged for Ubuntu
- https://crates.io/api/v1/crates/{hdrhistogram,procfs,schemars,blake3,serde,serde_json,clap,thiserror,anyhow,time,insta,assert_cmd,predicates}
  - all versions in the Standard Stack table verified 2026-08-30

### Secondary (MEDIUM confidence)

- http://web.archive.org/web/20260117201034/https://wiki.linuxfoundation.org/realtime/documentation/howto/tools/cyclictest/tracing
  - the break-trace plus tracemark methodology and the mandatory tracing-overhead calibration
  step, with the 130 us -> 190 us -> 180 us worked example. Wayback snapshot 2026-01-17; the live
  page was behind a Cloudflare interstitial. Page itself last modified 2022-01-19.
- https://www.osadl.org/Latency-plots.latency-plots.0.html - OSADL QA farm protocol: 5h33m runs
  twice daily, idle then defined mid-range load (1 Mbit/s network, 1 MB/s I/O, 1 MB/s memory),
  1 us resolution, log y axis, worst case as headline, and the argument that a long series is
  needed to establish worst case
- https://www.thegoodpenguin.co.uk/blog/improving-real-time-performance-with-the-realtime-linux-analysis-tool-rtla/
  - complete worked `rtla timerlat top --auto` investigation on an i.MX8QM, including the full
  decomposed output format and a real root cause (ondemand cpufreq governor) found from the stack
  trace. Published 2025-09-03. Vendor blog, so the method is the takeaway, not the numbers.
- https://api.github.com/repos/benchmark-action/github-action-benchmark - 1,251 stars, active 2026-07-23
- https://api.github.com/repos/bencherdev/bencher - 892 stars, active 2026-08-30

### Tertiary (LOW confidence, flagged for validation)

- https://retis.santannapisa.it/~d.casini/papers/2025/TC2025/Timerlat_TC.pdf - "Timerlat: Real-time
  Linux Scheduling Latency Measurements", IEEE Transactions on Computers 2025. Surfaced by the
  multi-source digest and cited above for provenance of the timerlat design; the PDF itself was
  not read in this pass. Worth reading before the PLAT-01 investigation if time allows.
- The hypothesis that the ~100 Hz burst is tick-related and the isolated spikes are a global
  serialising event is my own inference from the histogram structure described in CONTEXT.md. It
  is not sourced and must not be written up as a finding. It is offered only as an ordering
  heuristic for the D-20 investigation cycles.

## Metadata

**Research method:** Main-thread browser-harness pass per the global operating instruction that
browser-harness cannot be delegated to a subagent. Multi-source HTTP digest via
`research_topic.py` for landscape mapping (low yield on this niche, as the practice note warns),
then direct retrieval from kernel.org, Debian manpages, Ubuntu and Debian package indexes,
crates.io API, the GitHub API, OSADL, and the Wayback Machine for the Cloudflare-blocked LF wiki.

**Notable negative result:** the generic scraper sources (HN, dev.to, Lobsters, arXiv via the
digest, Stack Overflow) returned essentially nothing usable for this domain. Real-time Linux
latency debugging discourse lives in kernel documentation, vendor engineering blogs, OSADL, and
the linux-rt-users mailing list, not on the aggregator sites the digest covers. Future phases
touching RT tooling should go direct rather than starting with the digest.
