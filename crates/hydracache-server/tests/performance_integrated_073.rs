use std::io::{BufRead, BufReader};
use std::net::{SocketAddr, TcpListener as StdTcpListener};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bytes::Bytes;
use hydracache::{
    CacheOptions, ClusterEpoch, ConsumerIsolation, ConsumerIsolationConfig, DurableValueStore,
    HydraCache, MemorySnapshotRequest, NamespaceQuota, PartitionId, ReplicatedValueRecord,
    ReplicatedValueStore, Tenant, TenantRoster, TombstoneBudget, TombstoneTracker,
};
use hydracache_client_hc2::{
    ClientConfig, GrpcMtlsAdapter, GrpcMtlsConfig, Hc2Client, SubscriptionEvent,
};
use hydracache_client_protocol::{
    ClientFrame, ClientRequest, ClientRequestEnvelope, ClientResponse, ClientResponseEnvelope,
    ClientWireMessage, Namespace, StructuredKey,
};
use hydracache_client_transport_axum::{
    ClientIdentity, ClientSurfaceLimits, ClientSurfaceState, CLIENT_DATA_PATH,
    HYDRACACHE_ADMIN_HEADER, HYDRACACHE_CLIENT_ID_HEADER, HYDRACACHE_TENANT_HEADER,
};
use hydracache_server::{ADMIN_DRAIN_PATH, ADMIN_METRICS_PATH};
use rcgen::{
    BasicConstraints, CertificateParams, CertifiedIssuer, ExtendedKeyUsagePurpose, IsCa, KeyPair,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const OPERATIONS: usize = 1_000;
const PAYLOAD_BYTES: usize = 4_096;
const KEY_CARDINALITY: usize = 256;
const TAG_CARDINALITY: usize = 16;
const TTL: Duration = Duration::from_millis(50);
const CANARY: &str = "HYDRACACHE_CANARY_DEFECT";
const CANARY_MARKER: &str = "HC-CANARY-RED:W10";

struct TestPki {
    ca: String,
    server_cert: String,
    server_key: String,
    client_cert: String,
    client_key: String,
}

fn pki() -> TestPki {
    let mut ca_params = CertificateParams::new(Vec::<String>::new()).unwrap();
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    let ca = CertifiedIssuer::self_signed(ca_params, KeyPair::generate().unwrap()).unwrap();
    let server_key = KeyPair::generate().unwrap();
    let mut server_params = CertificateParams::new(vec!["localhost".to_owned()]).unwrap();
    server_params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
    let server_cert = server_params.signed_by(&server_key, &ca).unwrap();
    let client_key = KeyPair::generate().unwrap();
    let mut client_params = CertificateParams::new(vec!["w10-client".to_owned()]).unwrap();
    client_params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ClientAuth];
    let client_cert = client_params.signed_by(&client_key, &ca).unwrap();
    TestPki {
        ca: ca.pem(),
        server_cert: server_cert.pem(),
        server_key: server_key.serialize_pem(),
        client_cert: client_cert.pem(),
        client_key: client_key.serialize_pem(),
    }
}

fn reserve_addrs<const N: usize>() -> ([SocketAddr; N], Vec<StdTcpListener>) {
    let listeners = (0..N)
        .map(|_| StdTcpListener::bind("127.0.0.1:0").unwrap())
        .collect::<Vec<_>>();
    let addrs = listeners
        .iter()
        .map(|listener| listener.local_addr().unwrap())
        .collect::<Vec<_>>()
        .try_into()
        .expect("reservation count");
    (addrs, listeners)
}

fn temp_root(label: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "hydracache-w10-{label}-{}-{unique}",
        std::process::id()
    ))
}

fn write_pki(root: &Path, material: &TestPki) -> (PathBuf, PathBuf, PathBuf) {
    std::fs::create_dir_all(root).unwrap();
    let cert = root.join("server.pem");
    let key = root.join("server.key");
    let ca = root.join("clients.pem");
    std::fs::write(&cert, &material.server_cert).unwrap();
    std::fs::write(&key, &material.server_key).unwrap();
    std::fs::write(&ca, &material.ca).unwrap();
    (cert, key, ca)
}

struct Daemon {
    child: Child,
    root: PathBuf,
    client_addr: SocketAddr,
    admin_addr: SocketAddr,
    hc2_addr: SocketAddr,
    redis_addr: SocketAddr,
    material: TestPki,
}

impl Daemon {
    fn start(label: &str) -> Self {
        let root = temp_root(label);
        let material = pki();
        let (cert, key, ca) = write_pki(&root, &material);
        let ([client_addr, admin_addr, hc2_addr, redis_addr], reservations) = reserve_addrs::<4>();
        drop(reservations);
        let mut child = Command::new(env!("CARGO_BIN_EXE_hydracache-server"))
            .env("HYDRACACHE_CLIENT_API_ENABLED", "true")
            .env("HYDRACACHE_LISTEN_ADDR", client_addr.to_string())
            .env("HYDRACACHE_ADMIN_API_ENABLED", "true")
            .env("HYDRACACHE_ADMIN_ADDR", admin_addr.to_string())
            .env("HYDRACACHE_HC2_ENABLED", "true")
            .env("HYDRACACHE_HC2_ADDR", hc2_addr.to_string())
            .env("HYDRACACHE_HC2_CLUSTER_ID", "w10-integrated")
            .env("HYDRACACHE_REDIS_API_ENABLED", "true")
            .env("HYDRACACHE_REDIS_ADDR", redis_addr.to_string())
            .env("HYDRACACHE_TLS_ENABLED", "true")
            .env("HYDRACACHE_TLS_CERT_PATH", cert)
            .env("HYDRACACHE_TLS_KEY_PATH", key)
            .env("HYDRACACHE_TLS_CA_PATH", ca)
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let mut ready = String::new();
        BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut ready)
            .unwrap();
        assert_eq!(ready.trim(), r#"{"status":"ok"}"#);
        Self {
            child,
            root,
            client_addr,
            admin_addr,
            hc2_addr,
            redis_addr,
            material,
        }
    }

    async fn hc2(&self, client_id: &str) -> Hc2Client {
        let adapter = GrpcMtlsAdapter::new(
            GrpcMtlsConfig::new(
                format!("https://{}", self.hc2_addr),
                "localhost",
                self.material.ca.as_bytes(),
                self.material.client_cert.as_bytes(),
                self.material.client_key.as_bytes(),
            )
            .unwrap(),
        );
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            match Hc2Client::connect(&adapter, ClientConfig::new(client_id, "tenant-a")).await {
                Ok(client) => return client,
                Err(error) if tokio::time::Instant::now() < deadline => {
                    let _ = error;
                    tokio::time::sleep(Duration::from_millis(25)).await;
                }
                Err(error) => panic!("HC/2 listener did not become ready: {error}"),
            }
        }
    }

    async fn metrics(&self) -> String {
        reqwest::get(format!("http://{}{}", self.admin_addr, ADMIN_METRICS_PATH))
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .text()
            .await
            .unwrap()
    }

    async fn shutdown(mut self) {
        let response = reqwest::Client::new()
            .post(format!("http://{}{}", self.admin_addr, ADMIN_DRAIN_PATH))
            .header(HYDRACACHE_CLIENT_ID_HEADER, "w10-operator")
            .header(HYDRACACHE_TENANT_HEADER, "system")
            .header(HYDRACACHE_ADMIN_HEADER, "true")
            .send()
            .await
            .unwrap();
        assert!(response.status().is_success());
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                assert!(status.success(), "daemon exited with {status}");
                break;
            }
            assert!(std::time::Instant::now() < deadline, "daemon drain timeout");
            thread::sleep(Duration::from_millis(20));
        }
        std::fs::remove_dir_all(&self.root).unwrap();
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn hc1_frame(request: ClientRequestEnvelope) -> Vec<u8> {
    ClientFrame::from_message(&ClientWireMessage::Request(request))
        .unwrap()
        .encode()
        .unwrap()
        .to_vec()
}

async fn hc1_put(daemon: &Daemon, index: usize, value: &[u8]) {
    let request = ClientRequestEnvelope::new(
        format!("w10-hc1-{index}"),
        ClientRequest::Put {
            ns: Namespace::new("w10").unwrap(),
            key: StructuredKey::new(vec![hex_key("hc1", index)]).unwrap(),
            value: value.to_vec(),
            ttl_ms: None,
            dimensions: Vec::new(),
        },
    );
    let response = reqwest::Client::new()
        .post(format!("http://{}{}", daemon.client_addr, CLIENT_DATA_PATH))
        .header(HYDRACACHE_CLIENT_ID_HEADER, "w10-hc1")
        .header(HYDRACACHE_TENANT_HEADER, "tenant-a")
        .body(hc1_frame(request))
        .send()
        .await
        .unwrap();
    let status = response.status();
    let body = response.bytes().await.unwrap();
    assert!(status.is_success(), "HC/1 failed: {status} {body:?}");
    let frame = ClientFrame::decode(&body, 8 * 1024 * 1024).unwrap();
    let ClientWireMessage::Response(ClientResponseEnvelope { result, .. }) =
        frame.decode_message().unwrap()
    else {
        panic!("HC/1 returned a non-response frame");
    };
    assert!(matches!(result, Ok(ClientResponse::Stored)));
}

fn hex_key(prefix: &str, index: usize) -> String {
    format!("{prefix}-{index:04}")
        .bytes()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

async fn resp_pipeline(addr: SocketAddr, operations: usize, value: &[u8]) {
    let mut input = Vec::new();
    for index in 0..operations {
        let key = format!("resp-{}", index % KEY_CARDINALITY);
        input.extend_from_slice(
            format!(
                "*3\r\n$3\r\nSET\r\n${}\r\n{}\r\n${}\r\n",
                key.len(),
                key,
                value.len()
            )
            .as_bytes(),
        );
        input.extend_from_slice(value);
        input.extend_from_slice(b"\r\n");
    }
    input.extend_from_slice(b"*1\r\n$4\r\nQUIT\r\n");
    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    stream.write_all(&input).await.unwrap();
    stream.shutdown().await.unwrap();
    let mut output = Vec::new();
    stream.read_to_end(&mut output).await.unwrap();
    assert_eq!(
        output
            .windows(5)
            .filter(|window| *window == b"+OK\r\n")
            .count(),
        operations + 1
    );
}

fn assert_complete(cell: &str, attempted: usize, mut succeeded: usize) {
    if std::env::var(CANARY).as_deref() == Ok("W10") {
        succeeded = succeeded.saturating_sub(1);
    }
    assert_eq!(attempted, OPERATIONS, "{cell}: wrong scheduled volume");
    assert_eq!(
        succeeded, attempted,
        "{CANARY_MARKER}: {cell} incomplete outcome accounting"
    );
    println!(
        "{{\"schema_version\":1,\"cell\":\"{cell}\",\"attempted\":{attempted},\"success\":{succeeded},\"rejected\":0,\"timeout\":0,\"late\":0,\"incomplete\":0}}"
    );
}

fn assert_client_surface_expiry_releases_quota() {
    let namespace = Namespace::new("w10-expiry").unwrap();
    let roster = TenantRoster::new(vec![Tenant::new("tenant-a")
        .unwrap()
        .allow_client("w10-expiry")
        .namespace("w10-expiry", NamespaceQuota::new(PAYLOAD_BYTES as u64, 1))])
    .unwrap();
    let state = ClientSurfaceState::with_isolation(
        ClientSurfaceLimits::default(),
        ConsumerIsolation::new(roster, ConsumerIsolationConfig::default()),
    )
    .unwrap();
    let identity = ClientIdentity::new("w10-expiry", "tenant-a").unwrap();
    state.set_cache_time_for_tests(Some(1_000));
    let put = |request_id: &str, key: &str, ttl_ms| {
        state.dispatch_verified_request(
            &identity,
            ClientRequestEnvelope::new(
                request_id,
                ClientRequest::Put {
                    ns: namespace.clone(),
                    key: StructuredKey::new(vec![hex_key(key, 0)]).unwrap(),
                    value: vec![0x73; PAYLOAD_BYTES],
                    ttl_ms,
                    dimensions: Vec::new(),
                },
            ),
        )
    };
    assert!(put("expiring", "old", Some(TTL.as_millis() as u64))
        .result
        .is_ok());
    state.advance_cache_time_for_tests(TTL.as_millis() as u64 + 1);
    let expired = state.retained_state_for_diagnostics();
    assert_eq!(expired.store_entries, 0);
    assert_eq!(expired.value_bytes, 0);
    assert!(put("refill", "new", None).result.is_ok());
    let cleanup = state.dispatch_verified_request(
        &identity,
        ClientRequestEnvelope::new(
            "cleanup",
            ClientRequest::Invalidate {
                ns: namespace,
                key: StructuredKey::new(vec![hex_key("new", 0)]).unwrap(),
            },
        ),
    );
    assert!(cleanup.result.is_ok());
    let final_state = state.retained_state_for_diagnostics();
    assert_eq!(final_state.store_entries, 0);
    assert_eq!(final_state.value_bytes, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn event_delivery() {
    let daemon = Daemon::start("event-delivery");
    let client = daemon.hc2("w10-event").await;
    let mut subscription = client
        .subscribe(Bytes::from_static(b"event/"), 0)
        .await
        .unwrap();
    let payload = Bytes::from(vec![0x73; PAYLOAD_BYTES]);
    let mut watermark = subscription.initial_watermark();
    const STALLED_EVENTS: usize = 300;
    let mut stalled_puts = tokio::task::JoinSet::new();
    for index in 0..STALLED_EVENTS {
        let client = client.clone();
        let key = Bytes::from(format!("event/{:04}", index % KEY_CARDINALITY));
        let payload = payload.clone();
        stalled_puts.spawn(async move { client.put(key, payload, None, None).await });
    }
    tokio::time::sleep(Duration::from_millis(100)).await;
    for index in 0..STALLED_EVENTS {
        let event = tokio::time::timeout(Duration::from_secs(10), subscription.next())
            .await
            .unwrap_or_else(|_| panic!("HC/2 stalled event {index} did not drain"))
            .expect("HC/2 event stream closed");
        let SubscriptionEvent::Event(event) = event else {
            panic!("HC/2 emitted a gap or close during stalled delivery");
        };
        assert!(event.key.starts_with(b"event/"));
        assert!(event.watermark > watermark);
        watermark = event.watermark;
    }
    while let Some(result) = stalled_puts.join_next().await {
        result.unwrap().unwrap();
    }
    for index in STALLED_EVENTS..OPERATIONS {
        let key = Bytes::from(format!("event/{:04}", index % KEY_CARDINALITY));
        client
            .put(key.clone(), payload.clone(), None, None)
            .await
            .unwrap();
        let event = tokio::time::timeout(Duration::from_secs(2), subscription.next())
            .await
            .expect("HC/2 event timeout")
            .expect("HC/2 event stream closed");
        let SubscriptionEvent::Event(event) = event else {
            panic!("HC/2 emitted a gap or close during ordered delivery");
        };
        assert_eq!(event.key, key);
        assert!(event.watermark > watermark);
        watermark = event.watermark;
    }
    let metrics = client.metrics();
    assert_eq!(metrics.events, OPERATIONS as u64);
    assert_eq!(metrics.dropped_events, 0);
    subscription.close();
    client.close();
    for _ in 0..100 {
        let retained = client.retained_state();
        if retained.closed
            && retained.pending_invocations == 0
            && retained.pending_subscriptions == 0
            && retained.active_subscriptions == 0
            && retained.outbound_buffered_items == 0
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let retained = client.retained_state();
    assert!(retained.closed);
    assert_eq!(retained.pending_invocations, 0);
    assert_eq!(retained.pending_subscriptions, 0);
    assert_eq!(retained.active_subscriptions, 0);
    assert_eq!(retained.outbound_buffered_items, 0);
    for _ in 0..100 {
        let metrics = daemon.metrics().await;
        if metrics.contains("hydracache_hc2_connections{transport=\"grpc_bidirectional\"} 0")
            && metrics.contains("hydracache_hc2_subscriptions{transport=\"grpc_bidirectional\"} 0")
            && metrics
                .contains("hydracache_hc2_pending_invocations{transport=\"grpc_bidirectional\"} 0")
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let server_metrics = daemon.metrics().await;
    assert!(
        server_metrics.contains("hydracache_hc2_connections{transport=\"grpc_bidirectional\"} 0")
    );
    assert!(
        server_metrics.contains("hydracache_hc2_subscriptions{transport=\"grpc_bidirectional\"} 0")
    );
    assert!(server_metrics
        .contains("hydracache_hc2_pending_invocations{transport=\"grpc_bidirectional\"} 0"));
    assert_complete("event-delivery", OPERATIONS, OPERATIONS);
    daemon.shutdown().await;
}

#[tokio::test]
async fn expiry_tag_accounting() {
    assert_client_surface_expiry_releases_quota();
    let cache = HydraCache::local().max_capacity(2_000).build();
    let payload = Bytes::from(vec![0x73; PAYLOAD_BYTES]);
    let mut attempted = 0;
    for index in 0..400 {
        cache
            .put_encoded(
                &format!("entry-{}", index % KEY_CARDINALITY),
                payload.clone(),
                CacheOptions::new().tag(format!("tag-{}", index % TAG_CARDINALITY)),
            )
            .await
            .unwrap();
        attempted += 1;
    }
    for index in 0..200 {
        let _ = cache
            .get_encoded(&format!("entry-{}", index % KEY_CARDINALITY))
            .await
            .unwrap();
        attempted += 1;
    }
    for index in 0..160 {
        cache
            .invalidate_tag(&format!("tag-{}", index % TAG_CARDINALITY))
            .await
            .unwrap();
        attempted += 1;
    }
    for index in 0..120 {
        cache
            .put_encoded(
                &format!("ttl-{}", index % KEY_CARDINALITY),
                payload.clone(),
                CacheOptions::new()
                    .ttl(TTL)
                    .tag(format!("tag-{}", index % TAG_CARDINALITY)),
            )
            .await
            .unwrap();
        attempted += 1;
    }
    tokio::time::sleep(TTL + Duration::from_millis(20)).await;
    for index in 0..60 {
        assert!(cache
            .get_encoded(&format!("ttl-{}", index % KEY_CARDINALITY))
            .await
            .unwrap()
            .is_none());
        attempted += 1;
    }
    for index in 0..60 {
        cache
            .put_encoded(
                &format!("refill-{}", index % KEY_CARDINALITY),
                payload.clone(),
                CacheOptions::new().tag(format!("tag-{}", index % TAG_CARDINALITY)),
            )
            .await
            .unwrap();
        attempted += 1;
    }
    cache.flush().await.unwrap();
    let reconciliation = cache.reconcile_memory_footprint().await.unwrap();
    assert!(reconciliation.matched);
    assert_eq!(reconciliation.exact_entries, 0);
    assert_eq!(reconciliation.exact_tag_memberships, 0);
    assert_eq!(reconciliation.exact_estimated_retained_bytes, 0);
    let barrier = cache.memory_snapshot_barrier().unwrap();
    let snapshot = cache
        .memory_footprint_snapshot(MemorySnapshotRequest::Exact {
            acknowledged_epoch: barrier.epoch,
        })
        .await
        .unwrap();
    assert_eq!(snapshot.live_entries, 0);
    assert_eq!(snapshot.tag_memberships, 0);
    assert_eq!(snapshot.estimated_retained_bytes, 0);
    assert_complete("expiry-tag-accounting", attempted, attempted);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mixed_protocol() {
    let daemon = Daemon::start("mixed-protocol");
    let hc2 = daemon.hc2("w10-mixed").await;
    let mut subscription = hc2
        .subscribe(Bytes::from_static(b"mixed/"), 0)
        .await
        .unwrap();
    let payload = Bytes::from(vec![0x73; PAYLOAD_BYTES]);
    let mut hc2_success = 0;
    for index in 0..350 {
        let key = Bytes::from(format!("mixed/{:04}", index % KEY_CARDINALITY));
        hc2.put(key.clone(), payload.clone(), None, None)
            .await
            .unwrap();
        assert!(matches!(
            tokio::time::timeout(Duration::from_secs(2), subscription.next())
                .await
                .unwrap()
                .unwrap(),
            SubscriptionEvent::Event(event) if event.key == key
        ));
        hc2_success += 1;
    }
    resp_pipeline(daemon.redis_addr, 300, &payload).await;
    let resp_success = 300;
    for index in 0..150 {
        hc1_put(&daemon, index, &payload).await;
    }
    let hc1_success = 150;
    let direct = HydraCache::local().max_capacity(512).build();
    for index in 0..100 {
        direct
            .put_encoded(
                &format!("direct-{}", index % KEY_CARDINALITY),
                payload.clone(),
                CacheOptions::new().tag(format!("tag-{}", index % TAG_CARDINALITY)),
            )
            .await
            .unwrap();
    }
    let direct_success = 100;
    for index in 0..50 {
        direct
            .invalidate_tag(&format!("tag-{}", index % TAG_CARDINALITY))
            .await
            .unwrap();
    }
    let tag_success = 50;
    for index in 0..17 {
        direct
            .put_encoded(
                &format!("mixed-ttl-{index}"),
                payload.clone(),
                CacheOptions::new().ttl(TTL),
            )
            .await
            .unwrap();
    }
    tokio::time::sleep(TTL + Duration::from_millis(20)).await;
    for index in 0..17 {
        assert!(direct
            .get_encoded(&format!("mixed-ttl-{index}"))
            .await
            .unwrap()
            .is_none());
    }
    for index in 0..16 {
        direct
            .put_encoded(
                &format!("mixed-refill-{index}"),
                payload.clone(),
                CacheOptions::new(),
            )
            .await
            .unwrap();
    }
    let ttl_success = 50;
    let management_truth = daemon.metrics().await;
    assert!(management_truth
        .contains("hydracache_hc2_rejected_frames_total{transport=\"grpc_bidirectional\"} 0"));
    assert!(management_truth
        .contains("hydracache_hc2_pending_invocations{transport=\"grpc_bidirectional\"} 0"));
    subscription.close();
    hc2.close();
    direct.flush().await.unwrap();
    let reconciliation = direct.reconcile_memory_footprint().await.unwrap();
    assert!(reconciliation.matched);
    assert_eq!(reconciliation.exact_entries, 0);
    let succeeded =
        hc2_success + resp_success + hc1_success + direct_success + tag_success + ttl_success;
    assert_eq!(
        [
            hc2_success,
            resp_success,
            hc1_success,
            direct_success,
            tag_success,
            ttl_success
        ],
        [350, 300, 150, 100, 50, 50]
    );
    assert_complete("mixed-protocol", succeeded, succeeded);
    daemon.shutdown().await;
}

fn reopen(path: &Path, budget: u64) -> DurableValueStore {
    for _ in 0..100 {
        match DurableValueStore::open_with_budget(path, budget) {
            Ok(store) => return store,
            Err(error) if error.to_string().contains("could not acquire lock") => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("durable reopen failed: {error}"),
        }
    }
    panic!("durable lock was not released");
}

#[test]
fn durable_companion() {
    let root = temp_root("durable-companion");
    let budget = 2 * 1024 * 1024;
    let mut store = DurableValueStore::open_with_budget(&root, budget).unwrap();
    let record = |version: u64, fill: u8| {
        ReplicatedValueRecord::value(
            PartitionId::new(1),
            version,
            ClusterEpoch::new(1),
            vec![fill; PAYLOAD_BYTES],
        )
    };
    let mut attempted = 0;
    for index in 0..400 {
        store
            .upsert(
                format!("key-{}", index % KEY_CARDINALITY),
                record(1 + index as u64, index as u8),
            )
            .unwrap();
        attempted += 1;
    }
    for index in 0..200 {
        assert!(store
            .get(&format!("key-{}", index % KEY_CARDINALITY))
            .unwrap()
            .is_some());
        attempted += 1;
    }
    for index in 0..150 {
        store
            .upsert(
                format!("key-{}", index % KEY_CARDINALITY),
                record(1_000 + index as u64, 0x7a),
            )
            .unwrap();
        attempted += 1;
    }
    let mut tracker = TombstoneTracker::new(TombstoneBudget::new(512, budget));
    for index in 0..100 {
        let key = format!("key-{index}");
        store
            .tombstone(
                &key,
                PartitionId::new(1),
                2_000 + index as u64,
                ClusterEpoch::new(2),
            )
            .unwrap();
        tracker.admit(&key, 2_000 + index as u64, 128, Some(ClusterEpoch::new(2)));
        attempted += 1;
    }
    let before_gc = store.total_bytes().unwrap();
    let mut reclaimed = 0;
    for _ in 0..50 {
        reclaimed += store
            .collect_tombstone_garbage(&mut tracker, ClusterEpoch::new(2), KEY_CARDINALITY)
            .unwrap()
            .reclaimed_bytes;
        attempted += 1;
    }
    assert!(reclaimed > 0);
    assert!(store.total_bytes().unwrap() < before_gc);
    assert!(tracker.is_empty());
    store.flush().unwrap();
    drop(store);
    let store = reopen(&root, budget);
    for index in 100..150 {
        assert!(store.get(&format!("key-{index}")).unwrap().is_some());
        attempted += 1;
    }

    let rejected_root = temp_root("durable-rejected");
    let mut rejected = DurableValueStore::open_with_budget(&rejected_root, 1).unwrap();
    for index in 0..49 {
        assert!(rejected
            .upsert(format!("rejected-{index}"), record(1, 0x33))
            .is_err());
        attempted += 1;
    }
    assert_eq!(rejected.rejected_total(), 49);
    drop(rejected);

    let corrupt_root = temp_root("durable-corrupt");
    let mut corrupt = DurableValueStore::open_with_budget(&corrupt_root, budget).unwrap();
    corrupt.upsert("corrupt", record(1, 0x44)).unwrap();
    corrupt
        .put_raw_record_for_test("corrupt", b"invalid-checksum-envelope")
        .unwrap();
    assert!(corrupt.get("corrupt").is_err());
    attempted += 1;
    assert_complete("durable-companion", attempted, attempted);
    drop(corrupt);
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
    std::fs::remove_dir_all(rejected_root).unwrap();
    std::fs::remove_dir_all(corrupt_root).unwrap();
}
