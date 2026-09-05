//! Parses cyclictest `--json` output into a typed [`CyclictestSummary`], matching the exact
//! schema captured live from this project's reference rig (rt-tests 2.9-1ubuntu1, cyclictest
//! V 2.80; see `docs/rig/recon-2026-08-31/FINDINGS.md`, "What is in cyclictest --json"). Every
//! field here was read off a real capture rather than assumed from an older rt-tests version.
//!
//! There is no `overflow` key anywhere in this schema: cyclictest's `--json` only reports
//! histogram bins that were actually populated, so a run that never exceeds its configured
//! `--histogram`/`--histofall` bound carries no overflow information in JSON at all. Overflow
//! accounting (BENCH-05, the D-23 correction) is only available from the `.hist` file's
//! `# Histogram Overflows:` footer line; use [`crate::hist::parse_hist`] (or
//! [`crate::hist::parse_hist_file`]) for that, never this module.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use crate::hist::{CyclictestRun, HistError};

/// A full `cyclictest --json` capture. Deserialization is `deny_unknown_fields`: an rt-tests
/// version that adds a field fails loudly here rather than silently dropping data.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CyclictestSummary {
    pub file_version: u64,
    /// The literal JSON key includes a trailing colon (`"cmdline:"`), as cyclictest emits it.
    #[serde(rename = "cmdline:")]
    pub cmdline: String,
    /// The literal JSON key includes a trailing colon (`"rt_test_version:"`).
    #[serde(rename = "rt_test_version:")]
    pub rt_test_version: String,
    pub start_time: String,
    pub end_time: String,
    pub return_code: i64,
    pub sysinfo: SysInfo,
    pub num_threads: u64,
    pub resolution_in_ns: u64,
    /// Keyed by thread index as a string (`"0"` through `"num_threads - 1"`), matching the real
    /// schema rather than a dense integer-keyed array.
    pub thread: BTreeMap<String, ThreadSummary>,
}

/// `uname`-derived host information cyclictest embeds in every `--json` capture.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SysInfo {
    pub sysname: String,
    pub nodename: String,
    pub release: String,
    pub version: String,
    pub machine: String,
    pub realtime: u64,
}

/// Per-thread summary. `min`/`avg`/`max` are present directly, so a caller that only needs
/// summary statistics (not the full distribution) does not need the `.hist` file at all.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThreadSummary {
    /// Sparse: bin index (as a string) to sample count. Absent bins are implicitly zero; this
    /// is not a dense, contiguous range.
    pub histogram: BTreeMap<String, u64>,
    pub cycles: u64,
    pub min: u64,
    pub max: u64,
    pub avg: f64,
    pub cpu: u64,
    pub node: u64,
}

/// Reads and parses a `--json` file from disk.
pub fn parse_json_file(path: &Path) -> Result<CyclictestSummary, HistError> {
    let input = std::fs::read_to_string(path).map_err(|source| HistError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(serde_json::from_str(&input)?)
}

/// Cross-check a --json summary against the .hist file from the same run.
/// Both come from the same cyclictest invocation, so a disagreement means one of the two
/// files was truncated, edited, or paired with the wrong run.
///
/// Checks thread count, every per-thread maximum, and every per-thread sample count. The first
/// two alone are not enough: two files can agree on thread count and on every per-thread
/// maximum while disagreeing on how many samples were taken, and the sample count is the
/// denominator under every percentile and every over-gate fraction this project publishes.
/// Finding 6 of `01-EXTERNAL-AUDIT.md`.
pub fn reconcile(summary: &CyclictestSummary, run: &CyclictestRun) -> Result<(), HistError> {
    let json_threads = summary.num_threads as usize;
    if json_threads != run.threads {
        return Err(HistError::SummaryDisagreement {
            field: "thread count",
            json: json_threads.to_string(),
            hist: run.threads.to_string(),
        });
    }

    for (i, &hist_max) in run.max_us.iter().enumerate() {
        let key = i.to_string();
        let json_thread = summary
            .thread
            .get(&key)
            .ok_or(HistError::SummaryDisagreement {
                field: "thread index",
                json: format!("thread {key} absent from --json"),
                hist: format!("thread {key} present in .hist"),
            })?;
        if json_thread.max != hist_max {
            return Err(HistError::SummaryDisagreement {
                field: "per-thread maximum",
                json: json_thread.max.to_string(),
                hist: hist_max.to_string(),
            });
        }
    }

    for (i, &hist_samples) in samples_per_thread(run).iter().enumerate() {
        let key = i.to_string();
        let json_thread = summary
            .thread
            .get(&key)
            .ok_or(HistError::SummaryDisagreement {
                field: "thread index",
                json: format!("thread {key} absent from --json"),
                hist: format!("thread {key} present in .hist"),
            })?;
        if json_thread.cycles != hist_samples {
            return Err(HistError::SummaryDisagreement {
                field: "sample count",
                json: json_thread.cycles.to_string(),
                hist: hist_samples.to_string(),
            });
        }
    }

    Ok(())
}

/// Per-thread total sample count: binned samples plus the separately reported overflow count,
/// matching what a `--json` capture's `thread[N].cycles` counts. `CyclictestRun` carries no
/// single field for this; it is derived from `bins` (bin_us -> per-thread counts) and
/// `overflows` (per-thread overflow counts), the same two fields the maxima check above already
/// sources its per-thread data from.
fn samples_per_thread(run: &CyclictestRun) -> Vec<u64> {
    let mut counts = vec![0u64; run.threads];
    for thread_counts in run.bins.values() {
        for (count, sum) in thread_counts.iter().zip(counts.iter_mut()) {
            *sum += count;
        }
    }
    for (overflow, sum) in run.overflows.iter().zip(counts.iter_mut()) {
        *sum += overflow;
    }
    counts
}
