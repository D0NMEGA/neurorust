//! BENCH-06 index rendering: `measurements/INDEX.md`, generated from every run directory.
//!
//! There is no filter and no omission here. A run marked excluded from the series still gets a
//! row, with its reason in the reason column. This is how BENCH-06's "losing configurations
//! appear rather than being omitted" is operationalised in the published layout.

use nr_manifest::{ContaminationVerdict, InstrumentClass, ProvenanceTier, RunClass};

use crate::kebab;

/// One row of `measurements/INDEX.md`.
#[derive(Debug, Clone)]
pub struct RunSummary {
    /// Human-readable capture date, e.g. `"2026-08-31"`.
    pub date: String,
    pub run_id: String,
    pub run_class: RunClass,
    pub instrument_class: InstrumentClass,
    pub provenance_tier: ProvenanceTier,
    pub verdict: ContaminationVerdict,
    /// `None` when the run's histogram could not be parsed. Rendered as `unavailable`.
    /// Never substituted with zero: a substituted zero is indistinguishable from a run that
    /// really observed zero, and one of those is evidence while the other is a parse failure.
    /// Finding 6 of `01-EXTERNAL-AUDIT.md`.
    pub p99_us: Option<u64>,
    pub max_us: Option<u64>,
    /// Whether this run counts toward the regression series (the negation of the manifest's
    /// `excluded_from_series`).
    pub in_series: bool,
    /// The manifest's `exclusion_reason`, carried through unchanged when present.
    pub reason: Option<String>,
}

/// Renders `measurements/INDEX.md`. Every run directory gets a row here; a run excluded from
/// the series still appears, with `no` in the "in series" column and its reason in the "reason"
/// column (BENCH-06). There is no code path in this function that skips a summary.
pub fn render_index(summaries: &[RunSummary]) -> String {
    let mut out = String::new();
    out.push_str("# Measurement index\n\n");
    out.push_str(
        "Every run directory under `measurements/` has a row here, including a contaminated, \
         regressed, or refused run: BENCH-06 requires a losing configuration to be reported, \
         not omitted.\n\n",
    );
    out.push_str(
        "| date | run id | class | instrument class | provenance tier | verdict | p99 us | max us | in series | reason |\n",
    );
    out.push_str(
        "|------|--------|-------|-------------------|------------------|---------|--------|--------|-----------|--------|\n",
    );
    for summary in summaries {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            summary.date,
            summary.run_id,
            kebab(&summary.run_class),
            kebab(&summary.instrument_class),
            kebab(&summary.provenance_tier),
            kebab(&summary.verdict),
            render_optional_us(summary.p99_us),
            render_optional_us(summary.max_us),
            if summary.in_series { "yes" } else { "no" },
            summary.reason.as_deref().unwrap_or("-"),
        ));
    }
    out
}

/// Renders a `p99_us`/`max_us` value as its number, or the literal `unavailable` when the
/// run's histogram could not be parsed (never a substituted zero; see [`RunSummary::p99_us`]).
fn render_optional_us(value: Option<u64>) -> String {
    match value {
        Some(v) => v.to_string(),
        None => "unavailable".to_string(),
    }
}
