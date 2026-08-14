use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bytes::Bytes;
use hydracache_client_hc2::{ClientConfig, GrpcMtlsAdapter, GrpcMtlsConfig, Hc2Client};

struct ProductionFixture {
    child: Child,
    stdin: Option<std::process::ChildStdin>,
    stdout: BufReader<std::process::ChildStdout>,
    credentials: PathBuf,
    port: u16,
    ca: PathBuf,
    client_certificate: PathBuf,
    client_key: PathBuf,
}

impl ProductionFixture {
    fn start() -> Self {
        let fixture = std::env::var_os("HC2_COMPAT_INTEROP_SERVER")
            .expect("HC2_COMPAT_INTEROP_SERVER must identify the retained-client fixture");
        let daemon = std::env::var_os("HC2_COMPAT_PRODUCTION_DAEMON")
            .expect("HC2_COMPAT_PRODUCTION_DAEMON must identify the current daemon");
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let credentials = std::env::temp_dir().join(format!(
            "hydracache-hc2-retained-rust-production-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir(&credentials).expect("create credentials directory");
        let mut child = Command::new(fixture)
            .arg("--credentials-dir")
            .arg(&credentials)
            .arg("--daemon")
            .arg(daemon)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("start current production daemon fixture");
        let stdin = child.stdin.take().expect("fixture stdin");
        let stdout = child.stdout.take().expect("fixture stdout");
        let mut stdout = BufReader::new(stdout);
        let mut ready = String::new();
        stdout.read_line(&mut ready).expect("read READY receipt");
        let fields: Vec<_> = ready.trim_end().split('\t').collect();
        assert_eq!(fields.len(), 8, "malformed READY receipt: {ready:?}");
        assert_eq!(fields[0], "READY_DAEMON_V1");
        Self {
            child,
            stdin: Some(stdin),
            stdout,
            credentials,
            port: fields[1].parse().expect("READY port"),
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
                fs::read(&self.ca).expect("read CA"),
                fs::read(&self.client_certificate).expect("read client certificate"),
                fs::read(&self.client_key).expect("read client key"),
            )
            .expect("valid retained-client mTLS configuration"),
        )
    }

    fn finish(mut self) {
        let mut stdin = self.stdin.take().expect("fixture stdin remains owned");
        writeln!(stdin, "DRAIN").expect("request current daemon drain");
        drop(stdin);
        let mut receipt = String::new();
        self.stdout
            .read_to_string(&mut receipt)
            .expect("read CLOSED receipt");
        let status = self.child.wait().expect("wait for production fixture");
        let mut stderr = String::new();
        self.child
            .stderr
            .take()
            .expect("fixture stderr")
            .read_to_string(&mut stderr)
            .expect("read fixture stderr");
        assert!(
            status.success(),
            "fixture failed: {status}; stderr={stderr}"
        );
        assert_eq!(receipt.trim(), "CLOSED_DAEMON\tstatus=ok");
        fs::remove_dir_all(&self.credentials).expect("remove credentials directory");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn retained_rust_client_runs_against_current_production_daemon() {
    let fixture = ProductionFixture::start();
    let client = Hc2Client::connect(
        &fixture.adapter(),
        ClientConfig::new("retained-rust-client", "compat-tenant"),
    )
    .await
    .expect("retained client connects to current daemon");
    assert_eq!(client.cluster_id(), "daemon-proof");
    client
        .put(
            Bytes::from_static(b"retained-rust/key"),
            Bytes::from_static(b"retained-rust-value"),
            None,
            None,
        )
        .await
        .expect("retained client put");
    let value = client
        .get(Bytes::from_static(b"retained-rust/key"), None)
        .await
        .expect("retained client get")
        .expect("retained value");
    assert_eq!(value.value, Bytes::from_static(b"retained-rust-value"));
    client.close();
    drop(client);
    tokio::time::sleep(Duration::from_millis(50)).await;
    fixture.finish();
}
