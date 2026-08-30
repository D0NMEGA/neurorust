//! blake3 checksum and `validate::validate` behavior (D-12, D-13, ASVS V12).

use std::path::PathBuf;

use nr_manifest::checksum::{blake3_file, verify_artifact};
use nr_manifest::fields::{ArtifactKind, ArtifactRecord, RunManifest, StorageLocation};
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
