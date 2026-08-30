//! D-14/D-16 required-field enforcement: a manifest missing a required key must fail
//! to deserialize, and the provenance tier must have no default (T-1-03).

use nr_manifest::{AbsentField, ProvenanceTier, RunClass, RunManifest};

const FIXTURE: &str = include_str!("fixtures/minimal-manifest.json");

fn fixture_value() -> serde_json::Value {
    serde_json::from_str(FIXTURE).expect("fixture must be valid JSON")
}

fn remove_key_and_parse(key: &str) -> Result<RunManifest, serde_json::Error> {
    let mut value = fixture_value();
    value
        .as_object_mut()
        .expect("fixture is a JSON object")
        .remove(key);
    serde_json::from_value(value)
}

#[test]
fn fixture_parses_as_valid_manifest() {
    let manifest: RunManifest =
        serde_json::from_value(fixture_value()).expect("the complete fixture must deserialize");
    assert_eq!(manifest.schema_version, 1);
}

#[test]
fn required_fields_host_missing() {
    let err = remove_key_and_parse("host").expect_err("removing `host` must fail to deserialize");
    assert!(
        err.to_string().contains("host"),
        "error should name the missing field, got: {err}"
    );
}

#[test]
fn required_fields_preconditions_missing() {
    let err = remove_key_and_parse("preconditions")
        .expect_err("removing `preconditions` must fail to deserialize");
    assert!(
        err.to_string().contains("preconditions"),
        "error should name the missing field, got: {err}"
    );
}

#[test]
fn required_fields_artifacts_missing() {
    let err = remove_key_and_parse("artifacts")
        .expect_err("removing `artifacts` must fail to deserialize");
    assert!(
        err.to_string().contains("artifacts"),
        "error should name the missing field, got: {err}"
    );
}

#[test]
fn provenance_tier_required() {
    let err = remove_key_and_parse("provenance_tier")
        .expect_err("removing `provenance_tier` must fail: there is no default");
    assert!(
        err.to_string().contains("provenance_tier"),
        "error should name the missing field, got: {err}"
    );

    let value =
        serde_json::to_value(ProvenanceTier::Reconstructed).expect("ProvenanceTier must serialize");
    assert_eq!(
        value,
        serde_json::Value::String("reconstructed".to_string())
    );
}

#[test]
fn absent_field_roundtrip() {
    let original = AbsentField {
        field_path: "power.battery_percent".to_string(),
        reason: "not recorded in RIG.txt".to_string(),
    };
    let json = serde_json::to_string(&original).expect("AbsentField must serialize");
    let restored: AbsentField = serde_json::from_str(&json).expect("AbsentField must deserialize");
    assert_eq!(original, restored);
}

#[test]
fn run_class_roundtrip() {
    let cases = [
        (RunClass::Recon, "recon"),
        (RunClass::Screen, "screen"),
        (RunClass::CalibrationClean, "calibration-clean"),
        (
            RunClass::CalibrationContaminated,
            "calibration-contaminated",
        ),
        (RunClass::Investigation, "investigation"),
        (RunClass::Headline, "headline"),
        (RunClass::Weekly, "weekly"),
        (RunClass::Soak, "soak"),
    ];

    for (variant, expected) in cases {
        let json = serde_json::to_string(&variant).expect("RunClass must serialize");
        assert_eq!(json, format!("\"{expected}\""));

        let restored: RunClass =
            serde_json::from_str(&json).expect("RunClass must round trip through JSON");
        assert_eq!(restored, variant);
    }
}
