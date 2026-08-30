//! D-16 provenance tier and explicit field absence.
//!
//! `ProvenanceTier` distinguishes a manifest the harness wrote while the run happened
//! from one rebuilt afterward from other evidence. It is a required field on
//! [`crate::fields::RunManifest`], never an `Option` and never `#[serde(default)]`, so a
//! reconstructed manifest can never be silently mistaken for a harness-generated one.
//! RESEARCH.md pitfall 6 names the trap this avoids: defaulting the tier field.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Whether a manifest was written by the harness while the run happened, or rebuilt
/// afterward from other records (D-16).
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ProvenanceTier {
    /// The harness wrote this manifest at capture time; the full contract was
    /// populated by code that was present when the run happened.
    HarnessGenerated,
    /// Rebuilt after the fact from other evidence (e.g. `RIG.txt`, a README). Every
    /// field that could not be recovered is listed in `RunManifest::absent_fields`,
    /// never guessed.
    Reconstructed,
}

/// A single field a reconstructed manifest could not recover (D-16).
///
/// Recording an explicit absence, rather than guessing a value or silently omitting
/// the field, is what keeps a reconstructed manifest honest about what it actually
/// knows.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AbsentField {
    /// Dotted path into `RunManifest`, e.g. `"power.battery_percent"`.
    pub field_path: String,
    /// Why the field could not be recovered, e.g. `"not recorded in RIG.txt"`.
    pub reason: String,
}
