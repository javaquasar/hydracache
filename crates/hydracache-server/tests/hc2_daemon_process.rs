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
use hc2_pki::{pki, write_pki, TestPki};
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
    use hydracache_client_hc2::{BatchItemState, BatchOperation, GapReason, SubscriptionEvent};

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
    let oversized_key = cluster.key_for(0, None, "w10g-oversized/");
    let oversized = (0..4)
        .map(|_| BatchOperation::Put {
            key: oversized_key.clone(),
            value: Bytes::from(vec![255; 128 * 1024]),
            ttl: None,
        })
        .collect();
    let refused = client.batch_outcomes(oversized, None).await.unwrap();
    assert!(refused.iter().all(|item| {
        item.receipt.is_some_and(|receipt| {
            receipt.state == BatchItemState::NotApplied && receipt.applied.is_none()
        }) && item
            .result
            .as_ref()
            .is_err_and(|error| error.code() == ErrorCode::QuotaExceeded)
    }));
    wait_w10i_get(client, oversized_key, None).await;
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
        let receipt = item.receipt.unwrap();
        assert_eq!(receipt.state, BatchItemState::Confirmed);
        assert!(receipt.applied.is_some());
        let result = item.result.as_ref().unwrap();
        match index {
            3 => assert_eq!(result.value.as_ref().unwrap().value, value(b"one")),
            5 => assert_eq!(result.value.as_ref().unwrap().value, value(b"two")),
            6 => assert_eq!(result.value.as_ref().unwrap().value, value(b"one-again")),
            _ => assert!(result.mutation.unwrap().applied),
        }
    }
    let hot_receipt = items[0].receipt.unwrap();
    assert_eq!(items[3].receipt, Some(hot_receipt));
    assert_eq!(items[4].receipt, items[6].receipt);
    assert_eq!(
        items[4].receipt.unwrap().applied.unwrap().version,
        hot_receipt.applied.unwrap().version + 1
    );
    assert_ne!(
        items[1].receipt.unwrap().partition_id,
        hot_receipt.partition_id
    );
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
    retry_w10i(|| async {
        let repeated = cluster.clients[1]
            .batch_outcomes(mixed.clone(), Some(options.clone()))
            .await?;
        for (index, item) in repeated.into_iter().enumerate() {
            if matches!(index, 0 | 1 | 2 | 4) {
                assert!(item.result?.mutation.unwrap().applied);
                assert_eq!(item.receipt, items[index].receipt);
            }
        }
        Ok(())
    })
    .await;
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
    let interrupted_prefix = "w10g-interrupted/";
    let interrupted = cluster.key_for(0, Some(1), interrupted_prefix);
    let interrupted_before = retry_w10i(|| async {
        let items = client.batch_outcomes(vec![get(&interrupted)], None).await?;
        items[0].result.as_ref().map_err(Clone::clone)?;
        Ok(items[0].receipt.unwrap().applied.unwrap())
    })
    .await;
    let mut fault_events = client
        .subscribe_with_values(Bytes::from_static(interrupted_prefix.as_bytes()), 0, true)
        .await
        .unwrap();
    let interrupted_options = RequestOptions {
        timeout: Duration::from_secs(10),
        idempotency_key: Bytes::from(format!("w10e:{}:interrupted", interrupted_before.epoch)),
    };
    let interrupted_batch = (0..64)
        .map(|id| BatchOperation::Put {
            key: interrupted.clone(),
            value: Bytes::from(format!("interrupted-{id}")),
            ttl: None,
        })
        .chain(std::iter::once(get(&control)))
        .collect::<Vec<_>>();
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
    let pending = {
        let client = client.clone();
        let operations = interrupted_batch.clone();
        let options = interrupted_options.clone();
        tokio::spawn(async move { client.batch_outcomes(operations, Some(options)).await })
    };
    let applied_before_kill = wait_w10g_mutation(&mut fault_events, &interrupted).await;
    assert_eq!(applied_before_kill.value, value(b"interrupted-0"));
    assert!(
        !pending.is_finished(),
        "owner must stop before completing its subrequest"
    );
    let dead_pid = cluster.daemons[0].0.id();
    cluster.daemons[0].0.kill().unwrap();
    cluster.daemons[0].0.wait().unwrap();
    let lost = pending.await.unwrap().unwrap();
    assert!(lost[..64].iter().all(|item| {
        item.receipt.is_some_and(|receipt| {
            receipt.state == BatchItemState::OutcomeUnknown && receipt.applied.is_none()
        }) && item.result.as_ref().is_err_and(|error| {
            error.retry_advice() == hydracache_client_hc2::RetryAdvice::ReconnectIdempotent
        })
    }));
    assert_eq!(
        lost[64]
            .result
            .as_ref()
            .unwrap()
            .value
            .as_ref()
            .unwrap()
            .value,
        value(b"two")
    );
    let deadline = tokio::time::Instant::now() + Duration::from_secs(4);
    loop {
        let partial = client
            .batch_outcomes(vec![get(&third), get(&hot), get(&control)], None)
            .await;
        if let Ok(partial) = partial {
            if partial[0].result.is_ok() && partial[1].result.is_err() && partial[2].result.is_ok()
            {
                assert_ne!(partial[1].receipt.unwrap().state, BatchItemState::Confirmed);
                assert!(partial[1].receipt.unwrap().applied.is_none());
                assert_eq!(partial[0].receipt.unwrap().state, BatchItemState::Confirmed);
                assert_eq!(partial[2].receipt.unwrap().state, BatchItemState::Confirmed);
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
    let replay = retry_w10i(|| async {
        let replay = client
            .batch_outcomes(interrupted_batch.clone(), Some(interrupted_options.clone()))
            .await?;
        replay[0].result.as_ref().map_err(Clone::clone)?;
        Ok(replay)
    })
    .await;
    assert_eq!(
        replay[0].receipt.unwrap().applied.unwrap().epoch,
        interrupted_before.epoch
    );
    assert_eq!(
        replay[0].receipt.unwrap().applied.unwrap().version,
        interrupted_before.version + 1
    );
    let after = retry_w10i(|| async {
        let read = client.batch_outcomes(vec![get(&interrupted)], None).await?;
        read[0].result.as_ref().map_err(Clone::clone)?;
        Ok(read[0].receipt.unwrap().applied.unwrap())
    })
    .await;
    assert_eq!(after.epoch, interrupted_before.epoch);
    assert!(after.version > interrupted_before.version);
    let boundary = tokio::time::timeout(Duration::from_secs(8), events.next_delivery())
        .await
        .unwrap()
        .unwrap();
    assert!(
        matches!(boundary.event, SubscriptionEvent::Gap { after_watermark, .. } if after_watermark == watermark)
    );
    let details = boundary.gap.unwrap();
    assert_eq!(details.reason, GapReason::TopologyChanged);
    // The first epoch change can precede reassignment. An empty list is a stream-wide gap.
    assert!(
        details.partitions.is_empty()
            || details
                .partitions
                .iter()
                .any(|gap| { gap.partition_id == hot_receipt.partition_id })
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
    let boundary = tokio::time::timeout(Duration::from_secs(8), reconnect.next_delivery())
        .await
        .unwrap()
        .unwrap();
    assert!(
        matches!(boundary.event, SubscriptionEvent::Gap { after_watermark, .. } if after_watermark == resumed.watermark)
    );
    let details = boundary.gap.unwrap();
    assert_eq!(details.reason, GapReason::ResumeUnbound);
    assert!(details.partitions.is_empty());
    assert!(
        retry_w10i(|| client.delete(control.clone(), None))
            .await
            .applied
    );
    let removed = wait_w10g_mutation(&mut reconnect, &control).await;
    assert!(removed.removed && removed.value.is_empty());
    wait_w10i_get(&rejoined, control, None).await;
    drop((events, slow, key_only, reconnect, fault_events));
    wait_for_zero_hc2_subscriptions(&[cluster.spec.addresses[6], cluster.spec.addresses[8]]).await;
    let accepted = load.await.unwrap();
    assert!(
        accepted >= 20,
        "slow listener impaired foreground writes: {accepted}"
    );
    println!("W10g bulk sizes=1/8/64/256 partial_owner_failure=true lost_group_reply=true replay_proof_preserved=true listener_gap_and_reconnect=true killed_pid={dead_pid} rejoined_pid={} foreground_acks={accepted} retained_subscriptions=0", cluster.daemons[0].0.id());
}

async fn next_w10g_event(
    subscription: &mut hydracache_client_hc2::Subscription,
) -> hydracache_client_hc2::SubscriptionEvent {
    tokio::time::timeout(Duration::from_secs(8), subscription.next())
        .await
        .expect("cluster subscription did not deliver before its deadline")
        .expect("cluster subscription closed")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn w10g_reports_the_partition_whose_history_was_overwritten() {
    use hydracache_client_hc2::{BatchOperation, GapReason, SubscriptionEvent};

    if std::env::var("HYDRACACHE_RUN_DAEMON_PROCESS_E2E").as_deref() != Ok("1") {
        return;
    }
    let _guard = async_daemon_process_guard().await;
    let cluster = MemberCluster::start_fixture("w10g-history-gap", &pki(), true).await;
    let prefix = Bytes::from_static(b"w10g-history/");
    let key = cluster.key_for(1, Some(2), "w10g-history/");
    let owner = &cluster.clients[1];
    let initial = retry_w10i(|| async {
        let result = owner
            .batch_outcomes(vec![BatchOperation::Get { key: key.clone() }], None)
            .await?;
        result[0].result.as_ref().map_err(Clone::clone)?;
        Ok(result[0].receipt.unwrap())
    })
    .await;
    let mut subscription = cluster.clients[2]
        .subscribe_with_values(prefix, 0, true)
        .await
        .unwrap();
    cluster.spec.set_value_fault(2, true);
    for index in 0..24 {
        let value = Bytes::from(format!("history-{index}"));
        assert!(
            retry_w10i(|| owner.put(key.clone(), value.clone(), None, None))
                .await
                .applied
        );
    }
    cluster.spec.set_value_fault(3, false);
    let history = tokio::time::timeout(Duration::from_secs(8), subscription.next_delivery())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        history.event,
        SubscriptionEvent::Gap {
            after_watermark: 0,
            ..
        }
    ));
    let gap = history.gap.unwrap();
    assert_eq!(gap.reason, GapReason::HistoryLost);
    assert_eq!(gap.partitions.len(), 1);
    let partition = &gap.partitions[0];
    assert_eq!(partition.partition_id, initial.partition_id);
    assert_eq!(partition.previous_version, initial.applied.unwrap().version);
    assert!(partition.observed_version >= partition.previous_version + 24);
    assert!(
        retry_w10i(|| owner.put(key.clone(), Bytes::from_static(b"after-gap"), None, None))
            .await
            .applied
    );
    assert_eq!(
        wait_w10g_mutation(&mut subscription, &key).await.value,
        Bytes::from_static(b"after-gap")
    );
    println!(
        "W10g partition history gap partition={} previous={} observed={} resumed=true",
        partition.partition_id, partition.previous_version, partition.observed_version
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn w10g_packaged_java_facade_uses_three_public_endpoints() {
    let (Some(java), Some(classpath)) = (
        std::env::var_os("HYDRACACHE_W10G_JAVA"),
        std::env::var_os("HYDRACACHE_W10G_JAVA_CLASSPATH"),
    ) else {
        eprintln!("SKIP: W10g packaged Java probe requires its Java runtime and classpath");
        return;
    };
    let _guard = async_daemon_process_guard().await;
    let credentials = pki();
    let mut cluster = MemberCluster::start_fixture("w10g-java-facade", &credentials, false).await;
    let root = &cluster.spec.root;
    std::fs::write(root.join("java-client.pem"), credentials.client_cert).unwrap();
    std::fs::write(root.join("java-client.key"), credentials.client_key).unwrap();
    let prefix = b"HC\x02\x01\x00\x08w10g-map";
    let keys = [(0, 1), (1, 2), (2, 1)].map(|(primary, backup)| {
        cluster.key_for_with(primary, Some(backup), |n| {
            let mut key = prefix.to_vec();
            key.extend(format!("key-{n}").as_bytes());
            key
        })
    });
    for key in &keys {
        wait_w10i_get(&cluster.clients[2], key.clone(), None).await;
    }
    let log = std::fs::File::create(root.join("java-probe.log")).unwrap();
    let mut probe = ChildGuard(
        Command::new(java)
            .args(["-XX:ActiveProcessorCount=2", "-cp"])
            .arg(classpath)
            .arg("io.hydracache.hazelcast.W10gPackagedFacadeProbe")
            .arg(root)
            .args(cluster.spec.addresses[9..].iter().map(ToString::to_string))
            .args(
                keys.iter()
                    .map(|key| std::str::from_utf8(&key[prefix.len()..]).unwrap()),
            )
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap(),
    );
    await_java_marker(&mut probe.0, root, "java-ready").await;
    let killed = cluster.daemons[0].0.id();
    cluster.daemons[0].0.kill().unwrap();
    cluster.daemons[0].0.wait().unwrap();
    await_java_marker(&mut probe.0, root, "java-promoted").await;
    cluster.daemons[0] = cluster.spec.spawn(0);
    let rejoined = connect_w10i(&cluster.adapters[0], "w10i").await;
    wait_w10i_get(
        &rejoined,
        keys[0].clone(),
        Some(&Bytes::from_static(b"promoted")),
    )
    .await;
    std::fs::File::create(root.join("java-rejoined")).unwrap();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    loop {
        if let Some(status) = probe.0.try_wait().unwrap() {
            let report = std::fs::read_to_string(root.join("java-probe.log")).unwrap();
            assert!(status.success(), "Java probe failed: {report}");
            print!("{report}");
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "Java probe did not exit"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    wait_for_zero_hc2_subscriptions(&cluster.spec.addresses[6..9]).await;
    println!(
        "W10g packaged Java killed_pid={killed} rejoined_pid={} retained_subscriptions=0",
        cluster.daemons[0].0.id()
    );
}

async fn await_java_marker(probe: &mut Child, root: &Path, name: &str) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    while !root.join(name).is_file() {
        assert!(
            probe.try_wait().unwrap().is_none() && tokio::time::Instant::now() < deadline,
            "Java probe did not reach {name}: {}",
            std::fs::read_to_string(root.join("java-probe.log")).unwrap_or_default()
        );
        tokio::time::sleep(Duration::from_millis(15)).await;
    }
}

async fn wait_for_zero_hc2_subscriptions(addresses: &[SocketAddr]) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let http = reqwest::Client::new();
    loop {
        let mut empty = true;
        for address in addresses {
            let metrics = http
                .get(format!("http://{address}{ADMIN_METRICS_PATH}"))
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
            return;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "clustered subscriptions were retained after close"
        );
        tokio::time::sleep(Duration::from_millis(40)).await;
    }
}

async fn wait_w10g_mutation(
    subscription: &mut hydracache_client_hc2::Subscription,
    key: &Bytes,
) -> hydracache_client_hc2::CacheEvent {
    for _ in 0..8 {
        if let hydracache_client_hc2::SubscriptionEvent::Event(event) =
            next_w10g_event(subscription).await
        {
            if event.key == key {
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
    event_fault: Option<PathBuf>,
}

impl MemberClusterSpec {
    fn set_value_fault(&self, generation: u64, enabled: bool) {
        let target = self.event_fault.as_ref().unwrap();
        let incoming = target.with_extension("incoming");
        let rules = if enabled {
            vec![serde_json::json!({
                "from": self.names[2], "to": self.names[1], "action": {"kind": "value_drop"}
            })]
        } else {
            Vec::new()
        };
        std::fs::write(
            &incoming,
            serde_json::json!({
                "schema_version": 1, "generation": generation, "rules": rules
            })
            .to_string(),
        )
        .unwrap();
        std::fs::rename(incoming, target).unwrap();
    }

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
        if let Some(fault) = self.event_fault.as_ref().filter(|_| index == 2) {
            command.env("HYDRACACHE_TEST_RAFT_OUTBOUND_FAULT_FILE", fault);
        }
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
        Self::start_fixture(label, &pki(), false).await
    }

    async fn start_fixture(label: &str, pki: &TestPki, event_fault: bool) -> Self {
        let root = test_root(label);
        let (server_cert, server_key, ca) = write_pki(&root, pki);
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
            event_fault: event_fault.then(|| root.join("event-fault.json")),
            root,
            addresses,
            names,
            seeds,
            server_cert,
            server_key,
            ca,
        };
        if event_fault {
            spec.set_value_fault(1, false);
        }
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
        self.key_for_with(primary, backup, |n| format!("{prefix}{n}").into_bytes())
    }

    fn key_for_with(
        &self,
        primary: usize,
        backup: Option<usize>,
        key: impl Fn(u32) -> Vec<u8>,
    ) -> Bytes {
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
                let key = key(n);
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
