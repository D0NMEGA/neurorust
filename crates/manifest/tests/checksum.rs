//! blake3 checksum and `validate::validate` behavior (D-12, D-13, ASVS V12).

use std::path::PathBuf;

use nr_manifest::checksum::{blake3_file, verify_artifact};
use nr_manifest::fields::{
    AdmissionDisposition, AdmissionEvidence, AdmissionEvidenceSource, ArtifactKind, ArtifactRecord,
    RunManifest, SeriesAdmission, StorageLocation,
};
use nr_manifest::validate::{ValidationError, validate};

const FIXTURE_JSON: &str = include_str!("fixtures/minimal-manifest.json");

/// Hard-coded rather than computed: catches the fixture being silently modified.
const KNOWN_HIST_BLAKE3: &str = "495c45891c866a627334e74d810a5377be684beb6d8e8a17b7853f8c4cb9c88f";

fn histogram_fixtures_dir() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../histogram/tests/fixtures"
    ))
}

fn hist_artifact() -> ArtifactRecord {
    ArtifactRecord {
        path: "cyclictest-rt-isolated-idle-10m.hist".to_string(),
        bytes: 23399,
        blake3: KNOWN_HIST_BLAKE3.to_string(),
        kind: ArtifactKind::CyclictestHist,
        stored: StorageLocation::InRepo,
    }
}

fn fixture_manifest() -> RunManifest {
    serde_json::from_str(FIXTURE_JSON).expect("fixture must deserialize into RunManifest")
}

#[test]
fn checksum_matches_known_value() {
    let path = histogram_fixtures_dir().join("cyclictest-rt-isolated-idle-10m.hist");

    let first = blake3_file(&path).expect("fixture file must be readable");
    assert_eq!(first, KNOWN_HIST_BLAKE3);

    let second = blake3_file(&path).expect("recomputing must succeed");
    assert_eq!(first, second, "blake3 of the same file must be stable");
}

#[test]
fn checksum_mismatch_rejected() {
    let mut artifact = hist_artifact();
    // Alter one character of an otherwise-correct digest.
    artifact.blake3 = format!("f{}", &artifact.blake3[1..]);

    let result = verify_artifact(&histogram_fixtures_dir(), &artifact);
    match result {
        Err(ValidationError::ChecksumMismatch { path, .. }) => {
            assert_eq!(path, "cyclictest-rt-isolated-idle-10m.hist");
        }
        other => panic!("expected ChecksumMismatch, got {other:?}"),
    }
}

#[test]
fn checksum_missing_file_rejected() {
    let mut artifact = hist_artifact();
    artifact.path = "does-not-exist.hist".to_string();

    let result = verify_artifact(&histogram_fixtures_dir(), &artifact);
    assert!(
        matches!(result, Err(ValidationError::ArtifactMissing { .. })),
        "expected ArtifactMissing, got {result:?}"
    );
}

#[test]
fn path_traversal_rejected() {
    for bad_path in ["../secrets", "/etc/passwd"] {
        let mut artifact = hist_artifact();
        artifact.path = bad_path.to_string();

        let result = verify_artifact(&histogram_fixtures_dir(), &artifact);
        assert!(
            matches!(result, Err(ValidationError::UnsafeArtifactPath { .. })),
            "expected UnsafeArtifactPath for {bad_path}, got {result:?}"
        );
    }
}

#[test]
fn run_id_charset_rejected() {
    let mut manifest = fixture_manifest();
    manifest.run_id = "Bad Id/../x".to_string();

    let result = validate(&histogram_fixtures_dir(), &manifest);
    let errors = result.expect_err("a bad run_id must fail validation");
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, ValidationError::InvalidRunId { .. })),
        "expected InvalidRunId among: {errors:?}"
    );
}

#[test]
fn exclusion_reason_required() {
    let mut manifest = fixture_manifest();
    manifest.excluded_from_series = true;
    manifest.exclusion_reason = None;

    let result = validate(&histogram_fixtures_dir(), &manifest);
    let errors = result.expect_err("a missing exclusion reason must fail validation");
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, ValidationError::MissingExclusionReason)),
        "expected MissingExclusionReason among: {errors:?}"
    );
}

fn clean_admission() -> SeriesAdmission {
    SeriesAdmission {
        admitted: true,
        exclusions: vec![],
        evidence: vec![AdmissionEvidence {
            source: AdmissionEvidenceSource::Preconditions,
            observed: "14 pass, 0 fail, 0 not-applicable, 0 unavailable".to_string(),
            disposition: AdmissionDisposition::Clean,
        }],
    }
}

/// D-28: a manifest whose `series_admission.admitted` disagrees with `excluded_from_series`
/// fails validation, in both directions.
#[test]
fn validate_rejects_admission_disagreeing_with_exclusion() {
    let mut manifest = fixture_manifest();
    manifest.excluded_from_series = false;
    manifest.exclusion_reason = None;
    manifest.series_admission = Some(SeriesAdmission {
        admitted: true,
        ..clean_admission()
    });
    // excluded_from_series (false) already agrees with admitted (true); force disagreement.
    manifest.series_admission.as_mut().unwrap().admitted = false;

    let result = validate(&histogram_fixtures_dir(), &manifest);
    let errors = result.expect_err("admitted=false with excluded_from_series=false must fail");
    assert!(
        errors.iter().any(|e| matches!(
            e,
            ValidationError::AdmissionDisagreesWithExclusion {
                admitted: false,
                excluded: false
            }
        )),
        "expected AdmissionDisagreesWithExclusion among: {errors:?}"
    );

    // The mirror case: admitted=true with excluded_from_series=true.
    let mut manifest = fixture_manifest();
    manifest.excluded_from_series = true;
    manifest.exclusion_reason = Some("some reason".to_string());
    manifest.series_admission = Some(clean_admission());

    let result = validate(&histogram_fixtures_dir(), &manifest);
    let errors = result.expect_err("admitted=true with excluded_from_series=true must fail");
    assert!(
        errors.iter().any(|e| matches!(
            e,
            ValidationError::AdmissionDisagreesWithExclusion {
                admitted: true,
                excluded: true
            }
        )),
        "expected AdmissionDisagreesWithExclusion among: {errors:?}"
    );
}

/// D-28: an admitted run may not carry exclusions; `admitted` and a non-empty `exclusions`
/// list are mutually exclusive by construction, not merely by convention.
#[test]
fn validate_rejects_admitted_run_carrying_exclusions() {
    let mut manifest = fixture_manifest();
    manifest.excluded_from_series = false;
    manifest.exclusion_reason = None;
    manifest.series_admission = Some(SeriesAdmission {
        admitted: true,
        exclusions: vec!["fixture use: NRMEASURE_FACTS_FIXTURE".to_string()],
        ..clean_admission()
    });

    let result = validate(&histogram_fixtures_dir(), &manifest);
    let errors = result.expect_err("admitted=true with non-empty exclusions must fail");
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, ValidationError::AdmittedRunCarriesExclusions(_))),
        "expected AdmittedRunCarriesExclusions among: {errors:?}"
    );
}
