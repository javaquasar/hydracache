use hydracache_cluster_testkit::value_plane_capability_075::{
    CapabilityDescriptor, LocalCapabilityDescriptor, SurfaceSupport,
};
use serde_json::{json, Value};

const OPERATIONS: &str = include_str!("../../../docs/testing/imap/0.75/operation-contract.json");
const BOUNDS: &str = include_str!("../../../docs/testing/imap/0.75/resource-bounds.json");
const SURFACES: &str = include_str!("../../../docs/testing/imap/0.75/surface-equivalence.json");
const STATUS: &str = include_str!("../../../docs/testing/imap/0.75/status.json");

#[test]
fn actual_capabilities_keep_reference_owners_and_existing_sdk_separate() {
    let bounded = LocalCapabilityDescriptor::bounded();
    let unified = LocalCapabilityDescriptor::unified();
    let sdk = LocalCapabilityDescriptor::sdk_actual();
    assert_eq!(bounded.operations.len(), 21);
    assert_eq!(unified.operations.len(), 40);
    assert_eq!(sdk.operations.len(), 11);
    assert_eq!(unified.interface_features.len(), 8);
    assert!(unified
        .interface_features
        .contains("bounded_batch_entry_views"));
    assert!(bounded.interface_features.is_empty());
    assert!(sdk.interface_features.is_empty());
    assert!(bounded.operations.is_subset(&unified.operations));
    assert!(!sdk.operations.contains("get_all"));
    for descriptor in [bounded, unified, sdk] {
        assert!(!descriptor.production_enabled);
    }
}

#[test]
fn capability_descriptor_matches_provisional_contract_and_bounds() {
    let operations: Value = serde_json::from_str(OPERATIONS).unwrap();
    for surface in ["resp", "hc1", "hc2-rust", "hc2-java", "hazelcast-facade"] {
        let descriptor = CapabilityDescriptor::for_surface(surface).unwrap();
        assert!(!descriptor.production_enabled);
        assert_eq!(
            descriptor.operations.len(),
            operations["operations"].as_array().unwrap().len()
        );
        assert!(descriptor.interface_features.contains("rust_async_facade"));
        assert!(descriptor.interface_features.contains("java_async_facade"));
        assert!(descriptor
            .interface_features
            .contains("cursor_iterator_adapters"));
        assert!(descriptor
            .unsupported_families
            .contains("remote_entry_processor"));
        let timed = descriptor.operation("try_put").unwrap();
        assert!(timed.outcomes.contains(&"outcome_unknown".to_owned()));
        assert_eq!(timed.bounds["key_bytes"].maximum, 1_048_576);
        let support = &descriptor
            .operation("replace_all_detailed")
            .unwrap()
            .support;
        if matches!(surface, "resp" | "hc1") {
            assert!(
                matches!(support, SurfaceSupport::Unsupported { reason } if !reason.is_empty())
            );
        } else {
            assert_eq!(support, &SurfaceSupport::Planned);
        }
        assert!(descriptor.operation("execute_on_key").is_none());
    }
    assert!(CapabilityDescriptor::for_surface("unknown").is_err());
}

#[test]
fn capability_descriptor_rejects_incomplete_ambiguous_or_enabled_contracts() {
    let baseline: [Value; 4] =
        [OPERATIONS, BOUNDS, SURFACES, STATUS].map(|source| serde_json::from_str(source).unwrap());
    let check = |documents: &[Value; 4]| {
        CapabilityDescriptor::from_contracts(
            "hc2-rust",
            &documents[0].to_string(),
            &documents[1].to_string(),
            &documents[2].to_string(),
            &documents[3].to_string(),
        )
    };
    let edits: &[fn(&mut [Value; 4])] = &[
        |d| d[0]["status"] = json!("finalized"),
        |d| d[0]["api_extensions"]["production_enabled"] = json!(true),
        |d| d[3]["production_capability"] = json!("enabled"),
        |d| d[1]["bounds"][0]["maximum"] = json!(0),
        |d| d[0]["operations"][0]["bound_ids"] = json!(["unknown"]),
        |d| {
            d[2]["surfaces"][2]["cells"].as_array_mut().unwrap().pop();
        },
        |d| {
            let duplicate = d[0]["operations"][0].clone();
            d[0]["operations"].as_array_mut().unwrap().push(duplicate);
        },
        |d| d[0]["api_extensions"]["unsupported_families"] = json!([]),
        |d| d[0]["operations"][0]["disposition"] = json!("silent_emulation"),
        |d| d[0]["operations"][0]["retry_class"] = json!(""),
        |d| d[1]["bounds"][0]["minimum"] = json!(-1),
    ];
    assert!(check(&baseline).is_ok());
    for edit in edits {
        let mut documents = baseline.clone();
        edit(&mut documents);
        assert!(check(&documents).is_err());
    }
    assert!(
        CapabilityDescriptor::from_contracts("hc2-rust", "{", BOUNDS, SURFACES, STATUS).is_err()
    );
}
