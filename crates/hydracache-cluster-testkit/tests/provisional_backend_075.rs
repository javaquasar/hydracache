use async_trait::async_trait;
use hydracache_cluster_testkit::provisional_backend_075::{
    FaultInjectableBackend, NamespaceIdentity, PartitionRoute, PartitionRouteOracle,
    ProvisionalBackendError, ProvisionalCapability, ProvisionalClientDataBackend, ReplicaProof,
    ReplicaProofOracle,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Default)]
struct FakeLocalBackend {
    values: Mutex<BTreeMap<Vec<u8>, Vec<u8>>>,
}

#[async_trait]
impl ProvisionalClientDataBackend for FakeLocalBackend {
    async fn get(
        &self,
        _namespace: NamespaceIdentity,
        key: &[u8],
    ) -> Result<Option<Vec<u8>>, ProvisionalBackendError> {
        Ok(self.values.lock().unwrap().get(key).cloned())
    }

    async fn put(
        &self,
        _namespace: NamespaceIdentity,
        _route: PartitionRoute,
        key: Vec<u8>,
        value: Vec<u8>,
    ) -> Result<u64, ProvisionalBackendError> {
        self.values.lock().unwrap().insert(key, value);
        Ok(1)
    }
}

struct FakeDistributedOracle;

impl PartitionRouteOracle for FakeDistributedOracle {
    fn route(&self, namespace: NamespaceIdentity, key: &[u8]) -> PartitionRoute {
        PartitionRoute {
            partition: u32::from(key.first().copied().unwrap_or_default()) % 17,
            epoch: namespace.generation,
            owner: 2,
        }
    }
}

impl ReplicaProofOracle for FakeDistributedOracle {
    fn validate(&self, route: PartitionRoute, proof: ReplicaProof) -> bool {
        route.partition == proof.partition && route.epoch == proof.epoch && proof.version > 0
    }
}

#[async_trait]
impl FaultInjectableBackend for FakeDistributedOracle {
    async fn inject_before_owner_apply(
        &self,
        _route: PartitionRoute,
    ) -> Result<(), ProvisionalBackendError> {
        Err(ProvisionalBackendError::InjectedFault)
    }

    async fn inject_after_replica_proof(
        &self,
        _proof: ReplicaProof,
    ) -> Result<(), ProvisionalBackendError> {
        Ok(())
    }
}

#[tokio::test]
async fn local_and_distributed_fakes_share_the_provisional_seams() {
    let namespace = NamespaceIdentity {
        tenant: 1,
        namespace: 7,
        generation: 3,
    };
    let oracle = FakeDistributedOracle;
    let route = oracle.route(namespace, b"key");
    let backend = FakeLocalBackend::default();
    backend
        .put(namespace, route, b"key".to_vec(), b"value".to_vec())
        .await
        .unwrap();
    assert_eq!(
        backend.get(namespace, b"key").await.unwrap(),
        Some(b"value".to_vec())
    );
    assert!(oracle.validate(
        route,
        ReplicaProof {
            partition: route.partition,
            epoch: route.epoch,
            version: 1,
            backup: 3,
        }
    ));
}

#[test]
fn capability_has_no_enabled_state_before_074() {
    assert_eq!(
        ProvisionalCapability::disabled().require_enabled(),
        Err(ProvisionalBackendError::DisabledBlockedBy074)
    );
}

#[test]
fn production_sources_cannot_reach_the_provisional_testkit_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap();
    let mut offenders = Vec::new();
    for entry in fs::read_dir(root.join("crates")).unwrap() {
        let path = entry.unwrap().path();
        if path.file_name().and_then(|name| name.to_str()) == Some("hydracache-cluster-testkit")
            || path.file_name().and_then(|name| name.to_str()) == Some("xtask")
        {
            continue;
        }
        scan(&path, &mut offenders);
    }
    assert!(
        offenders.is_empty(),
        "production references: {offenders:#?}"
    );
}

fn scan(path: &Path, offenders: &mut Vec<PathBuf>) {
    if path.is_dir() {
        for entry in fs::read_dir(path).unwrap() {
            scan(&entry.unwrap().path(), offenders);
        }
        return;
    }
    let relevant = path.extension().and_then(|ext| ext.to_str()) == Some("rs")
        || path.file_name().and_then(|name| name.to_str()) == Some("Cargo.toml");
    if relevant
        && fs::read_to_string(path).is_ok_and(|source| source.contains("provisional_backend_075"))
    {
        offenders.push(path.to_path_buf());
    }
}
