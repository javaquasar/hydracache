use std::error::Error;
use std::fs;
use std::net::{SocketAddr, TcpListener as StdTcpListener};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bytes::Bytes;
use hydracache::{
    ClusterEpoch, DurableValueStore, PartitionId, ReplicatedSlot, ReplicatedValueRecord,
    ReplicatedValueStore, TombstoneBudget, TombstoneTracker,
};
use hydracache_client_hc2::{ClientConfig, GrpcMtlsAdapter, GrpcMtlsConfig, Hc2Client};
use hydracache_client_protocol::{
    ClientFrame, ClientRequest, ClientRequestEnvelope, ClientResponse, ClientResponseEnvelope,
    ClientWireMessage, Namespace, StructuredKey,
};
use hydracache_client_transport_axum::{
    CLIENT_DATA_PATH, HYDRACACHE_ADMIN_HEADER, HYDRACACHE_CLIENT_ID_HEADER,
    HYDRACACHE_TENANT_HEADER,
};
use rcgen::{
    BasicConstraints, CertificateParams, CertifiedIssuer, ExtendedKeyUsagePurpose, IsCa, KeyPair,
};
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

const PROFILE_ID: &str = "published-072-compatibility-073-v1";
const CANARY_MARKER: &str = "HC-CANARY-RED:W10-COMPAT";
const BUDGET: u64 = 2 * 1024 * 1024;

type DynError = Box<dyn Error>;

#[derive(Debug)]
struct Options {
    mode: String,
    role: String,
    source_sha: String,
    server_binary: Option<PathBuf>,
    store: Option<PathBuf>,
    output: PathBuf,
}

impl Options {
    fn parse() -> Result<Self, DynError> {
        let mut mode = None;
        let mut role = None;
        let mut source_sha = None;
        let mut server_binary = None;
        let mut store = None;
        let mut output = None;
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            let value = args
                .next()
                .ok_or_else(|| format!("missing value for {arg}"))?;
            match arg.as_str() {
                "--mode" => mode = Some(value),
                "--role" => role = Some(value),
                "--source-sha" => source_sha = Some(value),
                "--server-binary" => server_binary = Some(value.into()),
                "--store" => store = Some(value.into()),
                "--output" => output = Some(value.into()),
                _ => return Err(format!("unknown argument {arg}").into()),
            }
        }
        let options = Self {
            mode: mode.ok_or("--mode is required")?,
            role: role.ok_or("--role is required")?,
            source_sha: source_sha.ok_or("--source-sha is required")?,
            server_binary,
            store,
            output: output.ok_or("--output is required")?,
        };
        match options.mode.as_str() {
            "wire" if options.server_binary.is_none() => {
                return Err("wire mode requires --server-binary".into())
            }
            "durable-write" | "durable-transition" | "durable-rollback"
                if options.store.is_none() =>
            {
                return Err("durable mode requires --store".into())
            }
            "wire" | "durable-write" | "durable-transition" | "durable-rollback" => {}
            _ => return Err(format!("unknown mode {}", options.mode).into()),
        }
        Ok(options)
    }
}

#[derive(Serialize)]
struct Receipt {
    schema_version: u32,
    release: &'static str,
    profile_id: &'static str,
    mode: String,
    role: String,
    source_sha: String,
    result: &'static str,
    cases: Vec<String>,
    mutation_count: u64,
    canary_marker: &'static str,
}

#[tokio::main]
async fn main() -> Result<(), DynError> {
    let options = Options::parse()?;
    let (cases, mutation_count) = match options.mode.as_str() {
        "wire" => {
            wire(
                options
                    .server_binary
                    .as_deref()
                    .expect("validated server binary"),
                &options.role,
            )
            .await?
        }
        "durable-write" => durable_write(options.store.as_deref().expect("validated store"))?,
        "durable-transition" => {
            durable_transition(options.store.as_deref().expect("validated store"))?
        }
        "durable-rollback" => durable_rollback(options.store.as_deref().expect("validated store"))?,
        _ => unreachable!(),
    };
    let receipt = Receipt {
        schema_version: 1,
        release: "0.73",
        profile_id: PROFILE_ID,
        mode: options.mode,
        role: options.role,
        source_sha: options.source_sha,
        result: "passed",
        cases,
        mutation_count,
        canary_marker: CANARY_MARKER,
    };
    if let Some(parent) = options.output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(options.output, serde_json::to_vec_pretty(&receipt)?)?;
    Ok(())
}

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
    let mut client_params = CertificateParams::new(vec!["compatibility-073".to_owned()]).unwrap();
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
        .expect("reservation count");
    (addrs, listeners)
}

fn temp_root(label: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "hydracache-compatibility-073-{label}-{}-{unique}",
        std::process::id()
    ))
}

struct Daemon {
    child: Child,
    root: PathBuf,
    client_addr: SocketAddr,
    admin_addr: SocketAddr,
    hc2_addr: SocketAddr,
    redis_addr: SocketAddr,
    material: TestPki,
}

impl Daemon {
    fn start(server_binary: &Path) -> Result<Self, DynError> {
        let root = temp_root("daemon");
        fs::create_dir_all(&root)?;
        let material = pki();
        let cert = root.join("server.pem");
        let key = root.join("server.key");
        let ca = root.join("clients.pem");
        fs::write(&cert, &material.server_cert)?;
        fs::write(&key, &material.server_key)?;
        fs::write(&ca, &material.ca)?;
        let ([client_addr, admin_addr, hc2_addr, redis_addr], reservations) = reserve_addrs::<4>();
        drop(reservations);
        let mut child = Command::new(server_binary)
            .env("HYDRACACHE_CLIENT_API_ENABLED", "true")
            .env("HYDRACACHE_LISTEN_ADDR", client_addr.to_string())
            .env("HYDRACACHE_ADMIN_API_ENABLED", "true")
            .env("HYDRACACHE_ADMIN_ADDR", admin_addr.to_string())
            .env("HYDRACACHE_HC2_ENABLED", "true")
            .env("HYDRACACHE_HC2_ADDR", hc2_addr.to_string())
            .env("HYDRACACHE_HC2_CLUSTER_ID", "compatibility-073")
            .env("HYDRACACHE_REDIS_API_ENABLED", "true")
            .env("HYDRACACHE_REDIS_ADDR", redis_addr.to_string())
            .env("HYDRACACHE_TLS_ENABLED", "true")
            .env("HYDRACACHE_TLS_CERT_PATH", cert)
            .env("HYDRACACHE_TLS_KEY_PATH", key)
            .env("HYDRACACHE_TLS_CA_PATH", ca)
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;
        let mut ready = String::new();
        std::io::BufRead::read_line(
            &mut std::io::BufReader::new(child.stdout.take().unwrap()),
            &mut ready,
        )?;
        if ready.trim() != r#"{"status":"ok"}"# {
            return Err(format!("daemon readiness failed: {ready:?}").into());
        }
        Ok(Self {
            child,
            root,
            client_addr,
            admin_addr,
            hc2_addr,
            redis_addr,
            material,
        })
    }

    async fn hc2(&self, tenant: &str) -> Result<Hc2Client, DynError> {
        let adapter = GrpcMtlsAdapter::new(GrpcMtlsConfig::new(
            format!("https://{}", self.hc2_addr),
            "localhost",
            self.material.ca.as_bytes(),
            self.material.client_cert.as_bytes(),
            self.material.client_key.as_bytes(),
        )?);
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            match Hc2Client::connect(&adapter, ClientConfig::new("compatibility-073", tenant)).await
            {
                Ok(client) => return Ok(client),
                Err(_) if tokio::time::Instant::now() < deadline => {
                    tokio::time::sleep(Duration::from_millis(25)).await
                }
                Err(error) => return Err(error.into()),
            }
        }
    }

    async fn shutdown(mut self) -> Result<(), DynError> {
        operator(reqwest::Client::new().post(format!("http://{}/admin/drain", self.admin_addr)))
            .send()
            .await?
            .error_for_status()?;
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(status) = self.child.try_wait()? {
                if !status.success() {
                    return Err(format!("daemon exit {status}").into());
                }
                break;
            }
            if std::time::Instant::now() >= deadline {
                return Err("daemon drain timeout".into());
            }
            thread::sleep(Duration::from_millis(20));
        }
        fs::remove_dir_all(&self.root)?;
        Ok(())
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn operator(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder
        .header(HYDRACACHE_CLIENT_ID_HEADER, "compatibility-operator")
        .header(HYDRACACHE_TENANT_HEADER, "system")
        .header(HYDRACACHE_ADMIN_HEADER, "true")
}

async fn wire(server_binary: &Path, role: &str) -> Result<(Vec<String>, u64), DynError> {
    let daemon = Daemon::start(server_binary)?;
    let mut cases = Vec::new();
    let prefix = role.replace(|character: char| !character.is_ascii_alphanumeric(), "-");
    hc1_cases(&daemon, &prefix, &mut cases).await?;
    hc2_cases(&daemon, &prefix, &mut cases).await?;
    resp_cases(&daemon, &prefix, &mut cases).await?;
    management_cases(&daemon, &mut cases).await?;
    owners_zero(&daemon).await?;
    cases.push("drain-reconciles-live-owners".to_owned());
    daemon.shutdown().await?;
    Ok((cases, 8))
}

fn structured_key(bytes: &[u8]) -> StructuredKey {
    StructuredKey::new(vec![bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()])
    .unwrap()
}

async fn hc1(
    daemon: &Daemon,
    tenant: &str,
    id: &str,
    request: ClientRequest,
) -> Result<ClientResponse, DynError> {
    let envelope = ClientRequestEnvelope::new(id, request);
    let body = ClientFrame::from_message(&ClientWireMessage::Request(envelope))?
        .encode()?
        .to_vec();
    let bytes = reqwest::Client::new()
        .post(format!("http://{}{}", daemon.client_addr, CLIENT_DATA_PATH))
        .header(
            HYDRACACHE_CLIENT_ID_HEADER,
            format!("compatibility-{tenant}"),
        )
        .header(HYDRACACHE_TENANT_HEADER, tenant)
        .body(body)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    let frame = ClientFrame::decode(&bytes, 8 * 1024 * 1024)?;
    let ClientWireMessage::Response(ClientResponseEnvelope { result, .. }) =
        frame.decode_message()?
    else {
        return Err("HC1 returned a non-response frame".into());
    };
    result.map_err(|error| format!("{error:?}").into())
}

async fn hc1_cases(daemon: &Daemon, prefix: &str, cases: &mut Vec<String>) -> Result<(), DynError> {
    let ns = Namespace::new("compatibility").unwrap();
    let missing = structured_key(format!("{prefix}-hc1-missing").as_bytes());
    match hc1(
        daemon,
        "tenant-a",
        "hc1-empty",
        ClientRequest::Get {
            ns: ns.clone(),
            key: missing,
        },
    )
    .await?
    {
        ClientResponse::Value { value: None } => {}
        other => return Err(format!("HC1 empty read returned {other:?}").into()),
    }
    cases.push("hc1-empty-read".to_owned());

    let key = structured_key(&[0, 0xff, b'/', 0x7f]);
    let value = vec![0, 0xff, b'\r', b'\n', 0x73];
    let put = ClientRequest::Put {
        ns: ns.clone(),
        key: key.clone(),
        value: value.clone(),
        ttl_ms: None,
        dimensions: Vec::new(),
    };
    if hc1(daemon, "tenant-a", "hc1-put", put).await? != ClientResponse::Stored {
        return Err("HC1 binary put was not stored".into());
    }
    match hc1(
        daemon,
        "tenant-a",
        "hc1-get",
        ClientRequest::Get {
            ns: ns.clone(),
            key: key.clone(),
        },
    )
    .await?
    {
        ClientResponse::Value { value: Some(got) } if got == value => {}
        other => return Err(format!("HC1 binary get returned {other:?}").into()),
    }
    cases.push("hc1-put-read-structured-key-and-binary-value".to_owned());
    match hc1(
        daemon,
        "tenant-b",
        "hc1-tenant",
        ClientRequest::Get {
            ns: ns.clone(),
            key,
        },
    )
    .await?
    {
        ClientResponse::Value { value: None } => {}
        other => return Err(format!("HC1 cross-tenant read returned {other:?}").into()),
    }
    cases.push("hc1-tenant-isolation".to_owned());

    let ttl_key = structured_key(format!("{prefix}-hc1-ttl").as_bytes());
    hc1(
        daemon,
        "tenant-a",
        "hc1-ttl-put",
        ClientRequest::Put {
            ns: ns.clone(),
            key: ttl_key.clone(),
            value: b"ttl".to_vec(),
            ttl_ms: Some(100),
            dimensions: Vec::new(),
        },
    )
    .await?;
    tokio::time::sleep(Duration::from_millis(180)).await;
    match hc1(
        daemon,
        "tenant-a",
        "hc1-ttl-get",
        ClientRequest::Get { ns, key: ttl_key },
    )
    .await?
    {
        ClientResponse::Value { value: None } => {}
        other => return Err(format!("HC1 expired read returned {other:?}").into()),
    }
    cases.push("hc1-ttl-expiry".to_owned());

    let response = reqwest::Client::new()
        .post(format!("http://{}{}", daemon.client_addr, CLIENT_DATA_PATH))
        .header(HYDRACACHE_CLIENT_ID_HEADER, "compatibility-malformed")
        .header(HYDRACACHE_TENANT_HEADER, "tenant-a")
        .body(vec![0x48, 0x43, 0x01])
        .send()
        .await?;
    if response.status().is_success() {
        return Err("HC1 malformed frame was accepted".into());
    }
    cases.push("hc1-malformed-frame-rejected".to_owned());
    Ok(())
}

async fn hc2_cases(daemon: &Daemon, prefix: &str, cases: &mut Vec<String>) -> Result<(), DynError> {
    let a = daemon.hc2("tenant-a").await?;
    let b = daemon.hc2("tenant-b").await?;
    let missing = Bytes::from(format!("{prefix}/hc2/missing"));
    if a.get(missing, None).await?.is_some() {
        return Err("HC2 empty read found a value".into());
    }
    cases.push("hc2-empty-read".to_owned());
    let key = Bytes::from_static(b"hc2/\0\xff/bin");
    let value = Bytes::from_static(b"\0\xff\r\nvalue");
    if !a.put(key.clone(), value.clone(), None, None).await?.applied {
        return Err("HC2 binary put was not applied".into());
    }
    let got = a
        .get(key.clone(), None)
        .await?
        .ok_or("HC2 binary get miss")?;
    if got.value != value {
        return Err("HC2 binary value mismatch".into());
    }
    cases.push("hc2-put-read-binary-key-and-value".to_owned());
    if b.get(key, None).await?.is_some() {
        return Err("HC2 cross-tenant value was visible".into());
    }
    cases.push("hc2-tenant-isolation".to_owned());
    let ttl_key = Bytes::from(format!("{prefix}/hc2/ttl"));
    a.put(
        ttl_key.clone(),
        Bytes::from_static(b"ttl"),
        Some(Duration::from_millis(100)),
        None,
    )
    .await?;
    tokio::time::sleep(Duration::from_millis(180)).await;
    if a.get(ttl_key, None).await?.is_some() {
        return Err("HC2 expired value remained visible".into());
    }
    cases.push("hc2-ttl-expiry".to_owned());
    if a.get(Bytes::new(), None).await.is_ok() {
        return Err("HC2 empty malformed key was accepted".into());
    }
    cases.push("hc2-malformed-input-rejected-before-mutation".to_owned());
    a.close();
    b.close();
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
enum Resp {
    Simple(Vec<u8>),
    Bulk(Option<Vec<u8>>),
    Integer(i64),
    Error(Vec<u8>),
}

async fn resp_command(
    stream: &mut tokio::net::TcpStream,
    args: &[&[u8]],
) -> Result<Resp, DynError> {
    let mut request = format!("*{}\r\n", args.len()).into_bytes();
    for arg in args {
        request.extend_from_slice(format!("${}\r\n", arg.len()).as_bytes());
        request.extend_from_slice(arg);
        request.extend_from_slice(b"\r\n");
    }
    stream.write_all(&request).await?;
    let mut reader = BufReader::new(stream);
    let mut line = Vec::new();
    reader.read_until(b'\n', &mut line).await?;
    if !line.ends_with(b"\r\n") || line.is_empty() {
        return Err("invalid RESP response line".into());
    }
    let prefix = line[0];
    let payload = &line[1..line.len() - 2];
    match prefix {
        b'+' => Ok(Resp::Simple(payload.to_vec())),
        b'-' => Ok(Resp::Error(payload.to_vec())),
        b':' => Ok(Resp::Integer(std::str::from_utf8(payload)?.parse()?)),
        b'$' => {
            let length: i64 = std::str::from_utf8(payload)?.parse()?;
            if length == -1 {
                return Ok(Resp::Bulk(None));
            }
            let mut bytes = vec![0; length as usize + 2];
            reader.read_exact(&mut bytes).await?;
            if !bytes.ends_with(b"\r\n") {
                return Err("invalid RESP bulk terminator".into());
            }
            bytes.truncate(length as usize);
            Ok(Resp::Bulk(Some(bytes)))
        }
        _ => Err(format!("unsupported RESP response prefix {prefix}").into()),
    }
}

async fn resp_cases(
    daemon: &Daemon,
    prefix: &str,
    cases: &mut Vec<String>,
) -> Result<(), DynError> {
    let mut stream = tokio::net::TcpStream::connect(daemon.redis_addr).await?;
    let missing = format!("{prefix}:resp:missing");
    if resp_command(&mut stream, &[b"GET", missing.as_bytes()]).await? != Resp::Bulk(None) {
        return Err("RESP empty read found a value".into());
    }
    cases.push("resp-empty-read".to_owned());
    let key = b"resp:\0:\xff";
    let value = b"\0\xff\r\nvalue";
    if resp_command(&mut stream, &[b"SET", key, value]).await? != Resp::Simple(b"OK".to_vec()) {
        return Err("RESP binary SET failed".into());
    }
    if resp_command(&mut stream, &[b"GET", key]).await? != Resp::Bulk(Some(value.to_vec())) {
        return Err("RESP binary GET mismatch".into());
    }
    cases.push("resp-put-read-binary-key-and-value".to_owned());
    if resp_command(&mut stream, &[b"HC.TAG", key, b"compatibility-tag"]).await? != Resp::Integer(1)
    {
        return Err("RESP tag attachment failed".into());
    }
    if resp_command(&mut stream, &[b"HC.INVALIDATE_TAG", b"compatibility-tag"]).await?
        != Resp::Integer(1)
    {
        return Err("RESP tag invalidation failed".into());
    }
    if resp_command(&mut stream, &[b"GET", key]).await? != Resp::Bulk(None) {
        return Err("RESP tag invalidation retained value".into());
    }
    cases.push("resp-tagged-put-read-invalidate".to_owned());
    let ttl_key = format!("{prefix}:resp:ttl");
    if resp_command(
        &mut stream,
        &[b"SET", ttl_key.as_bytes(), b"ttl", b"PX", b"100"],
    )
    .await?
        != Resp::Simple(b"OK".to_vec())
    {
        return Err("RESP TTL SET failed".into());
    }
    tokio::time::sleep(Duration::from_millis(180)).await;
    if resp_command(&mut stream, &[b"GET", ttl_key.as_bytes()]).await? != Resp::Bulk(None) {
        return Err("RESP expired value remained visible".into());
    }
    cases.push("resp-ttl-expiry".to_owned());
    stream.write_all(b"*2\r\n$3\r\nSET\r\n$x\r\n").await?;
    let mut reader = BufReader::new(&mut stream);
    let mut line = Vec::new();
    reader.read_until(b'\n', &mut line).await?;
    if !line.starts_with(b"-") {
        return Err("RESP malformed frame did not fail loudly".into());
    }
    cases.push("resp-malformed-frame-rejected".to_owned());
    Ok(())
}

async fn management_cases(daemon: &Daemon, cases: &mut Vec<String>) -> Result<(), DynError> {
    let client = reqwest::Client::new();
    for path in [
        "/management/v1/capabilities",
        "/management/v1/dashboard",
        "/management/v1/formation",
        "/management/v1/cluster/members",
        "/management/v1/cluster/partitions",
        "/management/v1/persistence",
        "/management/v1/operations",
        "/management/v1/audit",
    ] {
        let response = operator(client.get(format!("http://{}{}", daemon.admin_addr, path)))
            .send()
            .await?
            .error_for_status()?;
        let body: serde_json::Value = serde_json::from_slice(&response.bytes().await?)?;
        if body.get("schema_version").and_then(|value| value.as_u64()) != Some(1) {
            return Err(format!("{path} did not return management schema v1").into());
        }
    }
    cases.push("management-routes-schema-v1".to_owned());
    let index = client
        .get(format!("http://{}/console/", daemon.admin_addr))
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    if !index.contains("<div id=\"app\"></div>") {
        return Err("console index shell missing".into());
    }
    let app = client
        .get(format!("http://{}/console/app.js", daemon.admin_addr))
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    for extension in [".css", ".js"] {
        let asset = extract_asset(&format!("{index}\n{app}"), extension)
            .ok_or_else(|| format!("console {extension} hashed asset missing"))?;
        client
            .get(format!("http://{}{}", daemon.admin_addr, asset))
            .send()
            .await?
            .error_for_status()?;
    }
    let unknown = client
        .get(format!(
            "http://{}/console/assets/compatibility-missing.js",
            daemon.admin_addr
        ))
        .send()
        .await?;
    if unknown.status() != reqwest::StatusCode::NOT_FOUND {
        return Err("unknown console asset did not return 404".into());
    }
    cases.push("console-index-hashed-assets-and-404".to_owned());
    Ok(())
}

fn extract_asset(body: &str, extension: &str) -> Option<String> {
    let marker = "./assets/";
    let mut rest = body;
    while let Some(start) = rest.find(marker) {
        let tail = &rest[start + 1..];
        let end = tail
            .find(['\"', '\'', ')', ';', '\n'])
            .unwrap_or(tail.len());
        let candidate = &tail[..end];
        if candidate.ends_with(extension) {
            return Some(format!("/console{candidate}"));
        }
        rest = &rest[start + marker.len()..];
    }
    None
}

async fn owners_zero(daemon: &Daemon) -> Result<(), DynError> {
    let client = reqwest::Client::new();
    for _ in 0..100 {
        let metrics = operator(client.get(format!("http://{}/metrics", daemon.admin_addr)))
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        if metrics.contains("hydracache_hc2_connections{transport=\"grpc_bidirectional\"} 0")
            && metrics.contains("hydracache_hc2_subscriptions{transport=\"grpc_bidirectional\"} 0")
            && metrics
                .contains("hydracache_hc2_pending_invocations{transport=\"grpc_bidirectional\"} 0")
        {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    Err("HC2 live owners did not return to zero".into())
}

fn live(version: u64, fill: u8) -> ReplicatedValueRecord {
    ReplicatedValueRecord::value(
        PartitionId::new(1),
        version,
        ClusterEpoch::new(version),
        vec![fill; 256],
    )
}

fn durable_write(root: &Path) -> Result<(Vec<String>, u64), DynError> {
    if root.exists() {
        return Err("durable-write requires an absent store path".into());
    }
    let mut store = DurableValueStore::open_with_budget(root, BUDGET)?;
    store.upsert("old-live", live(1, 0x72))?;
    store.tombstone(
        "old-tombstone",
        PartitionId::new(1),
        2,
        ClusterEpoch::new(2),
    )?;
    store.flush()?;
    verify_live(&store, "old-live", 0x72)?;
    verify_tombstone(&store, "old-tombstone")?;
    drop(store);
    let reopened = reopen(root, BUDGET)?;
    verify_live(&reopened, "old-live", 0x72)?;
    verify_tombstone(&reopened, "old-tombstone")?;
    Ok((
        vec![
            "b72-create-live-and-tombstone".to_owned(),
            "b72-flush-and-reopen".to_owned(),
        ],
        2,
    ))
}

fn durable_transition(root: &Path) -> Result<(Vec<String>, u64), DynError> {
    let mut store = reopen(root, BUDGET)?;
    verify_live(&store, "old-live", 0x72)?;
    verify_tombstone(&store, "old-tombstone")?;
    store.upsert("new-live", live(73, 0x73))?;
    store.flush()?;
    drop(store);
    let reopened = reopen(root, BUDGET)?;
    verify_live(&reopened, "old-live", 0x72)?;
    verify_live(&reopened, "new-live", 0x73)?;
    verify_tombstone(&reopened, "old-tombstone")?;
    drop(reopened);
    let cases = durable_fault_cases(root)?;
    Ok((cases, 1))
}

fn durable_rollback(root: &Path) -> Result<(Vec<String>, u64), DynError> {
    let store = reopen(root, BUDGET)?;
    verify_live(&store, "old-live", 0x72)?;
    verify_live(&store, "new-live", 0x73)?;
    verify_tombstone(&store, "old-tombstone")?;
    drop(store);
    let reopened = reopen(root, BUDGET)?;
    verify_live(&reopened, "new-live", 0x73)?;
    Ok((
        vec![
            "b72-read-old-and-candidate-writes".to_owned(),
            "b72-same-disk-reopen-after-candidate".to_owned(),
        ],
        0,
    ))
}

fn durable_fault_cases(root: &Path) -> Result<Vec<String>, DynError> {
    let gc_root = root.with_extension("gc-case");
    let budget_root = root.with_extension("budget-case");
    let corrupt_root = root.with_extension("corrupt-case");
    for path in [&gc_root, &budget_root, &corrupt_root] {
        if path.exists() {
            return Err(format!("fault-case path already exists: {}", path.display()).into());
        }
    }
    let mut gc = DurableValueStore::open_with_budget(&gc_root, BUDGET)?;
    gc.tombstone("gc", PartitionId::new(1), 10, ClusterEpoch::new(10))?;
    let mut tracker = TombstoneTracker::new(TombstoneBudget::new(8, 1024));
    tracker.admit("gc", 10, 1, Some(ClusterEpoch::new(10)));
    let report = gc.collect_tombstone_garbage(&mut tracker, ClusterEpoch::new(10), 1)?;
    if report.removed != 1 || report.reclaimed_bytes == 0 || gc.get("gc")?.is_some() {
        return Err("repair-confirmed tombstone GC failed".into());
    }
    drop(gc);

    let mut budget = DurableValueStore::open_with_budget(&budget_root, 1)?;
    if budget.upsert("too-large", live(1, 0x73)).is_ok() || budget.rejected_total() != 1 {
        return Err("durable budget rejection failed closed".into());
    }
    drop(budget);

    let mut corrupt = DurableValueStore::open_with_budget(&corrupt_root, BUDGET)?;
    corrupt.upsert("recoverable", live(1, 0x73))?;
    let original = corrupt
        .raw_record_for_test("recoverable")?
        .ok_or("raw durable record missing")?;
    corrupt.put_raw_record_for_test("recoverable", b"invalid")?;
    if corrupt.get("recoverable").is_ok() {
        return Err("checksum corruption was served".into());
    }
    corrupt.put_raw_record_for_test("recoverable", &original)?;
    verify_live(&corrupt, "recoverable", 0x73)?;
    drop(corrupt);
    for path in [&gc_root, &budget_root, &corrupt_root] {
        fs::remove_dir_all(path)?;
    }
    Ok(vec![
        "c73-read-old-write-new-and-reopen".to_owned(),
        "repair-confirmed-tombstone-gc".to_owned(),
        "budget-rejection-before-mutation".to_owned(),
        "checksum-corruption-loud-refusal-and-restore".to_owned(),
    ])
}

fn verify_live(store: &DurableValueStore, key: &str, fill: u8) -> Result<(), DynError> {
    let record = store
        .get(key)?
        .ok_or_else(|| format!("missing live record {key}"))?;
    match record.state {
        ReplicatedSlot::Value { value, .. } if value == vec![fill; 256] => Ok(()),
        other => Err(format!("unexpected live record {key}: {other:?}").into()),
    }
}

fn verify_tombstone(store: &DurableValueStore, key: &str) -> Result<(), DynError> {
    let record = store
        .get(key)?
        .ok_or_else(|| format!("missing tombstone record {key}"))?;
    if record.is_tombstone() {
        Ok(())
    } else {
        Err(format!("record {key} is not a tombstone").into())
    }
}

fn reopen(path: &Path, budget: u64) -> Result<DurableValueStore, DynError> {
    for _ in 0..100 {
        match DurableValueStore::open_with_budget(path, budget) {
            Ok(store) => return Ok(store),
            Err(error) if error.to_string().contains("could not acquire lock") => {
                thread::sleep(Duration::from_millis(10))
            }
            Err(error) => return Err(error.into()),
        }
    }
    Err("durable lock release timeout".into())
}
