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
use hydracache_server::imap_replicated_read::{ReplicaReadScope, D2_PARTITION_COUNT};
use hydracache_server::{
    ADMIN_CLUSTER_OVERVIEW_PATH, ADMIN_DRAIN_PATH, ADMIN_METRICS_PATH, ADMIN_RAFT_COMPACTION_PATH,
    ADMIN_READYZ_PATH, MANAGEMENT_CLUSTER_IMAP_PATH, MANAGEMENT_CONSENSUS_PROGRESS_PATH,
    MANAGEMENT_IMAP_PATH,
};
use sha2::{Digest, Sha256};
use tokio::sync::{Mutex, MutexGuard};

#[path = "support/hc2_pki.rs"]
mod hc2_pki;
use hc2_pki::{pki, write_pki, TestPki};
#[path = "support/imap_hc2_campaign.rs"]
mod imap_hc2_campaign;
#[cfg(target_os = "linux")]
#[path = "support/imap_chrony_process.rs"]
mod imap_chrony_process;
#[path = "support/imap_od3.rs"]
mod imap_od3;
#[path = "support/imap_sdk_cohort.rs"]
mod imap_sdk_cohort;

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
    daemon_command_for(
        Path::new(env!("CARGO_BIN_EXE_hydracache-server")),
        client,
        admin,
        hc2,
        cert,
        key,
        ca,
    )
}

fn daemon_command_for(
    binary: &Path,
    client: SocketAddr,
    admin: SocketAddr,
    hc2: SocketAddr,
    cert: &Path,
    key: &Path,
    ca: &Path,
) -> Command {
    let mut command = Command::new(binary);
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
        value_plane: Default::default(),
        chrony: None,
    });
    config.validate().unwrap();
    let profile = config.distributed_hc2.clone().unwrap();
    let round_trip: DistributedHc2Config =
        toml::from_str(&toml::to_string(&profile).unwrap()).unwrap();
    assert_eq!(round_trip, profile);
    assert!(!format!("{profile:?}").contains(&profile.client_cert_sha256));
    let mut hostile = profile.clone();
    hostile.value_plane.write_quorum = 1;
    config.distributed_hc2 = Some(hostile);
    assert!(matches!(
        config.validate(),
        Err(ServerConfigError::InvalidDistributedHc2(_))
    ));
    let mut hostile = profile.clone();
    hostile.value_plane.max_inflight = 0;
    config.distributed_hc2 = Some(hostile);
    assert!(matches!(
        config.validate(),
        Err(ServerConfigError::InvalidDistributedHc2(_))
    ));
    config.distributed_hc2 = Some(profile);
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
    use imap_hc2_campaign::{owner_checkpoint, OwnerKillAudit};

    if std::env::var("HYDRACACHE_RUN_DAEMON_PROCESS_E2E").as_deref() != Ok("1") {
        return;
    }
    let profile = imap_hc2_campaign::Profile::from_env();
    let _guard = async_daemon_process_guard().await;
    let mut cluster = MemberCluster::start_fixture(
        "distributed-imap",
        &pki(),
        MemberOptions {
            interrupt_copy: profile.interrupt_copy,
            ..Default::default()
        },
    )
    .await;
    let hot = cluster.key_for(0, Some(1), "w10i-\0-");
    let hot_partition = fixture_imap_scope().locate(hot.to_vec()).unwrap().0;
    let partition = hot_partition.value();
    let control = cluster.key_for(1, Some(2), "w10i-\0-");
    let http = w10h_http();
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

    // Only the put and the winning CAS advance this partition before the kill.
    let before = owner_checkpoint(&http, &cluster.spec, [0, 1], partition, 2, 1).await;
    let started = tokio::time::Instant::now();
    let load = tokio::spawn(imap_hc2_campaign::exercise_control(
        clients[2].clone(),
        clients[1].clone(),
        control,
        started,
        profile.duration,
        profile.ledger().unwrap(),
        profile.live_drain,
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
    let promoted = owner_checkpoint(
        &http,
        &cluster.spec,
        [1, 2],
        partition,
        2,
        before.epoch() + 1,
    )
    .await;
    before.assert_same_image(&promoted);
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
    wait_w10i_get(&clients[1], hot.clone(), Some(&recovered)).await;
    let recovered_cut =
        owner_checkpoint(&http, &cluster.spec, [1, 2], partition, 3, promoted.epoch()).await;
    cluster.daemons[0] = cluster.spec.spawn(0);
    clients[0] = connect_w10i(&cluster.adapters[0], "w10i").await;
    wait_w10i_get(&clients[0], hot.clone(), Some(&recovered)).await;
    let returned = wait_management_json(
        &http,
        cluster.spec.addresses[7],
        ADMIN_CLUSTER_OVERVIEW_PATH,
        |body| {
            body["source"] == "live"
                && body["members"].as_array().is_some_and(|members| {
                    members.iter().any(|member| {
                        member["node_id"] == cluster.spec.names[0]
                            && member["reachable"] == true
                            && member["generation"].as_u64().is_some_and(|generation| {
                                generation > before.primary().generation.value()
                            })
                    })
                })
        },
    )
    .await;
    let generation = returned["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|member| member["node_id"] == cluster.spec.names[0])
        .unwrap()["generation"]
        .as_u64()
        .unwrap();
    let returned_member = hydracache::PartitionReplicaIdentity {
        node: before.primary().node.clone(),
        generation: ClusterGeneration::new(generation),
    };
    let returned_epoch = wait_cluster_imap(&http, cluster.spec.addresses[7], |body| {
        body["complete"] == true
            && cluster_imap_members(body).len() == 3
            && body["epoch"]
                .as_u64()
                .is_some_and(|epoch| epoch > recovered_cut.epoch())
    })
    .await["epoch"]
        .as_u64()
        .unwrap();
    let rejoined_cut =
        owner_checkpoint(&http, &cluster.spec, [1, 2], partition, 3, returned_epoch).await;
    recovered_cut.assert_same_image(&rejoined_cut);
    let _removed =
        retry_w10i(|| clients[2].remove_if_value(hot.clone(), recovered.clone(), None)).await;
    wait_w10i_get(&clients[0], hot, None).await;
    let tombstoned = owner_checkpoint(
        &http,
        &cluster.spec,
        [1, 2],
        partition,
        4,
        rejoined_cut.epoch(),
    )
    .await;
    println!("W10i owner kill audited partition={partition} epochs={:?} returned_generation={generation}",
        [before.epoch(), promoted.epoch(), recovered_cut.epoch(), rejoined_cut.epoch(), tombstoned.epoch()]);
    let rejoined = cluster.daemons[0].0.id();
    let rejoined_at_ms = started.elapsed().as_millis() as u64;
    let drain = if profile.live_drain {
        Some(
            imap_hc2_campaign::exercise_live_drain(
                &mut cluster, started, profile.duration, profile.interrupt_copy, hot_partition,
                std::future::ready(()),
            ).await,
        )
    } else {
        None
    };
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
                audit: OwnerKillAudit {
                    partition,
                    before,
                    promoted,
                    recovered: recovered_cut,
                    rejoined: rejoined_cut,
                    tombstoned,
                    returned_member,
                    returned_epoch,
                },
            },
            drain,
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
    let cluster = MemberCluster::start_fixture(
        "w10g-history-gap",
        &pki(),
        MemberOptions {
            event_fault: true,
            ..Default::default()
        },
    )
    .await;
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
    let mut cluster =
        MemberCluster::start_fixture("w10g-java-facade", &credentials, MemberOptions::default())
            .await;
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
        keys[1].clone(),
        Some(&Bytes::from_static(b"after-gap")),
    )
    .await;
    let retained = keys[0].slice(prefix.len()..);
    wait_w10i_get(&rejoined, keys[0].clone(), Some(&retained)).await;
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn w10h_drains_primaries_and_backups_before_member_exit_under_load() {
    use hydracache_client_hc2::BatchOperation;
    if !w10h_enabled() {
        return;
    }
    let _guard = async_daemon_process_guard().await;
    let mut cluster = MemberCluster::start("w10h-managed-drain").await;
    let http = w10h_http();
    let [departing, surviving, other] = [6, 7, 8].map(|index| cluster.spec.addresses[index]);
    for address in [departing, surviving, other] {
        wait_imap_readiness(&http, address, "ready").await;
    }
    let hot = cluster.key_for(0, Some(1), "w10h-primary/");
    let backup = cluster.key_for(1, Some(0), "w10h-backup/");
    let load_key = cluster.key_for(1, Some(2), "w10h-load/");
    let options = RequestOptions {
        timeout: Duration::from_secs(4),
        idempotency_key: Bytes::from_static(b"w10h-drain-receipt"),
    };
    let seeded = Bytes::from_static(b"certified-before-drain");
    let put = BatchOperation::Put {
        key: hot.clone(),
        value: seeded.clone(),
        ttl: None,
    };
    let original = retry_w10i(|| async {
        let result = cluster.clients[1]
            .batch_outcomes(vec![put.clone()], Some(options.clone()))
            .await?;
        assert!(
            result[0]
                .result
                .as_ref()
                .map_err(Clone::clone)?
                .mutation
                .unwrap()
                .applied
        );
        Ok(result[0].receipt.unwrap())
    })
    .await;
    assert!(
        retry_w10i(|| cluster.clients[1].put(backup.clone(), seeded.clone(), None, None))
            .await
            .applied
    );
    assert!(
        retry_w10i(|| cluster.clients[1].put(load_key.clone(), seeded.clone(), None, None))
            .await
            .applied
    );
    let mut subscription = cluster.clients[0]
        .subscribe(Bytes::from_static(b"w10h-"), 0)
        .await
        .unwrap();
    wait_imap_readiness(&http, surviving, "ready").await;
    assert_eq!(
        http.get(format!("http://{surviving}{MANAGEMENT_IMAP_PATH}"))
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    let evidence =
        w10h_admin(http.get(format!("http://{surviving}{MANAGEMENT_IMAP_PATH}?limit=2")))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .json::<serde_json::Value>()
            .await
            .unwrap();
    assert_eq!(evidence["complete"], true);
    assert_eq!(evidence["partitions"].as_array().unwrap().len(), 2);
    assert_eq!(evidence["next_after"], 1);
    let page = w10h_admin(http.get(format!(
        "http://{surviving}{MANAGEMENT_IMAP_PATH}?limit=2&after=1&sequence={}",
        evidence["observation_sequence"]
    )))
    .send()
    .await
    .unwrap()
    .error_for_status()
    .unwrap()
    .json::<serde_json::Value>()
    .await
    .unwrap();
    assert_eq!(page["partitions"][0]["id"], 2);
    for (query, expected) in [("limit=65", 400), ("after=1", 400), ("sequence=0", 409)] {
        let status =
            w10h_admin(http.get(format!("http://{surviving}{MANAGEMENT_IMAP_PATH}?{query}")))
                .send()
                .await
                .unwrap()
                .status();
        assert_eq!(status, expected);
    }
    let proxy_metrics = imap_metrics(&http, surviving).await;
    assert!(imap_metric(&proxy_metrics, "hydracache_imap_proxy_total") >= 1);
    assert!(imap_metric(&proxy_metrics, "hydracache_imap_direct_total") >= 1);
    assert!(imap_metric(&proxy_metrics, "hydracache_imap_replica_ack_seconds_count") >= 1);
    assert!(!proxy_metrics
        .lines()
        .filter(|line| line.starts_with("hydracache_imap_"))
        .any(|line| line.contains("tenant=")
            || line.contains("node_id=")
            || line.contains("partition_id=")));
    let summary = wait_cluster_imap(&http, surviving, |body| {
        body["complete"] == true && body["ready"] == true && cluster_imap_members(body).len() == 3
    })
    .await;
    assert_eq!(
        http.get(format!("http://{surviving}{MANAGEMENT_CLUSTER_IMAP_PATH}"))
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    for row in cluster_imap_members(&summary) {
        assert!(row["node"].as_str().unwrap().starts_with("node-"));
        assert_eq!(row["observation"]["readiness"]["epoch"], summary["epoch"]);
        let counts = row["observation"]["partitions"].as_object().unwrap();
        assert_eq!(
            counts
                .values()
                .map(|count| count.as_u64().unwrap())
                .sum::<u64>(),
            u64::from(D2_PARTITION_COUNT.get())
        );
    }
    let summary = summary.to_string();
    assert!(cluster
        .spec
        .names
        .iter()
        .all(|name| !summary.contains(name)));
    assert!(!summary.contains("certified-before-drain"));
    let writer = cluster.clients[1].clone();
    let writer_key = load_key.clone();
    let load = tokio::spawn(async move {
        let stop = tokio::time::Instant::now() + Duration::from_secs(12);
        let mut accepted = 0;
        let mut last = None;
        while tokio::time::Instant::now() < stop {
            let value = Bytes::from(format!("w10h-load-{accepted}"));
            if writer
                .put(writer_key.clone(), value.clone(), None, None)
                .await
                .is_ok_and(|outcome| outcome.applied)
            {
                accepted += 1;
                last = Some(value);
            }
            tokio::time::sleep(Duration::from_millis(8)).await;
        }
        (accepted, last)
    });
    let drain =
        tokio::spawn(w10h_admin(http.post(format!("http://{departing}{ADMIN_DRAIN_PATH}"))).send());
    wait_imap_readiness(&http, departing, "draining").await;
    let closed = tokio::time::timeout(Duration::from_secs(4), subscription.next())
        .await
        .unwrap();
    assert!(
        closed.is_none()
            || matches!(
                closed,
                Some(hydracache_client_hc2::SubscriptionEvent::Closed(_))
            )
    );
    assert!(cluster.clients[0].get(hot.clone(), None).await.is_err());
    let response = drain.await.unwrap().unwrap().error_for_status().unwrap();
    let body = response.json::<serde_json::Value>().await.unwrap();
    assert_eq!(body["drain"]["timed_out"], false);
    wait_success(&mut cluster.daemons[0].0, Duration::from_secs(8));
    let (confirmed, last) = load.await.unwrap();
    assert!(
        confirmed >= 20,
        "surviving writers were not making progress: {confirmed}"
    );
    wait_imap_readiness(&http, other, "ready").await;
    wait_w10i_get(&cluster.clients[2], hot.clone(), Some(&seeded)).await;
    wait_w10i_get(&cluster.clients[2], backup, Some(&seeded)).await;
    wait_w10i_get(&cluster.clients[2], load_key, last.as_ref()).await;
    let replay = retry_w10i(|| async {
        let result = cluster.clients[2]
            .batch_outcomes(vec![put.clone()], Some(options.clone()))
            .await?;
        result[0].result.as_ref().map_err(Clone::clone)?;
        Ok(result[0].receipt.unwrap())
    })
    .await;
    assert_eq!(original, replay);
    wait_for_zero_hc2_subscriptions(&[surviving, other]).await;
    println!("W10h managed drain confirmed_under_load={confirmed} primary_and_backup_preserved=true original_receipt_preserved=true retained_subscriptions=0");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn w10h_failed_drain_keeps_membership_when_a_spare_is_down() {
    if !w10h_enabled() {
        return;
    }
    let _guard = async_daemon_process_guard().await;
    let mut cluster = MemberCluster::start_fixture(
        "w10h-failed-drain",
        &pki(),
        MemberOptions {
            drain_timeout_ms: Some(2_000),
            ..Default::default()
        },
    )
    .await;
    let http = w10h_http();
    let [departing, surviving] = [6, 7].map(|index| cluster.spec.addresses[index]);
    let affected = cluster.key_for(1, Some(2), "w10h-lost-backup/");
    let retained = cluster.key_for(0, Some(1), "w10h-retained/");
    let value = Bytes::from_static(b"preserve");
    for key in [&affected, &retained] {
        assert!(
            retry_w10i(|| cluster.clients[0].put(key.clone(), value.clone(), None, None))
                .await
                .applied
        );
    }
    wait_imap_readiness(&http, surviving, "ready").await;
    cluster.daemons[2].0.kill().unwrap();
    cluster.daemons[2].0.wait().unwrap();
    wait_imap_readiness(&http, surviving, "required_backup_unavailable").await;
    wait_cluster_imap(&http, surviving, |body| {
        body["complete"] == false
            && body["ready"] == false
            && cluster_imap_members(body)
                .iter()
                .any(|row| row["observation"].is_null() && row["issue"].is_string())
    })
    .await;
    let failed = w10h_admin(http.post(format!("http://{departing}{ADMIN_DRAIN_PATH}")))
        .send()
        .await
        .unwrap();
    assert_eq!(failed.status(), 504);
    assert_eq!(
        failed.json::<serde_json::Value>().await.unwrap()["drain"]["timed_out"],
        true
    );
    wait_imap_readiness(&http, departing, "draining").await;
    assert!(cluster.daemons[0].0.try_wait().unwrap().is_none());
    let metrics = imap_metrics(&http, departing).await;
    assert_eq!(imap_metric(&metrics, "hydracache_imap_drain_evacuated"), 0);
    assert_eq!(
        imap_metric(&metrics, "hydracache_imap_drain_membership_left"),
        0
    );
    let roster = http
        .get(format!("http://{surviving}{ADMIN_CLUSTER_OVERVIEW_PATH}"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert!(roster["members"]
        .as_array()
        .unwrap()
        .iter()
        .any(|member| member["node_id"] == cluster.spec.names[0]));
    cluster.daemons[2] = cluster.spec.spawn(2);
    let restored = connect_w10i(&cluster.adapters[2], "w10i").await;
    let retry_deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    loop {
        let response = w10h_admin(http.post(format!("http://{departing}{ADMIN_DRAIN_PATH}")))
            .send()
            .await
            .unwrap();
        if response.status().is_success() {
            assert_eq!(
                response.json::<serde_json::Value>().await.unwrap()["drain"]["timed_out"],
                false
            );
            break;
        }
        assert_eq!(response.status(), 504);
        assert!(
            tokio::time::Instant::now() < retry_deadline,
            "drain stayed incomplete after the spare rejoined"
        );
    }
    wait_success(&mut cluster.daemons[0].0, Duration::from_secs(8));
    wait_w10i_get(&restored, affected, Some(&value)).await;
    wait_w10i_get(&restored, retained, Some(&value)).await;
    println!("W10h incomplete drain readiness=draining replica_floor_enforced=true membership_retained=true retry_after_spare_rejoin=true");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn w10h_refuses_mixed_value_generation_before_owner_dispatch() {
    if !w10h_enabled() {
        return;
    }
    let _guard = async_daemon_process_guard().await;
    let mut cluster = MemberCluster::start("w10h-mixed-value-generation").await;
    let http = w10h_http();
    let address = cluster.spec.addresses[7];
    let key = cluster.key_for(1, Some(0), "w10h-mixed/");
    let value = Bytes::from_static(b"unchanged-through-mixed-mode");
    assert!(
        retry_w10i(|| cluster.clients[1].put(key.clone(), value.clone(), None, None))
            .await
            .applied
    );
    wait_imap_readiness(&http, address, "ready").await;
    cluster.daemons[2].0.kill().unwrap();
    cluster.daemons[2].0.wait().unwrap();
    cluster.daemons[2] = cluster.spec.spawn_with_value_profile(2, false);
    wait_imap_readiness(&http, address, "wire_generation_incompatible").await;
    wait_cluster_imap(&http, address, |body| {
        body["ready"] == false
            && body["complete"] == false
            && cluster_imap_members(body).iter().any(|row| {
                row["observation"]["readiness"]["reason"] == "wire_generation_incompatible"
            })
    })
    .await;
    let error = cluster.clients[1]
        .put(
            key.clone(),
            Bytes::from_static(b"must-not-apply"),
            None,
            None,
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::Unavailable);
    assert!(
        imap_metric(
            &imap_metrics(&http, address).await,
            "hydracache_imap_incompatible_total"
        ) >= 1
    );
    cluster.daemons[2].0.kill().unwrap();
    cluster.daemons[2].0.wait().unwrap();
    cluster.daemons[2] = cluster.spec.spawn(2);
    let restored = connect_w10i(&cluster.adapters[2], "w10i").await;
    wait_w10i_get(&restored, key, Some(&value)).await;
    wait_imap_readiness(&http, address, "ready").await;
    println!("W10h incompatible value generation rejected_before_dispatch=true compatible_generation_restored=true");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn w10h_published_b74_same_volume_rollback_before_value_activation() {
    if !w10h_enabled() {
        return;
    }
    let Some(b74) = published_b74() else {
        return;
    };
    let digest = sha256_hex(&std::fs::read(&b74).unwrap());
    let _guard = async_daemon_process_guard().await;
    let mut cluster = MemberCluster::start("w10h-published-b74").await;
    let http = w10h_http();
    let [current, previous] = [7, 8].map(|index| cluster.spec.addresses[index]);
    let before = wait_cluster_imap(&http, current, |body| body["ready"] == true).await;
    assert!(cluster_imap_members(&before)
        .iter()
        .all(|row| { row["observation"]["partitions"]["empty"] == D2_PARTITION_COUNT.get() }));
    cluster.daemons[2].0.kill().unwrap();
    cluster.daemons[2].0.wait().unwrap();
    let blank_volume = cluster.spec.root.join(&cluster.spec.names[2]);
    let check = run_b74_check(&blank_volume);
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&check.stdout).unwrap()["decision"],
        "compatible"
    );
    cluster.daemons[2] = cluster.spec.spawn_binary(2, false, &b74);
    wait_imap_readiness(&http, current, "wire_generation_incompatible").await;
    wait_cluster_imap(&http, current, |body| {
        body["ready"] == false
            && body["complete"] == false
            && cluster_imap_members(body)
                .iter()
                .any(|row| row["observation"].is_null() && row["issue"] == "incompatible")
    })
    .await;
    wait_management_json(
        &http,
        previous,
        MANAGEMENT_CONSENSUS_PROGRESS_PATH,
        |body| {
            body["data"]["items"]
                .as_array()
                .is_some_and(|items| items.len() == 3)
        },
    )
    .await;
    let key = cluster.key_for(1, Some(0), "w10h-b74-fence/");
    let value = Bytes::from_static(b"only-after-compatible-roster");
    let refusal = cluster.clients[1]
        .put(key.clone(), value.clone(), None, None)
        .await
        .unwrap_err();
    assert_eq!(refusal.code(), ErrorCode::Unavailable);
    cluster.daemons[2].0.kill().unwrap();
    cluster.daemons[2].0.wait().unwrap();
    cluster.daemons[2] = cluster.spec.spawn(2);
    let rejoined = connect_w10i(&cluster.adapters[2], "w10i").await;
    wait_cluster_imap(&http, current, |body| {
        body["ready"] == true && body["complete"] == true
    })
    .await;
    wait_w10i_get(&rejoined, key.clone(), None).await;
    assert!(
        retry_w10i(|| cluster.clients[1].put(key.clone(), value.clone(), None, None))
            .await
            .applied
    );
    wait_w10i_get(&rejoined, key, Some(&value)).await;
    println!("W10h published_b74_sha256={digest} same_volume_rollback_before_value_records=true legacy_snapshot_schema_1=true incompatible_before_dispatch=true reupgrade=true");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn w10i_b74_blocks_certified_log_and_snapshot_and_current_recovers() {
    use hydracache_client_hc2::BatchOperation;
    if !w10h_enabled() {
        return;
    }
    let Some(b74) = published_b74() else {
        return;
    };
    let _guard = async_daemon_process_guard().await;
    let mut cluster = MemberCluster::start_fixture(
        "w10i-b74-certified",
        &pki(),
        MemberOptions {
            raft_compaction: true,
            ..Default::default()
        },
    )
    .await;
    let victim = 2;
    let volume = cluster.spec.root.join(&cluster.spec.names[victim]);
    let live = run_b74_check(&volume);
    assert!(!live.status.success());
    assert!(live.stdout.is_empty());
    assert!(String::from_utf8_lossy(&live.stderr).contains("cannot lock member database"));

    let keep = cluster.key_for(1, Some(0), "w10i-b74-keep/");
    let remove = cluster.key_for(0, Some(1), "w10i-b74-remove/");
    let value = Bytes::from_static(b"certified-v2-value");
    let options = RequestOptions {
        timeout: Duration::from_secs(4),
        idempotency_key: Bytes::from_static(b"w10i-b74-retained-receipt"),
    };
    let put = BatchOperation::Put {
        key: keep.clone(),
        value: value.clone(),
        ttl: None,
    };
    let write = |client: Hc2Client| {
        let put = put.clone();
        let options = options.clone();
        async move {
            let results = client.batch_outcomes(vec![put], Some(options)).await?;
            results[0].result.as_ref().map_err(Clone::clone)?;
            Ok(results[0].receipt.unwrap())
        }
    };
    let receipt = retry_w10i(|| write(cluster.clients[1].clone())).await;
    assert!(
        retry_w10i(|| cluster.clients[1].put(remove.clone(), value.clone(), None, None))
            .await
            .applied
    );
    assert!(
        retry_w10i(|| cluster.clients[1].delete(remove.clone(), None))
            .await
            .applied
    );
    wait_w10i_get(&cluster.clients[victim], keep.clone(), Some(&value)).await;
    let http = w10h_http();
    let admin = cluster.spec.addresses[6 + victim];
    let committed = wait_management_json(
        &http,
        cluster.spec.addresses[6],
        ADMIN_RAFT_COMPACTION_PATH,
        |body| body["applied_index"].as_u64().is_some(),
    )
    .await["applied_index"]
        .as_u64()
        .unwrap();
    wait_management_json(&http, admin, ADMIN_RAFT_COMPACTION_PATH, |body| {
        body["applied_index"]
            .as_u64()
            .is_some_and(|index| index >= committed)
    })
    .await;

    cluster.daemons[victim].0.kill().unwrap();
    cluster.daemons[victim].0.wait().unwrap();
    let log_index = assert_b74_blocked(&volume, "raft_log");
    let log_copy = tempfile::tempdir_in(&cluster.spec.root).unwrap();
    let log_dir = log_copy.path().join("member");
    copy_member_fixture(&volume, &log_dir);
    assert_old_b74_refuses_copy(
        &cluster.spec,
        victim,
        &b74,
        &log_dir,
        "invalid raft metadata command",
    )
    .await;
    cluster.daemons[victim] = cluster.spec.spawn(victim);
    let rejoined = connect_w10i(&cluster.adapters[victim], "w10i").await;
    wait_w10i_get(&rejoined, keep.clone(), Some(&value)).await;
    wait_imap_readiness(&http, admin, "ready").await;
    let reply = w10h_admin(http.post(format!("http://{admin}{ADMIN_RAFT_COMPACTION_PATH}")))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    let index = reply["snapshot_index"]
        .as_u64()
        .unwrap_or_else(|| panic!("compaction did not report a durable snapshot: {reply}"));
    assert!(index >= log_index);

    cluster.daemons[victim].0.kill().unwrap();
    cluster.daemons[victim].0.wait().unwrap();
    assert_eq!(assert_b74_blocked(&volume, "raft_snapshot"), index);
    let snapshot_copy = tempfile::tempdir_in(&cluster.spec.root).unwrap();
    let snapshot_dir = snapshot_copy.path().join("member");
    copy_member_fixture(&volume, &snapshot_dir);
    assert_old_b74_refuses_copy(
        &cluster.spec,
        victim,
        &b74,
        &snapshot_dir,
        "unknown variant `CommitValuePrefix`",
    )
    .await;

    cluster.daemons[victim] = cluster.spec.spawn(victim);
    let recovered = connect_w10i(&cluster.adapters[victim], "w10i").await;
    wait_w10i_get(&recovered, keep, Some(&value)).await;
    wait_w10i_get(&recovered, remove, None).await;
    assert_eq!(retry_w10i(|| write(recovered.clone())).await, receipt);
    println!("W10i B74 rollback refused log_index={log_index} snapshot_index={index} source_bytes_unchanged=true published_b74_refused_both_copies=true compatible_restart_value_tombstone_receipt=true");
}

fn published_b74() -> Option<PathBuf> {
    let Some(path) = std::env::var_os("HYDRACACHE_W10H_B74_DAEMON") else {
        eprintln!("SKIP: set HYDRACACHE_W10H_B74_DAEMON to the published Linux binary");
        return None;
    };
    let path = PathBuf::from(path);
    assert_eq!(
        sha256_hex(&std::fs::read(&path).unwrap()),
        "60a37932c554e58a54303f1196c347948fbe0dde162d5a3e32cef5e00f0dd10a"
    );
    Some(path)
}

fn run_b74_check(volume: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_hydracache-server"))
        .arg("check-b74-rollback")
        .arg(volume)
        .output()
        .unwrap()
}

fn assert_b74_blocked(volume: &Path, reason: &str) -> u64 {
    let mut before = std::collections::BTreeMap::new();
    fn read_files(
        root: &Path,
        path: &Path,
        manifest: &mut std::collections::BTreeMap<PathBuf, String>,
    ) {
        for entry in std::fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() {
                read_files(root, &path, manifest);
            } else {
                manifest.insert(
                    path.strip_prefix(root).unwrap().to_path_buf(),
                    sha256_hex(&std::fs::read(path).unwrap()),
                );
            }
        }
    }
    read_files(volume, volume, &mut before);
    let check = run_b74_check(volume);
    assert!(!check.status.success());
    let report: serde_json::Value = serde_json::from_slice(&check.stdout)
        .unwrap_or_else(|err| panic!("{err}: {}", String::from_utf8_lossy(&check.stderr)));
    assert_eq!(report["decision"], "blocked");
    assert_eq!(report["reason"], reason);
    let mut after = std::collections::BTreeMap::new();
    read_files(volume, volume, &mut after);
    assert_eq!(before, after);
    report["raft_index"].as_u64().unwrap()
}

fn copy_member_fixture(source: &Path, destination: &Path) {
    std::fs::create_dir(destination).unwrap();
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let target = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_member_fixture(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

async fn assert_old_b74_refuses_copy(
    spec: &MemberClusterSpec,
    victim: usize,
    binary: &Path,
    copy: &Path,
    reason: &str,
) {
    let ([cluster, admin, client, hc2], reservations) = reserve_addrs::<4>();
    let mut command = daemon_command_for(
        binary,
        client,
        admin,
        hc2,
        &spec.server_cert,
        &spec.server_key,
        &spec.ca,
    );
    command
        .env("HYDRACACHE_ROLE", "member")
        .env("HYDRACACHE_CLIENT_API_ENABLED", "false")
        .env("HYDRACACHE_HC2_ENABLED", "false")
        .env_remove("HYDRACACHE_DISTRIBUTED_HC2_CONFIG")
        .env("HYDRACACHE_CLUSTER_ADDR", cluster.to_string())
        .env("HYDRACACHE_CLUSTER_ADVERTISE_ADDR", cluster.to_string())
        .env("HYDRACACHE_NODE_ID", &spec.names[victim])
        .env("HYDRACACHE_SEEDS", cluster.to_string())
        .env("HYDRACACHE_STORAGE_DIR", copy)
        .env("HYDRACACHE_CLUSTER_AUTH_KEY_ID", "w10i-fixture")
        .env(
            "HYDRACACHE_CLUSTER_AUTH_TOKEN_FILE",
            spec.root.join("cluster-token"),
        );
    drop(reservations);
    let mut child = ChildGuard(command.spawn().unwrap());
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            assert!(!status.success());
            let mut stderr = String::new();
            child
                .0
                .stderr
                .take()
                .unwrap()
                .read_to_string(&mut stderr)
                .unwrap();
            assert!(
                stderr.contains(reason),
                "published 0.74 refused for a different reason: {stderr}"
            );
            return;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "published 0.74 started with incompatible durable IMap metadata"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

fn cluster_imap_members(body: &serde_json::Value) -> &[serde_json::Value] {
    body["members"].as_array().map_or(&[], Vec::as_slice)
}

async fn wait_cluster_imap(
    http: &reqwest::Client,
    address: SocketAddr,
    predicate: impl Fn(&serde_json::Value) -> bool,
) -> serde_json::Value {
    wait_management_json(http, address, MANAGEMENT_CLUSTER_IMAP_PATH, predicate).await
}

async fn wait_management_json(
    http: &reqwest::Client,
    address: SocketAddr,
    path: &str,
    predicate: impl Fn(&serde_json::Value) -> bool,
) -> serde_json::Value {
    let expires = tokio::time::Instant::now() + Duration::from_secs(20);
    loop {
        let response = w10h_admin(http.get(format!("http://{address}{path}")))
            .timeout(Duration::from_secs(2))
            .send()
            .await;
        let mut last = None;
        if let Ok(response) = response {
            let status = response.status();
            if let Ok(body) = response.json::<serde_json::Value>().await {
                if status.is_success() && predicate(&body) {
                    return body;
                }
                last = Some((status, body));
            }
        }
        assert!(
            tokio::time::Instant::now() < expires,
            "{path} at {address} did not reach the expected state: {last:?}"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

fn w10h_enabled() -> bool {
    std::env::var("HYDRACACHE_RUN_DAEMON_PROCESS_E2E").as_deref() == Ok("1")
}

fn w10h_http() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(40))
        .build()
        .unwrap()
}

fn w10h_admin(request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    request
        .header(HYDRACACHE_CLIENT_ID_HEADER, "w10h-probe")
        .header(HYDRACACHE_TENANT_HEADER, "system")
        .header(HYDRACACHE_ADMIN_HEADER, "true")
}

async fn wait_imap_readiness(http: &reqwest::Client, address: SocketAddr, expected: &str) {
    let start = tokio::time::Instant::now();
    loop {
        let mut observed = None;
        if let Ok(response) = http
            .get(format!("http://{address}{ADMIN_READYZ_PATH}"))
            .timeout(Duration::from_secs(1))
            .send()
            .await
        {
            let status = response.status();
            if let Ok(body) = response.json::<serde_json::Value>().await {
                if body["imap"]["reason"] == expected
                    && status.as_u16() == if expected == "ready" { 200 } else { 503 }
                {
                    return;
                }
                observed = Some((status, body));
            }
        }
        assert!(
            start.elapsed() < Duration::from_secs(20),
            "readiness {address} never became {expected}: {observed:?}"
        );
        tokio::time::sleep(Duration::from_millis(40)).await;
    }
}

async fn imap_metrics(http: &reqwest::Client, address: SocketAddr) -> String {
    http.get(format!("http://{address}{ADMIN_METRICS_PATH}"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .text()
        .await
        .unwrap()
}

fn imap_metric(metrics: &str, name: &str) -> u64 {
    let prefix = format!("{name} ");
    metrics
        .lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .unwrap()
        .parse()
        .unwrap()
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
    outbound_faults: [Option<PathBuf>; 3],
    drain_timeout_ms: Option<u64>,
    raft_compaction: bool,
    chrony_feed: Option<PathBuf>,
}

#[derive(Default)]
struct MemberOptions {
    event_fault: bool,
    interrupt_copy: bool,
    drain_timeout_ms: Option<u64>,
    raft_compaction: bool,
    chrony_fixture: bool,
}

impl MemberClusterSpec {
    fn set_value_fault(&self, generation: u64, enabled: bool) {
        self.set_outbound_fault(
            2,
            1,
            generation,
            enabled.then(|| serde_json::json!({"kind": "value_drop"})),
        );
    }

    fn set_outbound_fault(
        &self,
        from: usize,
        to: usize,
        generation: u64,
        action: Option<serde_json::Value>,
    ) {
        let target = self.outbound_faults[from].as_ref().unwrap();
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        let incoming = target.with_extension("incoming");
        let rules: Vec<_> = action
            .into_iter()
            .map(|action| {
                serde_json::json!({
                    "from": self.names[from], "to": self.names[to], "action": action
                })
            })
            .collect();
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
        self.spawn_with_value_profile(index, true)
    }

    fn spawn_with_value_profile(&self, index: usize, value_profile: bool) -> ChildGuard {
        self.spawn_binary(
            index,
            value_profile,
            Path::new(env!("CARGO_BIN_EXE_hydracache-server")),
        )
    }

    fn spawn_binary(&self, index: usize, value_profile: bool, binary: &Path) -> ChildGuard {
        self.spawn_member(index, value_profile, binary, "bootstrap")
    }

    fn spawn_after_drain(&self, index: usize) -> ChildGuard {
        self.spawn_member(
            index,
            true,
            Path::new(env!("CARGO_BIN_EXE_hydracache-server")),
            "join",
        )
    }

    fn spawn_member(
        &self,
        index: usize,
        value_profile: bool,
        binary: &Path,
        start: &str,
    ) -> ChildGuard {
        let log = std::fs::File::options()
            .create(true)
            .append(true)
            .open(self.root.join(format!("member-{index}.log")))
            .unwrap();
        let mut command = daemon_command_for(
            binary,
            self.addresses[3 + index],
            self.addresses[6 + index],
            self.addresses[9 + index],
            &self.server_cert,
            &self.server_key,
            &self.ca,
        );
        if let Some(fault) = &self.outbound_faults[index] {
            command.env("HYDRACACHE_TEST_RAFT_OUTBOUND_FAULT_FILE", fault);
        }
        if let Some(timeout) = self.drain_timeout_ms {
            command.env("HYDRACACHE_DRAIN_TIMEOUT_MS", timeout.to_string());
        }
        if self.raft_compaction {
            command.env("HYDRACACHE_RAFT_COMPACTION", "true");
        }
        if let Some(feed) = &self.chrony_feed {
            command.env("HYDRACACHE_TEST_CHRONYC_FEED", feed);
        }
        if value_profile {
            command.env(
                "HYDRACACHE_DISTRIBUTED_HC2_CONFIG",
                self.root.join("imap.toml"),
            );
        } else {
            command
                .env("HYDRACACHE_HC2_ENABLED", "false")
                .env_remove("HYDRACACHE_DISTRIBUTED_HC2_CONFIG");
        }
        ChildGuard(
            command
                .env("HYDRACACHE_CLIENT_API_ENABLED", "false")
                .env("HYDRACACHE_ROLE", "member")
                .env("HYDRACACHE_CLUSTER_START", start)
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
        Self::start_fixture(label, &pki(), MemberOptions::default()).await
    }

    async fn start_fixture(label: &str, pki: &TestPki, options: MemberOptions) -> Self {
        let root = test_root(label);
        let (server_cert, server_key, ca) = write_pki(&root, pki);
        let mut pem = std::io::BufReader::new(pki.client_cert.as_bytes());
        let leaf = <rustls::pki_types::CertificateDer as rustls::pki_types::pem::PemObject>
            ::from_pem_reader(&mut pem).unwrap();
        let chrony_feed = options.chrony_fixture.then(|| root.join("chronyc-feed"));
        let chrony = chrony_feed.as_ref().map(|feed| {
            std::fs::write(feed, "live").unwrap();
            hydracache_server::IMapChronyConfig {
                chronyc: Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/chronyc"),
                references: vec!["192.0.2.1".parse().unwrap()],
                sample_interval_ms: 250,
                policy: hydracache_server::imap_chrony::ChronyPolicy {
                    maximum_uncertainty_ms: 100,
                    maximum_snapshot_age_ms: 1_000,
                    maximum_reference_age_ms: 1_000,
                    maximum_acquisition_ms: 75,
                    maximum_drift_ppm: 500,
                },
            }
        });
        let profile = hydracache_server::DistributedHc2Config {
            tenant: "w10i".into(),
            client_cert_sha256: sha256_hex(leaf.as_ref()),
            value_plane: Default::default(),
            chrony,
        };
        std::fs::write(root.join("imap.toml"), toml::to_string(&profile).unwrap()).unwrap();
        std::fs::write(
            root.join("cluster-token"),
            format!("fixture-only-{}", root.display()),
        )
        .unwrap();
        // Socket-derived names affect rendezvous placement. Select a roster that
        // can actually exercise each directed primary/backup pair in this suite.
        let (addresses, reservations, names) = (0..16)
            .find_map(|_| {
                let (addresses, reservations) = reserve_addrs::<12>();
                let names: Vec<String> = addresses[..3]
                    .iter()
                    .map(|addr| format!("member-{}", addr.to_string().replace(['.', ':'], "-")))
                    .collect();
                let members = placement_members(&names);
                let mut pairs = std::collections::BTreeMap::new();
                for n in 0..D2_PARTITION_COUNT.get() {
                    let placement = RendezvousClusterOwnership
                        .replicas_for_key(&format!("partition-{n}"), &members, 2)
                        .unwrap();
                    *pairs
                        .entry((placement.primary, placement.backups))
                        .or_insert(0) += 1;
                }
                let original_pair = (members[0].node_id.clone(), vec![members[1].node_id.clone()]);
                (pairs.len() == 6 && (!options.interrupt_copy || pairs[&original_pair] >= 2))
                    .then_some((addresses, reservations, names))
            })
            .expect("fixture needs all six primary/backup pairs");
        let seeds = addresses[..3]
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let spec = MemberClusterSpec {
            outbound_faults: [
                options
                    .interrupt_copy
                    .then(|| root.join(&names[0]).join("copy-fault.json")),
                None,
                options
                    .event_fault
                    .then(|| root.join(&names[2]).join("event-fault.json")),
            ],
            drain_timeout_ms: options.drain_timeout_ms,
            raft_compaction: options.raft_compaction,
            chrony_feed,
            root,
            addresses,
            names,
            seeds,
            server_cert,
            server_key,
            ca,
        };
        if options.event_fault {
            spec.set_value_fault(1, false);
        }
        if options.interrupt_copy {
            spec.set_outbound_fault(0, 2, 1, None);
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
        for &address in &addresses[6..9] {
            let endpoint = format!("http://{address}{ADMIN_CLUSTER_OVERVIEW_PATH}");
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
                    "three members were never admitted at {address}"
                );
                tokio::time::sleep(Duration::from_millis(70)).await;
            }
            wait_imap_readiness(&status, address, "ready").await;
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
        self.pick_key(primary, backup, key, |_| true)
    }

    fn pick_key(
        &self,
        primary: usize,
        backup: Option<usize>,
        key: impl Fn(u32) -> Vec<u8>,
        allow_partition: impl Fn(hydracache::PartitionId) -> bool,
    ) -> Bytes {
        let scope = fixture_imap_scope();
        let members = placement_members(&self.spec.names);
        (0..5000)
            .find_map(|n| {
                let key = key(n);
                let (partition, _) = scope.locate(key.clone()).unwrap();
                if !allow_partition(partition) {
                    return None;
                }
                let ranking = RendezvousClusterOwnership
                    .replicas_for_key(&format!("partition-{}", partition.value()), &members, 2)
                    .unwrap();
                (ranking.primary == members[primary].node_id
                    && backup.is_none_or(|b| ranking.backups == [members[b].node_id.clone()]))
                .then(|| Bytes::from(key))
            })
            .unwrap_or_else(|| panic!("no key for primary {primary} and backup {backup:?}"))
    }
}

fn fixture_imap_scope() -> ReplicaReadScope {
    ReplicaReadScope::new(
        "w10i".to_owned(),
        Namespace::new("hc2").unwrap(),
        std::num::NonZeroU64::MIN,
        |_| None,
    )
}

fn placement_members(names: &[String]) -> Vec<ClusterMember> {
    names
        .iter()
        .map(|name| ClusterMember {
            node_id: ClusterNodeId::new(name),
            generation: ClusterGeneration::new(1),
            role: ClusterRole::Member,
            epoch: ClusterEpoch::new(3),
            endpoints: Default::default(),
            metadata: Default::default(),
        })
        .collect()
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
