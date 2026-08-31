//! The D-08 coverage record: one entry per ISO week, either a run reference or a gap with a
//! reason. A missed week is recorded as an explicit gap and is never backfilled: this module
//! offers no function that can synthesise a metric value for a week with no run. The only thing
//! [`record_gap`] and [`record_gaps`] can ever write is a [`WeekRecord::Gap`], and that variant
//! carries a reason and no metric field at all, so backfilling is not just discouraged, it is
//! not representable.

use serde::{Deserialize, Serialize};
use time::{Date, Weekday};

use crate::series::StageMetrics;

/// Current version of this schema.
pub const SCHEMA_VERSION: u32 = 1;

/// The D-08 coverage record.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Coverage {
    /// Must equal [`SCHEMA_VERSION`].
    pub schema_version: u32,
    /// Ordered by ISO week.
    pub weeks: Vec<WeekRecord>,
}

impl Coverage {
    /// An empty coverage record.
    pub fn new() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            weeks: Vec::new(),
        }
    }
}

impl Default for Coverage {
    fn default() -> Self {
        Self::new()
    }
}

/// One ISO week's coverage: either the runs recorded that week, or a gap with a reason.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum WeekRecord {
    Run {
        iso_week: String,
        run_ids: Vec<String>,
    },
    Gap {
        iso_week: String,
        reason: GapReason,
    },
}

impl WeekRecord {
    fn iso_week(&self) -> &str {
        match self {
            WeekRecord::Run { iso_week, .. } | WeekRecord::Gap { iso_week, .. } => iso_week,
        }
    }
}

/// Why a week has no recorded run (D-08). A missed week is honest information about the rig's
/// real schedule, not a defect to paper over with a backfilled catch-up run.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum GapReason {
    /// The machine was off, or the weekly timer did not fire.
    NoRunRecorded,
    /// The harness refused the run on a D-06 precondition violation. D-08 says this records the
    /// same gap a missed week does.
    RefusedOnPrecondition {
        check: String,
    },
    RunFailed {
        detail: String,
    },
}

/// Records a single week's gap, with `reason` naming why. A generic building block: this crate's
/// own [`record_gaps`] uses it in a loop for [`GapReason::NoRunRecorded`], and a caller that
/// catches a D-06 refusal (`nr-capture`'s `preconditions::RefusalError`) calls it directly with
/// [`GapReason::RefusedOnPrecondition`], per D-08's rule that a refused run produces the same
/// recorded gap a missed week does.
pub fn record_gap(coverage: &mut Coverage, iso_week: impl Into<String>, reason: GapReason) {
    coverage.weeks.push(WeekRecord::Gap {
        iso_week: iso_week.into(),
        reason,
    });
}

/// Fills every ISO week strictly between `coverage`'s last recorded week and `up_to`'s week with
/// a [`GapReason::NoRunRecorded`] gap, and returns the list of week labels it filled.
///
/// Never synthesises a metric value for the weeks it fills: every entry `record_gaps` writes is
/// a [`WeekRecord::Gap`], which has no field capable of holding one. Does nothing if `coverage`
/// has no prior recorded week, since there is then no earlier week to measure a gap from, and
/// does nothing for `up_to`'s own week, which a caller records separately (as a `Run` entry, not
/// a gap) once the run itself is appended to the series.
pub fn record_gaps(coverage: &mut Coverage, up_to: &StageMetrics) -> Vec<String> {
    let Some((mut year, mut week)) = coverage
        .weeks
        .last()
        .and_then(|last| parse_iso_week(last.iso_week()))
    else {
        return Vec::new();
    };
    let Some(target) = parse_iso_week(&up_to.iso_week) else {
        return Vec::new();
    };

    let mut filled = Vec::new();
    loop {
        let next = next_iso_week(year, week);
        if next >= target {
            break;
        }
        (year, week) = next;
        let label = format_iso_week(year, week);
        record_gap(coverage, label.clone(), GapReason::NoRunRecorded);
        filled.push(label);
    }
    filled
}

/// Parses `"2026-W35"` into `(2026, 35)`.
fn parse_iso_week(label: &str) -> Option<(i32, u8)> {
    let (year, week) = label.split_once("-W")?;
    Some((year.parse().ok()?, week.parse().ok()?))
}

/// Formats `(2026, 35)` as `"2026-W35"`.
fn format_iso_week(year: i32, week: u8) -> String {
    format!("{year}-W{week:02}")
}

/// The next ISO week after `(year, week)`, correctly rolling over into the next ISO
/// week-numbering year (which starts at week 1, or in a 53-week year, wraps at week 53) rather
/// than assuming every year has exactly 52 weeks.
fn next_iso_week(year: i32, week: u8) -> (i32, u8) {
    let date = Date::from_iso_week_date(year, week, Weekday::Monday)
        .expect("an ISO week produced by this module is always valid");
    let next_date = date
        .checked_add(time::Duration::days(7))
        .expect("adding 7 days to a valid calendar date does not overflow");
    let (next_year, next_week, _) = next_date.to_iso_week_date();
    (next_year, next_week)
}
