use std::fs;
use std::io::ErrorKind;
use std::net::{SocketAddr, TcpListener, UdpSocket};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use hydracache_server::{ServerConfig, ServerRole, ServerRuntime};
use reqwest::{Client, StatusCode};

const NODE_ID: &str = "raft-wire-corpus-member";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(3);
const ERROR_BODY_LIMIT: usize = 8 * 1024;
const AXUM_DEFAULT_BODY_LIMIT: usize = 2 * 1024 * 1024;

static STORAGE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn raft_http_socket_corpus_rejects_before_unbounded_allocation() {
    std::env::remove_var("HYDRACACHE_GRID_INPROC");
    let storage_dir = unique_storage_dir();
    let _ = fs::remove_dir_all(&storage_dir);
    let address = reserve_loopback_addr();
    let config = member_config(address, storage_dir.clone());
    let mut runtime = ServerRuntime::new(config.clone()).unwrap().start();
    wait_for_listener_and_leader(&runtime, address).await;

    let before = runtime.admin_status();
    let raft_before = runtime
        .raft_compaction_status()
        .expect("Sled-backed member must expose durable raft progress");
    let identity_path = storage_dir.join("node-identity.json");
    let identity_before = fs::read(&identity_path).expect("member identity must be durable");
    let client = Client::builder().timeout(REQUEST_TIMEOUT).build().unwrap();

    for (seed, expected) in [
        ("truncated-json.seed", StatusCode::BAD_REQUEST),
        ("invalid-base64.json", StatusCode::INTERNAL_SERVER_ERROR),
        ("malformed-protobuf.json", StatusCode::INTERNAL_SERVER_ERROR),
    ] {
        let body =
            fs::read(raft_wire_corpus().join(seed)).expect("committed seed must be readable");
        let status = post_raft_frame(&client, address, body).await;
        assert_eq!(status, expected, "unexpected rejection for seed {seed}");
        assert_eq!(
            runtime.admin_status(),
            before,
            "malformed seed {seed} mutated the live Sled-backed raft state"
        );
        assert_eq!(
            runtime.raft_compaction_status().unwrap(),
            raft_before,
            "malformed seed {seed} changed durable raft log progress"
        );
        assert_eq!(
            fs::read(&identity_path).unwrap(),
            identity_before,
            "malformed seed {seed} changed durable member identity"
        );
    }

    let status = post_raft_frame(&client, address, oversized_raft_body(before.term)).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(
        runtime.admin_status(),
        before,
        "oversized body reached and mutated the raft handler"
    );
    assert_eq!(
        runtime.raft_compaction_status().unwrap(),
        raft_before,
        "oversized body changed durable raft log progress"
    );

    let _ = runtime.shutdown();
    drop(runtime);

    let restart_address = reserve_loopback_addr();
    let mut restart_config = config;
    restart_config.cluster_addr = restart_address;
    restart_config.cluster_advertise_addr = Some(restart_address.to_string());
    restart_config.seeds = vec![restart_address.to_string()];
    let mut restarted = ServerRuntime::new(restart_config).unwrap().start();
    wait_for_listener_and_leader(&restarted, restart_address).await;
    let recovered = restarted.admin_status();
    assert_eq!(recovered.members, before.members);
    assert_eq!(recovered.voters, before.voters);
    assert_eq!(recovered.leader.as_deref(), Some(NODE_ID));
    assert_eq!(fs::read(&identity_path).unwrap(), identity_before);

    let _ = restarted.shutdown();
    drop(restarted);
    let _ = fs::remove_dir_all(storage_dir);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn canary_raft_socket_accepts_an_oversized_body_without_bound() {
    std::env::remove_var("HYDRACACHE_GRID_INPROC");
    let storage_dir = unique_storage_dir();
    let _ = fs::remove_dir_all(&storage_dir);
    let address = reserve_loopback_addr();
    let config = member_config(address, storage_dir.clone());
    let mut runtime = ServerRuntime::new(config).unwrap().start();
    wait_for_listener_and_leader(&runtime, address).await;

    let client = Client::builder().timeout(REQUEST_TIMEOUT).build().unwrap();
    let status = post_raft_frame(
        &client,
        address,
        oversized_raft_body(runtime.admin_status().term),
    )
    .await;

    let _ = runtime.shutdown();
    drop(runtime);
    let _ = fs::remove_dir_all(storage_dir);

    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W9") {
        assert!(
            status.is_success(),
            "HC-CANARY-RED:W9 oversized raft HTTP body was accepted without a bound"
        );
    }
    assert_eq!(
        status,
        StatusCode::PAYLOAD_TOO_LARGE,
        "the real raft listener must enforce its existing request-body bound"
    );
}

async fn post_raft_frame(client: &Client, address: SocketAddr, body: Vec<u8>) -> StatusCode {
    let response = tokio::time::timeout(
        REQUEST_TIMEOUT,
        client
            .post(format!("http://{address}/cluster/raft/append"))
            .header("content-type", "application/json")
            .body(body)
            .send(),
    )
    .await
    .expect("raft corpus request exceeded the bounded request timeout")
    .expect("raft corpus request must receive an HTTP rejection");
    let status = response.status();
    let error_body = tokio::time::timeout(REQUEST_TIMEOUT, response.bytes())
        .await
        .expect("raft rejection body exceeded the bounded response timeout")
        .expect("raft rejection body must be readable");
    assert!(
        error_body.len() <= ERROR_BODY_LIMIT,
        "raft rejection body exceeded {ERROR_BODY_LIMIT} bytes"
    );
    status
}

fn oversized_raft_body(term: u64) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "from": NODE_ID,
        "to": NODE_ID,
        "term": term,
        "payload_base64": "A".repeat(AXUM_DEFAULT_BODY_LIMIT),
    }))
    .unwrap()
}

async fn wait_for_listener_and_leader(runtime: &ServerRuntime, address: SocketAddr) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let status = runtime.admin_status();
            if status.leader.as_deref() == Some(NODE_ID)
                && status.members == 1
                && status.voters == 1
                && tokio::net::TcpStream::connect(address).await.is_ok()
            {
                return;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("real raft socket did not become ready with a single-node leader");
}

fn member_config(cluster_addr: SocketAddr, storage_dir: PathBuf) -> ServerConfig {
    ServerConfig {
        role: ServerRole::Member,
        cluster_addr,
        cluster_advertise_addr: Some(cluster_addr.to_string()),
        node_id: Some(NODE_ID.to_owned()),
        seeds: vec![cluster_addr.to_string()],
        storage_dir: Some(storage_dir),
        drain_timeout_ms: 1_000,
        ..ServerConfig::default()
    }
}

fn raft_wire_corpus() -> &'static Path {
    Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fuzz/corpus/raft_wire_frame"
    ))
}

fn unique_storage_dir() -> PathBuf {
    let sequence = STORAGE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    PathBuf::from(format!(
        "target/test-hydracache-server/raft-wire-corpus-{}-{sequence}",
        std::process::id()
    ))
}

fn reserve_loopback_addr() -> SocketAddr {
    loop {
        let udp = UdpSocket::bind("127.0.0.1:0").expect("loopback UDP port must be reservable");
        let address = udp.local_addr().unwrap();
        match TcpListener::bind(address) {
            Ok(tcp) => {
                drop(tcp);
                drop(udp);
                return address;
            }
            Err(error)
                if matches!(
                    error.kind(),
                    ErrorKind::AddrInUse | ErrorKind::PermissionDenied
                ) => {}
            Err(error) => panic!("failed to reserve loopback TCP port {address}: {error}"),
        }
    }
}
