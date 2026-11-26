use std::io::{BufRead, BufReader, Read};
use std::net::{SocketAddr, TcpListener as StdTcpListener};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bytes::Bytes;
use hydracache::{
    ClusterEpoch, ClusterGeneration, ClusterMember, ClusterNodeId, ClusterReplicationStrategy,
    ClusterRole, RendezvousClusterOwnership,
};
use hydracache_client_hc2::{
    Capability, ClientConfig, ErrorCode, GrpcMtlsAdapter, GrpcMtlsConfig, Hc2Client, RequestOptions,
};
use hydracache_client_protocol::{
    ClientFrame, ClientRequest, ClientRequestEnvelope, ClientResponse, ClientResponseEnvelope,
    ClientWireMessage, Namespace, StructuredKey,
};
use hydracache_client_transport_axum::{
    CLIENT_DATA_PATH, HYDRACACHE_ADMIN_HEADER, HYDRACACHE_CLIENT_ID_HEADER,
    HYDRACACHE_TENANT_HEADER,
};
use hydracache_server::imap_replicated_read::ReplicaReadScope;
use hydracache_server::{ADMIN_CLUSTER_OVERVIEW_PATH, ADMIN_DRAIN_PATH, ADMIN_METRICS_PATH};
use sha2::{Digest, Sha256};
use tokio::sync::{Mutex, MutexGuard};

#[path = "support/hc2_pki.rs"]
mod hc2_pki;
use hc2_pki::{pki, write_pki};
#[path = "support/imap_hc2_campaign.rs"]
mod imap_hc2_campaign;

// These tests release reserved ports before a child process binds them. Keep
// the hand-off and the post-exit availability assertions isolated from the
// other tests in this binary so they cannot steal one another's addresses.
static DAEMON_PROCESS_LOCK: Mutex<()> = Mutex::const_new(());

fn daemon_process_guard() -> MutexGuard<'static, ()> {
    DAEMON_PROCESS_LOCK.blocking_lock()
}

async fn async_daemon_process_guard() -> MutexGuard<'static, ()> {
    DAEMON_PROCESS_LOCK.lock().await
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
        .expect("reservation count must match the requested address count");
    (addrs, listeners)
}

fn test_root(label: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    PathBuf::from(format!(
        "target/test-hc2-daemon-{label}-{}-{unique}",
        std::process::id()
    ))
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex_digest(&Sha256::digest(bytes))
}

fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn daemon_command(
    client: SocketAddr,
    admin: SocketAddr,
    hc2: SocketAddr,
    cert: &Path,
    key: &Path,
    ca: &Path,
) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_hydracache-server"));
    command
        .env("HYDRACACHE_CLIENT_API_ENABLED", "true")
        .env("HYDRACACHE_LISTEN_ADDR", client.to_string())
        .env("HYDRACACHE_ADMIN_API_ENABLED", "true")
        .env("HYDRACACHE_ADMIN_ADDR", admin.to_string())
        .env("HYDRACACHE_HC2_ENABLED", "true")
        .env("HYDRACACHE_HC2_ADDR", hc2.to_string())
        .env("HYDRACACHE_HC2_CLUSTER_ID", "daemon-proof")
        .env("HYDRACACHE_TLS_ENABLED", "true")
        .env("HYDRACACHE_TLS_CERT_PATH", cert)
        .env("HYDRACACHE_TLS_KEY_PATH", key)
        .env("HYDRACACHE_TLS_CA_PATH", ca)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

fn wait_success(child: &mut Child, timeout: Duration) {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            let mut stderr = String::new();
            if let Some(pipe) = child.stderr.as_mut() {
                pipe.read_to_string(&mut stderr).unwrap();
            }
            assert!(status.success(), "daemon failed: {status}; stderr={stderr}");
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "daemon did not exit after drain"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn member_hc2_has_no_node_local_fallback() {
    use hydracache_server::{
        ClusterAuthConfig, DistributedHc2Config, Hc2ClientPlaneConfig, ServerConfig,
        ServerConfigError, ServerRole, TlsConfig,
    };

    let root = test_root("distributed-hc2-config");
    std::fs::create_dir_all(&root).unwrap();
    let token = root.join("cluster-token");
    std::fs::write(&token, "configuration-test-token").unwrap();
    let mut config = ServerConfig {
        role: ServerRole::Member,
        seeds: vec!["127.0.0.1:7000".into()],
        storage_dir: Some(PathBuf::from("member-store")),
        hc2_client_plane: Hc2ClientPlaneConfig {
            enabled: true,
            ..Default::default()
        },
        tls: TlsConfig {
            enabled: true,
            cert_path: Some("server.pem".into()),
            key_path: Some("server.key".into()),
            ca_path: Some("ca.pem".into()),
            ..Default::default()
        },
        cluster_auth: ClusterAuthConfig {
            key_id: Some("key".into()),
            token_file: Some(token.clone()),
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(matches!(
        config.validate(),
        Err(ServerConfigError::InvalidDistributedHc2(_))
    ));
    config.distributed_hc2 = Some(DistributedHc2Config {
        tenant: "tenant-a".into(),
        client_cert_sha256: "a".repeat(64),
    });
    config.validate().unwrap();
    config.redis_api.enabled = true;
    assert!(matches!(
        config.validate(),
        Err(ServerConfigError::InvalidDistributedHc2(_))
    ));
    config.redis_api.enabled = false;
    config.distributed_hc2.as_mut().unwrap().client_cert_sha256 = "BAD".into();
    assert!(matches!(
        config.validate(),
        Err(ServerConfigError::InvalidDistributedHc2(_))
    ));
    std::fs::remove_file(token).unwrap();
    std::fs::remove_dir(root).unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn distributed_hc2_routes_public_requests_and_recovers_after_owner_kill() {
    if std::env::var("HYDRACACHE_RUN_DAEMON_PROCESS_E2E").as_deref() != Ok("1") {
        return;
    }
    let profile = imap_hc2_campaign::Profile::from_env();
    let _guard = async_daemon_process_guard().await;
    let mut cluster = MemberCluster::start("distributed-imap").await;
    let hot = cluster.key_for(0, Some(1), "w10i-\0-");
    let control = cluster.key_for(1, Some(2), "w10i-\0-");
    let clients = &mut cluster.clients;
    let initial = Bytes::from_static(b"initial");
    let winner = Bytes::from_static(b"confirmed-before-kill");
    let recovered = Bytes::from_static(b"confirmed-after-kill");
    wait_w10i_get(&clients[1], control.clone(), None).await;
    assert!(
        clients[2]
            .put(hot.clone(), initial.clone(), None, None)
            .await
            .unwrap()
            .applied
    );
    wait_w10i_get(&clients[1], hot.clone(), Some(&initial)).await;
    assert_eq!(
        clients[0]
            .get(hot.clone(), None)
            .await
            .unwrap()
            .unwrap()
            .value,
        initial
    );
    let not_admitted = connect_w10i(&cluster.adapters[2], "different-tenant").await;
    assert_eq!(
        not_admitted
            .get(hot.clone(), None)
            .await
            .unwrap_err()
            .code(),
        ErrorCode::Unauthorized
    );
    assert_eq!(
        clients[2]
            .put(
                hot.clone(),
                winner.clone(),
                Some(Duration::from_secs(1)),
                None
            )
            .await
            .unwrap_err()
            .code(),
        ErrorCode::Unsupported
    );
    let options = RequestOptions {
        timeout: Duration::from_secs(2),
        idempotency_key: Bytes::from_static(b"w10i-before-kill"),
    };
    assert!(
        clients[1]
            .compare_and_set(
                hot.clone(),
                initial.clone(),
                winner.clone(),
                None,
                Some(options.clone())
            )
            .await
            .unwrap()
            .applied
    );
    assert!(
        !clients[2]
            .compare_and_set(hot.clone(), initial.clone(), recovered.clone(), None, None)
            .await
            .unwrap()
            .applied
    );

    let started = tokio::time::Instant::now();
    let load = tokio::spawn(imap_hc2_campaign::exercise_control(
        clients[2].clone(),
        clients[1].clone(),
        control,
        started,
        profile.duration,
        profile.ledger().unwrap(),
    ));
    tokio::time::sleep_until(started + profile.duration / 3).await;
    let killed = cluster.daemons[0].0.id();
    let killed_at_ms = started.elapsed().as_millis() as u64;
    cluster.daemons[0].0.kill().unwrap();
    cluster.daemons[0].0.wait().unwrap();
    wait_w10i_get(&clients[2], hot.clone(), Some(&winner)).await;
    let replay = retry_w10i(|| {
        clients[1].compare_and_set(
            hot.clone(),
            initial.clone(),
            winner.clone(),
            None,
            Some(options.clone()),
        )
    })
    .await;
    assert!(replay.applied);
    let ready = tokio::time::Instant::now() + Duration::from_secs(18);
    loop {
        match clients[2]
            .compare_and_set(hot.clone(), winner.clone(), recovered.clone(), None, None)
            .await
        {
            Ok(result) => {
                if !result.applied {
                    // An earlier unacknowledged conditional write can already have committed.
                    wait_w10i_get(&clients[1], hot.clone(), Some(&recovered)).await;
                }
                break;
            }
            Err(error) if tokio::time::Instant::now() < ready => {
                if clients[1]
                    .get(hot.clone(), None)
                    .await
                    .is_ok_and(|value| value.is_some_and(|entry| entry.value == recovered))
                {
                    break;
                }
                assert!(
                    matches!(
                        error.code(),
                        ErrorCode::Unavailable | ErrorCode::DeadlineExceeded
                    ),
                    "{error}"
                );
                tokio::time::sleep(Duration::from_millis(90)).await;
            }
            Err(error) => panic!("fresh write was still blocked after owner kill: {error}"),
        }
    }
    let recovered_at_ms = started.elapsed().as_millis() as u64;
    cluster.daemons[0] = cluster.spec.spawn(0);
    clients[0] = connect_w10i(&cluster.adapters[0], "w10i").await;
    wait_w10i_get(&clients[0], hot.clone(), Some(&recovered)).await;
    let _removed =
        retry_w10i(|| clients[2].remove_if_value(hot.clone(), recovered.clone(), None)).await;
    wait_w10i_get(&clients[0], hot, None).await;
    let rejoined = cluster.daemons[0].0.id();
    let rejoined_at_ms = started.elapsed().as_millis() as u64;
    let counts = load.await.unwrap();
    assert!(counts.acknowledgements > 0);
    profile
        .receipt(
            started,
            &counts,
            imap_hc2_campaign::Fault {
                killed_pid: killed,
                killed_at_ms,
                recovered_at_ms,
                rejoined_pid: rejoined,
                rejoined_at_ms,
            },
        )
        .unwrap();
    println!("W10i public HC/2 killed_pid={killed} rejoined_pid={rejoined} checks={counts:?} retained_cas_and_tombstone=true");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn w10g_distributed_bulk_and_cluster_listeners_recover_under_load() {
    use hydracache_client_hc2::{BatchOperation, SubscriptionEvent};

    if std::env::var("HYDRACACHE_RUN_DAEMON_PROCESS_E2E").as_deref() != Ok("1") {
        return;
    }
    let _guard = async_daemon_process_guard().await;
    let mut cluster = MemberCluster::start("w10g-bulk-listener").await;
    let prefix = "w10g-event/";
    let hot = cluster.key_for(0, Some(1), &format!("{prefix}hot/"));
    let control = cluster.key_for(1, Some(2), &format!("{prefix}control/"));
    let third = cluster.key_for(2, Some(1), &format!("{prefix}third/"));
    let load_key = cluster.key_for(1, Some(2), "w10g-load/");
    let client = &cluster.clients[2];
    let value = |text: &'static [u8]| Bytes::from_static(text);
    let put = |key: &Bytes, bytes: &'static [u8]| BatchOperation::Put {
        key: key.clone(),
        value: value(bytes),
        ttl: None,
    };
    let get = |key: &Bytes| BatchOperation::Get { key: key.clone() };
    wait_w10i_get(client, control.clone(), None).await;
    let mut events = client
        .subscribe_with_values(Bytes::from_static(b"w10g-event/"), 0, true)
        .await
        .unwrap();
    let mut key_only = client
        .subscribe(Bytes::from_static(b"w10g-event/control/"), 0)
        .await
        .unwrap();
    let slow = client
        .subscribe(Bytes::from_static(b"w10g-load/"), 0)
        .await
        .unwrap();

    let invalid_key = value(b"w10g-invalid");
    let invalid = vec![
        put(&invalid_key, b"must-not-apply"),
        BatchOperation::Put {
            key: third.clone(),
            value: value(b"ttl"),
            ttl: Some(Duration::from_secs(1)),
        },
    ];
    assert_eq!(
        client
            .batch_outcomes(invalid, None)
            .await
            .unwrap_err()
            .code(),
        ErrorCode::Unsupported
    );
    wait_w10i_get(client, invalid_key, None).await;
    let options = RequestOptions {
        timeout: Duration::from_secs(8),
        idempotency_key: value(b"w10g-initial"),
    };
    let mixed = vec![
        put(&hot, b"one"),
        put(&third, b"three"),
        put(&control, b"two"),
        get(&hot),
        put(&hot, b"one-again"),
        get(&control),
        get(&hot),
    ];
    let items = client
        .batch_outcomes(mixed.clone(), Some(options.clone()))
        .await
        .unwrap();
    assert_eq!(items.len(), mixed.len());
    for (index, item) in items.iter().enumerate() {
        assert_eq!(item.item_id as usize, index + 1);
        assert!(item.topology_epoch > 0);
        let result = item.result.as_ref().unwrap();
        match index {
            3 => assert_eq!(result.value.as_ref().unwrap().value, value(b"one")),
            5 => assert_eq!(result.value.as_ref().unwrap().value, value(b"two")),
            6 => assert_eq!(result.value.as_ref().unwrap().value, value(b"one-again")),
            _ => assert!(result.mutation.unwrap().applied),
        }
    }
    let mut observed = std::collections::BTreeMap::<Bytes, Vec<Bytes>>::new();
    let mut watermark = 0;
    for _ in 0..4 {
        let SubscriptionEvent::Event(event) = next_w10g_event(&mut events).await else {
            panic!("initial certified mutations require a continuous stream");
        };
        assert_eq!(event.watermark, watermark + 1);
        watermark = event.watermark;
        observed.entry(event.key).or_default().push(event.value);
    }
    assert_eq!(observed[&hot], [value(b"one"), value(b"one-again")]);
    assert_eq!(observed[&third], [value(b"three")]);
    assert_eq!(observed[&control], [value(b"two")]);
    let SubscriptionEvent::Event(projected) = next_w10g_event(&mut key_only).await else {
        panic!("expected filtered key event");
    };
    assert_eq!(projected.key, control);
    assert!(projected.value.is_empty());
    let repeated = cluster.clients[1]
        .batch_outcomes(mixed, Some(options))
        .await
        .unwrap();
    for index in [0, 1, 2, 4] {
        assert!(
            repeated[index]
                .result
                .as_ref()
                .unwrap()
                .mutation
                .unwrap()
                .applied
        );
    }
    assert!(
        tokio::time::timeout(Duration::from_millis(250), events.next())
            .await
            .is_err()
    );
    for count in [1, 8, 64, 256] {
        let keys = [&hot, &control, &third];
        let reads = (0..count)
            .map(|index| get(keys[index % keys.len()]))
            .collect();
        let read_options = RequestOptions::with_timeout(Duration::from_secs(12)).unwrap();
        let results = client
            .batch_outcomes(reads, Some(read_options))
            .await
            .unwrap();
        assert_eq!(results.len(), count);
        for (index, item) in results.into_iter().enumerate() {
            assert_eq!(item.item_id as usize, index + 1);
            let expected = [value(b"one-again"), value(b"two"), value(b"three")];
            assert_eq!(
                item.result.unwrap().value.unwrap().value,
                expected[index % 3]
            );
        }
    }

    let writer = client.clone();
    let load = tokio::spawn(async move {
        let mut accepted = 0;
        let stop = tokio::time::Instant::now() + Duration::from_secs(8);
        while tokio::time::Instant::now() < stop {
            if writer
                .put(load_key.clone(), value(b"load"), None, None)
                .await
                .is_ok_and(|result| result.applied)
            {
                accepted += 1;
            }
            tokio::time::sleep(Duration::from_millis(6)).await;
        }
        accepted
    });
    tokio::time::sleep(Duration::from_millis(250)).await;
    let dead_pid = cluster.daemons[0].0.id();
    cluster.daemons[0].0.kill().unwrap();
    cluster.daemons[0].0.wait().unwrap();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(4);
    loop {
        let partial = client
            .batch_outcomes(vec![get(&third), get(&hot), get(&control)], None)
            .await;
        if let Ok(partial) = partial {
            if partial[0].result.is_ok() && partial[1].result.is_err() && partial[2].result.is_ok()
            {
                assert_eq!(
                    partial[0]
                        .result
                        .as_ref()
                        .unwrap()
                        .value
                        .as_ref()
                        .unwrap()
                        .value,
                    value(b"three")
                );
                assert_eq!(
                    partial[2]
                        .result
                        .as_ref()
                        .unwrap()
                        .value
                        .as_ref()
                        .unwrap()
                        .value,
                    value(b"two")
                );
                break;
            }
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "owner outage did not expose a partial batch"
        );
        tokio::time::sleep(Duration::from_millis(30)).await;
    }
    wait_w10i_get(client, hot.clone(), Some(&value(b"one-again"))).await;
    assert!(
        matches!(next_w10g_event(&mut events).await, SubscriptionEvent::Gap { after_watermark, .. } if after_watermark == watermark)
    );
    assert!(
        retry_w10i(|| client.put(hot.clone(), value(b"recovered"), None, None))
            .await
            .applied
    );
    let resumed = wait_w10g_mutation(&mut events, &hot).await;
    assert_eq!(resumed.value, value(b"recovered"));

    cluster.daemons[0] = cluster.spec.spawn(0);
    let rejoined = connect_w10i(&cluster.adapters[0], "w10i").await;
    wait_w10i_get(&rejoined, hot.clone(), Some(&value(b"recovered"))).await;
    let mut reconnect = rejoined
        .subscribe_with_values(value(b"w10g-event/"), resumed.watermark, true)
        .await
        .unwrap();
    assert!(
        matches!(next_w10g_event(&mut reconnect).await, SubscriptionEvent::Gap { after_watermark, .. } if after_watermark == resumed.watermark)
    );
    assert!(
        retry_w10i(|| client.delete(control.clone(), None))
            .await
            .applied
    );
    let removed = wait_w10g_mutation(&mut reconnect, &control).await;
    assert!(removed.removed && removed.value.is_empty());
    wait_w10i_get(&rejoined, control, None).await;
    drop((events, slow, key_only, reconnect));
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let http = reqwest::Client::new();
    loop {
        let mut empty = true;
        for index in [0, 2] {
            let metrics = http
                .get(format!(
                    "http://{}{ADMIN_METRICS_PATH}",
                    cluster.spec.addresses[6 + index]
                ))
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap();
            empty &= metrics
                .contains("hydracache_hc2_subscriptions{transport=\"grpc_bidirectional\"} 0");
        }
        if empty {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "clustered subscriptions were retained after close"
        );
        tokio::time::sleep(Duration::from_millis(40)).await;
    }
    let accepted = load.await.unwrap();
    assert!(
        accepted >= 20,
        "slow listener impaired foreground writes: {accepted}"
    );
    println!("W10g bulk sizes=1/8/64/256 partial_owner_failure=true listener_gap_and_reconnect=true killed_pid={dead_pid} rejoined_pid={} foreground_acks={accepted} retained_subscriptions=0", cluster.daemons[0].0.id());
}

async fn next_w10g_event(
    subscription: &mut hydracache_client_hc2::Subscription,
) -> hydracache_client_hc2::SubscriptionEvent {
    tokio::time::timeout(Duration::from_secs(8), subscription.next())
        .await
        .expect("cluster subscription did not deliver before its deadline")
        .expect("cluster subscription closed")
}

async fn wait_w10g_mutation(
    subscription: &mut hydracache_client_hc2::Subscription,
    key: &Bytes,
) -> hydracache_client_hc2::CacheEvent {
    for _ in 0..8 {
        if let hydracache_client_hc2::SubscriptionEvent::Event(event) =
            next_w10g_event(subscription).await
        {
            if &event.key == key {
                return event;
            }
        }
    }
    panic!("missing cluster event for the target key");
}

struct MemberClusterSpec {
    root: PathBuf,
    addresses: [SocketAddr; 12],
    names: Vec<String>,
    seeds: String,
    server_cert: PathBuf,
    server_key: PathBuf,
    ca: PathBuf,
}

impl MemberClusterSpec {
    fn spawn(&self, index: usize) -> ChildGuard {
        let log = std::fs::File::options()
            .create(true)
            .append(true)
            .open(self.root.join(format!("member-{index}.log")))
            .unwrap();
        let mut command = daemon_command(
            self.addresses[3 + index],
            self.addresses[6 + index],
            self.addresses[9 + index],
            &self.server_cert,
            &self.server_key,
            &self.ca,
        );
        ChildGuard(
            command
                .env("HYDRACACHE_CLIENT_API_ENABLED", "false")
                .env("HYDRACACHE_ROLE", "member")
                .env("HYDRACACHE_CLUSTER_ADDR", self.addresses[index].to_string())
                .env(
                    "HYDRACACHE_CLUSTER_ADVERTISE_ADDR",
                    self.addresses[index].to_string(),
                )
                .env("HYDRACACHE_NODE_ID", &self.names[index])
                .env("HYDRACACHE_SEEDS", &self.seeds)
                .env("HYDRACACHE_STORAGE_DIR", self.root.join(&self.names[index]))
                .env("HYDRACACHE_CLUSTER_AUTH_KEY_ID", "w10i-fixture")
                .env(
                    "HYDRACACHE_CLUSTER_AUTH_TOKEN_FILE",
                    self.root.join("cluster-token"),
                )
                .env(
                    "HYDRACACHE_DISTRIBUTED_HC2_CONFIG",
                    self.root.join("imap.toml"),
                )
                .env("HYDRACACHE_CLIENT_BATCH_PROFILE", "w11-256")
                .stdout(Stdio::from(log.try_clone().unwrap()))
                .stderr(Stdio::from(log))
                .spawn()
                .unwrap(),
        )
    }
}

struct MemberCluster {
    spec: MemberClusterSpec,
    daemons: Vec<ChildGuard>,
    adapters: Vec<GrpcMtlsAdapter>,
    clients: Vec<Hc2Client>,
}

impl MemberCluster {
    async fn start(label: &str) -> Self {
        let root = test_root(label);
        let pki = pki();
        let (server_cert, server_key, ca) = write_pki(&root, &pki);
        let mut pem = std::io::BufReader::new(pki.client_cert.as_bytes());
        let leaf = <rustls::pki_types::CertificateDer as rustls::pki_types::pem::PemObject>
            ::from_pem_reader(&mut pem).unwrap();
        std::fs::write(
            root.join("imap.toml"),
            format!(
                "tenant = \"w10i\"\nclient_cert_sha256 = \"{}\"\n",
                sha256_hex(leaf.as_ref())
            ),
        )
        .unwrap();
        std::fs::write(
            root.join("cluster-token"),
            format!("fixture-only-{}", root.display()),
        )
        .unwrap();
        let (addresses, reservations) = reserve_addrs::<12>();
        let names = addresses[..3]
            .iter()
            .map(|addr| format!("member-{}", addr.to_string().replace(['.', ':'], "-")))
            .collect();
        let seeds = addresses[..3]
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let spec = MemberClusterSpec {
            root,
            addresses,
            names,
            seeds,
            server_cert,
            server_key,
            ca,
        };
        drop(reservations);
        let daemons = (0..3).map(|index| spec.spawn(index)).collect();
        let adapters: Vec<_> = addresses[9..]
            .iter()
            .map(|address| {
                GrpcMtlsAdapter::new(
                    GrpcMtlsConfig::new(
                        format!("https://{address}"),
                        "localhost",
                        pki.ca.as_bytes(),
                        pki.client_cert.as_bytes(),
                        pki.client_key.as_bytes(),
                    )
                    .unwrap(),
                )
            })
            .collect();
        let mut clients = Vec::new();
        for adapter in &adapters {
            clients.push(connect_w10i(adapter, "w10i").await);
        }
        println!("HC/2 member process artifacts {}", spec.root.display());
        let deadline = tokio::time::Instant::now() + Duration::from_secs(18);
        let status = reqwest::Client::new();
        let endpoint = format!("http://{}{ADMIN_CLUSTER_OVERVIEW_PATH}", addresses[7]);
        loop {
            if let Ok(response) = status.get(&endpoint).send().await {
                if let Ok(status) = response.json::<serde_json::Value>().await {
                    if status["members"]
                        .as_array()
                        .is_some_and(|members| members.len() == 3)
                    {
                        break;
                    }
                }
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "three members were never admitted"
            );
            tokio::time::sleep(Duration::from_millis(70)).await;
        }
        Self {
            spec,
            daemons,
            adapters,
            clients,
        }
    }

    fn key_for(&self, primary: usize, backup: Option<usize>, prefix: &str) -> Bytes {
        let scope = ReplicaReadScope::new(
            "w10i".to_owned(),
            Namespace::new("hc2").unwrap(),
            std::num::NonZeroU64::MIN,
            |_| None,
        );
        let members: Vec<_> = self
            .spec
            .names
            .iter()
            .map(|name| ClusterMember {
                node_id: ClusterNodeId::new(name),
                generation: ClusterGeneration::new(1),
                role: ClusterRole::Member,
                epoch: ClusterEpoch::new(3),
                endpoints: Default::default(),
                metadata: Default::default(),
            })
            .collect();
        (0..5000)
            .find_map(|n| {
                let key = format!("{prefix}{n}").into_bytes();
                let (partition, _) = scope.locate(key.clone()).unwrap();
                let ranking = RendezvousClusterOwnership
                    .replicas_for_key(&format!("partition-{}", partition.value()), &members, 2)
                    .unwrap();
                (ranking.primary == members[primary].node_id
                    && backup.is_none_or(|b| ranking.backups == [members[b].node_id.clone()]))
                .then(|| Bytes::from(key))
            })
            .unwrap()
    }
}

async fn connect_w10i(adapter: &GrpcMtlsAdapter, tenant: &str) -> Hc2Client {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(18);
    loop {
        let mut config = ClientConfig::new("w10i-sdk", tenant);
        config.capabilities = vec![
            Capability::Data,
            Capability::Batch,
            Capability::Subscriptions,
        ];
        match Hc2Client::connect(adapter, config).await {
            Ok(client) => return client,
            Err(error) => assert!(
                tokio::time::Instant::now() < deadline,
                "HC/2 member did not connect: {error}"
            ),
        }
        tokio::time::sleep(Duration::from_millis(70)).await;
    }
}

async fn retry_w10i<T, F>(mut call: impl FnMut() -> F) -> T
where
    F: std::future::Future<Output = Result<T, hydracache_client_hc2::ClientError>>,
{
    let deadline = tokio::time::Instant::now() + Duration::from_secs(18);
    loop {
        match call().await {
            Ok(reply) => return reply,
            Err(error)
                if tokio::time::Instant::now() < deadline
                    && matches!(
                        error.code(),
                        ErrorCode::Unavailable | ErrorCode::DeadlineExceeded
                    ) =>
            {
                tokio::time::sleep(Duration::from_millis(70)).await;
            }
            Err(error) => panic!("HC/2 did not recover after owner change: {error}"),
        }
    }
}

async fn wait_w10i_get(client: &Hc2Client, key: Bytes, expected: Option<&Bytes>) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(18);
    loop {
        let reply = client.get(key.clone(), None).await;
        if let Ok(value) = &reply {
            if value.as_ref().map(|entry| &entry.value) == expected {
                return;
            }
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "expected HC/2 value {expected:?}; got {reply:?}"
        );
        tokio::time::sleep(Duration::from_millis(70)).await;
    }
}

fn hc1_frame(request: ClientRequestEnvelope) -> Vec<u8> {
    ClientFrame::from_message(&ClientWireMessage::Request(request))
        .unwrap()
        .encode()
        .unwrap()
        .to_vec()
}

async fn hc1_response(response: reqwest::Response) -> ClientResponseEnvelope {
    let status = response.status();
    let bytes = response.bytes().await.unwrap();
    assert!(
        status.is_success(),
        "HC/1 rejected request: {status}; body={bytes:?}"
    );
    let frame = ClientFrame::decode(&bytes, 1024 * 1024).unwrap();
    let ClientWireMessage::Response(response) = frame.decode_message().unwrap() else {
        panic!("HC/1 returned a non-response frame");
    };
    response
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_daemon_shares_hc1_hc2_dispatch_and_exits_on_drain() {
    let _guard = async_daemon_process_guard().await;
    let root = test_root("coexistence");
    let pki = pki();
    let (cert, key, ca) = write_pki(&root, &pki);
    let ([client_addr, admin_addr, hc2_addr], reservations) = reserve_addrs::<3>();
    drop(reservations);
    let mut command = daemon_command(client_addr, admin_addr, hc2_addr, &cert, &key, &ca);
    command.stderr(Stdio::inherit());
    let child = command.spawn().unwrap();
    let mut child = ChildGuard(child);
    let mut stdout = BufReader::new(child.0.stdout.take().unwrap());
    let mut ready = String::new();
    stdout.read_line(&mut ready).unwrap();
    assert_eq!(ready.trim(), r#"{"status":"ok"}"#);

    let namespace = Namespace::new("hc2").unwrap();
    let key = StructuredKey::new(vec!["6863312d6b6579".to_owned()]).unwrap();
    let put = ClientRequestEnvelope::new(
        "hc1-put",
        ClientRequest::Put {
            ns: namespace.clone(),
            key: key.clone(),
            value: b"from-hc1".to_vec(),
            ttl_ms: None,
            dimensions: Vec::new(),
        },
    );
    let http = reqwest::Client::new();
    let response = http
        .post(format!("http://{client_addr}{CLIENT_DATA_PATH}"))
        .header(HYDRACACHE_CLIENT_ID_HEADER, "hc1-client")
        .header(HYDRACACHE_TENANT_HEADER, "tenant-a")
        .body(hc1_frame(put))
        .send()
        .await
        .unwrap();
    assert!(matches!(
        hc1_response(response).await.result,
        Ok(ClientResponse::Stored)
    ));

    let adapter = GrpcMtlsAdapter::new(
        GrpcMtlsConfig::new(
            format!("https://{hc2_addr}"),
            "localhost",
            pki.ca.as_bytes(),
            pki.client_cert.as_bytes(),
            pki.client_key.as_bytes(),
        )
        .unwrap(),
    );
    let hc2 = {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        loop {
            match Hc2Client::connect(&adapter, ClientConfig::new("hc2-client", "tenant-a")).await {
                Ok(client) => break client,
                Err(error) if tokio::time::Instant::now() < deadline => {
                    tokio::time::sleep(Duration::from_millis(25)).await;
                    let _ = error;
                }
                Err(error) => panic!("HC/2 listener never became ready: {error}"),
            }
        }
    };
    assert_eq!(hc2.protocol_generation(), 6);
    assert_eq!(hc2.preferred_protocol_generation(), 6);
    assert!(!hc2.negotiated_generation_deprecated());
    let value = hc2
        .get(Bytes::from_static(b"hc1-key"), None)
        .await
        .unwrap()
        .expect("HC/2 must observe the HC/1 write");
    assert_eq!(value.value, Bytes::from_static(b"from-hc1"));
    hc2.put(
        Bytes::from_static(b"hc2-key"),
        Bytes::from_static(b"from-hc2"),
        None,
        None,
    )
    .await
    .unwrap();

    let metrics = http
        .get(format!("http://{admin_addr}{ADMIN_METRICS_PATH}"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(metrics.contains("hydracache_hc2_connections{transport=\"grpc_bidirectional\"} 1"));
    assert!(
        metrics.contains("hydracache_hc2_pending_invocations{transport=\"grpc_bidirectional\"} 0")
    );
    assert!(metrics
        .contains("hydracache_hc2_rejected_frames_total{transport=\"grpc_bidirectional\"} 0"));
    for forbidden in ["tenant-a", "hc2-client", "localhost", "hc2-key"] {
        assert!(!metrics.contains(forbidden));
    }

    let mut legacy_config = ClientConfig::new("hc2-generation-5-client", "tenant-a");
    legacy_config.protocol_generation = 5;
    let generation_5 = Hc2Client::connect(&adapter, legacy_config).await.unwrap();
    assert_eq!(generation_5.protocol_generation(), 5);
    assert_eq!(generation_5.preferred_protocol_generation(), 6);
    assert!(generation_5.negotiated_generation_deprecated());
    let legacy_value = generation_5
        .get(Bytes::from_static(b"hc2-key"), None)
        .await
        .unwrap()
        .expect("generation-5 client must read generation-6 daemon state");
    assert_eq!(legacy_value.value, Bytes::from_static(b"from-hc2"));
    generation_5.close();

    let get = ClientRequestEnvelope::new(
        "hc1-get",
        ClientRequest::Get {
            ns: namespace,
            key: StructuredKey::new(vec!["6863322d6b6579".to_owned()]).unwrap(),
        },
    );
    let response = http
        .post(format!("http://{client_addr}{CLIENT_DATA_PATH}"))
        .header(HYDRACACHE_CLIENT_ID_HEADER, "hc1-client")
        .header(HYDRACACHE_TENANT_HEADER, "tenant-a")
        .body(hc1_frame(get))
        .send()
        .await
        .unwrap();
    assert!(matches!(
        hc1_response(response).await.result,
        Ok(ClientResponse::Value { value: Some(value) }) if value == b"from-hc2"
    ));
    hc2.close();

    let drained = http
        .post(format!("http://{admin_addr}{ADMIN_DRAIN_PATH}"))
        .header(HYDRACACHE_CLIENT_ID_HEADER, "operator")
        .header(HYDRACACHE_TENANT_HEADER, "system")
        .header(HYDRACACHE_ADMIN_HEADER, "true")
        .send()
        .await
        .unwrap();
    assert!(drained.status().is_success());
    wait_success(&mut child.0, Duration::from_secs(5));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn port_conflict_fails_before_any_listener_accepts() {
    let _guard = daemon_process_guard();
    let root = test_root("conflict");
    let pki = pki();
    let (cert, key, ca) = write_pki(&root, &pki);
    let ([client_addr, admin_addr, hc2_addr], mut reservations) = reserve_addrs::<3>();
    let blocker = reservations
        .pop()
        .expect("the HC/2 address reservation must exist");
    drop(reservations);
    assert_eq!(blocker.local_addr().unwrap(), hc2_addr);
    let output = daemon_command(client_addr, admin_addr, hc2_addr, &cert, &key, &ca)
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr).to_ascii_lowercase();
    assert!(
        stderr.contains("address"),
        "unexpected bind failure: {stderr}"
    );
    assert!(StdTcpListener::bind(client_addr).is_ok());
    assert!(StdTcpListener::bind(admin_addr).is_ok());
    drop(blocker);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn unreadable_hc2_tls_fails_before_readiness_or_listener_startup() {
    let _guard = daemon_process_guard();
    let root = test_root("missing-tls");
    std::fs::create_dir_all(&root).unwrap();
    let ([client_addr, admin_addr, hc2_addr], reservations) = reserve_addrs::<3>();
    drop(reservations);
    let output = daemon_command(
        client_addr,
        admin_addr,
        hc2_addr,
        &root.join("missing-server.pem"),
        &root.join("missing-server.key"),
        &root.join("missing-clients.pem"),
    )
    .output()
    .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty(), "readiness must not be published");
    assert!(StdTcpListener::bind(client_addr).is_ok());
    assert!(StdTcpListener::bind(admin_addr).is_ok());
    assert!(StdTcpListener::bind(hc2_addr).is_ok());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn reserved_listener_addresses_are_distinct_and_held() {
    let _guard = daemon_process_guard();
    let (addrs, reservations) = reserve_addrs::<8>();
    for (index, addr) in addrs.iter().enumerate() {
        assert!(
            StdTcpListener::bind(addr).is_err(),
            "reservation {index} was not held"
        );
        for other in &addrs[index + 1..] {
            assert_ne!(addr, other, "reserved listener addresses must be unique");
        }
    }
    drop(reservations);
}
