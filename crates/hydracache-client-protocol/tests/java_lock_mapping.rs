use std::fs;
use std::path::PathBuf;

use hydracache_client_protocol::java_migration::{
    JavaLockOperation, JavaLockProtocolFamily, UnsupportedHazelcastApiManifest,
    JAVA_MIGRATION_CONTRACT_VERSION,
};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

#[test]
fn lock_apis_are_now_supported_mapping_not_rejected() {
    let manifest = UnsupportedHazelcastApiManifest::checked_in().unwrap();

    for api in ["IMap.lock", "IMap.tryLock", "FencedLock"] {
        assert!(manifest.find(api).is_none(), "{api} should not be refused");
        assert!(manifest
            .find_supported(api)
            .expect("supported lock mapping")
            .migration_hint
            .contains("HydraFencedLock"));
    }

    for api in [
        "IMap.executeOnKey",
        "HazelcastInstance.getSql",
        "HazelcastInstance.getExecutorService",
        "ReplicatedMap",
        "ReliableTopic",
    ] {
        assert!(manifest.find(api).is_some(), "{api} remains unsupported");
    }
}

#[test]
fn java_lock_operation_maps_to_wire_family() {
    assert_eq!(
        JavaLockOperation::LockAndGetFence.protocol_family(),
        JavaLockProtocolFamily::TryLock {
            returns_fence: true,
            blocking_wait: true,
        }
    );
    assert_eq!(
        JavaLockOperation::TryLock.protocol_family(),
        JavaLockProtocolFamily::TryLock {
            returns_fence: true,
            blocking_wait: false,
        }
    );
    assert_eq!(
        JavaLockOperation::Unlock.protocol_family(),
        JavaLockProtocolFamily::Unlock
    );
    assert_eq!(
        JavaLockOperation::GetFence.protocol_family(),
        JavaLockProtocolFamily::GetLockOwnership
    );
}

#[test]
fn migration_contract_version_bumped_and_documented() {
    assert_eq!(JAVA_MIGRATION_CONTRACT_VERSION, 2);

    let root = repo_root();
    let compat = fs::read_to_string(root.join("docs/COMPAT.md")).expect("compat");
    let docs = fs::read_to_string(root.join("docs/integrations/java-migration.md"))
        .expect("java migration docs");

    assert!(compat.contains("| Java migration toolkit contract | `2` |"));
    assert!(docs.contains("Hazelcast Lock Mapping"));
    assert!(docs.contains("forceUnlock"));
    assert!(docs.contains("not Hazelcast wire compatibility"));
}

#[test]
fn force_unlock_is_marked_privileged() {
    assert!(JavaLockOperation::ForceUnlock.is_privileged());
    assert_eq!(
        JavaLockOperation::ForceUnlock.protocol_family(),
        JavaLockProtocolFamily::ForceUnlock { privileged: true }
    );
}
