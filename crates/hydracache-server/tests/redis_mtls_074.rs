//! Opt-in production RESP mTLS proofs; no performance or qualification claims.
use hydracache_client_transport_axum::ClientSurfaceState;
use hydracache_redis_compat::{RedisAuthConfig, RedisListenerConfig, RedisRespServer};
use hydracache_server::{
    serve_redis_listener, RedisApiConfig, RedisTlsAcceptor, ServerConfig, ServerConfigError,
    ServerRole, ServerRuntime, TlsConfig,
};
use rcgen::{
    BasicConstraints, CertificateParams, CertifiedIssuer, ExtendedKeyUsagePurpose, IsCa, KeyPair,
};
use rustls::{
    pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer, ServerName},
    RootCertStore,
};
use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::watch,
    task::JoinHandle,
};
use tokio_rustls::TlsConnector;

#[test]
fn redis_mtls_configuration_is_opt_in_and_never_dormant_or_authless() {
    let original = ServerConfig::default();
    assert!(original.redis_api.mtls_client_ca_path.is_none());
    let old_toml = toml::to_string(&original).unwrap();
    assert!(!old_toml.contains("mtls_client_ca_path"));
    assert!(ServerConfig::from_toml_str(&old_toml)
        .unwrap()
        .redis_api
        .mtls_client_ca_path
        .is_none());
    for enabled in [false, true] {
        for rediss in [false, true] {
            for auth in [false, true] {
                let config = ServerConfig {
                    redis_api: RedisApiConfig {
                        enabled,
                        rediss_enabled: rediss,
                        auth_required: auth,
                        mtls_client_ca_path: Some(PathBuf::from("client-ca.pem")),
                        ..Default::default()
                    },
                    ..Default::default()
                };
                assert!(matches!(
                    config.validate(),
                    Err(ServerConfigError::RedisMtlsRequiresTlsAndAuth)
                ));
            }
        }
    }
}

#[derive(Clone, Copy)]
enum ClientKind {
    Valid,
    Foreign,
    Expired,
    Future,
    WrongEku,
}

#[test]
fn generated_mtls_configuration_requires_all_security_gates() {
    use proptest::{prelude::*, test_runner::TestRunner};
    let pki = Pki::new(ClientKind::Valid);
    TestRunner::deterministic()
        .run(
            &(
                any::<bool>(),
                any::<bool>(),
                any::<bool>(),
                any::<bool>(),
                any::<bool>(),
            ),
            |(enabled, rediss, tls, auth, nonempty)| {
                let mut config = pki.config.clone();
                config.redis_api.enabled = enabled;
                config.redis_api.rediss_enabled = rediss;
                config.tls.enabled = tls;
                config.redis_api.auth_required = auth;
                if !nonempty {
                    config.redis_api.mtls_client_ca_path = Some(PathBuf::new());
                }
                prop_assert_eq!(
                    config.validate().is_ok(),
                    enabled && rediss && tls && auth && nonempty
                );
                Ok(())
            },
        )
        .unwrap();
}

struct Pki {
    config: ServerConfig,
    ca: String,
    client_cert: String,
    client_key: String,
    directory: PathBuf,
}
impl Pki {
    fn new(kind: ClientKind) -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let directory = PathBuf::from("target/test-hydracache-server/mtls-074").join(format!(
            "{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir_all(&directory).unwrap();
        let mut params = CertificateParams::new(Vec::<String>::new()).unwrap();
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        let ca =
            CertifiedIssuer::self_signed(params.clone(), KeyPair::generate().unwrap()).unwrap();
        let foreign = CertifiedIssuer::self_signed(params, KeyPair::generate().unwrap()).unwrap();
        let server_key = KeyPair::generate().unwrap();
        let mut server_params = CertificateParams::new(vec!["localhost".to_owned()]).unwrap();
        server_params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
        let server_cert = server_params.signed_by(&server_key, &ca).unwrap();
        let client_key = KeyPair::generate().unwrap();
        let mut client_params = CertificateParams::new(vec!["client-074".to_owned()]).unwrap();
        client_params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ClientAuth];
        match kind {
            ClientKind::Expired => {
                client_params.not_before = rcgen::date_time_ymd(2019, 1, 1);
                client_params.not_after = rcgen::date_time_ymd(2020, 1, 1);
            }
            ClientKind::Future => {
                client_params.not_before = rcgen::date_time_ymd(2035, 1, 1);
                client_params.not_after = rcgen::date_time_ymd(2036, 1, 1);
            }
            ClientKind::WrongEku => {
                client_params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth]
            }
            _ => {}
        }
        let issuer = if matches!(kind, ClientKind::Foreign) {
            &foreign
        } else {
            &ca
        };
        let client_cert = client_params.signed_by(&client_key, issuer).unwrap().pem();
        fs::write(directory.join("server.pem"), server_cert.pem()).unwrap();
        fs::write(directory.join("server.key"), server_key.serialize_pem()).unwrap();
        fs::write(directory.join("clients.pem"), ca.pem()).unwrap();
        fs::write(directory.join("auth-token"), "mtls-test-token\n").unwrap();
        let config = ServerConfig {
            role: ServerRole::Local,
            tls: TlsConfig {
                enabled: true,
                cert_path: Some(directory.join("server.pem")),
                key_path: Some(directory.join("server.key")),
                ca_path: Some(directory.join("clients.pem")),
                acknowledge_insecure: false,
            },
            redis_api: RedisApiConfig {
                enabled: true,
                rediss_enabled: true,
                auth_required: true,
                auth_token_file: Some(directory.join("auth-token")),
                mtls_client_ca_path: Some(directory.join("clients.pem")),
                ..Default::default()
            },
            ..Default::default()
        };
        Self {
            config,
            ca: ca.pem(),
            client_cert,
            client_key: client_key.serialize_pem(),
            directory,
        }
    }
    fn connector(&self, client_cert: bool, server_ca: &str) -> TlsConnector {
        let mut roots = RootCertStore::empty();
        for cert in CertificateDer::pem_slice_iter(server_ca.as_bytes()) {
            roots.add(cert.unwrap()).unwrap();
        }
        let config = rustls::ClientConfig::builder().with_root_certificates(roots);
        let config = if client_cert {
            config
                .with_client_auth_cert(
                    CertificateDer::pem_slice_iter(self.client_cert.as_bytes())
                        .collect::<Result<Vec<_>, _>>()
                        .unwrap(),
                    PrivateKeyDer::from_pem_slice(self.client_key.as_bytes()).unwrap(),
                )
                .unwrap()
        } else {
            config.with_no_client_auth()
        };
        TlsConnector::from(Arc::new(config))
    }
}

struct Running {
    runtime: Arc<Mutex<ServerRuntime>>,
    state: Arc<ClientSurfaceState>,
    address: std::net::SocketAddr,
    shutdown: watch::Sender<bool>,
    task: JoinHandle<Result<(), hydracache_server::RedisTcpError>>,
}
impl Running {
    async fn start(pki: &Pki, shared: Option<Arc<ClientSurfaceState>>, tenant: &str) -> Self {
        let runtime = Arc::new(Mutex::new(
            ServerRuntime::new(pki.config.clone()).unwrap().start(),
        ));
        let tls = runtime
            .lock()
            .unwrap()
            .redis_tls_acceptor()
            .unwrap()
            .unwrap();
        assert!(tls.requires_client_certificate());
        let state =
            shared.unwrap_or_else(|| runtime.lock().unwrap().client_dispatch_state().unwrap());
        let server = Arc::new(
            RedisRespServer::new(
                Arc::clone(&state),
                RedisListenerConfig {
                    tenant: tenant.to_owned(),
                    namespace: "default".to_owned(),
                    auth: RedisAuthConfig::required("mtls-test-token"),
                    ..Default::default()
                },
            )
            .unwrap(),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (shutdown, receiver) = watch::channel(false);
        let owner = Arc::clone(&runtime);
        let task = tokio::spawn(serve_redis_listener(
            listener,
            server,
            owner,
            Some(tls),
            receiver,
        ));
        Self {
            runtime,
            state,
            address,
            shutdown,
            task,
        }
    }
    async fn stop(self) {
        self.shutdown.send(true).unwrap();
        tokio::time::timeout(Duration::from_secs(5), self.task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(self.runtime.lock().unwrap().redis_active_connections(), 0);
    }
}
fn wire(parts: &[&[u8]]) -> Vec<u8> {
    let mut result = format!("*{}\r\n", parts.len()).into_bytes();
    for part in parts {
        result.extend_from_slice(format!("${}\r\n", part.len()).as_bytes());
        result.extend_from_slice(part);
        result.extend_from_slice(b"\r\n");
    }
    result
}
fn auth() -> Vec<u8> {
    wire(&[b"AUTH", b"mtls-test-token"])
}
fn quit() -> Vec<u8> {
    wire(&[b"QUIT"])
}
async fn exchange(
    running: &Running,
    pki: &Pki,
    client_cert: bool,
    server_ca: &str,
    name: &'static str,
    bytes: &[u8],
) -> std::io::Result<Vec<u8>> {
    tokio::time::timeout(Duration::from_secs(5), async {
        let connector = pki.connector(client_cert, server_ca);
        let tcp = TcpStream::connect(running.address).await?;
        let mut tls = connector
            .connect(ServerName::try_from(name).unwrap(), tcp)
            .await?;
        tls.write_all(bytes).await?;
        let mut output = Vec::new();
        match tls.read_to_end(&mut output).await {
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => {}
            Err(error) => {
                assert!(
                    output.is_empty(),
                    "TLS failure after application response bytes"
                );
                return Err(error);
            }
        }
        Ok(output)
    })
    .await
    .map_err(|_| std::io::Error::new(std::io::ErrorKind::TimedOut, "fixture exchange deadline"))?
}

#[tokio::test]
async fn redis_mtls_valid_resp2_resp3_and_auth_keep_binary_bytes() {
    let pki = Pki::new(ClientKind::Valid);
    let running = Running::start(&pki, None, "tenant-a").await;
    for dialect in [b"2".as_slice(), b"3"] {
        let mut requests = wire(&[b"HELLO", dialect, b"AUTH", b"default", b"mtls-test-token"]);
        requests.extend(wire(&[b"SET", b"key", b"\0\xffb"]));
        requests.extend(wire(&[b"GET", b"key"]));
        requests.extend(wire(&[b"GET", b"missing"]));
        requests.extend(quit());
        let output = exchange(&running, &pki, true, &pki.ca, "localhost", &requests)
            .await
            .unwrap();
        assert!(output.starts_with(if dialect == b"3" {
            b"%7\r\n"
        } else {
            b"*14\r\n"
        }));
        assert!(output.ends_with(if dialect == b"3" {
            b"+OK\r\n$3\r\n\0\xffb\r\n_\r\n+OK\r\n"
        } else {
            b"+OK\r\n$3\r\n\0\xffb\r\n$-1\r\n+OK\r\n"
        }));
    }
    assert_eq!(running.state.dispatch_attempts(), 6);
    running.stop().await;
}

#[tokio::test]
async fn redis_mtls_invalid_clients_never_reach_resp_or_dispatch() {
    for kind in [
        ClientKind::Valid,
        ClientKind::Foreign,
        ClientKind::Expired,
        ClientKind::Future,
        ClientKind::WrongEku,
    ] {
        let pki = Pki::new(kind);
        let running = Running::start(&pki, None, "tenant-a").await;
        let mut requests = auth();
        requests.extend(wire(&[b"SET", b"denied", b"value"]));
        requests.extend(quit());
        let result = exchange(
            &running,
            &pki,
            !matches!(kind, ClientKind::Valid),
            &pki.ca,
            "localhost",
            &requests,
        )
        .await;
        match result {
            Ok(output) => assert!(output.is_empty()),
            Err(error) => assert_ne!(error.kind(), std::io::ErrorKind::TimedOut),
        }
        assert_eq!(running.state.dispatch_attempts(), 0);
        running.stop().await;
    }
}

#[tokio::test]
async fn redis_mtls_client_checks_server_ca_and_hostname() {
    let pki = Pki::new(ClientKind::Valid);
    let foreign = Pki::new(ClientKind::Valid);
    let running = Running::start(&pki, None, "tenant-a").await;
    for (ca, name) in [(&foreign.ca, "localhost"), (&pki.ca, "wrong.invalid")] {
        assert!(exchange(&running, &pki, true, ca, name, &quit())
            .await
            .is_err());
        assert_eq!(running.state.dispatch_attempts(), 0);
    }
    running.stop().await;
}

#[tokio::test]
async fn redis_mtls_certificate_does_not_bypass_auth_or_select_tenant() {
    let pki = Pki::new(ClientKind::Valid);
    let a = Running::start(&pki, None, "tenant-a").await;
    let b = Running::start(&pki, Some(Arc::clone(&a.state)), "tenant-b").await;
    let mut requests = wire(&[b"SET", b"key", b"denied"]);
    requests.extend(wire(&[b"AUTH", b"other-tenant", b"wrong-token"]));
    requests.extend(wire(&[b"AUTH", b"other-tenant", b"mtls-test-token"]));
    requests.extend(wire(&[b"SET", b"key", b"denied"]));
    requests.extend(quit());
    let output = exchange(&a, &pki, true, &pki.ca, "localhost", &requests)
        .await
        .unwrap();
    assert!(output.starts_with(b"-NOAUTH"));
    assert_eq!(
        output
            .windows(b"NOAUTH".len())
            .filter(|part| *part == b"NOAUTH")
            .count(),
        2
    );
    assert!(output
        .windows(b"WRONGPASS".len())
        .any(|part| part == b"WRONGPASS"));
    assert!(!output
        .windows(b"wrong-token".len())
        .any(|part| part == b"wrong-token"));
    assert_eq!(a.state.dispatch_attempts(), 0);
    let mut put = auth();
    put.extend(wire(&[b"SET", b"key", b"a"]));
    put.extend(quit());
    assert_eq!(
        exchange(&a, &pki, true, &pki.ca, "localhost", &put)
            .await
            .unwrap(),
        b"+OK\r\n+OK\r\n+OK\r\n"
    );
    let mut get = auth();
    get.extend(wire(&[b"GET", b"key"]));
    get.extend(quit());
    assert_eq!(
        exchange(&b, &pki, true, &pki.ca, "localhost", &get)
            .await
            .unwrap(),
        b"+OK\r\n$-1\r\n+OK\r\n"
    );
    assert_eq!(
        exchange(&a, &pki, true, &pki.ca, "localhost", &get)
            .await
            .unwrap(),
        b"+OK\r\n$1\r\na\r\n+OK\r\n"
    );
    let mut unauthenticated = wire(&[b"GET", b"key"]);
    unauthenticated.extend(quit());
    assert_eq!(
        exchange(&a, &pki, true, &pki.ca, "localhost", &unauthenticated)
            .await
            .unwrap(),
        b"-NOAUTH Authentication required.\r\n+OK\r\n"
    );
    assert_eq!(a.state.dispatch_attempts(), 3);
    b.stop().await;
    a.stop().await;
}

#[test]
fn redis_mtls_client_ca_startup_is_bounded_and_has_no_global_ca_fallback() {
    let pki = Pki::new(ClientKind::Valid);
    let ca = pki.directory.join("clients.pem");
    let runtime = |path: PathBuf| {
        let mut config = pki.config.clone();
        config.redis_api.mtls_client_ca_path = Some(path);
        config.tls.ca_path = Some(ca.clone());
        ServerRuntime::new(config).unwrap()
    };
    for bytes in [
        Vec::new(),
        b"not a certificate".to_vec(),
        b"-----BEGIN CERTIFICATE-----\n!!\n-----END CERTIFICATE-----".to_vec(),
        b"-----BEGIN CERTIFICATE-----\nYWJj\n-----END CERTIFICATE-----".to_vec(),
        pki.ca.repeat(17).into_bytes(),
        vec![b'a'; 262145],
    ] {
        let path = pki.directory.join("bad-ca.pem");
        fs::write(&path, bytes).unwrap();
        assert!(runtime(path).redis_tls_acceptor().is_err());
    }
    assert!(runtime(pki.directory.join("missing.pem"))
        .redis_tls_acceptor()
        .is_err());
    let path = pki.directory.join("exact-ca.pem");
    let mut bytes = pki.ca.repeat(16).into_bytes();
    bytes.resize(262144, b' ');
    fs::write(&path, &bytes).unwrap();
    assert!(runtime(path.clone())
        .redis_tls_acceptor()
        .unwrap()
        .unwrap()
        .requires_client_certificate());
    bytes.push(b' ');
    fs::write(&path, bytes).unwrap();
    assert!(runtime(path).redis_tls_acceptor().is_err());
    let mut empty = pki.config.clone();
    empty.redis_api.mtls_client_ca_path = Some(PathBuf::new());
    assert!(matches!(
        empty.validate(),
        Err(ServerConfigError::RedisMtlsRequiresTlsAndAuth)
    ));
    let valid = toml::to_string(&pki.config).unwrap();
    assert_eq!(ServerConfig::from_toml_str(&valid).unwrap(), pki.config);
    let legacy = RedisTlsAcceptor::from_tls_config(&pki.config.tls).unwrap();
    assert!(!legacy.requires_client_certificate());
    // The dedicated inbound bundle works even when the unrelated global CA
    // cannot be read; its presence still satisfies the existing config rule.
    let mut separate = pki.config.clone();
    separate.tls.ca_path = Some(pki.directory.join("unreadable-global-ca.pem"));
    assert!(ServerRuntime::new(separate)
        .unwrap()
        .redis_tls_acceptor()
        .unwrap()
        .unwrap()
        .requires_client_certificate());
    let mut missing = pki.config.tls.clone();
    missing.cert_path = None;
    assert!(matches!(
        RedisTlsAcceptor::from_tls_config_with_client_ca(&missing, &ca),
        Err(hydracache_server::redis_tcp::RedisTlsError::MissingCertPath)
    ));
    missing.cert_path = pki.config.tls.cert_path.clone();
    missing.key_path = None;
    assert!(matches!(
        RedisTlsAcceptor::from_tls_config_with_client_ca(&missing, &ca),
        Err(hydracache_server::redis_tcp::RedisTlsError::MissingKeyPath)
    ));
}

#[tokio::test]
async fn redis_mtls_connection_cap_and_shutdown_release_incomplete_handshakes() {
    let pki = Pki::new(ClientKind::Valid);
    let running = Running::start(&pki, None, "tenant-a").await;
    let mut sockets = Vec::new();
    for expected in 1..=128 {
        sockets.push(TcpStream::connect(running.address).await.unwrap());
        tokio::time::timeout(Duration::from_secs(2), async {
            while running.runtime.lock().unwrap().redis_active_connections() != expected {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    }
    let mut extra = TcpStream::connect(running.address).await.unwrap();
    let mut byte = [0];
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(2), extra.read(&mut byte))
            .await
            .unwrap()
            .unwrap(),
        0
    );
    assert_eq!(
        running.runtime.lock().unwrap().redis_active_connections(),
        128
    );
    assert_eq!(running.state.dispatch_attempts(), 0);
    drop(sockets.pop().unwrap());
    tokio::time::timeout(Duration::from_secs(2), async {
        while running.runtime.lock().unwrap().redis_active_connections() != 127 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    sockets.push(TcpStream::connect(running.address).await.unwrap());
    tokio::time::timeout(Duration::from_secs(2), async {
        while running.runtime.lock().unwrap().redis_active_connections() != 128 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    running.stop().await;
    for mut socket in sockets {
        assert_eq!(socket.read(&mut byte).await.unwrap(), 0);
    }
}

#[tokio::test(start_paused = true)]
async fn redis_mtls_stalled_handshake_times_out_before_dispatch() {
    let pki = Pki::new(ClientKind::Valid);
    let running = Running::start(&pki, None, "tenant-a").await;
    let mut tcp = TcpStream::connect(running.address).await.unwrap();
    while running.runtime.lock().unwrap().redis_active_connections() == 0 {
        tokio::task::yield_now().await;
    }
    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_secs(5)).await;
    let mut byte = [0];
    assert_eq!(tcp.read(&mut byte).await.unwrap(), 0);
    assert_eq!(
        running.runtime.lock().unwrap().redis_active_connections(),
        0
    );
    assert_eq!(running.state.dispatch_attempts(), 0);
    running.stop().await;
}

#[tokio::test]
async fn redis_mtls_shutdown_joins_authenticated_and_fragmented_resp_owners() {
    let pki = Pki::new(ClientKind::Valid);
    let running = Running::start(&pki, None, "tenant-a").await;
    let mut streams = Vec::new();
    for _ in 0..2 {
        let tcp = TcpStream::connect(running.address).await.unwrap();
        let mut tls = pki
            .connector(true, &pki.ca)
            .connect(ServerName::try_from("localhost").unwrap(), tcp)
            .await
            .unwrap();
        tls.write_all(&auth()).await.unwrap();
        let mut ack = [0; 5];
        tls.read_exact(&mut ack).await.unwrap();
        assert_eq!(&ack, b"+OK\r\n");
        streams.push(tls);
    }
    let set = wire(&[b"SET", b"partial", b"value"]);
    streams[1].write_all(&set[..set.len() - 1]).await.unwrap();
    assert_eq!(
        running.runtime.lock().unwrap().redis_active_connections(),
        2
    );
    let state = Arc::clone(&running.state);
    running.stop().await;
    for mut stream in streams {
        let mut output = Vec::new();
        let result = stream.read_to_end(&mut output).await;
        assert!(
            result.is_ok()
                || matches!(
                    result.as_ref().unwrap_err().kind(),
                    std::io::ErrorKind::UnexpectedEof | std::io::ErrorKind::ConnectionReset
                ),
            "owned connection was not closed as expected: {result:?}"
        );
        assert!(output.is_empty());
    }
    assert_eq!(state.dispatch_attempts(), 0);
}

#[tokio::test]
async fn redis_mtls_disconnect_and_completed_task_reuse_keep_binary_payload() {
    let pki = Pki::new(ClientKind::Valid);
    let running = Running::start(&pki, None, "tenant-a").await;
    let disconnected = TcpStream::connect(running.address).await.unwrap();
    drop(disconnected);
    let payload = (0..4096).map(|index| index as u8).collect::<Vec<_>>();
    let mut requests = auth();
    requests.extend(wire(&[b"SET", b"large", &payload]));
    requests.extend(wire(&[b"GET", b"large"]));
    requests.extend(quit());
    let output = exchange(&running, &pki, true, &pki.ca, "localhost", &requests)
        .await
        .unwrap();
    let mut expected = b"+OK\r\n+OK\r\n$4096\r\n".to_vec();
    expected.extend(&payload);
    expected.extend(b"\r\n+OK\r\n");
    assert_eq!(output, expected);
    assert_eq!(running.state.dispatch_attempts(), 2);
    running.stop().await;
}
