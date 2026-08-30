//! Generates the JSON Schema for [`crate::fields::RunManifest`] from the Rust types
//! (RESEARCH.md pattern 3: schema-generated, not schema-written), so D-04's
//! cross-phase contract cannot drift from the types Phase 2 and Phase 3 write against.

use crate::fields::RunManifest;

/// Pretty-printed JSON Schema for [`RunManifest`], with a trailing newline. Compared
/// byte-for-byte against the committed `schemas/manifest.schema.json` by the
/// `schema_up_to_date` test.
pub fn manifest_schema_json() -> String {
    let schema = schemars::schema_for!(RunManifest);
    let mut json =
        serde_json::to_string_pretty(&schema).expect("a generated schema always serializes");
    json.push('\n');
    json
}
