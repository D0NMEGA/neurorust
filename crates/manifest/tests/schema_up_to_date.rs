//! The committed schema is generated, never hand-authored (RESEARCH.md pattern 3).
//! CI runs this without `UPDATE_SCHEMAS` set, so a drifted schema fails the build
//! rather than silently diverging from the types Phase 2 and Phase 3 write against.

#[test]
fn schema_up_to_date() {
    let generated = nr_manifest::schema::manifest_schema_json();
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../schemas/manifest.schema.json"
    );

    if std::env::var("UPDATE_SCHEMAS").is_ok() {
        std::fs::write(path, &generated).unwrap();
    }

    let committed = std::fs::read_to_string(path).unwrap();
    assert_eq!(
        generated, committed,
        "schemas/manifest.schema.json is stale. Regenerate with: \
         UPDATE_SCHEMAS=1 cargo test -p nr-manifest schema_up_to_date"
    );
}
