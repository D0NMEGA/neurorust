//! Metrics series, baseline comparison, and index rendering for neurorust.
//!
//! Owns the BENCH-08 series types, D-11 baseline comparison, D-08 coverage record, the
//! D-22 PLAT-03 decomposition, and BENCH-06 index rendering.

pub mod baseline;
pub mod coverage;
pub mod index;
pub mod report;
pub mod schema;
pub mod series;

/// Renders any `#[serde(rename_all = "kebab-case")]` enum value as the same kebab-case string
/// its JSON form uses, e.g. `ContaminationVerdict::Contaminated` -> `"contaminated"`. Shared by
/// [`report`] and [`index`], so a published report and the index it feeds always agree with the
/// manifest's own on-disk vocabulary.
pub(crate) fn kebab<T: serde::Serialize>(value: &T) -> String {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(s)) => s,
        other => format!("{other:?}"),
    }
}
