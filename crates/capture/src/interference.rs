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

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

use nr_manifest::{
    ContaminationVerdict, CpuCounter, InterferenceDelta, InterferenceSnapshot,
    InterferenceSnapshotPair,
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

/// D-17: contamination thresholds, expressed per run hour. `Uncalibrated` is the
/// shipped state (`config/contamination-thresholds.json`) until plan 01-11 runs the
/// calibration pair; [`verdict`] always returns
/// [`ContaminationVerdict::Uncalibrated`] for it, regardless of the observed deltas.
#[derive(Debug, Clone)]
pub enum Thresholds {
    Uncalibrated,
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

/// The result of comparing a before/after pair against thresholds. Wraps
/// [`nr_manifest::InterferenceSnapshotPair`] (unchanged from plan 01-03's schema,
/// which has no field for a human-readable reason) with one: D-15 requires the
/// verdict to name the offending counter and CPU. A future caller (nr-cli, plan
/// 01-07) folds `reason` into `RunManifest::exclusion_reason` when `pair.verdict` is
/// `Contaminated` (BENCH-06: the run is retained and published, only excluded from
/// the headline series).
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

/// Computes the delta and renders D-15's verdict. `thresholds` are expressed per run
/// hour and scaled by `run_duration`, so a 1 hour weekly run and a 12 hour soak use
/// the same configuration. A `Contaminated` verdict never drops the run (BENCH-06);
/// that decision belongs to the caller, using `reason`.
pub fn verdict(
    before: InterferenceSnapshot,
    after: InterferenceSnapshot,
    thresholds: &Thresholds,
    run_duration: Duration,
) -> VerdictOutcome {
    let delta = compute_delta(&before, &after);

    let (contamination, reason) = match thresholds {
        Thresholds::Uncalibrated => (ContaminationVerdict::Uncalibrated, None),
        Thresholds::Calibrated(limits) => evaluate(&delta, limits, run_duration),
    };

    VerdictOutcome {
        pair: InterferenceSnapshotPair {
            before,
            after,
            delta,
            verdict: contamination,
        },
        reason,
    }
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
}
