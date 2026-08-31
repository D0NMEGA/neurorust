//! The D-11 regression comparison, checked against thresholds that live in the committed,
//! reviewable `metrics/baseline.json`, never in this file. This module contains no threshold
//! literal: every number `compare` uses comes from a loaded [`Baseline`], so a change to the
//! judgement is always a reviewable diff to that file, not a code change on one laptop.
//!
//! `Contaminated` and `Uncalibrated` are distinct outcomes from `Pass`, and neither can ever be
//! reported as one: BENCH-06 and D-15 both require a losing run to be visible as what it is.

use std::path::Path;

use nr_manifest::RunClass;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::series::StageMetrics;

/// Errors loading a [`Baseline`] from disk.
#[derive(Debug, Error)]
pub enum BaselineError {
    #[error("failed to read baseline file {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse baseline JSON: {0}")]
    Parse(#[from] serde_json::Error),
}

/// One metric's regression threshold: fail when the observed value exceeds
/// `max(baseline * (1 + relative_pct / 100), baseline + absolute_us)`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Threshold {
    pub relative_pct: f64,
    pub absolute_us: u64,
}

/// The two thresholds D-11 checks independently.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Thresholds {
    pub p99: Threshold,
    pub max: Threshold,
}

/// One committed baseline value for one (rig, run class, stage) triple. D-10 tags run classes
/// so they are never averaged together: a weekly run is compared only against a weekly baseline
/// entry, never a soak.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BaselineEntry {
    pub rig_slug: String,
    pub run_class: RunClass,
    pub stage: String,
    pub p99_us: u64,
    pub max_us: u64,
}

/// The committed `metrics/baseline.json`: reviewable thresholds plus the baseline entries they
/// are checked against. Changed only by a human in a reviewed commit (D-07).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Baseline {
    pub schema_version: u32,
    pub thresholds: Thresholds,
    pub note: String,
    pub entries: Vec<BaselineEntry>,
}

impl Baseline {
    /// Reads and parses a `Baseline` from `path` (normally `metrics/baseline.json`).
    pub fn load(path: &Path) -> Result<Self, BaselineError> {
        let text = std::fs::read_to_string(path).map_err(|source| BaselineError::Io {
            path: path.display().to_string(),
            source,
        })?;
        Ok(serde_json::from_str(&text)?)
    }
}

/// Which of the two D-11 metrics regressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegressedMetric {
    P99,
    Max,
}

/// One metric that exceeded its threshold.
#[derive(Debug, Clone, PartialEq)]
pub struct MetricRegression {
    pub metric: RegressedMetric,
    pub observed_us: u64,
    pub threshold_us: u64,
}

/// The outcome of comparing one [`StageMetrics`] entry against a [`Baseline`]. `Contaminated`,
/// `Uncalibrated` and `NoComparableBaseline` are distinct from `Pass`, and nothing in this module
/// maps any of them to it: a regression gate that silently passes what it cannot compare is
/// decorative, not a gate.
#[derive(Debug, Clone, PartialEq)]
pub enum RegressionVerdict {
    Pass {
        p99_headroom_us: i64,
        max_headroom_us: i64,
    },
    /// One entry per metric that regressed.
    Fail { failures: Vec<MetricRegression> },
    /// D-15 excludes a contaminated run from the headline series; D-11 still requires it to be
    /// reported as evidence, not silently passed. The caller records a coverage gap for it
    /// rather than a pass.
    SkippedContaminated { reason: String },
    /// No calibrated thresholds exist yet (D-17) to render a real verdict against.
    SkippedUncalibrated { reason: String },
    /// The triple (rig, run class, stage) has no matching entry in the baseline. A neutral
    /// result, reported but never treated as a failure or defaulted to a pass.
    NoComparableBaseline {
        stage: String,
        run_class: RunClass,
        rig_slug: String,
    },
}

/// Compares `entry` against `baseline`, matching on the (rig, run class, stage) triple.
pub fn compare(entry: &StageMetrics, baseline: &Baseline) -> RegressionVerdict {
    use nr_manifest::ContaminationVerdict;

    match entry.contamination_verdict {
        ContaminationVerdict::Contaminated => {
            return RegressionVerdict::SkippedContaminated {
                reason: entry
                    .exclusion_reason
                    .clone()
                    .unwrap_or_else(|| format!("run {} is marked contaminated", entry.run_id)),
            };
        }
        ContaminationVerdict::Uncalibrated => {
            return RegressionVerdict::SkippedUncalibrated {
                reason: entry.exclusion_reason.clone().unwrap_or_else(|| {
                    "no calibrated contamination thresholds exist yet (D-17)".to_string()
                }),
            };
        }
        ContaminationVerdict::Clean => {}
    }

    let Some(baseline_entry) = baseline.entries.iter().find(|candidate| {
        candidate.rig_slug == entry.rig_slug
            && candidate.run_class == entry.run_class
            && candidate.stage == entry.stage
    }) else {
        return RegressionVerdict::NoComparableBaseline {
            stage: entry.stage.clone(),
            run_class: entry.run_class.clone(),
            rig_slug: entry.rig_slug.clone(),
        };
    };

    let p99_threshold_us = threshold_us(baseline_entry.p99_us, &baseline.thresholds.p99);
    let max_threshold_us = threshold_us(baseline_entry.max_us, &baseline.thresholds.max);

    let mut failures = Vec::new();
    if entry.p99_us > p99_threshold_us {
        failures.push(MetricRegression {
            metric: RegressedMetric::P99,
            observed_us: entry.p99_us,
            threshold_us: p99_threshold_us,
        });
    }
    if entry.max_us > max_threshold_us {
        failures.push(MetricRegression {
            metric: RegressedMetric::Max,
            observed_us: entry.max_us,
            threshold_us: max_threshold_us,
        });
    }

    if failures.is_empty() {
        RegressionVerdict::Pass {
            p99_headroom_us: p99_threshold_us as i64 - entry.p99_us as i64,
            max_headroom_us: max_threshold_us as i64 - entry.max_us as i64,
        }
    } else {
        RegressionVerdict::Fail { failures }
    }
}

/// `max(baseline * (1 + relative_pct / 100), baseline + absolute_us)`, with the relative term
/// computed in integer microseconds and rounded down, so the boundary is deterministic and a
/// reviewer can reproduce it by hand.
fn threshold_us(baseline_us: u64, threshold: &Threshold) -> u64 {
    let relative_term_us = (baseline_us as f64 * threshold.relative_pct / 100.0).floor() as u64;
    let relative_threshold_us = baseline_us + relative_term_us;
    let absolute_threshold_us = baseline_us + threshold.absolute_us;
    relative_threshold_us.max(absolute_threshold_us)
}
