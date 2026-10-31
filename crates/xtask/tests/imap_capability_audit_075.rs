use std::path::Path;
use xtask::imap_capability_audit::{check_at_root, check_sources, AuditSources};

const DESCRIPTOR: &str = include_str!("../../../tests/java-imap-benchmark/src/main/java/io/hydracache/imap/semantic/IMapCapabilityDescriptor.java");
const JAVA: &str = include_str!("../../../tests/java-imap-benchmark/src/main/java/io/hydracache/imap/semantic/UnifiedIMapService.java");
const RUST: &str = include_str!("../../hydracache-cluster-testkit/src/value_plane_service_075.rs");
const CONTRACT: &str = include_str!("../../../docs/testing/imap/0.75/operation-contract.json");
const STATUS: &str = include_str!("../../../docs/testing/imap/0.75/status.json");
const MAP: &str = include_str!("../../../sdks/java/hydracache-hazelcast-facade/src/main/java/io/hydracache/hazelcast/HydraMap.java");
const LOCK: &str = include_str!("../../../sdks/java/hydracache-hazelcast-facade/src/main/java/io/hydracache/hazelcast/HydraFencedLock.java");
const PROPERTIES: &str = include_str!("../../../sdks/java/hydracache-hazelcast-facade/src/main/resources/META-INF/hydracache/hazelcast-capabilities.properties");
fn sources() -> AuditSources<'static> {
    AuditSources {
        java_descriptor: DESCRIPTOR,
        java_service: JAVA,
        rust_service: RUST,
        operation_contract: CONTRACT,
        status: STATUS,
        sdk_map: MAP,
        sdk_lock: LOCK,
        sdk_properties: PROPERTIES,
    }
}

#[test]
fn capability_audit_keeps_actual_reference_sdk_and_planned_claims_distinct() {
    assert!(check_sources(&sources()).unwrap().is_empty());
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    assert!(check_at_root(root, "0.75").unwrap().is_empty());
    assert!(!check_at_root(root, "0.74").unwrap().is_empty());
}

#[test]
fn capability_audit_rejects_code_contract_manifest_and_comment_spoof_drift() {
    for target in [
        "java",
        "rust",
        "descriptor",
        "sdk",
        "properties",
        "production",
    ] {
        let mut sources = sources();
        let changed;
        match target {
            "java" => {
                changed = JAVA.replace("\"local_statistics\" ->", "\"unregistered_statistics\" ->");
                sources.java_service = &changed;
            }
            "rust" => {
                changed = RUST.replace("| \"local_statistics\"", "| \"unregistered_statistics\"");
                sources.rust_service = &changed;
            }
            "descriptor" => {
                changed =
                    DESCRIPTOR.replace("\"safe_outcome_retirement\"", "\"fabricated_feature\"");
                sources.java_descriptor = &changed;
            }
            "sdk" => {
                changed = MAP.replace(
                    "Optional<V> get(K key)",
                    "Optional<V> newUnregisteredGet(K key)",
                );
                sources.sdk_map = &changed;
            }
            "properties" => {
                changed = PROPERTIES.replace("IMap.get,IMap.put", "IMap.get,IMap.getAll,IMap.put");
                sources.sdk_properties = &changed;
            }
            _ => {
                changed = STATUS.replace("disabled_fail_closed", "enabled");
                sources.status = &changed;
            }
        }
        assert!(!check_sources(&sources).unwrap().is_empty(), "{target}");
    }
    let mut sources = sources();
    let changed = RUST.replace("| \"local_statistics\"", "/* | \"local_statistics\" */");
    sources.rust_service = &changed;
    assert!(!check_sources(&sources).unwrap().is_empty());
    sources = super_sources();
    sources.sdk_properties = "supported=IMap.get\nsupported=IMap.put\n";
    assert!(check_sources(&sources).is_err());
    sources = super_sources();
    sources.java_descriptor = "/* unterminated";
    assert!(check_sources(&sources).is_err());
    assert!(xtask::imap_capability_audit::run(vec!["--release".into(), "0.74".into()]).is_err());
}
fn super_sources() -> AuditSources<'static> {
    sources()
}
