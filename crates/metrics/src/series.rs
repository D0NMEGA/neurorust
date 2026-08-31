//! The BENCH-08 metrics series: one entry per run per stage.
//!
//! `stage` is an open string rather than a closed enum (D-04), because Phase 2's STOP-07 and
//! Phase 3's SUBS-06 write their own stage names into this same contract. Percentiles always
//! come from [`nr_histogram::percentiles::Percentiles`]'s overflow-inclusive path; this module
//! never reaches for the sibling, overflow-excluding computation, because the numbers here are
//! what the project publishes.
//!
//! [`append`] is the only mutation this module offers. There is no edit and no delete: a run
//! that is contaminated, regressed, or refused still gets its entry (BENCH-06), because a bad
//! run is evidence, not something to withhold. The one and only rejection is a duplicate
//! `run_id`, because a run is recorded once.

use nr_manifest::{ContaminationVerdict, InstrumentClass, RunClass};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;

/// Current version of this schema. Bump on any breaking change to [`MetricsSeries`].
pub const SCHEMA_VERSION: u32 = 1;

/// The BENCH-08 series: a weekly committed JSON carrying p50, p95 and p99 for every stage
/// instrumented so far.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MetricsSeries {
    /// Must equal [`SCHEMA_VERSION`].
    pub schema_version: u32,
    /// Append only, ordered by `utc_start`. See [`append`].
    pub entries: Vec<StageMetrics>,
}

/// One run's measurement of one stage. `stage` is open by design (D-04):
/// `"cyclictest.wakeup_latency"` and `"hwlatdetect.firmware_gap"` today,
/// `"emergency_stop.abort_latency"` from Phase 2's STOP-07, and
/// `"sched_deadline.wakeup_latency"` / `"sched_fifo.wakeup_latency"` from Phase 3's SUBS-06.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StageMetrics {
    /// Matches the manifest `run_id`, so the series points at its evidence.
    pub run_id: String,
    pub run_class: RunClass,
    pub instrument_class: InstrumentClass,
    #[serde(with = "time::serde::rfc3339")]
    #[schemars(with = "String")]
    pub utc_start: OffsetDateTime,
    /// The D-08 coverage key, e.g. `"2026-W35"`.
    pub iso_week: String,
    pub rig_slug: String,
    /// e.g. `"cyclictest"`, `"hwlatdetect"`.
    pub tool: String,
    /// e.g. `"cyclictest.wakeup_latency"`. Open by design (D-04); see the module doc comment.
    pub stage: String,
    /// Binned samples plus overflows.
    pub sample_count: u64,
    pub overflow_count: u64,
    pub p50_us: u64,
    pub p95_us: u64,
    pub p99_us: u64,
    pub p999_us: u64,
    /// Exact worst case, never the overflow bound.
    pub max_us: u64,
    pub contamination_verdict: ContaminationVerdict,
    /// BENCH-06: a contaminated or otherwise non-headline run is still recorded here, but
    /// excluded from the regression series. See [`Self::exclusion_reason`].
    pub excluded_from_series: bool,
    /// Required to be `Some` and non-empty when `excluded_from_series` is true.
    pub exclusion_reason: Option<String>,
    /// blake3 of the manifest this entry's numbers were derived from, so a published figure can
    /// always be traced back to the capture and environment snapshot behind it.
    pub manifest_blake3: String,
}

/// Errors appending to a [`MetricsSeries`].
#[derive(Debug, Error, PartialEq, Eq)]
pub enum SeriesError {
    /// A run is recorded once. `append` rejects a duplicate `run_id` rather than writing a
    /// second copy.
    #[error("run_id {0:?} already exists in the series")]
    DuplicateRunId(String),
}

/// Appends `entry` to `series`, keeping `entries` sorted by `utc_start`.
///
/// This is the only mutation `MetricsSeries` offers: there is no edit and no delete. Nothing
/// here inspects `contamination_verdict` or `excluded_from_series` to decide whether to keep an
/// entry, because BENCH-06 and D-11 both require that a contaminated, regressed, or refused run
/// is still reported, not dropped. The one rejection is a duplicate `run_id`.
pub fn append(series: &mut MetricsSeries, entry: StageMetrics) -> Result<(), SeriesError> {
    if series
        .entries
        .iter()
        .any(|existing| existing.run_id == entry.run_id)
    {
        return Err(SeriesError::DuplicateRunId(entry.run_id));
    }
    let insert_at = series
        .entries
        .partition_point(|existing| existing.utc_start <= entry.utc_start);
    series.entries.insert(insert_at, entry);
    Ok(())
}
