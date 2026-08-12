use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bytes::Bytes;
use hydracache_client_hc2::{
    BatchOperation, ClientConfig, ErrorCode, GrpcMtlsAdapter, GrpcMtlsConfig, Hc2Client,
    InvocationRetryPolicy, ReconnectEndpoint, ReconnectPolicy, RecoveringHc2Client,
    SubscriptionEvent,
};

struct InteropProcess {
    child: Child,
    stdout: BufReader<std::process::ChildStdout>,
    credentials: PathBuf,
    port: u16,
    ca: PathBuf,
    client_certificate: PathBuf,
    client_key: PathBuf,
}

struct ProductionDaemonProcess {
    child: Child,
    stdin: Option<std::process::ChildStdin>,
    stdout: BufReader<std::process::ChildStdout>,
    credentials: PathBuf,
    port: u16,
    ca: PathBuf,
    client_certificate: PathBuf,
    client_key: PathBuf,
}

impl InteropProcess {
    fn start() -> Self {
        let binary = std::env::var_os("HC2_RUST_INTEROP_SERVER")
            .expect("HC2_RUST_INTEROP_SERVER must identify the separately built HC/2 peer");
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let credentials = std::env::temp_dir().join(format!(
            "hydracache-hc2-rust-sdk-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir(&credentials).expect("create credentials directory");
        let mut child = Command::new(binary)
            .arg("--credentials-dir")
            .arg(&credentials)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("start HC/2 peer");
        let stdout = child.stdout.take().expect("peer stdout");
        let mut stdout = BufReader::new(stdout);
        let mut ready = String::new();
        stdout.read_line(&mut ready).expect("read READY receipt");
        let fields: Vec<_> = ready.trim_end().split('\t').collect();
        assert_eq!(fields.len(), 5, "malformed READY receipt: {ready:?}");
        assert_eq!(fields[0], "READY");
        Self {
            child,
            stdout,
            credentials,
            port: fields[1].parse().expect("READY port"),
            ca: PathBuf::from(fields[2]),
            client_certificate: PathBuf::from(fields[3]),
            client_key: PathBuf::from(fields[4]),
        }
    }

    fn adapter(&self) -> GrpcMtlsAdapter {
        GrpcMtlsAdapter::new(
            GrpcMtlsConfig::new(
                format!("https://127.0.0.1:{}", self.port),
                "localhost",
                fs::read(&self.ca).expect("read CA"),
                fs::read(&self.client_certificate).expect("read client certificate"),
                fs::read(&self.client_key).expect("read client key"),
            )
            .expect("valid mTLS configuration"),
        )
    }

    fn finish(mut self) {
        let mut receipt = String::new();
        self.stdout
            .read_to_string(&mut receipt)
            .expect("read CLOSED receipt");
        let status = self.child.wait().expect("wait for HC/2 peer");
        let mut stderr = String::new();
        self.child
            .stderr
            .take()
            .expect("peer stderr")
            .read_to_string(&mut stderr)
            .expect("read peer stderr");
        assert!(status.success(), "peer failed: {status}; stderr={stderr}");
        assert!(
            receipt.contains(
                "CLOSED\tinvocations=6\tevents=5\tactive_subscriptions=0\tactive_sessions=0"
            ),
            "unexpected CLOSED receipt: {receipt:?}"
        );
        fs::remove_dir_all(&self.credentials).expect("remove credentials directory");
    }
}

impl ProductionDaemonProcess {
    fn start() -> Self {
        Self::start_from_env("HC2_RUST_PRODUCTION_DAEMON")
    }

    fn start_from_env(daemon_env: &str) -> Self {
        let fixture = std::env::var_os("HC2_RUST_INTEROP_SERVER")
            .expect("HC2_RUST_INTEROP_SERVER must identify the credential fixture");
        let daemon = std::env::var_os(daemon_env)
            .unwrap_or_else(|| panic!("{daemon_env} must identify hydracache-server"));
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let credentials = std::env::temp_dir().join(format!(
            "hydracache-hc2-rust-production-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir(&credentials).expect("create production credentials directory");
        let mut child = Command::new(fixture)
            .arg("--credentials-dir")
            .arg(&credentials)
            .arg("--daemon")
            .arg(daemon)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("start production daemon fixture");
        let stdin = child.stdin.take().expect("fixture stdin");
        let stdout = child.stdout.take().expect("fixture stdout");
        let mut stdout = BufReader::new(stdout);
        let mut ready = String::new();
        stdout
            .read_line(&mut ready)
            .expect("read production READY receipt");
        let fields: Vec<_> = ready.trim_end().split('\t').collect();
        assert_eq!(
            fields.len(),
            6,
            "malformed production READY receipt: {ready:?}"
        );
        assert_eq!(fields[0], "READY_DAEMON");
        Self {
            child,
            stdin: Some(stdin),
            stdout,
            credentials,
            port: fields[1].parse().expect("production READY port"),
            ca: PathBuf::from(fields[3]),
            client_certificate: PathBuf::from(fields[4]),
            client_key: PathBuf::from(fields[5]),
        }
    }

    fn adapter(&self) -> GrpcMtlsAdapter {
        GrpcMtlsAdapter::new(
            GrpcMtlsConfig::new(
                format!("https://127.0.0.1:{}", self.port),
                "localhost",
                fs::read(&self.ca).expect("read production CA"),
                fs::read(&self.client_certificate).expect("read production client certificate"),
                fs::read(&self.client_key).expect("read production client key"),
            )
            .expect("valid production mTLS configuration"),
        )
    }

    fn finish(mut self) {
        let mut stdin = self.stdin.take().expect("fixture stdin remains owned");
        writeln!(stdin, "DRAIN").expect("request production drain");
        drop(stdin);
        let mut receipt = String::new();
        self.stdout
            .read_to_string(&mut receipt)
            .expect("read production CLOSED receipt");
        let status = self
            .child
            .wait()
            .expect("wait for production daemon fixture");
        let mut stderr = String::new();
        self.child
            .stderr
            .take()
            .expect("fixture stderr")
            .read_to_string(&mut stderr)
            .expect("read production fixture stderr");
        assert!(
            status.success(),
            "production fixture failed: {status}; stderr={stderr}"
        );
        assert_eq!(receipt.trim(), "CLOSED_DAEMON\tstatus=ok");
        fs::remove_dir_all(&self.credentials).expect("remove production credentials directory");
    }
}

fn rolling_policies() -> (ReconnectPolicy, InvocationRetryPolicy) {
    (
        ReconnectPolicy::new(
            3,
            Duration::from_millis(1),
            Duration::from_millis(4),
            Duration::ZERO,
            0x68,
        )
        .expect("valid rolling reconnect policy"),
        InvocationRetryPolicy::new(2).expect("valid rolling retry policy"),
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rust_sdk_is_bounded_cancel_safe_and_process_interoperable() {
    if std::env::var_os("HC2_RUST_INTEROP_SERVER").is_none() {
        eprintln!("SKIP: run `cargo xtask client-plane-rust-sdk-check` for process evidence");
        return;
    }
    let process = InteropProcess::start();
    let mut config = ClientConfig::new("rust-sdk-process-proof", "tenant-a");
    config.default_request_timeout = Duration::from_secs(2);
    config.limits.max_pending_invocations = 1;
    config.limits.max_subscription_events = 1;
    let client = Hc2Client::connect(&process.adapter(), config)
        .await
        .expect("connect through mandatory mTLS");
    assert_eq!(client.cluster_id(), "hc2-java-interop");

    let delayed_client = client.clone();
    let delayed = tokio::spawn(async move {
        delayed_client
            .get(Bytes::from_static(b"__hc2_delay__"), None)
            .await
    });
    tokio::time::sleep(Duration::from_millis(50)).await;
    let quota = client
        .get(Bytes::from_static(b"blocked-by-bound"), None)
        .await
        .expect_err("second concurrent invocation must fail closed");
    assert_eq!(quota.code(), ErrorCode::QuotaExceeded);
    delayed.abort();
    let _ = delayed.await;
    tokio::time::sleep(Duration::from_millis(600)).await;
    let cancellation = client.metrics();
    assert_eq!(cancellation.cancelled, 1);
    assert_eq!(cancellation.late_responses, 1);
    assert_eq!(cancellation.pending_invocations, 0);

    let mut subscription = client
        .subscribe(Bytes::from_static(b"event/"), 0)
        .await
        .expect("subscribe");
    let session = client
        .open_session(Duration::from_secs(3))
        .await
        .expect("open fenced session");
    assert_eq!(session.fence().expect("active fence"), 1);

    for index in 0..3 {
        client
            .put(
                Bytes::from(format!("event/{index}")),
                Bytes::from(format!("value-{index}")),
                None,
                None,
            )
            .await
            .expect("put and emit event");
    }
    tokio::time::timeout(Duration::from_secs(2), async {
        while client.metrics().dropped_events < 2 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("three multiplexed event frames must reach the bounded client queue");
    assert!(matches!(
        subscription.next().await,
        Some(SubscriptionEvent::Event(_))
    ));
    client
        .put(
            Bytes::from_static(b"event/repair"),
            Bytes::from_static(b"value-repair"),
            None,
            None,
        )
        .await
        .expect("trigger repair boundary");
    assert!(matches!(
        subscription.next().await,
        Some(SubscriptionEvent::Gap {
            after_watermark: 3,
            ..
        })
    ));

    let batch = client
        .batch(
            vec![
                BatchOperation::Put {
                    key: Bytes::from_static(b"batch/key"),
                    value: Bytes::from_static(b"batch-value"),
                    ttl: None,
                },
                BatchOperation::Get {
                    key: Bytes::from_static(b"batch/key"),
                },
            ],
            None,
        )
        .await
        .expect("bounded batch");
    assert_eq!(batch.len(), 2);
    assert_eq!(client.topology().nodes.len(), 1);
    assert!(client.metrics().dropped_events >= 3);

    subscription.close();
    session.close();
    tokio::time::sleep(Duration::from_millis(150)).await;
    client.close();
    drop(client);
    process.finish();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rust_sdk_uses_the_real_production_daemon_and_drains_cleanly() {
    if std::env::var_os("HC2_RUST_INTEROP_SERVER").is_none()
        || std::env::var_os("HC2_RUST_PRODUCTION_DAEMON").is_none()
    {
        eprintln!("SKIP: run `cargo xtask client-plane-rust-sdk-check` for daemon evidence");
        return;
    }
    let process = ProductionDaemonProcess::start();
    let mut config = ClientConfig::new("rust-production-proof", "tenant-a");
    if let Some(generation) = std::env::var_os("HC2_RUST_PROTOCOL_GENERATION") {
        config.protocol_generation = generation
            .to_string_lossy()
            .parse()
            .expect("HC2_RUST_PROTOCOL_GENERATION must be an integer");
    }
    let client = Hc2Client::connect(&process.adapter(), config)
        .await
        .expect("connect Rust SDK to production daemon");
    assert_eq!(client.cluster_id(), "daemon-proof");
    if client.protocol_generation() == 5 {
        assert_eq!(client.preferred_protocol_generation(), 5);
        assert!(!client.negotiated_generation_deprecated());
    }

    client
        .put(
            Bytes::from_static(b"production/key"),
            Bytes::from_static(b"production-value"),
            None,
            None,
        )
        .await
        .expect("production put");
    let value = client
        .get(Bytes::from_static(b"production/key"), None)
        .await
        .expect("production get")
        .expect("production value");
    assert_eq!(value.value, Bytes::from_static(b"production-value"));

    let mut subscription = client
        .subscribe(Bytes::from_static(b"production/event/"), 0)
        .await
        .expect("production subscription");
    client
        .put(
            Bytes::from_static(b"production/event/1"),
            Bytes::from_static(b"event-value"),
            None,
            None,
        )
        .await
        .expect("production event mutation");
    assert!(matches!(
        subscription.next().await,
        Some(SubscriptionEvent::Event(_))
    ));
    let session = client
        .open_session(Duration::from_secs(3))
        .await
        .expect("production fenced session");
    assert_eq!(session.fence().expect("production fence"), 1);

    subscription.close();
    session.close();
    tokio::time::sleep(Duration::from_millis(50)).await;
    client.close();
    drop(client);
    process.finish();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn generation_five_client_rolls_from_retained_to_current_production_daemon() {
    if std::env::var_os("HC2_RUST_INTEROP_SERVER").is_none()
        || std::env::var_os("HC2_RUST_RETAINED_PRODUCTION_DAEMON").is_none()
        || std::env::var_os("HC2_RUST_PRODUCTION_DAEMON").is_none()
    {
        eprintln!("SKIP: run the retained HC/2 compatibility harness for rolling evidence");
        return;
    }

    let retained = ProductionDaemonProcess::start_from_env("HC2_RUST_RETAINED_PRODUCTION_DAEMON");
    let current = ProductionDaemonProcess::start();
    let endpoints = vec![
        ReconnectEndpoint::new("retained-generation-5", Arc::new(retained.adapter()))
            .expect("retained endpoint"),
        ReconnectEndpoint::new("current-generation-6", Arc::new(current.adapter()))
            .expect("current endpoint"),
    ];
    let mut config = ClientConfig::new("rust-rolling-generation-proof", "tenant-a");
    config.protocol_generation = 5;
    let (reconnect_policy, retry_policy) = rolling_policies();
    let client = RecoveringHc2Client::connect(endpoints, config, reconnect_policy, retry_policy)
        .await
        .expect("connect to retained generation-5 daemon");
    assert_eq!(client.current_endpoint(), "retained-generation-5");
    assert_eq!(client.connection_generation(), 1);
    assert_eq!(client.protocol_generation(), 5);
    assert_eq!(client.preferred_protocol_generation(), 5);
    assert!(!client.negotiated_generation_deprecated());

    client
        .put(
            Bytes::from_static(b"rolling/retained"),
            Bytes::from_static(b"retained-value"),
            None,
            None,
        )
        .await
        .expect("invoke retained daemon before replacement");

    client
        .prefer_endpoint("current-generation-6")
        .expect("select current daemon");
    client
        .reconnect_now()
        .await
        .expect("replace retained daemon connection with current daemon");
    assert_eq!(client.current_endpoint(), "current-generation-6");
    assert_eq!(client.connection_generation(), 2);
    assert_eq!(client.protocol_generation(), 5);
    assert_eq!(client.preferred_protocol_generation(), 6);
    assert!(client.negotiated_generation_deprecated());
    client
        .put(
            Bytes::from_static(b"rolling/current"),
            Bytes::from_static(b"current-value"),
            None,
            None,
        )
        .await
        .expect("invoke current daemon after replacement");
    let current_value = client
        .get(Bytes::from_static(b"rolling/current"), None)
        .await
        .expect("read current daemon after replacement")
        .expect("current daemon value");
    assert_eq!(current_value.value, Bytes::from_static(b"current-value"));

    client.close();
    drop(client);
    retained.finish();
    current.finish();
}
