use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, UdpSocket};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use hydracache_server::{
    AdminApiConfig, BackupConfig, ClientApiConfig, ClusterAuthConfig, ServerAdminStatus,
    ServerConfig, ServerConfigError, ServerRole, ServerRuntime, StatusSource, TlsConfig,
};
use serde_json::json;

fn member_config(name: &str) -> ServerConfig {
    ServerConfig {
        role: ServerRole::Member,
        listen_addr: "127.0.0.1:18080".parse().unwrap(),
        cluster_addr: "127.0.0.1:0".parse().unwrap(),
        node_id: None,
        seeds: vec!["127.0.0.1:0".to_owned()],
        storage_dir: Some(PathBuf::from(format!(
            "target/test-hydracache-grid-host/{name}"
        ))),
        drain_timeout_ms: 1_000,
        tls: TlsConfig::default(),
        cluster_auth: ClusterAuthConfig::default(),
        backup: BackupConfig::default(),
        client_api: ClientApiConfig::default(),
        admin_api: AdminApiConfig::default(),
        ..ServerConfig::default()
    }
}

fn local_config() -> ServerConfig {
    ServerConfig {
        role: ServerRole::Local,
        seeds: Vec::new(),
        storage_dir: None,
        ..member_config("local")
    }
}

fn client_config() -> ServerConfig {
    ServerConfig {
        role: ServerRole::Client,
        storage_dir: None,
        ..member_config("client")
    }
}

fn grid_env_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

fn configured_tls() -> TlsConfig {
    TlsConfig {
        enabled: true,
        cert_path: Some(PathBuf::from(
            "target/test-hydracache-grid-host/tls/cert.pem",
        )),
        key_path: Some(PathBuf::from(
            "target/test-hydracache-grid-host/tls/key.pem",
        )),
        ca_path: Some(PathBuf::from("target/test-hydracache-grid-host/tls/ca.pem")),
        acknowledge_insecure: false,
    }
}

fn configure_cluster_auth(config: &mut ServerConfig, name: &str) {
    let dir = PathBuf::from(format!("target/test-hydracache-grid-host/{name}/auth"));
    std::fs::create_dir_all(&dir).unwrap();
    let token_file = dir.join("token");
    std::fs::write(&token_file, "secret\n").unwrap();
    config.cluster_auth.key_id = Some("k1".to_owned());
    config.cluster_auth.token_file = Some(token_file);
}

fn configure_test_tls(config: &mut ServerConfig, name: &str) {
    let material =
        write_test_tls_material(Path::new("target/test-hydracache-grid-host").join(name));
    config.tls = TlsConfig {
        enabled: true,
        cert_path: Some(material.cert_path),
        key_path: Some(material.key_path),
        ca_path: Some(material.ca_path),
        acknowledge_insecure: false,
    };
}

struct TestTlsMaterial {
    cert_path: PathBuf,
    key_path: PathBuf,
    ca_path: PathBuf,
}

fn write_test_tls_material(dir: PathBuf) -> TestTlsMaterial {
    std::fs::create_dir_all(&dir).unwrap();
    let rcgen::CertifiedKey { cert, signing_key } =
        rcgen::generate_simple_self_signed(["127.0.0.1".to_owned(), "localhost".to_owned()])
            .unwrap();
    let cert_path = dir.join("cert.pem");
    let key_path = dir.join("key.pem");
    let ca_path = dir.join("ca.pem");
    std::fs::write(&cert_path, cert.pem()).unwrap();
    std::fs::write(&key_path, signing_key.serialize_pem()).unwrap();
    std::fs::write(&ca_path, cert.pem()).unwrap();
    TestTlsMaterial {
        cert_path,
        key_path,
        ca_path,
    }
}

#[test]
fn member_builds_networked_triad_with_shared_raft_control_plane() {
    let _env = grid_env_lock();
    std::env::remove_var("HYDRACACHE_GRID_INPROC");
    let config = member_config("networked-shared-raft");
    let storage_dir = config.storage_dir.clone().unwrap();
    let _ = std::fs::remove_dir_all(&storage_dir);

    let runtime = ServerRuntime::new(config).unwrap().start();

    let status = runtime.admin_status();
    assert_eq!(status.source, StatusSource::Live);
    let leader = status.leader.clone().expect("networked raft leader");
    assert!(leader.starts_with("member-"));
    assert_eq!(status.term, 1);
    assert!(status.quorum_ok);
    assert_eq!(status.members, 1);
    assert!(storage_dir.join("raft-log").is_dir());
    assert_eq!(
        runtime.cache().cluster_diagnostics().unwrap().member_count,
        status.members as usize
    );

    let overview = serde_json::to_value(runtime.cluster_overview()).unwrap();
    assert_eq!(overview["source"], "live");
    assert_eq!(overview["leader"]["node_id"], json!(leader));
    assert_eq!(overview["members"].as_array().unwrap().len(), 1);
    assert_eq!(overview["members"][0]["role"], "member");
    assert_eq!(overview["members"][0]["reachable"], true);
    assert_eq!(overview["members"][0]["reachability"], "reachable");
    assert_eq!(overview["members"][0]["generation"], 1);
}

#[test]
fn local_and_client_roles_stay_modeled() {
    let local = ServerRuntime::new(local_config()).unwrap().start();
    let client = ServerRuntime::new(client_config()).unwrap().start();

    assert_eq!(local.admin_status().source, StatusSource::Modeled);
    assert_eq!(client.admin_status().source, StatusSource::Modeled);
    assert_eq!(local.admin_status().members, 0);
    assert_eq!(client.admin_status().members, 0);
}

#[test]
fn member_without_storage_or_seeds_is_rejected_loud() {
    let mut missing_storage = member_config("missing-storage");
    missing_storage.storage_dir = None;
    assert!(matches!(
        ServerRuntime::new(missing_storage),
        Err(ServerConfigError::MissingStorageDir)
    ));

    let mut missing_seeds = member_config("missing-seeds");
    missing_seeds.seeds.clear();
    assert!(matches!(
        ServerRuntime::new(missing_seeds),
        Err(ServerConfigError::MissingSeeds)
    ));
}

#[test]
fn inproc_fallback_still_builds_under_env_flag() {
    let _env = grid_env_lock();
    std::env::set_var("HYDRACACHE_GRID_INPROC", "1");
    let runtime = ServerRuntime::new(member_config("inproc-fallback"))
        .unwrap()
        .start();
    std::env::remove_var("HYDRACACHE_GRID_INPROC");

    let status = runtime.admin_status();
    assert_eq!(status.source, StatusSource::Live);
    assert_eq!(status.leader, None);
    assert_eq!(status.term, 1);
    assert!(status.quorum_ok);
    assert_eq!(status.members, 1);
}

#[test]
fn draining_member_leaves_raft_config_cleanly() {
    let _env = grid_env_lock();
    std::env::remove_var("HYDRACACHE_GRID_INPROC");
    let mut runtime = ServerRuntime::new(member_config("draining-leaves-raft"))
        .unwrap()
        .start();
    assert_eq!(runtime.admin_status().members, 1);

    let drain = runtime.shutdown();

    assert!(!drain.timed_out);
    assert_eq!(
        runtime.cache().cluster_diagnostics().unwrap().member_count,
        0
    );
    let status = runtime.admin_status();
    assert_eq!(status.members, 0);
    assert!(!status.quorum_ok);
}

#[test]
fn non_loopback_member_without_tls_is_rejected_loud() {
    let mut config = member_config("non-loopback-without-tls");
    config.cluster_addr = "0.0.0.0:17057".parse().unwrap();
    config.seeds = vec!["0.0.0.0:17057".to_owned()];

    assert!(matches!(
        ServerRuntime::new(config),
        Err(ServerConfigError::NonLoopbackWithoutTls)
    ));
}

#[test]
fn tls_enabled_member_without_cluster_auth_fails_loud_at_startup() {
    let _env = grid_env_lock();
    std::env::remove_var("HYDRACACHE_GRID_INPROC");
    let mut config = member_config("tls-without-cluster-auth");
    config.tls = configured_tls();

    let error = ServerRuntime::new(config).unwrap_err();

    assert!(matches!(error, ServerConfigError::GridHostStart(_)));
    assert!(
        error.to_string().contains("[cluster_auth]"),
        "error should name missing cluster_auth: {error}"
    );
}

#[test]
fn tls_member_with_unreadable_cert_fails_loud_at_startup() {
    let _env = grid_env_lock();
    std::env::remove_var("HYDRACACHE_GRID_INPROC");
    let mut config = member_config("tls-unreadable-cert");
    configure_cluster_auth(&mut config, "tls-unreadable-cert");
    config.tls = configured_tls();

    let error = ServerRuntime::new(config).unwrap_err();

    assert!(matches!(error, ServerConfigError::GridHostStart(_)));
    assert!(
        error.to_string().contains("cert")
            && error
                .to_string()
                .contains("target/test-hydracache-grid-host/tls/cert.pem"),
        "error should name the unreadable TLS cert path: {error}"
    );
}

#[test]
fn cluster_listener_rejects_plaintext_when_tls_enabled() {
    let _env = grid_env_lock();
    std::env::remove_var("HYDRACACHE_GRID_INPROC");
    let addr = reserve_loopback_addrs(1).remove(0);
    let mut config = member_config("tls-listener-rejects-plaintext");
    config.cluster_addr = addr;
    config.seeds = vec![addr.to_string()];
    configure_cluster_auth(&mut config, "tls-listener-rejects-plaintext");
    configure_test_tls(&mut config, "tls-listener-rejects-plaintext/tls");

    let mut runtime = ServerRuntime::new(config).unwrap().start();
    let rejected = wait_until(Duration::from_secs(3), || {
        let Ok(mut stream) =
            std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(250))
        else {
            return false;
        };
        stream
            .set_read_timeout(Some(Duration::from_millis(250)))
            .unwrap();
        stream
            .write_all(b"POST /cluster/v1/raft/append HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 0\r\n\r\n")
            .unwrap();
        let mut buffer = [0_u8; 16];
        match stream.read(&mut buffer) {
            Ok(0) => true,
            Ok(bytes) => !String::from_utf8_lossy(&buffer[..bytes]).starts_with("HTTP/"),
            Err(_) => true,
        }
    });

    let _ = runtime.shutdown();
    assert!(
        rejected,
        "plain HTTP unexpectedly reached the TLS cluster listener on {addr}"
    );
}

#[test]
fn member_identity_persists_across_address_change() {
    let _env = grid_env_lock();
    std::env::remove_var("HYDRACACHE_GRID_INPROC");
    let addrs = reserve_loopback_addrs(2);
    let mut config = member_config("identity-persists-address-change");
    let storage_dir = config.storage_dir.clone().unwrap();
    let _ = std::fs::remove_dir_all(&storage_dir);
    config.cluster_addr = addrs[0];
    config.seeds = vec![addrs[0].to_string()];

    let mut runtime = ServerRuntime::new(config.clone()).unwrap().start();
    let first_identity = read_node_identity(&storage_dir);
    let first_node_id = first_identity["node_id"].as_str().unwrap().to_owned();
    let first_raft_node_id = first_identity["raft_node_id"].as_u64().unwrap();
    assert_eq!(first_node_id, member_node_id_for_addr(addrs[0]));
    assert_eq!(
        runtime.admin_status().leader.as_deref(),
        Some(first_node_id.as_str())
    );
    let _ = runtime.shutdown();

    config.cluster_addr = addrs[1];
    config.seeds = vec![addrs[1].to_string()];
    let mut restarted = ServerRuntime::new(config).unwrap().start();
    let second_identity = read_node_identity(&storage_dir);

    assert_eq!(second_identity["node_id"], first_identity["node_id"]);
    assert_eq!(
        second_identity["raft_node_id"].as_u64(),
        Some(first_raft_node_id)
    );
    assert_eq!(
        restarted.admin_status().leader.as_deref(),
        Some(first_node_id.as_str())
    );
    let _ = restarted.shutdown();
}

#[test]
fn configured_node_id_conflicting_with_persisted_identity_fails_loud() {
    let _env = grid_env_lock();
    std::env::remove_var("HYDRACACHE_GRID_INPROC");
    let mut config = member_config("identity-conflict");
    let storage_dir = config.storage_dir.clone().unwrap();
    let _ = std::fs::remove_dir_all(&storage_dir);
    config.node_id = Some("member-pinned".to_owned());

    let mut runtime = ServerRuntime::new(config.clone()).unwrap().start();
    let identity = read_node_identity(&storage_dir);
    assert_eq!(identity["node_id"], "member-pinned");
    let _ = runtime.shutdown();

    config.node_id = Some("member-other".to_owned());
    let error = ServerRuntime::new(config).unwrap_err();

    assert!(matches!(error, ServerConfigError::GridHostStart(_)));
    assert!(
        error
            .to_string()
            .contains("conflicts with persisted node identity"),
        "conflicting node_id should fail loud: {error}"
    );
}

#[test]
fn future_node_identity_format_fails_loud() {
    let _env = grid_env_lock();
    std::env::remove_var("HYDRACACHE_GRID_INPROC");
    let config = member_config("identity-future-format");
    let storage_dir = config.storage_dir.clone().unwrap();
    let _ = std::fs::remove_dir_all(&storage_dir);
    std::fs::create_dir_all(&storage_dir).unwrap();
    std::fs::write(
        storage_dir.join("node-identity.json"),
        r#"{
  "format_version": 999,
  "cluster": "hydracache",
  "node_id": "member-future",
  "raft_node_id": 1
}"#,
    )
    .unwrap();

    let error = ServerRuntime::new(config).unwrap_err();

    assert!(matches!(error, ServerConfigError::GridHostStart(_)));
    assert!(
        error
            .to_string()
            .contains("unknown future node identity format version 999"),
        "future node identity format should fail loud: {error}"
    );
}

#[test]
fn multi_node_members_form_a_cluster_and_elect_one_leader() {
    if !networked_daemon_e2e_enabled() {
        return;
    }

    let _env = grid_env_lock();
    std::env::remove_var("HYDRACACHE_GRID_INPROC");
    let addrs = reserve_loopback_addrs(3);
    let seeds = addrs.iter().map(ToString::to_string).collect::<Vec<_>>();
    let mut configs = addrs
        .iter()
        .enumerate()
        .map(|(index, addr)| {
            let mut config = member_config(&format!("networked-daemon-e2e-{index}"));
            config.cluster_addr = *addr;
            config.seeds = seeds.clone();
            if let Some(storage_dir) = &config.storage_dir {
                let _ = std::fs::remove_dir_all(storage_dir);
            }
            config
        })
        .collect::<Vec<_>>();

    let handles = configs
        .drain(..)
        .map(|config| thread::spawn(move || ServerRuntime::new(config).unwrap().start()))
        .collect::<Vec<_>>();
    let runtimes = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();
    let mut runtimes = runtimes.into_iter().map(Some).collect::<Vec<_>>();

    let converged = wait_until(Duration::from_secs(10), || {
        let statuses = active_statuses(&runtimes);
        let leaders = leaders(&statuses);
        leaders.len() == 1
            && statuses
                .iter()
                .all(|status| status.members == 3 && status.quorum_ok)
    });
    assert!(
        converged,
        "three networked daemon members did not converge to one leader: {:?}",
        active_statuses(&runtimes)
    );
    let expected_member_ids = addrs
        .iter()
        .map(|addr| member_node_id_for_addr(*addr))
        .collect::<BTreeSet<_>>();
    for member_ids in active_member_id_sets(&runtimes) {
        assert_eq!(
            member_ids, expected_member_ids,
            "converged daemon did not expose the full committed member set"
        );
    }

    let first_leader = active_statuses(&runtimes)[0]
        .leader
        .clone()
        .expect("leader after convergence");
    let follower_index = addrs
        .iter()
        .position(|addr| member_node_id_for_addr(*addr) != first_leader)
        .expect("at least one follower belongs to the spawned daemon set");
    let drained_follower = member_node_id_for_addr(addrs[follower_index]);
    let expected_after_follower_drain = expected_member_ids
        .iter()
        .filter(|node_id| *node_id != &drained_follower)
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut follower_runtime = runtimes[follower_index].take().unwrap();
    let follower_drain = follower_runtime.shutdown();
    assert!(!follower_drain.timed_out);
    drop(follower_runtime);

    let follower_removed = wait_until(Duration::from_secs(10), || {
        let statuses = active_statuses(&runtimes);
        let leaders = leaders(&statuses);
        leaders.len() == 1
            && statuses
                .iter()
                .all(|status| status.members == 2 && status.quorum_ok)
    });
    assert!(
        follower_removed,
        "remaining daemon members did not converge after follower drain: follower={drained_follower}, statuses={:?}",
        active_statuses(&runtimes)
    );
    for member_ids in active_member_id_sets(&runtimes) {
        assert_eq!(
            member_ids, expected_after_follower_drain,
            "survivor did not converge to the committed member set after follower drain"
        );
    }

    let old_leader = active_statuses(&runtimes)[0]
        .leader
        .clone()
        .expect("leader after follower drain");
    let expected_survivor_ids = expected_after_follower_drain
        .iter()
        .filter(|node_id| *node_id != &old_leader)
        .cloned()
        .collect::<BTreeSet<_>>();
    let leader_index = addrs
        .iter()
        .position(|addr| member_node_id_for_addr(*addr) == old_leader)
        .expect("leader belongs to one spawned daemon");
    let mut old_leader_runtime = runtimes[leader_index].take().unwrap();
    let drain = old_leader_runtime.shutdown();
    assert!(!drain.timed_out);
    drop(old_leader_runtime);

    let re_elected = wait_until(Duration::from_secs(10), || {
        let statuses = active_statuses(&runtimes);
        let leaders = leaders(&statuses);
        leaders.len() == 1
            && !leaders.contains(&old_leader)
            && statuses
                .iter()
                .all(|status| status.members == 1 && status.quorum_ok)
    });
    assert!(
        re_elected,
        "remaining daemon members did not re-elect after leader drain: old={old_leader}, statuses={:?}",
        active_statuses(&runtimes)
    );
    for member_ids in active_member_id_sets(&runtimes) {
        assert_eq!(
            member_ids, expected_survivor_ids,
            "survivor did not converge to the committed member set after leader drain"
        );
    }

    for runtime in runtimes.iter_mut().filter_map(Option::as_mut) {
        let _ = runtime.shutdown();
    }
}

fn networked_daemon_e2e_enabled() -> bool {
    std::env::var("HYDRACACHE_RUN_NETWORKED_DAEMON_E2E")
        .map(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

fn reserve_loopback_addrs(count: usize) -> Vec<SocketAddr> {
    let mut addrs = Vec::new();
    while addrs.len() < count {
        let tcp = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = tcp.local_addr().unwrap();
        let udp = UdpSocket::bind(addr).unwrap();
        drop(udp);
        drop(tcp);
        if !addrs.contains(&addr) {
            addrs.push(addr);
        }
    }
    addrs
}

fn wait_until(timeout: Duration, mut predicate: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if predicate() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        thread::sleep(Duration::from_millis(50));
    }
}

fn active_statuses(runtimes: &[Option<ServerRuntime>]) -> Vec<ServerAdminStatus> {
    runtimes
        .iter()
        .filter_map(Option::as_ref)
        .map(ServerRuntime::admin_status)
        .collect()
}

fn leaders(statuses: &[ServerAdminStatus]) -> BTreeSet<String> {
    statuses
        .iter()
        .filter_map(|status| status.leader.clone())
        .collect()
}

fn active_member_id_sets(runtimes: &[Option<ServerRuntime>]) -> Vec<BTreeSet<String>> {
    runtimes
        .iter()
        .filter_map(Option::as_ref)
        .map(|runtime| {
            let overview = serde_json::to_value(runtime.cluster_overview()).unwrap();
            overview["members"]
                .as_array()
                .unwrap()
                .iter()
                .map(|member| member["node_id"].as_str().unwrap().to_owned())
                .collect()
        })
        .collect()
}

fn read_node_identity(storage_dir: &Path) -> serde_json::Value {
    let text = std::fs::read_to_string(storage_dir.join("node-identity.json")).unwrap();
    serde_json::from_str(&text).unwrap()
}

fn member_node_id_for_addr(addr: SocketAddr) -> String {
    let suffix = addr
        .to_string()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    format!("member-{suffix}")
}
