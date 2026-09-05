//! The D-15 interference diff and automatic contamination verdict.
//!
//! **A finding, not an assumption**: RESEARCH.md and this plan's own action text
//! expected `procfs` 0.18.0 to parse `/proc/interrupts` (`procfs::interrupts()`).
//! Verified against the published crate sources (`procfs-0.18.0` and its
//! `procfs-core-0.18.0` dependency): neither contains any `/proc/interrupts` or
//! per-IRQ-row parsing at all. `/proc/stat`'s aggregate counters
//! (`procfs::KernelStats`) are unrelated to the per-CPU, per-IRQ-label table this
//! module needs. This module therefore parses the text format itself, in
//! [`parse_proc_interrupts`], sharing the exact same parser between the live path
//! and the fixture-driven tests, which is a stronger answer to RESEARCH.md pitfall 5
//! than "the same format, reimplemented twice" would have been.
//!
//! Counters tracked, and why: CAL (function-call IPIs) and TLB (shootdowns) are the
//! pair the 2026-08-28 post-mortem found diagnostic (an SSH session's process
//! creation broadcast TLB shootdown IPIs to the isolated cores, and CAL counts on
//! CPUs 6-11 reached roughly 137k). RES (rescheduling interrupts) is the closest true
//! per-CPU proxy for scheduling interference; it fills
//! [`nr_manifest::InterferenceSnapshot::context_switches`], since `/proc/stat`'s
//! `ctxt` counter is a single machine-wide total with no per-CPU breakdown, and that
//! field's type (`Vec<CpuCounter>`, one entry per isolated CPU) requires one. Device
//! IRQ totals (every numbered IRQ row, summed per CPU) fill `irqs`.
//!
//! **D-24 addendum (2026-09-02): the counters above are not a sufficient detector.**
//! The D-17 calibration pair (`measurements/2026-09-01-precision3591-calibration-clean`,
//! 3600s, vs `measurements/2026-09-02-precision3591-calibration-contaminated`, 900s,
//! differing in contamination and also in duration) does not separate on these counters
//! at any usable magnitude, while producing a worst case 50x higher (78 us clean vs
//! 3856 us contaminated).
//!
//! **Corrected 2026-09-04.** This note previously claimed the contaminated arm showed
//! FEWER CAL/TLB/RES/device-IRQ counts than the clean arm. That is wrong, and an
//! external audit caught it. Summing each arm's recorded per-CPU deltas over the
//! isolated cores and normalising by the recorded durations:
//!
//! | Counter    | Clean, 3600s | Contaminated, 900s | Clean/hour | Contaminated/hour |
//! |------------|-------------:|-------------------:|-----------:|------------------:|
//! | CAL        |            6 |                 12 |          6 |                48 |
//! | TLB        |            6 |                  6 |          6 |                24 |
//! | RES        |           55 |                 24 |         55 |                96 |
//! | Device IRQ |         1127 |                211 |       1127 |               844 |
//!
//! Only device IRQs invert. CAL is higher on the contaminated arm even before
//! normalising. The earlier reading compared raw totals across a 4x duration
//! difference and drew the opposite conclusion from the data.
//!
//! The counters are still not a usable detector, but for a different and more
//! interesting reason: the magnitudes are absurd. One to two CAL IPIs per isolated CPU
//! over an hour, against the roughly 137,000 the 2026-08-28 evidence led the project to
//! expect on a contaminated run. Whatever these snapshots are counting, it is not the
//! interference the D-15 design assumed, and two observations of unequal duration are
//! not a calibration set in any case. Treat them as diagnostics, not as a classifier
//! input, until that discrepancy is explained.
//!
//! The previously offered explanation, that `irqaffinity=0-5,12-21` keeps a global
//! stall from registering as per-core interrupt traffic, is withdrawn: `irqaffinity`
//! sets a default affinity mask for device IRQs and is not a mechanism that suppresses
//! CAL/TLB/RES IPI accounting on the cores those IPIs are delivered to. It was a
//! plausible story rather than a tested one.
//!
//! What separates the two runs instead is the tail of the latency distribution, not
//! its bulk: p50/p95/p99/p99.9 are nearly identical between them, but the global
//! maximum diverges sharply from p99 (a ratio of 8.7 clean vs 428.4 contaminated), and
//! every isolated thread's own maximum lands within a narrow band of every other
//! thread's (a spread of 3.6% contaminated vs 76.9% clean). [`compute_tail_metrics`]
//! turns this into three per-run numbers ([`nr_manifest::TailMetrics`]);
//! [`evaluate_tail`] turns those into a verdict. The interference counters above are
//! retained and still recorded: they remain useful evidence for a different class of
//! contamination (for example a genuinely busy IRQ storm bleeding onto an isolated
//! core), they are simply no longer the sole basis for the verdict.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

use nr_histogram::hist::CyclictestRun;
use nr_histogram::percentiles::PercentileError;
use nr_manifest::{
    ContaminationVerdict, CpuCounter, InterferenceDelta, InterferenceSnapshot,
    InterferenceSnapshotPair, TailMetrics,
};
use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum InterferenceError {
    #[error("failed to read {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("/proc/interrupts has no header line naming any CPU column")]
    MissingHeader,
    #[error("/proc/interrupts is missing the required '{label}' row")]
    MissingRow { label: &'static str },
    #[error("'{label}' row has {found} per-CPU columns, expected {expected}")]
    RowWidthMismatch {
        label: &'static str,
        expected: usize,
        found: usize,
    },
    #[error("isolated cpu{cpu} is not present in /proc/interrupts (only {num_cpus} CPU columns)")]
    CpuOutOfRange { cpu: u32, num_cpus: usize },
    #[error("failed to parse thresholds JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("thresholds status must be 'uncalibrated' or 'calibrated', found {0:?}")]
    InvalidStatus(String),
    #[error("status is 'calibrated' but at least one per_run_hour value is null")]
    CalibratedMissingThresholds,
    #[error(
        "status is 'calibrated' but calibration.derived_from names fewer than 2 runs; \
         a threshold set that cannot say which runs it came from is not calibrated"
    )]
    CalibratedMissingProvenance,
    #[error("failed to compute D-24 tail metrics: {0}")]
    TailPercentile(#[from] PercentileError),
    #[error(
        "status is 'provisional' but tail_metrics thresholds are missing or incomplete: both \
         tail_excursion_ratio_max and thread_max_spread_min are required"
    )]
    ProvisionalMissingThresholds,
    #[error(
        "status is 'provisional' but calibration.derived_from names fewer than 2 runs; a \
         threshold set that cannot say which runs informed it is not provisional, it is \
         invented"
    )]
    ProvisionalMissingProvenance,
}

// ---------------------------------------------------------------------------------
// Parsing /proc/interrupts
// ---------------------------------------------------------------------------------

#[derive(Debug)]
struct ParsedInterrupts {
    num_cpus: usize,
    named_rows: BTreeMap<&'static str, Vec<u64>>,
    /// Per-CPU sum across every numbered (device) IRQ row.
    device_irq_totals: Vec<u64>,
}

const TRACKED_ROWS: [&str; 3] = ["CAL", "TLB", "RES"];

fn tracked_label(label: &str) -> Option<&'static str> {
    match label {
        "CAL" => Some("CAL"),
        "TLB" => Some("TLB"),
        "RES" => Some("RES"),
        _ => None,
    }
}

/// Parses the `/proc/interrupts` text format: a header naming `CPU0..CPUn-1`, then
/// one row per IRQ. A numbered row (`  1:`, `162:`, ...) carries a per-CPU count
/// followed by a driver/device descriptor; a named aggregate row (`CAL:`, `LOC:`,
/// `ERR:`, ...) carries either a full per-CPU breakdown or, for a couple of rows
/// (`ERR`, `MIS`), a single machine-wide count. Neither kind of trailing text is
/// numeric, so greedily consuming up to `num_cpus` integer tokens after the label
/// and stopping at the first non-integer token separates the counts from the
/// descriptor without needing to parse the descriptor at all.
fn parse_proc_interrupts(text: &str) -> Result<ParsedInterrupts, InterferenceError> {
    let mut lines = text.lines();
    let header = loop {
        let Some(line) = lines.next() else {
            return Err(InterferenceError::MissingHeader);
        };
        if line.split_whitespace().any(|tok| tok.starts_with("CPU")) {
            break line;
        }
    };
    let num_cpus = header
        .split_whitespace()
        .filter(|tok| tok.starts_with("CPU"))
        .count();
    if num_cpus == 0 {
        return Err(InterferenceError::MissingHeader);
    }

    let mut named_rows: BTreeMap<&'static str, Vec<u64>> = BTreeMap::new();
    let mut device_irq_totals = vec![0u64; num_cpus];

    for line in lines {
        let mut tokens = line.split_whitespace();
        let Some(raw_label) = tokens.next() else {
            continue;
        };
        let label = raw_label.trim_end_matches(':');
        if label.is_empty() {
            continue;
        }

        let mut values = Vec::with_capacity(num_cpus);
        for tok in tokens.by_ref() {
            if values.len() == num_cpus {
                break;
            }
            match tok.parse::<u64>() {
                Ok(v) => values.push(v),
                Err(_) => break,
            }
        }

        if let Some(tracked) = tracked_label(label) {
            if values.len() != num_cpus {
                return Err(InterferenceError::RowWidthMismatch {
                    label: tracked,
                    expected: num_cpus,
                    found: values.len(),
                });
            }
            named_rows.insert(tracked, values);
        } else if label.chars().all(|c| c.is_ascii_digit()) {
            for (total, v) in device_irq_totals.iter_mut().zip(values.iter()) {
                *total += v;
            }
        }
    }

    for label in TRACKED_ROWS {
        if !named_rows.contains_key(label) {
            return Err(InterferenceError::MissingRow { label });
        }
    }

    Ok(ParsedInterrupts {
        num_cpus,
        named_rows,
        device_irq_totals,
    })
}

fn build_snapshot(
    parsed: &ParsedInterrupts,
    isolated_cpus: &[u32],
) -> Result<InterferenceSnapshot, InterferenceError> {
    for &cpu in isolated_cpus {
        if cpu as usize >= parsed.num_cpus {
            return Err(InterferenceError::CpuOutOfRange {
                cpu,
                num_cpus: parsed.num_cpus,
            });
        }
    }

    let cal = &parsed.named_rows["CAL"];
    let tlb = &parsed.named_rows["TLB"];
    let res = &parsed.named_rows["RES"];

    let mut cal_ipis = Vec::with_capacity(isolated_cpus.len());
    let mut tlb_ipis = Vec::with_capacity(isolated_cpus.len());
    let mut context_switches = Vec::with_capacity(isolated_cpus.len());
    let mut irqs = Vec::with_capacity(isolated_cpus.len());

    for &cpu in isolated_cpus {
        let idx = cpu as usize;
        cal_ipis.push(CpuCounter {
            cpu,
            count: cal[idx],
        });
        tlb_ipis.push(CpuCounter {
            cpu,
            count: tlb[idx],
        });
        context_switches.push(CpuCounter {
            cpu,
            count: res[idx],
        });
        irqs.push(CpuCounter {
            cpu,
            count: parsed.device_irq_totals[idx],
        });
    }

    Ok(InterferenceSnapshot {
        isolated_cpus: isolated_cpus.to_vec(),
        cal_ipis,
        tlb_ipis,
        context_switches,
        irqs,
    })
}

/// Parses `text` as `/proc/interrupts` and builds an [`InterferenceSnapshot`] scoped
/// to `isolated_cpus`. The function both the live path and every test call.
pub fn snapshot_from_text(
    text: &str,
    isolated_cpus: &[u32],
) -> Result<InterferenceSnapshot, InterferenceError> {
    let parsed = parse_proc_interrupts(text)?;
    build_snapshot(&parsed, isolated_cpus)
}

/// Reads the real `/proc/interrupts` and builds a snapshot scoped to
/// `isolated_cpus`. See the module documentation for why this reads the file
/// directly rather than through `procfs`.
#[cfg(target_os = "linux")]
pub fn snapshot(isolated_cpus: &[u32]) -> Result<InterferenceSnapshot, InterferenceError> {
    let text =
        std::fs::read_to_string("/proc/interrupts").map_err(|source| InterferenceError::Io {
            path: "/proc/interrupts".to_string(),
            source,
        })?;
    snapshot_from_text(&text, isolated_cpus)
}

// ---------------------------------------------------------------------------------
// Thresholds (D-17)
// ---------------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ThresholdsFile {
    #[allow(dead_code)]
    schema_version: u32,
    status: String,
    calibration: CalibrationBlock,
    per_run_hour: PerRunHour,
    /// Present for `status: "provisional"`. Absent from a file predating D-24, which is
    /// fine: only the `"provisional"` arm of [`Thresholds::parse`] reads it.
    tail_metrics: Option<TailThresholdsRaw>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct CalibrationBlock {
    derived_from: Vec<String>,
    #[allow(dead_code)]
    note: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PerRunHour {
    cal_delta_max: Option<f64>,
    tlb_delta_max: Option<f64>,
    res_delta_max: Option<f64>,
    device_irq_delta_max: Option<f64>,
    context_switch_delta_max: Option<f64>,
}

/// D-24 tail-metric thresholds, as loaded from JSON. Both fields are required for
/// `status: "provisional"`; see [`Thresholds::parse`].
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct TailThresholdsRaw {
    tail_excursion_ratio_max: Option<f64>,
    thread_max_spread_min: Option<f64>,
}

/// D-24 tail-metric thresholds, validated and ready to evaluate against. Loaded only
/// for [`Thresholds::Provisional`].
#[derive(Debug, Clone)]
pub struct TailThresholds {
    pub tail_excursion_ratio_max: f64,
    pub thread_max_spread_min: f64,
}

/// D-17: contamination thresholds. `Uncalibrated` was the shipped state
/// (`config/contamination-thresholds.json`) before plan 01-11 ran the D-17 calibration
/// pair; [`verdict`] always returns [`ContaminationVerdict::Uncalibrated`] for it,
/// regardless of the observed deltas or tail metrics.
#[derive(Debug, Clone)]
pub enum Thresholds {
    Uncalibrated,
    /// D-24: a threshold set derived from exactly 2 runs (the D-17 calibration pair),
    /// evaluated for real against the tail metrics, but never presented as calibrated.
    /// Two runs are not a calibration set: the durations differ (3600s clean vs 900s
    /// contaminated), and `max` is duration-sensitive (a longer clean run has more
    /// chances to catch a rare excursion, so the clean arm's observed 8.7 excursion
    /// ratio is a lower bound on what a clean 4 hour headline run might show; its
    /// 76.9% spread is also computed over very small values, 18 to 78 us, where
    /// relative spread is naturally large and noisy).
    ///
    /// [`verdict`] still reports a genuine, metric-driven `Clean`/`Contaminated`
    /// outcome under this variant, rather than a flat `Uncalibrated` regardless of how
    /// extreme a run's own numbers are: that outcome is real, human-readable evidence.
    /// What it is not is a licence to publish: `nr-cli`'s `determine_exclusion` checks
    /// [`nr_manifest::InterferenceSnapshotPair::thresholds_provisional`] and never
    /// admits a run to the headline series on the strength of a provisional verdict
    /// alone. Only a future threshold set calibrated from many runs, not two, may do
    /// that.
    Provisional(TailThresholds),
    Calibrated(CalibratedThresholds),
}

#[derive(Debug, Clone)]
pub struct CalibratedThresholds {
    pub cal_delta_max: f64,
    pub tlb_delta_max: f64,
    pub res_delta_max: f64,
    pub device_irq_delta_max: f64,
    /// Loaded and required (a `calibrated` file must set every `per_run_hour`
    /// value), but not yet compared against in [`verdict`]: this crate's
    /// `context_switches` counter is populated from the RES row (see the module
    /// documentation), which `res_delta_max` already thresholds. Reserved for a
    /// future, independent per-CPU context-switch source.
    pub context_switch_delta_max: f64,
    pub derived_from: Vec<String>,
}

impl Thresholds {
    pub fn load(path: &Path) -> Result<Self, InterferenceError> {
        let text = std::fs::read_to_string(path).map_err(|source| InterferenceError::Io {
            path: path.display().to_string(),
            source,
        })?;
        Self::parse(&text)
    }

    /// Named `parse` rather than `from_str`: this type intentionally does not
    /// implement `std::str::FromStr` (loading a `Thresholds` is a deliberate,
    /// named step in the D-15 flow, not a generic string conversion).
    pub fn parse(text: &str) -> Result<Self, InterferenceError> {
        let file: ThresholdsFile = serde_json::from_str(text)?;

        match file.status.as_str() {
            "uncalibrated" => Ok(Thresholds::Uncalibrated),
            "provisional" => {
                let tail = file
                    .tail_metrics
                    .as_ref()
                    .ok_or(InterferenceError::ProvisionalMissingThresholds)?;
                let (Some(ratio_max), Some(spread_min)) =
                    (tail.tail_excursion_ratio_max, tail.thread_max_spread_min)
                else {
                    return Err(InterferenceError::ProvisionalMissingThresholds);
                };
                if file.calibration.derived_from.len() < 2 {
                    return Err(InterferenceError::ProvisionalMissingProvenance);
                }
                Ok(Thresholds::Provisional(TailThresholds {
                    tail_excursion_ratio_max: ratio_max,
                    thread_max_spread_min: spread_min,
                }))
            }
            "calibrated" => {
                let p = &file.per_run_hour;
                let (Some(cal), Some(tlb), Some(res), Some(irq), Some(ctxt)) = (
                    p.cal_delta_max,
                    p.tlb_delta_max,
                    p.res_delta_max,
                    p.device_irq_delta_max,
                    p.context_switch_delta_max,
                ) else {
                    return Err(InterferenceError::CalibratedMissingThresholds);
                };
                if file.calibration.derived_from.len() < 2 {
                    return Err(InterferenceError::CalibratedMissingProvenance);
                }
                Ok(Thresholds::Calibrated(CalibratedThresholds {
                    cal_delta_max: cal,
                    tlb_delta_max: tlb,
                    res_delta_max: res,
                    device_irq_delta_max: irq,
                    context_switch_delta_max: ctxt,
                    derived_from: file.calibration.derived_from,
                }))
            }
            other => Err(InterferenceError::InvalidStatus(other.to_string())),
        }
    }
}

// ---------------------------------------------------------------------------------
// The verdict (D-15)
// ---------------------------------------------------------------------------------

/// The result of comparing a before/after pair, plus the D-24 tail metrics, against
/// thresholds. Wraps [`nr_manifest::InterferenceSnapshotPair`] with one thing it has no
/// field for: a human-readable reason naming the offending counter and CPU (D-15) or
/// the offending tail-metric comparison (D-24). `nr-cli` folds `reason` into
/// `RunManifest::exclusion_reason` when `pair.verdict` is `Contaminated` (BENCH-06: the
/// run is retained and published, only excluded from the headline series).
#[derive(Debug, Clone)]
pub struct VerdictOutcome {
    pub pair: InterferenceSnapshotPair,
    pub reason: Option<String>,
}

fn diff_counters(before: &[CpuCounter], after: &[CpuCounter]) -> Vec<CpuCounter> {
    let before_map: BTreeMap<u32, u64> = before.iter().map(|c| (c.cpu, c.count)).collect();
    after
        .iter()
        .map(|c| {
            let prev = before_map.get(&c.cpu).copied().unwrap_or(0);
            CpuCounter {
                cpu: c.cpu,
                count: c.count.saturating_sub(prev),
            }
        })
        .collect()
}

fn compute_delta(before: &InterferenceSnapshot, after: &InterferenceSnapshot) -> InterferenceDelta {
    InterferenceDelta {
        cal_ipis: diff_counters(&before.cal_ipis, &after.cal_ipis),
        tlb_ipis: diff_counters(&before.tlb_ipis, &after.tlb_ipis),
        context_switches: diff_counters(&before.context_switches, &after.context_switches),
        irqs: diff_counters(&before.irqs, &after.irqs),
    }
}

/// D-24: computes [`TailMetrics`] from `run`'s own histogram. See the module
/// documentation for what each field means and why it separates the D-17 calibration
/// pair where the interference counters above do not.
fn compute_tail_metrics(
    run: &CyclictestRun,
    run_duration: Duration,
) -> Result<TailMetrics, InterferenceError> {
    let percentiles = run.percentiles(&[0.99])?;
    let p99_us = percentiles.values.first().map(|&(_, v)| v).unwrap_or(0);
    let max_us = percentiles.max_us;

    // A run with p99 == 0 has no meaningful ratio to compute; recorded as 0.0 rather
    // than dividing by zero. Not observed on any real capture this project has taken.
    let tail_excursion_ratio = if p99_us == 0 {
        0.0
    } else {
        max_us as f64 / p99_us as f64
    };

    let thread_max_max = run.max_us.iter().copied().max().unwrap_or(0);
    let thread_max_min = run.max_us.iter().copied().min().unwrap_or(0);
    let thread_max_spread = if thread_max_max == 0 {
        0.0
    } else {
        (thread_max_max - thread_max_min) as f64 / thread_max_max as f64
    };

    // `.max(f64::MIN_POSITIVE)` mirrors `evaluate`'s own guard below: a zero duration
    // must never produce an infinite or NaN rate.
    let overflow_rate_per_s =
        percentiles.overflow_samples as f64 / run_duration.as_secs_f64().max(f64::MIN_POSITIVE);

    Ok(TailMetrics {
        tail_excursion_ratio,
        thread_max_spread,
        overflow_rate_per_s,
    })
}

fn evaluate(
    delta: &InterferenceDelta,
    limits: &CalibratedThresholds,
    run_duration: Duration,
) -> (ContaminationVerdict, Option<String>) {
    let hours = (run_duration.as_secs_f64() / 3600.0).max(f64::MIN_POSITIVE);

    let checks: [(&str, &[CpuCounter], f64); 4] = [
        ("CAL", &delta.cal_ipis, limits.cal_delta_max),
        ("TLB", &delta.tlb_ipis, limits.tlb_delta_max),
        ("RES", &delta.context_switches, limits.res_delta_max),
        ("device IRQ", &delta.irqs, limits.device_irq_delta_max),
    ];

    for (name, counters, per_hour_limit) in checks {
        let limit = per_hour_limit * hours;
        for counter in counters {
            if counter.count as f64 > limit {
                let reason = format!(
                    "{name} delta {} on cpu{} exceeds {per_hour_limit} per run hour",
                    counter.count, counter.cpu
                );
                return (ContaminationVerdict::Contaminated, Some(reason));
            }
        }
    }

    (ContaminationVerdict::Clean, None)
}

/// D-24's tail-based verdict: `Contaminated` only when BOTH the tail excursion ratio is
/// high AND the thread-max spread is low (the global-stall signature described in the
/// module documentation). Neither alone is sufficient: a high ratio with high spread is
/// one noisy thread, not a global event, and a low spread with a low ratio is just a
/// clean run whose per-thread maxima happen to sit close together (small numbers have
/// naturally high relative noise). Deliberately does not gate on `overflow_rate_per_s`:
/// it is recorded on every run (see [`TailMetrics`]), but it depends on the histogram
/// bound and is coarser evidence than the other two, so it is not part of this
/// decision.
fn evaluate_tail(
    metrics: &TailMetrics,
    limits: &TailThresholds,
) -> (ContaminationVerdict, Option<String>) {
    let excursion_high = metrics.tail_excursion_ratio > limits.tail_excursion_ratio_max;
    let spread_low = metrics.thread_max_spread < limits.thread_max_spread_min;

    if excursion_high && spread_low {
        let reason = format!(
            "tail excursion ratio {:.1} exceeds the provisional limit {} and thread-max spread \
             {:.3} is below the provisional limit {}: the global-stall signature (a rare, \
             catastrophic excursion landing at nearly the same value on every isolated thread)",
            metrics.tail_excursion_ratio,
            limits.tail_excursion_ratio_max,
            metrics.thread_max_spread,
            limits.thread_max_spread_min
        );
        (ContaminationVerdict::Contaminated, Some(reason))
    } else {
        (ContaminationVerdict::Clean, None)
    }
}

/// Computes the delta, the D-24 tail metrics, and renders D-15's verdict. `thresholds`
/// (the interference-counter kind) are expressed per run hour and scaled by
/// `run_duration`, so a 1 hour weekly run and a 12 hour soak use the same
/// configuration; this scaling is unchanged by D-24 and applies only to the
/// `Calibrated` arm below. A `Contaminated` verdict never drops the run (BENCH-06);
/// that decision belongs to the caller, using `reason`.
pub fn verdict(
    before: InterferenceSnapshot,
    after: InterferenceSnapshot,
    run: &CyclictestRun,
    thresholds: &Thresholds,
    run_duration: Duration,
) -> Result<VerdictOutcome, InterferenceError> {
    let delta = compute_delta(&before, &after);
    let tail_metrics = compute_tail_metrics(run, run_duration)?;

    let (contamination, reason, thresholds_provisional) = match thresholds {
        Thresholds::Uncalibrated => (ContaminationVerdict::Uncalibrated, None, false),
        Thresholds::Provisional(limits) => {
            let (verdict, reason) = evaluate_tail(&tail_metrics, limits);
            (verdict, reason, true)
        }
        Thresholds::Calibrated(limits) => {
            let (verdict, reason) = evaluate(&delta, limits, run_duration);
            (verdict, reason, false)
        }
    };

    Ok(VerdictOutcome {
        pair: InterferenceSnapshotPair {
            before,
            after,
            delta,
            tail_metrics: Some(tail_metrics),
            thresholds_provisional: Some(thresholds_provisional),
            verdict: contamination,
        },
        reason,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
            CPU0       CPU1       \n\
   1:          5          6 IR-IO-APIC    1-edge      i8042\n\
 CAL:        100        200   Function call interrupts\n\
 TLB:         10         20   TLB shootdowns\n\
 RES:          1          2   Rescheduling interrupts\n\
 ERR:          0\n";

    #[test]
    fn parse_proc_interrupts_reads_named_rows_and_device_total() {
        let parsed = parse_proc_interrupts(SAMPLE).expect("parses");
        assert_eq!(parsed.num_cpus, 2);
        assert_eq!(parsed.named_rows["CAL"], vec![100, 200]);
        assert_eq!(parsed.named_rows["TLB"], vec![10, 20]);
        assert_eq!(parsed.named_rows["RES"], vec![1, 2]);
        assert_eq!(parsed.device_irq_totals, vec![5, 6]);
    }

    #[test]
    fn parse_proc_interrupts_rejects_missing_row() {
        let no_res = "            CPU0       \n CAL:        100   x\n TLB:         10   x\n";
        let err = parse_proc_interrupts(no_res).unwrap_err();
        assert!(matches!(
            err,
            InterferenceError::MissingRow { label: "RES" }
        ));
    }

    #[test]
    fn parse_proc_interrupts_rejects_missing_header() {
        let err = parse_proc_interrupts("").unwrap_err();
        assert!(matches!(err, InterferenceError::MissingHeader));
    }

    #[test]
    fn thresholds_uncalibrated_loads() {
        let json = r#"{"schema_version":1,"status":"uncalibrated","calibration":{"derived_from":[],"note":"x"},"per_run_hour":{"cal_delta_max":null,"tlb_delta_max":null,"res_delta_max":null,"device_irq_delta_max":null,"context_switch_delta_max":null}}"#;
        assert!(matches!(
            Thresholds::parse(json).unwrap(),
            Thresholds::Uncalibrated
        ));
    }

    #[test]
    fn thresholds_calibrated_requires_every_value_present() {
        let json = r#"{"schema_version":1,"status":"calibrated","calibration":{"derived_from":["a","b"],"note":"x"},"per_run_hour":{"cal_delta_max":1.0,"tlb_delta_max":1.0,"res_delta_max":1.0,"device_irq_delta_max":1.0,"context_switch_delta_max":null}}"#;
        assert!(matches!(
            Thresholds::parse(json).unwrap_err(),
            InterferenceError::CalibratedMissingThresholds
        ));
    }

    #[test]
    fn thresholds_rejects_unknown_status() {
        let json = r#"{"schema_version":1,"status":"guessed","calibration":{"derived_from":[],"note":"x"},"per_run_hour":{"cal_delta_max":null,"tlb_delta_max":null,"res_delta_max":null,"device_irq_delta_max":null,"context_switch_delta_max":null}}"#;
        assert!(matches!(
            Thresholds::parse(json).unwrap_err(),
            InterferenceError::InvalidStatus(_)
        ));
    }

    #[test]
    fn thresholds_provisional_loads_with_tail_metrics() {
        let json = r#"{"schema_version":1,"status":"provisional","calibration":{"derived_from":["a","b"],"note":"x"},"per_run_hour":{"cal_delta_max":null,"tlb_delta_max":null,"res_delta_max":null,"device_irq_delta_max":null,"context_switch_delta_max":null},"tail_metrics":{"tail_excursion_ratio_max":50.0,"thread_max_spread_min":0.2}}"#;
        let thresholds = Thresholds::parse(json).expect("valid provisional thresholds");
        match thresholds {
            Thresholds::Provisional(limits) => {
                assert_eq!(limits.tail_excursion_ratio_max, 50.0);
                assert_eq!(limits.thread_max_spread_min, 0.2);
            }
            other => panic!("expected Provisional, got {other:?}"),
        }
    }

    #[test]
    fn thresholds_provisional_requires_tail_metrics_present() {
        let json = r#"{"schema_version":1,"status":"provisional","calibration":{"derived_from":["a","b"],"note":"x"},"per_run_hour":{"cal_delta_max":null,"tlb_delta_max":null,"res_delta_max":null,"device_irq_delta_max":null,"context_switch_delta_max":null}}"#;
        assert!(matches!(
            Thresholds::parse(json).unwrap_err(),
            InterferenceError::ProvisionalMissingThresholds
        ));
    }

    #[test]
    fn thresholds_provisional_requires_calibration_provenance() {
        let json = r#"{"schema_version":1,"status":"provisional","calibration":{"derived_from":[],"note":"x"},"per_run_hour":{"cal_delta_max":null,"tlb_delta_max":null,"res_delta_max":null,"device_irq_delta_max":null,"context_switch_delta_max":null},"tail_metrics":{"tail_excursion_ratio_max":50.0,"thread_max_spread_min":0.2}}"#;
        assert!(matches!(
            Thresholds::parse(json).unwrap_err(),
            InterferenceError::ProvisionalMissingProvenance
        ));
    }

    /// D-24: a fresh, minimal, valid single-thread run, used so `verdict` (which now
    /// always computes tail metrics) has something real to compute over. Not the D-17
    /// calibration pair itself; see `crates/capture/tests/interference.rs` for the
    /// tests proving the metrics separate those two real captures.
    fn sample_cyclictest_run() -> CyclictestRun {
        let hist = "# Histogram\n\
                     000001 000001\n\
                     000002 000002\n\
                     # Min Latencies: 00001\n\
                     # Avg Latencies: 00001\n\
                     # Max Latencies: 00002\n\
                     # Histogram Overflows: 00000\n\
                     # Histogram Overflow at cycle number:\n\
                     # Thread 0: \n";
        nr_histogram::hist::parse_hist(hist, Some(400)).expect("the minimal fixture parses")
    }

    #[test]
    fn verdict_computes_tail_metrics_even_under_uncalibrated_thresholds() {
        let snapshot = InterferenceSnapshot {
            isolated_cpus: vec![6],
            cal_ipis: vec![CpuCounter { cpu: 6, count: 0 }],
            tlb_ipis: vec![CpuCounter { cpu: 6, count: 0 }],
            context_switches: vec![CpuCounter { cpu: 6, count: 0 }],
            irqs: vec![CpuCounter { cpu: 6, count: 0 }],
        };
        let outcome = verdict(
            snapshot.clone(),
            snapshot,
            &sample_cyclictest_run(),
            &Thresholds::Uncalibrated,
            Duration::from_secs(1),
        )
        .expect("verdict computes over a valid run");

        // Uncalibrated still means Uncalibrated (D-17's own rule is untouched by D-24),
        // but the tail metrics themselves are always recorded regardless.
        assert_eq!(outcome.pair.verdict, ContaminationVerdict::Uncalibrated);
        assert_eq!(outcome.pair.thresholds_provisional, Some(false));
        let tail = outcome
            .pair
            .tail_metrics
            .expect("verdict always populates tail_metrics");
        assert_eq!(
            tail.thread_max_spread, 0.0,
            "a single thread has zero spread"
        );
    }

    #[test]
    fn evaluate_tail_requires_both_high_ratio_and_low_spread() {
        let limits = TailThresholds {
            tail_excursion_ratio_max: 50.0,
            thread_max_spread_min: 0.2,
        };

        // High ratio, high spread: one noisy thread, not a global stall.
        let (noisy_thread, _) = evaluate_tail(
            &TailMetrics {
                tail_excursion_ratio: 100.0,
                thread_max_spread: 0.9,
                overflow_rate_per_s: 0.0,
            },
            &limits,
        );
        assert_eq!(noisy_thread, ContaminationVerdict::Clean);

        // Low spread, low ratio: a clean run whose small maxima sit close together.
        let (small_and_close, _) = evaluate_tail(
            &TailMetrics {
                tail_excursion_ratio: 1.0,
                thread_max_spread: 0.05,
                overflow_rate_per_s: 0.0,
            },
            &limits,
        );
        assert_eq!(small_and_close, ContaminationVerdict::Clean);

        // Both: the global-stall signature.
        let (global_stall, reason) = evaluate_tail(
            &TailMetrics {
                tail_excursion_ratio: 428.4,
                thread_max_spread: 0.036,
                overflow_rate_per_s: 0.8822,
            },
            &limits,
        );
        assert_eq!(global_stall, ContaminationVerdict::Contaminated);
        assert!(reason.is_some());
    }
}
