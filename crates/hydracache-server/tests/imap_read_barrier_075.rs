use std::collections::BTreeMap;
use std::future::Future;
use std::num::{NonZeroU32, NonZeroU64};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use bytes::Bytes;
use hydracache::{
    CacheResult, ClusterCandidate, ClusterControlPlane, ClusterGeneration, PartitionId,
    ReplicationConfig,
};
use hydracache_client_hc2::{
    ClientConfig, ErrorCode, GrpcMtlsAdapter, GrpcMtlsConfig, Hc2Client, RequestOptions,
};
use hydracache_client_protocol::{
    ClientErrorCode, ClientFrame, ClientRequest, ClientRequestEnvelope, ClientResponse,
    ClientResponseEnvelope, ClientWireMessage, Namespace, StructuredKey,
};
use hydracache_client_transport_axum::{
    ClientBackendOutcome, ClientSurfaceDispatcher, CLIENT_DATA_PATH, HYDRACACHE_CLIENT_ID_HEADER,
    HYDRACACHE_TENANT_HEADER,
};
use hydracache_cluster_raft::{RaftMessageSink, RaftMetadataRuntime, RaftWireMessage};
use hydracache_cluster_testkit::{RaftFilterAction, RaftPacketFilter, RuntimeRaftCluster};
use hydracache_server::imap_owner_read::{
    OwnerReadLane, OwnerReadProgress, OwnerReadRoute, OwnerReadValues, RaftOwnerReadBackend,
};
use hydracache_server::imap_read_barrier::ReadInvocation;
use hydracache_server::imap_topology::PartitionTopology;
use hydracache_server::{serve_hc2_listener, Hc2ClientPlaneService, Hc2ListenerTls, TlsConfig};
use raft::eraftpb::MessageType;
use tokio::net::TcpListener;

#[path = "support/hc2_pki.rs"]
mod hc2_pki;

const PARTITIONS: NonZeroU32 = NonZeroU32::new(271).unwrap();

fn stored_key() -> StructuredKey {
    StructuredKey::new(vec!["6f726465722d31".into()]).unwrap()
}

fn replication() -> ReplicationConfig {
    ReplicationConfig {
        replication_factor: 2,
        sync_backups: 1,
        ..ReplicationConfig::local_first()
    }
}

struct Network(Mutex<RuntimeRaftCluster>);

impl Network {
    async fn new() -> Arc<Self> {
        let mut cluster = RuntimeRaftCluster::three_node();
        cluster.campaign(1);
        let network = Arc::new(Self(Mutex::new(cluster)));
        for node in ["1", "2", "3"] {
            network
                .apply(move |leader| async move {
                    leader
                        .join_member(
                            ClusterCandidate::member(node).generation(ClusterGeneration::new(1)),
                        )
                        .await
                        .unwrap();
                })
                .await;
        }
        network
    }

    async fn apply<F, Fut>(&self, command: F)
    where
        F: FnOnce(Arc<RaftMetadataRuntime>) -> Fut,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let leader = self.0.lock().unwrap().node(1);
        let pending = tokio::spawn(command(leader.clone()));
        tokio::time::timeout(Duration::from_secs(3), async {
            while !pending.is_finished() {
                self.0
                    .lock()
                    .unwrap()
                    .drain_until_idle(leader.take_outbound_messages());
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        pending.await.unwrap();
    }

    fn node(&self, id: u64) -> Arc<RaftMetadataRuntime> {
        self.0.lock().unwrap().node(id)
    }

    fn topology(&self) -> PartitionTopology {
        PartitionTopology::from_snapshot(
            self.node(1).partition_snapshot().unwrap(),
            PARTITIONS,
            replication(),
        )
        .unwrap()
    }

    fn follower_partition(&self) -> PartitionId {
        let topology = self.topology();
        (0..PARTITIONS.get())
            .map(PartitionId::new)
            .find(|id| topology.placement(*id).unwrap().primary.node.as_str() == "2")
            .unwrap()
    }

    fn backend(
        self: &Arc<Self>,
        partition: PartitionId,
    ) -> (Arc<ClientSurfaceDispatcher>, Arc<Values>) {
        let placement = self.topology().placement(partition).unwrap();
        let raft = self.node(placement.primary.node.as_str().parse().unwrap());
        let values = Arc::new(Values {
            partition,
            state: Mutex::new(ValueState {
                progress: OwnerReadProgress {
                    namespace_generation: NonZeroU64::new(1).unwrap(),
                    epoch: placement.epoch,
                    applied_prefix: 8,
                    required_prefix: 8,
                },
                entries: BTreeMap::from([(stored_key(), b"retained-value".to_vec())]),
            }),
            lookups: AtomicUsize::new(0),
            before_lane: Mutex::new(None),
        });
        let backend = RaftOwnerReadBackend::new(
            raft,
            self.clone(),
            ValueSource(values.clone()),
            placement.primary,
            PARTITIONS,
            replication(),
        );
        let dispatcher = Arc::new(
            ClientSurfaceDispatcher::from_backend(Default::default(), Arc::new(backend)).unwrap(),
        );
        (dispatcher, values)
    }
}

#[async_trait::async_trait]
impl RaftMessageSink for Network {
    async fn send(&self, message: RaftWireMessage) -> CacheResult<()> {
        self.0.lock().unwrap().drain_until_idle([message]);
        Ok(())
    }
}

struct Values {
    partition: PartitionId,
    state: Mutex<ValueState>,
    lookups: AtomicUsize,
    before_lane: Mutex<Option<Box<dyn FnOnce() + Send>>>,
}

struct ValueSource(Arc<Values>);

struct ValueState {
    progress: OwnerReadProgress,
    entries: BTreeMap<StructuredKey, Vec<u8>>,
}

struct Lane<'a> {
    state: MutexGuard<'a, ValueState>,
    lookups: &'a AtomicUsize,
}

impl OwnerReadLane for Lane<'_> {
    fn progress(&self) -> OwnerReadProgress {
        self.state.progress
    }

    fn lookup(&self, key: &StructuredKey) -> ClientBackendOutcome {
        self.lookups.fetch_add(1, Ordering::SeqCst);
        Ok(ClientResponse::Value {
            value: self.state.entries.get(key).cloned(),
        })
    }
}

impl OwnerReadValues for ValueSource {
    type Lane<'a> = Lane<'a>;

    fn route(
        &self,
        tenant: &str,
        namespace: &Namespace,
        _: &StructuredKey,
    ) -> Option<OwnerReadRoute> {
        (tenant == "tenant-a" && namespace.as_str() == "hc2").then(|| OwnerReadRoute {
            namespace_generation: self.0.state.lock().unwrap().progress.namespace_generation,
            partition: self.0.partition,
        })
    }

    fn try_lock_lane(&self, _: &ReadInvocation<'_>) -> Option<Self::Lane<'_>> {
        let before_lane = self.0.before_lane.lock().unwrap().take();
        if let Some(action) = before_lane {
            action();
        }
        Some(Lane {
            state: self.0.state.try_lock().ok()?,
            lookups: &self.0.lookups,
        })
    }
}

struct Hc1Socket {
    url: String,
    listener: tokio::task::JoinHandle<()>,
    client: reqwest::Client,
}

impl Hc1Socket {
    async fn start(dispatcher: Arc<ClientSurfaceDispatcher>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!(
            "http://{}{CLIENT_DATA_PATH}",
            listener.local_addr().unwrap()
        );
        let task =
            tokio::spawn(async move { axum::serve(listener, dispatcher.routes()).await.unwrap() });
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        Self {
            url,
            listener: task,
            client,
        }
    }

    async fn read(&self, tenant: &str, deadline_ms: Option<u64>) -> ClientResponseEnvelope {
        let request = ClientRequest::Get {
            ns: Namespace::new("hc2").unwrap(),
            key: stored_key(),
        };
        let mut request = ClientRequestEnvelope::new("same-client-id", request);
        request.deadline_ms = deadline_ms;
        let frame = ClientFrame::from_message(&ClientWireMessage::Request(request))
            .unwrap()
            .encode()
            .unwrap();
        let body = self
            .client
            .post(&self.url)
            .header(HYDRACACHE_CLIENT_ID_HEADER, "imap-d2")
            .header(HYDRACACHE_TENANT_HEADER, tenant)
            .body(frame)
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .bytes()
            .await
            .unwrap();
        let ClientWireMessage::Response(response) = ClientFrame::decode(&body, 1 << 20)
            .unwrap()
            .decode_message()
            .unwrap()
        else {
            panic!("expected HC/1 response")
        };
        assert_eq!(response.request_id, "same-client-id");
        response
    }
}

impl Drop for Hc1Socket {
    fn drop(&mut self) {
        self.listener.abort();
    }
}

#[tokio::test]
async fn hc1_isolated_follower_owner_refuses_retained_value_and_requires_a_fresh_quorum() {
    let network = Network::new().await;
    let partition = network.follower_partition();
    let (dispatcher, values) = network.backend(partition);
    let client = Hc1Socket::start(dispatcher).await;
    let found = ClientResponse::Value {
        value: Some(b"retained-value".to_vec()),
    };
    assert_eq!(client.read("tenant-a", None).await.result.unwrap(), found);
    assert_eq!(values.lookups.load(Ordering::SeqCst), 1);

    let filters = network.0.lock().unwrap().filters();
    filters.isolate(2, [1, 3]);
    let refused = client.read("tenant-a", Some(50)).await.result.unwrap_err();
    assert_eq!(refused.code, ClientErrorCode::DeadlineExceeded);
    assert!(!filters.dropped().is_empty());
    assert_eq!(values.lookups.load(Ordering::SeqCst), 1);
    assert!(values
        .state
        .lock()
        .unwrap()
        .entries
        .contains_key(&stored_key()));

    filters.recover();
    assert_eq!(client.read("tenant-a", None).await.result.unwrap(), found);
    assert_eq!(values.lookups.load(Ordering::SeqCst), 2);
    assert_eq!(
        client
            .read("another-tenant", None)
            .await
            .result
            .unwrap_err()
            .code,
        ClientErrorCode::BackendUnavailable
    );
    assert_eq!(values.lookups.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn hc1_waits_for_quorum_index_to_be_applied_before_lookup() {
    let network = Network::new().await;
    let partition = network.follower_partition();
    let (dispatcher, values) = network.backend(partition);
    let client = Hc1Socket::start(dispatcher).await;
    let filters = network.0.lock().unwrap().filters();
    filters.add_filter(
        RaftPacketFilter::new()
            .from(1)
            .to(2)
            .message_type(MessageType::MsgAppend)
            .action(RaftFilterAction::Drop),
    );
    network
        .apply(|leader| async move {
            leader
                .join_client(ClusterCandidate::client("metadata-ahead"))
                .await
                .unwrap();
        })
        .await;
    let (leader, follower) = (network.node(1), network.node(2));
    assert!(follower.snapshot().applied_index < leader.snapshot().commit_index);
    let committed = leader.snapshot().commit_index;

    let mut response = std::pin::pin!(client.read("tenant-a", None));
    let saw_quorum = async {
        loop {
            if network.0.lock().unwrap().delivered().iter().any(|message| {
                let message = message.decode().unwrap();
                message.to == 2
                    && message.index == committed
                    && message.get_msg_type() == MessageType::MsgReadIndexResp
            }) {
                return;
            }
            tokio::task::yield_now().await;
        }
    };
    tokio::select! {
        biased;
        result = &mut response => panic!("response before metadata application: {result:?}"),
        result = tokio::time::timeout(Duration::from_secs(3), saw_quorum) => result.unwrap(),
    }
    assert!(follower.snapshot().applied_index < committed);
    assert_eq!(values.lookups.load(Ordering::SeqCst), 0);
    filters.recover();
    network
        .0
        .lock()
        .unwrap()
        .drain_until_idle(leader.tick().unwrap());
    assert_eq!(
        response.await.result.unwrap(),
        ClientResponse::Value {
            value: Some(b"retained-value".to_vec())
        }
    );
    assert_eq!(values.lookups.load(Ordering::SeqCst), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn hc1_promotion_after_read_index_fences_old_lookup_and_requires_new_value_prefix() {
    let network = Network::new().await;
    let partition = network.follower_partition();
    let old_owner = network.topology().placement(partition).unwrap().primary;
    let (dispatcher, old_values) = network.backend(partition);
    *old_values.before_lane.lock().unwrap() = Some(Box::new({
        let network = network.clone();
        move || {
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(network.apply(
                    move |leader| async move {
                        assert!(leader
                            .leave(&old_owner.node, old_owner.generation)
                            .await
                            .unwrap()
                            .is_some());
                    },
                ))
            })
        }
    }));
    let old = Hc1Socket::start(dispatcher).await;
    let refused = old.read("tenant-a", None).await.result.unwrap_err();
    assert_eq!(refused.code, ClientErrorCode::BackendUnavailable);
    assert_eq!(old_values.lookups.load(Ordering::SeqCst), 0);
    assert_ne!(
        network
            .topology()
            .placement(partition)
            .unwrap()
            .primary
            .node
            .as_str(),
        "2"
    );

    let (dispatcher, new_values) = network.backend(partition);
    new_values.state.lock().unwrap().progress.applied_prefix = 7;
    let new = Hc1Socket::start(dispatcher).await;
    assert_eq!(
        new.read("tenant-a", None).await.result.unwrap_err().code,
        ClientErrorCode::BackendUnavailable
    );
    assert_eq!(new_values.lookups.load(Ordering::SeqCst), 0);
    new_values.state.lock().unwrap().progress.applied_prefix = 8;
    assert_eq!(
        new.read("tenant-a", None).await.result.unwrap(),
        ClientResponse::Value {
            value: Some(b"retained-value".to_vec())
        }
    );
    assert_eq!(new_values.lookups.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn hc2_socket_reports_deadline_and_owner_authority_from_the_same_backend() {
    let network = Network::new().await;
    let partition = network.follower_partition();
    let (dispatcher, values) = network.backend(partition);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let pki = hc2_pki::pki();
    let root = std::env::temp_dir().join(format!("hydracache-imap-d2-{}", std::process::id()));
    let (cert, key, ca) = hc2_pki::write_pki(&root, &pki);
    let tls = Hc2ListenerTls::from_server_config(&TlsConfig {
        enabled: true,
        cert_path: Some(cert.clone()),
        key_path: Some(key.clone()),
        ca_path: Some(ca.clone()),
        ..TlsConfig::default()
    })
    .unwrap();
    for path in [cert, key, ca] {
        std::fs::remove_file(path).unwrap();
    }
    std::fs::remove_dir(&root).unwrap();
    let service = Hc2ClientPlaneService::from_dispatcher(dispatcher, "orders");
    let (shutdown, receiver) = tokio::sync::watch::channel(false);
    let server = tokio::spawn(serve_hc2_listener(listener, service.clone(), tls, receiver));
    let adapter = GrpcMtlsAdapter::new(
        GrpcMtlsConfig::new(
            format!("https://{address}"),
            "localhost",
            pki.ca.as_bytes(),
            pki.client_cert.as_bytes(),
            pki.client_key.as_bytes(),
        )
        .unwrap(),
    );
    let client = Hc2Client::connect(&adapter, ClientConfig::new("hc2-client", "tenant-a"))
        .await
        .unwrap();
    let key = Bytes::from_static(b"order-1");
    let expected = Bytes::from_static(b"retained-value");
    assert_eq!(
        client.get(key.clone(), None).await.unwrap().unwrap().value,
        expected
    );

    values.state.lock().unwrap().progress.applied_prefix = 7;
    assert_eq!(
        client.get(key.clone(), None).await.unwrap_err().code(),
        ErrorCode::Unavailable
    );
    values.state.lock().unwrap().progress.applied_prefix = 8;
    let filters = network.0.lock().unwrap().filters();
    filters.isolate(2, [1, 3]);
    let budget = RequestOptions::with_timeout(Duration::from_millis(80)).unwrap();
    assert_eq!(
        client
            .get(key.clone(), Some(budget))
            .await
            .unwrap_err()
            .code(),
        ErrorCode::DeadlineExceeded
    );
    assert!(!filters.dropped().is_empty());
    assert_eq!(values.lookups.load(Ordering::SeqCst), 1);
    filters.recover();
    assert_eq!(
        client.get(key, None).await.unwrap().unwrap().value,
        expected
    );
    assert_eq!(values.lookups.load(Ordering::SeqCst), 2);
    drop(client);
    shutdown.send(true).unwrap();
    tokio::time::timeout(Duration::from_secs(3), server)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}
