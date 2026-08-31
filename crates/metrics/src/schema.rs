//! Generates the JSON Schema for [`crate::series::MetricsSeries`] from the Rust types
//! (RESEARCH.md pattern 3: schema-generated, not schema-written). This is the BENCH-08 contract
//! Phase 2's STOP-07 and Phase 3's SUBS-06 write their own stage entries into (D-04), so it must
//! never drift silently from the types those plans build against.

use crate::series::MetricsSeries;

/// Pretty-printed JSON Schema for [`MetricsSeries`], with a trailing newline. Compared
/// byte-for-byte against the committed `schemas/metrics.schema.json` by the
/// `metrics_schema_up_to_date` test.
pub fn metrics_schema_json() -> String {
    let schema = schemars::schema_for!(MetricsSeries);
    let mut json =
        serde_json::to_string_pretty(&schema).expect("a generated schema always serializes");
    json.push('\n');
    json
}
