//! Structural, path-safety, and cross-field validation for a `RunManifest` (D-13).
//!
//! `validate` collects every failure rather than stopping at the first, so a single
//! `nrmeasure verify` invocation reports every problem in one pass.

use std::path::Path;

use thiserror::Error;

use crate::checksum::verify_artifact;
use crate::fields::{RunManifest, SCHEMA_VERSION};
use crate::provenance::ProvenanceTier;

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("run_id {run_id:?} does not match ^[a-z0-9][a-z0-9-]{{0,63}}$")]
    InvalidRunId { run_id: String },
    #[error("artifact path {path:?} is unsafe: absolute or contains a parent-directory component")]
    UnsafeArtifactPath { path: String },
    #[error("artifact {path:?} does not exist under the run directory")]
    ArtifactMissing { path: String },
    #[error("checksum mismatch for {path:?}: recorded {recorded}, computed {computed}")]
    ChecksumMismatch {
        path: String,
        recorded: String,
        computed: String,
    },
    #[error("manifest has no artifacts")]
    EmptyArtifacts,
    #[error("harness-generated manifest has no precondition results")]
    EmptyPreconditions,
    #[error("excluded_from_series is true but exclusion_reason is missing or empty")]
    MissingExclusionReason,
    #[error(
        "series_admission.admitted is {admitted} but excluded_from_series is {excluded}: a \
         manifest may not summarise its own admission record incorrectly"
    )]
    AdmissionDisagreesWithExclusion { admitted: bool, excluded: bool },
    #[error("series_admission.admitted is true but exclusions is non-empty: {0:?}")]
    AdmittedRunCarriesExclusions(Vec<String>),
    #[error(
        "reconstructed manifest has empty preconditions but does not record it as an absent field"
    )]
    ReconstructedWithoutAbsentFields,
    #[error("schema_version {found} is not supported (supported: {supported})")]
    SchemaVersionUnsupported { found: u32, supported: u32 },
}

/// Validates `manifest` against `run_dir`, collecting every failure rather than
/// short-circuiting on the first so a caller can report all problems in one pass.
pub fn validate(run_dir: &Path, manifest: &RunManifest) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();

    if !is_valid_run_id(&manifest.run_id) {
        errors.push(ValidationError::InvalidRunId {
            run_id: manifest.run_id.clone(),
        });
    }

    if manifest.artifacts.is_empty() {
        errors.push(ValidationError::EmptyArtifacts);
    }
    for artifact in &manifest.artifacts {
        if let Err(err) = verify_artifact(run_dir, artifact) {
            errors.push(err);
        }
    }

    match manifest.provenance_tier {
        ProvenanceTier::HarnessGenerated => {
            if manifest.preconditions.is_empty() {
                errors.push(ValidationError::EmptyPreconditions);
            }
        }
        ProvenanceTier::Reconstructed => {
            let preconditions_explained = manifest
                .absent_fields
                .iter()
                .any(|absent| absent.field_path == "preconditions");
            if manifest.preconditions.is_empty() && !preconditions_explained {
                errors.push(ValidationError::ReconstructedWithoutAbsentFields);
            }
        }
    }

    if manifest.excluded_from_series {
        let reason_present = manifest
            .exclusion_reason
            .as_deref()
            .is_some_and(|reason| !reason.trim().is_empty());
        if !reason_present {
            errors.push(ValidationError::MissingExclusionReason);
        }
    }

    // Runs only when `series_admission` is `Some`, so every manifest committed before D-28
    // (all twelve under `measurements/` today) is unaffected.
    if let Some(admission) = &manifest.series_admission {
        if admission.admitted == manifest.excluded_from_series {
            errors.push(ValidationError::AdmissionDisagreesWithExclusion {
                admitted: admission.admitted,
                excluded: manifest.excluded_from_series,
            });
        }
        if admission.admitted && !admission.exclusions.is_empty() {
            errors.push(ValidationError::AdmittedRunCarriesExclusions(
                admission.exclusions.clone(),
            ));
        }
    }

    if manifest.schema_version != SCHEMA_VERSION {
        errors.push(ValidationError::SchemaVersionUnsupported {
            found: manifest.schema_version,
            supported: SCHEMA_VERSION,
        });
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Checks `run_id` against `^[a-z0-9][a-z0-9-]{0,63}$` by hand; a directory name
/// derived from user-controlled input is exactly the traversal surface ASVS V12
/// warns about, and a regex dependency is unnecessary for a charset this small.
fn is_valid_run_id(run_id: &str) -> bool {
    let mut chars = run_id.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_lowercase() || first.is_ascii_digit()) {
        return false;
    }
    if chars.clone().count() > 63 {
        return false;
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}
