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

#[cfg(test)]
mod tests {
    use super::*;
    use nr_manifest::{ContaminationVerdict, ProvenanceTier};

    /// BENCH-06: a failed attempt (task 1's `AttemptRecord`, no manifest) gets a
    /// row here too, with `unavailable` in both numeric columns, `no` in the in
    /// series column, and the failure stage and message in the reason column.
    /// Finding 7 of `01-EXTERNAL-AUDIT.md`.
    #[test]
    fn index_lists_a_failed_attempt() {
        let failed = RunSummary {
            date: "2026-09-06".to_string(),
            run_id: "2026-09-06-precision3591-recon".to_string(),
            run_class: RunClass::Recon,
            instrument_class: InstrumentClass::HeadlineSeries,
            provenance_tier: ProvenanceTier::HarnessGenerated,
            verdict: None,
            p99_us: None,
            max_us: None,
            in_series: false,
            reason: Some(
                "attempt failed at parse: failed to parse cyclictest's .hist output".to_string(),
            ),
            outcome: RunOutcome::FailedAttempt,
        };

        let index = render_index(&[failed]);
        let row = index
            .lines()
            .find(|line| line.contains("2026-09-06-precision3591-recon"))
            .expect("a row for the failed attempt");
        assert!(row.contains("unavailable"), "row: {row}");
        assert!(row.contains("| no |"), "row: {row}");
        assert!(row.contains("attempt failed at parse"), "row: {row}");

        // A measured run's verdict cell is unaffected: still the real verdict, not
        // "unavailable".
        let measured = RunSummary {
            date: "2026-09-06".to_string(),
            run_id: "2026-09-06-precision3591-measured".to_string(),
            run_class: RunClass::Weekly,
            instrument_class: InstrumentClass::HeadlineSeries,
            provenance_tier: ProvenanceTier::HarnessGenerated,
            verdict: Some(ContaminationVerdict::Clean),
            p99_us: Some(9),
            max_us: Some(30),
            in_series: true,
            reason: None,
            outcome: RunOutcome::Measured,
        };
        let index = render_index(&[measured]);
        let row = index
            .lines()
            .find(|line| line.contains("2026-09-06-precision3591-measured"))
            .expect("a row for the measured run");
        assert!(row.contains("clean"), "row: {row}");
        assert!(!row.contains("unavailable"), "row: {row}");
    }
}
