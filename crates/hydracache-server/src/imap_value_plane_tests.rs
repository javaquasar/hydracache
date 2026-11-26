use std::num::NonZeroU64;
use std::sync::atomic::AtomicBool;

use hydracache::{ClusterCandidate, ClusterControlPlane, TimeObservation};
use hydracache_client_protocol::{
    CasExpectation, ClientRequestEnvelope, CompareValueExpireMode, ConditionalPutCondition,
    LockConsistency,
};
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
    last: Mutex<Option<ReplicaApply>>,
}

#[async_trait::async_trait]
impl ReplicaTransport for PeerTransport {
    async fn apply(&self, message: ReplicaApply) -> CacheResult<ReplicaApplyAck> {
        if self.hold.load(Ordering::Acquire) {
            self.entered.add_permits(1);
            self.release.acquire().await.unwrap().forget();
        }
        let source = message.mutation.owner.node.clone();
        *self.last.lock().unwrap() = Some(message.clone());
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

    async fn probe(&self, message: ReplicaPrefix) -> CacheResult<ReplicaPrefix> {
        let snapshot = self
            .raft
            .partition_snapshot()
            .ok_or_else(replica_rejected)?;
        let plan = snapshot
            .value_promotions
            .get(&message.partition)
            .ok_or_else(replica_rejected)?;
        let peer = self
            .replicas
            .iter()
            .find(|peer| peer.local == message.target)
            .ok_or_else(replica_rejected)?;
        peer.prove_replica(
            &snapshot,
            plan.new_owner.node.as_str(),
            message.epoch.value(),
            message,
        )
    }
}

struct Fixture {
    dispatcher: Arc<ClientSurfaceDispatcher>,
    transport: Arc<PeerTransport>,
    owner: Arc<ValueReplica>,
    partition: PartitionId,
    backend: Arc<ValueBackend<hydracache_cluster_raft::InMemoryRaftLogStore>>,
    clock: Arc<ManualClock>,
}

#[test]
fn a_scoped_id_cannot_start_an_operation_in_a_different_epoch() {
    let epoch = ClusterEpoch::new(4);
    assert!(new_identity(None, epoch, true));
    assert!(new_identity(Some("opaque"), epoch, false));
    assert!(!new_identity(Some("opaque"), epoch, true));
    assert!(new_identity(Some("w10e:4:nonce"), epoch, true));
    for id in [
        "w10e:3:nonce",
        "w10e:5:nonce",
        "w10e:4:",
        "w10e:4",
        "w10e:invalid:nonce",
    ] {
        assert!(!new_identity(Some(id), epoch, true));
    }
}

#[tokio::test]
async fn forwarded_hc2_write_requires_exact_epoch_generation_principal_and_partition() {
    use crate::config::DistributedHc2Config;
    use crate::imap_hc2_route::{OwnerCall, RoutedValueBackend};
    use serde_json::json;

    let fixture = Fixture::new(1).await;
    let profile = DistributedHc2Config {
        tenant: "tenant".into(),
        client_cert_sha256: "a".repeat(64),
    };
    let principal = profile.principal();
    let backend = RoutedValueBackend::new(fixture.backend.clone(), profile, 1024);
    let snapshot = fixture.backend.raft.partition_snapshot().unwrap();
    let peer = fixture.transport.replicas[0].local.clone();
    let (ns, key) = parts();
    let envelope = ClientRequestEnvelope::new(
        "forward",
        ClientRequest::Put {
            ns,
            key,
            value: b"forwarded".to_vec(),
            ttl_ms: None,
            dimensions: Vec::new(),
        },
    );
    let original = json!({
        "version": 1, "epoch": snapshot.epoch, "target": fixture.owner.local,
        "source": peer, "partition": fixture.partition, "principal": principal,
        "tenant": "tenant", "expires_at_ms": crate::hc2::unix_time_ms() + 2_000, "envelope": envelope
    });
    for (field, invalid) in [
        (
            "epoch",
            json!(ClusterEpoch::new(snapshot.epoch.value() - 1)),
        ),
        (
            "partition",
            json!(PartitionId::new(fixture.partition.value() + 1)),
        ),
        ("source", json!({"node": peer.node, "generation": 999})),
        ("target", json!(peer)),
        ("principal", json!("foreign")),
        ("tenant", json!("foreign")),
        ("expires_at_ms", json!(0)),
        ("version", json!(2)),
    ] {
        let mut bad = original.clone();
        bad[field] = invalid;
        let call: OwnerCall = serde_json::from_value(bad).unwrap();
        assert!(
            backend
                .receive(peer.node.as_str(), snapshot.epoch.value(), call)
                .await
                .outcome
                .is_err(),
            "{field}"
        );
    }
    let call = serde_json::from_value(original.clone()).unwrap();
    assert!(backend
        .receive("unadmitted", snapshot.epoch.value(), call)
        .await
        .outcome
        .is_err());
    assert_eq!(fixture.owner.owner_applies(), 0);
    let call = serde_json::from_value(original).unwrap();
    assert_eq!(
        backend
            .receive(peer.node.as_str(), snapshot.epoch.value(), call)
            .await
            .outcome
            .unwrap(),
        ClientResponse::Stored
    );
    assert_eq!(fixture.version(), 1);
}

#[tokio::test]
async fn promotion_refuses_a_designated_backup_without_the_exact_prefix() {
    let fixture = Fixture::new(1).await;
    let raft = &fixture.backend.raft;
    let epoch = raft.partition_snapshot().unwrap().epoch;
    let first = PartitionValuePrefix {
        partition: fixture.partition,
        epoch,
        version: 1,
        previous: [0; 32],
        digest: [1; 32],
    };
    raft.propose_value_prefix(first).unwrap();
    let designated = &fixture.transport.replicas[0];
    assert!(designated
        .promote(
            raft,
            &fixture.backend.raft_transport,
            fixture.partition,
            vec![fixture.owner.local.clone()]
        )
        .await
        .is_err());
    let snapshot = raft.partition_snapshot().unwrap();
    assert_eq!(snapshot.epoch, epoch);
    assert_eq!(snapshot.value_prefixes[&fixture.partition], first);
    assert!(snapshot.value_promotions.is_empty());
}

#[derive(Default)]
struct ManualClock(Mutex<Option<TimeObservation>>);

impl ClusterTimeAuthority for ManualClock {
    fn observe(&self) -> Option<TimeObservation> {
        *self.0.lock().unwrap()
    }
}

#[test]
fn observation_rejects_unadmitted_error_and_unrepresentable_bounds() {
    assert_eq!(
        TimeObservation {
            now_ms: 100,
            uncertainty_ms: 2
        }
        .bounds(2),
        Some((98, 102))
    );
    for now_ms in [0, u64::MAX] {
        assert_eq!(
            TimeObservation {
                now_ms,
                uncertainty_ms: 2
            }
            .bounds(2),
            None
        );
    }
    assert_eq!(
        TimeObservation {
            now_ms: 100,
            uncertainty_ms: 3
        }
        .bounds(2),
        None
    );
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
        let clock = Arc::new(ManualClock::default());
        let owner = Arc::new(
            ValueReplica::with_journal(
                ValueJournal::temporary(),
                placement.primary.clone(),
                scope(),
                replication,
            )
            .unwrap()
            .with_clock(Some(ReplicaClock {
                authority: clock.clone(),
                maximum_uncertainty_ms: 10,
            })),
        );
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
            last: Mutex::new(None),
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
        let backend = Arc::new(ValueBackend {
            raft,
            raft_transport,
            value_transport: Some(transport.clone()),
            replica: owner.clone(),
            read,
        });
        Self {
            dispatcher: Arc::new(
                ClientSurfaceDispatcher::from_backend(Default::default(), backend.clone()).unwrap(),
            ),
            transport,
            owner,
            partition,
            backend,
            clock,
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

    async fn command(&self, request: ClientRequest, id: Option<&str>) -> ClientBackendOutcome {
        let mut envelope = ClientRequestEnvelope::new("w10d", request);
        envelope.idempotency_key = id.map(str::to_owned);
        self.dispatcher
            .dispatch_verified_request(&identity(), envelope)
            .await
            .result
    }

    fn time(&self, now_ms: u64, uncertainty_ms: u64) {
        *self.clock.0.lock().unwrap() = Some(TimeObservation {
            now_ms,
            uncertainty_ms,
        });
    }

    async fn pending_ttl(&self) {
        let (ns, key) = parts();
        assert_eq!(
            self.command(
                ClientRequest::Put {
                    ns,
                    key,
                    value: b"ttl".to_vec(),
                    ttl_ms: Some(10),
                    dimensions: Vec::new(),
                },
                None
            )
            .await
            .unwrap(),
            ClientResponse::Stored
        );
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

#[tokio::test]
async fn raft_certificate_without_the_durable_image_refuses_get_before_key_lookup() {
    let fixture = Fixture::new(1).await;
    let raft = &fixture.transport.raft;
    let epoch = raft.partition_snapshot().unwrap().epoch;
    raft.propose_value_prefix(PartitionValuePrefix {
        partition: fixture.partition,
        epoch,
        version: 1,
        previous: [0; 32],
        digest: [7; 32],
    })
    .unwrap();
    assert_eq!(
        fixture.get().await.unwrap_err().code,
        ClientErrorCode::BackendUnavailable
    );
    assert_eq!(fixture.owner.store_lookups(), 0);
}

#[test]
fn journal_rejects_another_scope_or_generation() {
    let journal = ValueJournal::temporary();
    journal.bind_scope("tenant", "orders", 1).unwrap();
    journal.bind_scope("tenant", "orders", 1).unwrap();
    assert!(journal.bind_scope("other", "orders", 1).is_err());
    assert!(journal.bind_scope("tenant", "orders", 2).is_err());
}

#[tokio::test]
async fn ttl_is_computed_once_and_expires_only_after_the_lower_bound() {
    let fixture = Fixture::new(1).await;
    fixture.time(100, 2);
    fixture.pending_ttl().await;
    let late_value = fixture.transport.last.lock().unwrap().clone().unwrap();
    let snapshot = fixture.transport.raft.partition_snapshot().unwrap();
    let cert = snapshot.value_prefixes[&fixture.partition];
    let peer = &fixture.transport.replicas[0];
    let (_, key) = scope().locate(b"key".to_vec()).unwrap();
    let meta = peer.journal.load(fixture.partition).unwrap();
    let record = peer
        .journal
        .read_entry(fixture.partition, &key, &meta, Some(cert))
        .unwrap()
        .unwrap();
    assert_eq!(record.expires_not_before_ms, Some(112));
    assert!(peer.time().is_none());
    let (ns, key) = parts();
    assert!(matches!(
        fixture
            .command(ClientRequest::GetTtl { ns, key }, None)
            .await
            .unwrap(),
        ClientResponse::Ttl {
            state: TtlState::ExpiresIn { ttl_ms: 10 }
        }
    ));

    fixture.time(110, 11);
    assert_eq!(
        fixture.get().await.unwrap_err().code,
        ClientErrorCode::BackendUnavailable
    );
    fixture.backend.expire_batch(&mut None).await;
    assert_eq!(fixture.version(), 1);
    fixture.time(111, 2);
    assert_eq!(
        fixture.get().await.unwrap_err().code,
        ClientErrorCode::BackendUnavailable
    );
    fixture.backend.expire_batch(&mut None).await;
    assert_eq!(fixture.version(), 1);
    fixture.time(114, 2);
    assert_eq!(
        fixture.get().await.unwrap_err().code,
        ClientErrorCode::BackendUnavailable
    );
    fixture.backend.expire_batch(&mut None).await;
    assert_eq!(fixture.version(), 2);
    fixture.time(90, 0);
    assert_eq!(
        fixture.get().await.unwrap(),
        ClientResponse::Value { value: None }
    );
    assert_eq!(peer.replica_applies(), 2);
    let snapshot = fixture.transport.raft.partition_snapshot().unwrap();
    let source = late_value.mutation.owner.node.clone();
    assert!(peer
        .apply_replica(&snapshot, source.as_str(), late_value.epoch(), late_value)
        .is_err());
    assert_eq!(peer.replica_applies(), 2);
}

#[tokio::test]
async fn conditional_ttl_extension_preserves_the_previous_deadline_and_refuses_mismatch() {
    let fixture = Fixture::new(1).await;
    fixture.time(100, 2);
    fixture.pending_ttl().await;
    let (ns, key) = parts();
    let extend = |value: &[u8]| ClientRequest::CompareValueAndExpire {
        ns: ns.clone(),
        key: key.clone(),
        expected_value: value.to_vec(),
        ttl_ms: 5,
        mode: CompareValueExpireMode::AddToRemaining,
    };
    assert_eq!(
        fixture.command(extend(b"wrong"), None).await.unwrap(),
        ClientResponse::CompareValueApplied { applied: false }
    );
    assert_eq!(fixture.version(), 1);
    assert_eq!(
        fixture.command(extend(b"ttl"), None).await.unwrap(),
        ClientResponse::CompareValueApplied { applied: true }
    );
    let ttl = |ns, key| ClientRequest::GetTtl { ns, key };
    assert_eq!(
        fixture
            .command(ttl(ns.clone(), key.clone()), None)
            .await
            .unwrap(),
        ClientResponse::Ttl {
            state: TtlState::ExpiresIn { ttl_ms: 15 }
        }
    );
    assert_eq!(
        fixture
            .command(
                ClientRequest::Expire {
                    ns: ns.clone(),
                    key: key.clone(),
                    ttl_ms: 0
                },
                None
            )
            .await
            .unwrap(),
        ClientResponse::Expiry { applied: true }
    );
    assert_eq!(
        fixture.command(ttl(ns, key), None).await.unwrap(),
        ClientResponse::Ttl {
            state: TtlState::Missing
        }
    );
}

#[tokio::test]
async fn stale_deadline_ticket_cannot_remove_a_persisted_replacement() {
    let fixture = Fixture::new(1).await;
    fixture.time(100, 2);
    fixture.pending_ttl().await;
    fixture.time(103, 2);
    let (ns, key) = parts();
    assert_eq!(
        fixture
            .command(ClientRequest::Persist { ns, key }, None)
            .await
            .unwrap(),
        ClientResponse::Expiry { applied: true }
    );
    fixture.time(150, 2);
    fixture.backend.expire_batch(&mut None).await;
    assert_eq!(fixture.version(), 2);
    assert_eq!(
        fixture.get().await.unwrap(),
        ClientResponse::Value {
            value: Some(b"ttl".to_vec())
        }
    );
}

#[tokio::test]
async fn owner_serializes_if_absent_and_replays_mismatch_without_a_second_decision() {
    let fixture = Fixture::new(1).await;
    let (ns, key) = parts();
    let nx = ClientRequest::ConditionalPut {
        ns: ns.clone(),
        key: key.clone(),
        value: b"first".to_vec(),
        ttl_ms: None,
        condition: ConditionalPutCondition::IfAbsent,
    };
    let (a, b) = tokio::join!(
        fixture.command(nx.clone(), None),
        fixture.command(nx.clone(), None)
    );
    assert_eq!(
        [a, b]
            .into_iter()
            .filter(|res| matches!(res, Ok(ClientResponse::ConditionalStored { stored: true })))
            .count(),
        1
    );
    assert_eq!(fixture.version(), 1);
    assert_eq!(
        fixture.command(nx, None).await.unwrap(),
        ClientResponse::ConditionalStored { stored: false }
    );
    let cas = ClientRequest::CompareAndSet {
        ns: ns.clone(),
        key: key.clone(),
        expected: CasExpectation::Exact(b"later".to_vec()),
        new_value: b"ghost".to_vec(),
        level: LockConsistency::Quorum,
    };
    let mismatch = ClientResponse::CasMismatch {
        current: Some(b"first".to_vec()),
    };
    assert_eq!(
        fixture.command(cas.clone(), Some("cas")).await.unwrap(),
        mismatch
    );
    assert_eq!(
        fixture.get().await.unwrap(),
        ClientResponse::Value {
            value: Some(b"first".to_vec())
        }
    );
    fixture.put(b"later", "overwrite").await.unwrap();
    assert_eq!(fixture.command(cas, Some("cas")).await.unwrap(), mismatch);
    assert_eq!(fixture.version(), 3);
    let replace = ClientRequest::CompareAndSet {
        ns: ns.clone(),
        key: key.clone(),
        expected: CasExpectation::Present,
        new_value: b"replacement".to_vec(),
        level: LockConsistency::Quorum,
    };
    assert_eq!(
        fixture.command(replace, None).await.unwrap(),
        ClientResponse::CasApplied { new_version: 4 }
    );
    assert!(matches!(
        fixture
            .command(
                ClientRequest::RemoveIfValue {
                    ns: ns.clone(),
                    key: key.clone(),
                    expected: b"later".to_vec(),
                    level: LockConsistency::Quorum
                },
                None
            )
            .await
            .unwrap(),
        ClientResponse::CasMismatch { .. }
    ));
    assert_eq!(
        fixture
            .command(
                ClientRequest::RemoveIfValue {
                    ns,
                    key,
                    expected: b"replacement".to_vec(),
                    level: LockConsistency::Quorum
                },
                None
            )
            .await
            .unwrap(),
        ClientResponse::CasApplied { new_version: 5 }
    );
    assert_eq!(
        fixture.get().await.unwrap(),
        ClientResponse::Value { value: None }
    );
}
