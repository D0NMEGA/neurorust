//! The committed attempt schema is generated, never hand-authored, exactly like
//! `schemas/manifest.schema.json` (see `schema_up_to_date.rs`). CI runs this without
//! `UPDATE_SCHEMAS` set, so a drifted schema fails the build rather than silently
//! diverging from `AttemptRecord`.

#[test]
fn attempt_schema_up_to_date() {
    let generated = nr_manifest::schema::attempt_schema_json();
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../schemas/attempt.schema.json"
    );

    if std::env::var("UPDATE_SCHEMAS").is_ok() {
        std::fs::write(path, &generated).unwrap();
    }

    let committed = std::fs::read_to_string(path).unwrap();
    assert_eq!(
        generated, committed,
        "schemas/attempt.schema.json is stale. Regenerate with: \
         UPDATE_SCHEMAS=1 cargo test -p nr-manifest attempt_schema_up_to_date"
    );
}
