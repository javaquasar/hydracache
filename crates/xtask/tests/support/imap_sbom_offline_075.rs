use super::*;

#[test]
fn offline_schema_retrieval_has_a_closed_catalog_and_never_fetches_unknown_uris() {
    use jsonschema::Retrieve;
    for uri in [
        "http://cyclonedx.org/schema/spdx.schema.json",
        "https://cyclonedx.org/schema/spdx.schema.json",
        "http://cyclonedx.org/schema/jsf-0.82.schema.json",
        "https://cyclonedx.org/schema/jsf-0.82.schema.json",
    ] {
        let uri = jsonschema::Uri::parse(uri.to_owned()).unwrap();
        assert!(OfflineSchemas.retrieve(&uri).unwrap().is_object());
    }
    for uri in [
        "https://example.invalid/foreign.json",
        "file:///foreign.json",
    ] {
        let uri = jsonschema::Uri::parse(uri.to_owned()).unwrap();
        assert_eq!(
            OfflineSchemas.retrieve(&uri).unwrap_err().to_string(),
            "unregistered offline schema reference"
        );
    }
}

#[test]
fn missing_root_components_refuse_before_attempting_to_read_any_product_binary() {
    let root = tempfile::tempdir().unwrap();
    assert!(current_roots(root.path(), &BTreeMap::new())
        .unwrap_err()
        .to_string()
        .contains("ambiguous/missing artifact component hydracache-server"));
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
}
