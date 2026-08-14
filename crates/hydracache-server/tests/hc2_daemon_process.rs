use std::io::{BufRead, BufReader, Read};
use std::net::{SocketAddr, TcpListener as StdTcpListener};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bytes::Bytes;
use hydracache_client_hc2::{ClientConfig, GrpcMtlsAdapter, GrpcMtlsConfig, Hc2Client};
use hydracache_client_protocol::{
    ClientFrame, ClientRequest, ClientRequestEnvelope, ClientResponse, ClientResponseEnvelope,
    ClientWireMessage, Namespace, StructuredKey,
};
use hydracache_client_transport_axum::{
    CLIENT_DATA_PATH, HYDRACACHE_ADMIN_HEADER, HYDRACACHE_CLIENT_ID_HEADER,
    HYDRACACHE_TENANT_HEADER,
};
use hydracache_server::{ADMIN_DRAIN_PATH, ADMIN_METRICS_PATH};
use rcgen::{
    BasicConstraints, CertificateParams, CertifiedIssuer, ExtendedKeyUsagePurpose, IsCa, KeyPair,
};

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
    let mut client_params = CertificateParams::new(vec!["hc2-client".to_owned()]).unwrap();
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

fn write_pki(root: &Path, pki: &TestPki) -> (PathBuf, PathBuf, PathBuf) {
    std::fs::create_dir_all(root).unwrap();
    let cert = root.join("server.pem");
    let key = root.join("server.key");
    let ca = root.join("clients.pem");
    std::fs::write(&cert, &pki.server_cert).unwrap();
    std::fs::write(&key, &pki.server_key).unwrap();
    std::fs::write(&ca, &pki.ca).unwrap();
    (cert, key, ca)
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
    for addr in addrs {
        assert!(StdTcpListener::bind(addr).is_ok());
    }
}
