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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn distributed_hc2_routes_public_requests_and_recovers_after_owner_kill() {
    if std::env::var("HYDRACACHE_RUN_DAEMON_PROCESS_E2E").as_deref() != Ok("1") {
        return;
    }
    let _guard = async_daemon_process_guard().await;
    let root = test_root("distributed-imap");
    let pki = pki();
    let (server_cert, server_key, ca) = write_pki(&root, &pki);
    let mut pem = std::io::BufReader::new(pki.client_cert.as_bytes());
    let leaf =
        <rustls::pki_types::CertificateDer as rustls::pki_types::pem::PemObject>::from_pem_reader(
            &mut pem,
        )
        .unwrap();
    let profile = root.join("imap.toml");
    std::fs::write(
        &profile,
        format!(
            "tenant = \"w10i\"\nclient_cert_sha256 = \"{:x}\"\n",
            Sha256::digest(&leaf)
        ),
    )
    .unwrap();
    let token = root.join("cluster-token");
    std::fs::write(&token, format!("fixture-only-{}", root.display())).unwrap();
    let (addresses, reservations) = reserve_addrs::<12>();
    let names = addresses[..3]
        .iter()
        .map(|addr| format!("member-{}", addr.to_string().replace(['.', ':'], "-")))
        .collect::<Vec<_>>();
    let seeds = addresses[..3]
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let spawn = |index: usize| {
        let log = std::fs::File::options()
            .create(true)
            .append(true)
            .open(root.join(format!("member-{index}.log")))
            .unwrap();
        let mut command = daemon_command(
            addresses[3 + index],
            addresses[6 + index],
            addresses[9 + index],
            &server_cert,
            &server_key,
            &ca,
        );
        ChildGuard(
            command
                .env("HYDRACACHE_CLIENT_API_ENABLED", "false")
                .env("HYDRACACHE_ROLE", "member")
                .env("HYDRACACHE_CLUSTER_ADDR", addresses[index].to_string())
                .env(
                    "HYDRACACHE_CLUSTER_ADVERTISE_ADDR",
                    addresses[index].to_string(),
                )
                .env("HYDRACACHE_NODE_ID", &names[index])
                .env("HYDRACACHE_SEEDS", &seeds)
                .env("HYDRACACHE_STORAGE_DIR", root.join(&names[index]))
                .env("HYDRACACHE_CLUSTER_AUTH_KEY_ID", "w10i-fixture")
                .env("HYDRACACHE_CLUSTER_AUTH_TOKEN_FILE", &token)
                .env("HYDRACACHE_DISTRIBUTED_HC2_CONFIG", &profile)
                .stdout(Stdio::from(log.try_clone().unwrap()))
                .stderr(Stdio::from(log))
                .spawn()
                .unwrap(),
        )
    };
    drop(reservations);
    let mut daemons: Vec<_> = (0..3).map(spawn).collect();
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
    let scope = ReplicaReadScope::new(
        "w10i".to_owned(),
        Namespace::new("hc2").unwrap(),
        std::num::NonZeroU64::MIN,
        |_| None,
    );
    let members: Vec<_> = names
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
    let key_for = |primary: usize, backup: Option<usize>| {
        (0..5000)
            .find_map(|n| {
                let key = format!("w10i-\0-{n}").into_bytes();
                let (partition, _) = scope.locate(key.clone()).unwrap();
                let ranking = RendezvousClusterOwnership
                    .replicas_for_key(&format!("partition-{}", partition.value()), &members, 2)
                    .unwrap();
                (ranking.primary == members[primary].node_id
                    && backup.is_none_or(|b| ranking.backups == [members[b].node_id.clone()]))
                .then(|| Bytes::from(key))
            })
            .unwrap()
    };
    let hot = key_for(0, Some(1));
    let control = key_for(1, Some(2));
    let initial = Bytes::from_static(b"initial");
    let winner = Bytes::from_static(b"confirmed-before-kill");
    let recovered = Bytes::from_static(b"confirmed-after-kill");
    println!("W10i daemon process artifacts {}", root.display());
    let deadline = tokio::time::Instant::now() + Duration::from_secs(18);
    let status = reqwest::Client::new();
    loop {
        let endpoint = format!("http://{}{ADMIN_CLUSTER_OVERVIEW_PATH}", addresses[7]);
        if let Ok(response) = status.get(endpoint).send().await {
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
    let not_admitted = connect_w10i(&adapters[2], "different-tenant").await;
    assert_eq!(
        not_admitted.get(hot.clone(), None).await.unwrap_err().code,
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
            .code,
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

    let control_client = clients[2].clone();
    let load = tokio::spawn(async move {
        let mut acknowledgements = 0;
        let end = tokio::time::Instant::now() + Duration::from_secs(5);
        while tokio::time::Instant::now() < end {
            if control_client
                .put(control.clone(), Bytes::from_static(b"load"), None, None)
                .await
                .is_ok_and(|reply| reply.applied)
            {
                acknowledgements += 1;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        acknowledgements
    });
    let killed = daemons[0].0.id();
    daemons[0].0.kill().unwrap();
    daemons[0].0.wait().unwrap();
    wait_w10i_get(&clients[2], hot.clone(), Some(&winner)).await;
    assert!(
        clients[1]
            .compare_and_set(hot.clone(), initial, winner.clone(), None, Some(options))
            .await
            .unwrap()
            .applied
    );
    let ready = tokio::time::Instant::now() + Duration::from_secs(18);
    loop {
        match clients[2]
            .compare_and_set(hot.clone(), winner.clone(), recovered.clone(), None, None)
            .await
        {
            Ok(result) => {
                assert!(result.applied);
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
                        error.code,
                        ErrorCode::Unavailable | ErrorCode::DeadlineExceeded
                    ),
                    "{error}"
                );
                tokio::time::sleep(Duration::from_millis(90)).await;
            }
            Err(error) => panic!("fresh write was still blocked after owner kill: {error}"),
        }
    }
    assert!(load.await.unwrap() > 0);
    daemons[0] = spawn(0);
    clients[0] = connect_w10i(&adapters[0], "w10i").await;
    wait_w10i_get(&clients[0], hot.clone(), Some(&recovered)).await;
    assert!(
        clients[2]
            .remove_if_value(hot.clone(), recovered, None)
            .await
            .unwrap()
            .applied
    );
    wait_w10i_get(&clients[0], hot, None).await;
    println!(
        "W10i public HC/2 killed_pid={killed} rejoined_pid={} retained_cas_and_tombstone=true",
        daemons[0].0.id()
    );
}

async fn connect_w10i(adapter: &GrpcMtlsAdapter, tenant: &str) -> Hc2Client {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(18);
    loop {
        let mut config = ClientConfig::new("w10i-sdk", tenant);
        config.capabilities = vec![Capability::Data];
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
