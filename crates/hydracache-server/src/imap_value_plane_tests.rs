use std::num::NonZeroU64;
use std::sync::atomic::AtomicBool;

use hydracache::{ClusterCandidate, ClusterControlPlane};
use hydracache_client_protocol::ClientRequestEnvelope;
use hydracache_client_transport_axum::{ClientIdentity, ClientSurfaceDispatcher};
use hydracache_cluster_raft::InMemoryRaftMessageSink;

use super::*;

fn scope() -> ReplicaReadScope {
    ReplicaReadScope::new(
        "tenant".into(),
        Namespace::new("orders").unwrap(),
        NonZeroU64::new(1).unwrap(),
        |key| {
            let [part] = key.segments() else { return None };
            Some(part.as_bytes().to_vec())
        },
    )
}

fn replica(local: PartitionMember, replication: ReplicationConfig) -> Arc<ValueReplica> {
    Arc::new(
        ValueReplica::with_journal(ValueJournal::temporary(), local, scope(), replication).unwrap(),
    )
}

struct PeerTransport {
    raft: Arc<RaftMetadataRuntime>,
    replicas: Vec<Arc<ValueReplica>>,
    corrupt_ack: AtomicBool,
    hold: AtomicBool,
    entered: Semaphore,
    release: Semaphore,
}

#[async_trait::async_trait]
impl ReplicaTransport for PeerTransport {
    async fn apply(&self, message: ReplicaApply) -> CacheResult<ReplicaApplyAck> {
        if self.hold.load(Ordering::Acquire) {
            self.entered.add_permits(1);
            self.release.acquire().await.unwrap().forget();
        }
        let source = message.mutation.owner.node.clone();
        let peer = self
            .replicas
            .iter()
            .find(|peer| peer.local == message.target)
            .unwrap();
        let mut ack = peer.apply_replica(
            &self.raft.partition_snapshot().unwrap(),
            source.as_str(),
            message.epoch(),
            message,
        )?;
        if self.corrupt_ack.load(Ordering::Acquire) {
            ack.replica.generation = hydracache::ClusterGeneration::new(99);
        }
        Ok(ack)
    }
}

struct Fixture {
    dispatcher: Arc<ClientSurfaceDispatcher>,
    transport: Arc<PeerTransport>,
    owner: Arc<ValueReplica>,
    partition: PartitionId,
}

impl Fixture {
    async fn new(backups: usize) -> Self {
        let raft = Arc::new(RaftMetadataRuntime::single_node("w10c", 1).unwrap());
        for node in ["a", "b", "c"] {
            raft.join_member(ClusterCandidate::member(node))
                .await
                .unwrap();
        }
        let replication = ReplicationConfig {
            replication_factor: backups + 1,
            sync_backups: backups,
            ..ReplicationConfig::local_first()
        };
        let partition = scope().locate(b"key".to_vec()).unwrap().0;
        let placement = PartitionTopology::from_snapshot(
            raft.partition_snapshot().unwrap(),
            D2_PARTITION_COUNT,
            replication,
        )
        .unwrap()
        .placement(partition)
        .unwrap();
        let owner = replica(placement.primary.clone(), replication);
        let transport = Arc::new(PeerTransport {
            raft: raft.clone(),
            replicas: placement
                .backups
                .into_iter()
                .map(|member| replica(member, replication))
                .collect(),
            corrupt_ack: AtomicBool::new(false),
            hold: AtomicBool::new(false),
            entered: Semaphore::new(0),
            release: Semaphore::new(0),
        });
        let raft_transport = Arc::new(InMemoryRaftMessageSink::default());
        let read = RaftOwnerReadBackend::new(
            raft.clone(),
            raft_transport.clone(),
            owner.clone(),
            placement.primary,
            D2_PARTITION_COUNT,
            replication,
        );
        let backend = ValueBackend {
            raft,
            raft_transport,
            value_transport: Some(transport.clone()),
            replica: owner.clone(),
            read,
        };
        Self {
            dispatcher: Arc::new(
                ClientSurfaceDispatcher::from_backend(Default::default(), Arc::new(backend))
                    .unwrap(),
            ),
            transport,
            owner,
            partition,
        }
    }

    fn version(&self) -> u64 {
        self.transport
            .raft
            .partition_snapshot()
            .unwrap()
            .value_prefixes
            .get(&self.partition)
            .map_or(0, |prefix| prefix.version)
    }

    async fn put(&self, value: &[u8], id: &str) -> ClientBackendOutcome {
        let (ns, key) = parts();
        self.dispatcher
            .dispatch_verified_request(
                &identity(),
                ClientRequestEnvelope::new(
                    "put",
                    ClientRequest::Put {
                        ns,
                        key,
                        value: value.to_vec(),
                        ttl_ms: None,
                        dimensions: Vec::new(),
                    },
                )
                .with_idempotency_key(id),
            )
            .await
            .result
    }

    async fn get(&self) -> ClientBackendOutcome {
        let (ns, key) = parts();
        self.dispatcher
            .dispatch_verified_request(
                &identity(),
                ClientRequestEnvelope::new("get", ClientRequest::Get { ns, key }),
            )
            .await
            .result
    }
}

fn identity() -> ClientIdentity {
    ClientIdentity::new("client", "tenant").unwrap()
}

fn parts() -> (Namespace, StructuredKey) {
    (
        Namespace::new("orders").unwrap(),
        StructuredKey::new(vec!["key".into()]).unwrap(),
    )
}

#[tokio::test]
async fn zero_one_two_backup_profiles_commit_and_replay_one_version() {
    for backups in 0..=2 {
        let fixture = Fixture::new(backups).await;
        assert_eq!(
            fixture.put(b"v", "same").await.unwrap(),
            ClientResponse::Stored
        );
        assert_eq!(
            fixture.put(b"v", "same").await.unwrap(),
            ClientResponse::Stored
        );
        assert_eq!(
            fixture.put(b"different", "same").await.unwrap_err().code,
            ClientErrorCode::Conflict
        );
        assert_eq!(fixture.version(), 1);
        assert_eq!(
            fixture.get().await.unwrap(),
            ClientResponse::Value {
                value: Some(b"v".to_vec())
            }
        );
        for peer in &fixture.transport.replicas {
            assert_eq!(peer.replica_applies(), 1);
        }
    }
}

#[tokio::test]
async fn bad_replica_ack_cannot_certify_a_flushed_record() {
    let fixture = Fixture::new(1).await;
    fixture.transport.corrupt_ack.store(true, Ordering::Release);
    let error = fixture.put(b"pending", "write").await.unwrap_err();
    assert_eq!(error.code, ClientErrorCode::BackendUnavailable);
    assert_eq!(fixture.version(), 0);
    assert_eq!(fixture.transport.replicas[0].replica_applies(), 1);
    assert_eq!(
        fixture.get().await.unwrap(),
        ClientResponse::Value { value: None }
    );
    fixture
        .transport
        .corrupt_ack
        .store(false, Ordering::Release);
    assert_eq!(
        fixture.put(b"pending", "write").await.unwrap(),
        ClientResponse::Stored
    );
    assert_eq!(fixture.version(), 1);
}

#[tokio::test]
async fn blocked_replica_holds_response_and_cancel_releases_the_owner_lane() {
    let fixture = Arc::new(Fixture::new(1).await);
    fixture.transport.hold.store(true, Ordering::Release);
    let write = tokio::spawn({
        let fixture = fixture.clone();
        async move { fixture.put(b"pending", "write").await }
    });
    tokio::time::timeout(Duration::from_secs(1), fixture.transport.entered.acquire())
        .await
        .unwrap()
        .unwrap()
        .forget();
    assert!(!write.is_finished());
    assert_eq!(fixture.version(), 0);
    assert_eq!(
        fixture.put(b"other", "other").await.unwrap_err().code,
        ClientErrorCode::BackendUnavailable
    );
    write.abort();
    assert!(write.await.unwrap_err().is_cancelled());
    fixture.transport.hold.store(false, Ordering::Release);
    assert_eq!(
        fixture.put(b"pending", "write").await.unwrap(),
        ClientResponse::Stored
    );
    assert_eq!(fixture.owner.store_lookups(), 0);
    assert_eq!(fixture.version(), 1);
}
