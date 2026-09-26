use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::net::{SocketAddr, TcpListener as StdTcpListener};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use bytes::Bytes;
use hydracache::{
    CacheOptions, ClusterEpoch, DurableValueStore, HydraCache, PartitionId, ReplicatedValueRecord,
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
use hydracache_loadgen::{
    run_open_loop, OpenLoopConfig, OpenLoopObservation, PreloadOutcome, Target, TargetError,
    TargetOutcome, TargetRequest,
};
use rcgen::{
    BasicConstraints, CertificateParams, CertifiedIssuer, ExtendedKeyUsagePurpose, IsCa, KeyPair,
};
use serde::Serialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;

const FOCUSED_PROFILE_ID: &str = "integrated-focused-host-073-v1";
const LONG_PROFILE_ID: &str = "integrated-long-run-073-v1";
const I73_SHA: &str = "e757556d3a31d565f52a9561d6d4e555bb1cc373";
const C73_SHA: &str = "7e3070894aa51af96cdcb3e350eff923a309e1fa";
const CANARY_MARKER: &str = "HC-CANARY-RED:W10-HOST";
const PAYLOAD_BYTES: usize = 4_096;
const KEY_CARDINALITY: u64 = 256;
const TAG_CARDINALITY: u64 = 16;
const TTL: Duration = Duration::from_millis(50);
const RESP_CONNECTIONS: usize = 16;

#[derive(Clone, Copy)]
enum Surface {
    Hc2 = 0,
    Resp = 1,
    Hc1 = 2,
    Direct = 3,
    Tag = 4,
    Ttl = 5,
}

impl Surface {
    const ALL: [Self; 6] = [
        Self::Hc2,
        Self::Resp,
        Self::Hc1,
        Self::Direct,
        Self::Tag,
        Self::Ttl,
    ];

    fn from_sequence(sequence: u64) -> Self {
        match sequence % 100 {
            0..=34 => Self::Hc2,
            35..=64 => Self::Resp,
            65..=79 => Self::Hc1,
            80..=89 => Self::Direct,
            90..=94 => Self::Tag,
            _ => Self::Ttl,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Hc2 => "hc2",
            Self::Resp => "resp",
            Self::Hc1 => "hc1",
            Self::Direct => "direct",
            Self::Tag => "tag_invalidation",
            Self::Ttl => "ttl_expire_refill",
        }
    }

    fn expected_count(self, operations: u64) -> u64 {
        let (start, width) = match self {
            Self::Hc2 => (0, 35),
            Self::Resp => (35, 30),
            Self::Hc1 => (65, 15),
            Self::Direct => (80, 10),
            Self::Tag => (90, 5),
            Self::Ttl => (95, 5),
        };
        operations / 100 * width + (operations % 100).saturating_sub(start).min(width)
    }
}

#[derive(Default)]
struct SurfaceCounter {
    attempted: AtomicU64,
    success: AtomicU64,
    rejected: AtomicU64,
    timeout: AtomicU64,
    late: AtomicU64,
    incomplete: AtomicU64,
}

#[derive(Debug, Clone, Serialize)]
struct SurfaceReceipt {
    attempted: u64,
    success: u64,
    rejected: u64,
    timeout: u64,
    late: u64,
    incomplete: u64,
}

impl SurfaceCounter {
    fn reset(&self) {
        for counter in [
            &self.attempted,
            &self.success,
            &self.rejected,
            &self.timeout,
            &self.late,
            &self.incomplete,
        ] {
            counter.store(0, Ordering::Release);
        }
    }

    fn receipt(&self) -> SurfaceReceipt {
        let load = |counter: &AtomicU64| counter.load(Ordering::Acquire);
        SurfaceReceipt {
            attempted: load(&self.attempted),
            success: load(&self.success),
            rejected: load(&self.rejected),
            timeout: load(&self.timeout),
            late: load(&self.late),
            incomplete: load(&self.incomplete),
        }
    }
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
    let mut client_params = CertificateParams::new(vec!["integrated-073".to_owned()]).unwrap();
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
        "hydracache-integrated-073-{label}-{}-{unique}",
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
    fn start(server_binary: &Path, daemon_cpu_set: &str) -> Result<Self, Box<dyn Error>> {
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
        let mut command = daemon_command(server_binary, daemon_cpu_set);
        let mut child = command
            .env("HYDRACACHE_CLIENT_API_ENABLED", "true")
            .env("HYDRACACHE_LISTEN_ADDR", client_addr.to_string())
            .env("HYDRACACHE_ADMIN_API_ENABLED", "true")
            .env("HYDRACACHE_ADMIN_ADDR", admin_addr.to_string())
            .env("HYDRACACHE_HC2_ENABLED", "true")
            .env("HYDRACACHE_HC2_ADDR", hc2_addr.to_string())
            .env("HYDRACACHE_HC2_CLUSTER_ID", "integrated-073")
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

    async fn hc2(&self) -> Result<Hc2Client, Box<dyn Error>> {
        let adapter = GrpcMtlsAdapter::new(GrpcMtlsConfig::new(
            format!("https://{}", self.hc2_addr),
            "localhost",
            self.material.ca.as_bytes(),
            self.material.client_cert.as_bytes(),
            self.material.client_key.as_bytes(),
        )?);
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            match Hc2Client::connect(&adapter, ClientConfig::new("integrated-073", "tenant-a"))
                .await
            {
                Ok(client) => return Ok(client),
                Err(_) if tokio::time::Instant::now() < deadline => {
                    tokio::time::sleep(Duration::from_millis(25)).await
                }
                Err(error) => return Err(error.into()),
            }
        }
    }

    async fn shutdown(mut self) -> Result<(), Box<dyn Error>> {
        let response = reqwest::Client::new()
            .post(format!("http://{}/admin/drain", self.admin_addr))
            .header(HYDRACACHE_CLIENT_ID_HEADER, "integrated-operator")
            .header(HYDRACACHE_TENANT_HEADER, "system")
            .header(HYDRACACHE_ADMIN_HEADER, "true")
            .send()
            .await?;
        if !response.status().is_success() {
            return Err("daemon drain rejected".into());
        }
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

    async fn wait_for_hc2_owners_zero(&self) -> Result<(), Box<dyn Error>> {
        let client = reqwest::Client::new();
        for _ in 0..100 {
            let metrics = client
                .get(format!("http://{}/metrics", self.admin_addr))
                .header(HYDRACACHE_CLIENT_ID_HEADER, "integrated-operator")
                .header(HYDRACACHE_TENANT_HEADER, "system")
                .header(HYDRACACHE_ADMIN_HEADER, "true")
                .send()
                .await?
                .error_for_status()?
                .text()
                .await?;
            if metrics.contains("hydracache_hc2_connections{transport=\"grpc_bidirectional\"} 0")
                && metrics
                    .contains("hydracache_hc2_subscriptions{transport=\"grpc_bidirectional\"} 0")
                && metrics.contains(
                    "hydracache_hc2_pending_invocations{transport=\"grpc_bidirectional\"} 0",
                )
            {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        Err("HC2 live owners did not return to zero".into())
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

struct MixedTarget {
    hc2: Hc2Client,
    hc1: reqwest::Client,
    hc1_addr: SocketAddr,
    resp: Vec<Mutex<tokio::net::TcpStream>>,
    direct: HydraCache,
    payload: Bytes,
    measured: AtomicBool,
    counters: [SurfaceCounter; 6],
    events: Arc<AtomicU64>,
}

impl MixedTarget {
    async fn new(daemon: &Daemon) -> Result<Arc<Self>, Box<dyn Error>> {
        let hc2 = daemon.hc2().await?;
        let mut subscription = hc2.subscribe(Bytes::from_static(b"hc2/"), 0).await?;
        let events = Arc::new(AtomicU64::new(0));
        let event_count = Arc::clone(&events);
        tokio::spawn(async move {
            while let Some(event) = subscription.next().await {
                if matches!(event, hydracache_client_hc2::SubscriptionEvent::Event(_)) {
                    event_count.fetch_add(1, Ordering::Relaxed);
                }
            }
        });
        let mut resp = Vec::with_capacity(RESP_CONNECTIONS);
        for _ in 0..RESP_CONNECTIONS {
            resp.push(Mutex::new(
                tokio::net::TcpStream::connect(daemon.redis_addr).await?,
            ));
        }
        Ok(Arc::new(Self {
            hc2,
            hc1: reqwest::Client::new(),
            hc1_addr: daemon.client_addr,
            resp,
            direct: HydraCache::local().max_capacity(4_096).build(),
            payload: Bytes::from(vec![0x73; PAYLOAD_BYTES]),
            measured: AtomicBool::new(false),
            counters: std::array::from_fn(|_| SurfaceCounter::default()),
            events,
        }))
    }

    fn begin_measurement(&self) {
        for counter in &self.counters {
            counter.reset();
        }
        self.measured.store(true, Ordering::Release);
    }

    async fn wait_for_events(&self, expected: u64) -> Result<(), Box<dyn Error>> {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        loop {
            let observed = self.events.load(Ordering::Acquire);
            if observed == expected {
                return Ok(());
            }
            if observed > expected {
                return Err(format!(
                    "HC2 event accounting exceeded expectation: expected {expected}, observed {observed}"
                )
                .into());
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(format!(
                    "HC2 event drain timeout: expected {expected}, observed {observed}"
                )
                .into());
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    async fn operation(
        &self,
        surface: Surface,
        sequence: u64,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let key_index = sequence % KEY_CARDINALITY;
        match surface {
            Surface::Hc2 => {
                self.hc2
                    .put(
                        Bytes::from(format!("hc2/{key_index:04}")),
                        self.payload.clone(),
                        None,
                        None,
                    )
                    .await?;
            }
            Surface::Resp => {
                let key = format!("resp-{key_index:04}");
                let mut input = format!(
                    "*3\r\n$3\r\nSET\r\n${}\r\n{}\r\n${}\r\n",
                    key.len(),
                    key,
                    self.payload.len()
                )
                .into_bytes();
                input.extend_from_slice(&self.payload);
                input.extend_from_slice(b"\r\n");
                let mut stream = self.resp[sequence as usize % self.resp.len()].lock().await;
                stream.write_all(&input).await?;
                let mut response = [0_u8; 5];
                stream.read_exact(&mut response).await?;
                if &response != b"+OK\r\n" {
                    return Err("unexpected RESP response".into());
                }
            }
            Surface::Hc1 => {
                let request = ClientRequestEnvelope::new(
                    format!("hc1-{sequence}"),
                    ClientRequest::Put {
                        ns: Namespace::new("integrated").unwrap(),
                        key: StructuredKey::new(vec![hex_key("hc1", key_index)]).unwrap(),
                        value: self.payload.to_vec(),
                        ttl_ms: None,
                        dimensions: Vec::new(),
                    },
                );
                let body = ClientFrame::from_message(&ClientWireMessage::Request(request))?
                    .encode()?
                    .to_vec();
                let response = self
                    .hc1
                    .post(format!("http://{}{}", self.hc1_addr, CLIENT_DATA_PATH))
                    .header(HYDRACACHE_CLIENT_ID_HEADER, "integrated-hc1")
                    .header(HYDRACACHE_TENANT_HEADER, "tenant-a")
                    .body(body)
                    .send()
                    .await?;
                let bytes = response.error_for_status()?.bytes().await?;
                let frame = ClientFrame::decode(&bytes, 8 * 1024 * 1024)?;
                let ClientWireMessage::Response(ClientResponseEnvelope {
                    result: Ok(ClientResponse::Stored),
                    ..
                }) = frame.decode_message()?
                else {
                    return Err("unexpected HC1 response".into());
                };
            }
            Surface::Direct => {
                self.direct
                    .put_encoded(
                        &format!("direct-{key_index:04}"),
                        self.payload.clone(),
                        CacheOptions::new().tag(format!("tag-{}", key_index % TAG_CARDINALITY)),
                    )
                    .await?
            }
            Surface::Tag => {
                let tag = format!("tag-{}", key_index % TAG_CARDINALITY);
                self.direct.invalidate_tag(&tag).await?;
                self.direct
                    .put_encoded(
                        &format!("direct-{key_index:04}"),
                        self.payload.clone(),
                        CacheOptions::new().tag(tag),
                    )
                    .await?;
            }
            Surface::Ttl => {
                let key = format!("ttl-{key_index:04}");
                let should_put = if sequence.is_multiple_of(2) {
                    true
                } else {
                    self.direct.get_encoded(&key).await?.is_none()
                };
                if should_put {
                    self.direct
                        .put_encoded(&key, self.payload.clone(), CacheOptions::new().ttl(TTL))
                        .await?;
                }
            }
        }
        Ok(())
    }

    fn surface_receipts(&self) -> BTreeMap<&'static str, SurfaceReceipt> {
        Surface::ALL
            .into_iter()
            .map(|surface| (surface.name(), self.counters[surface as usize].receipt()))
            .collect()
    }
}

#[async_trait]
impl Target for MixedTarget {
    async fn reset(&self) -> Result<String, TargetError> {
        self.direct
            .flush()
            .await
            .map_err(|error| TargetError::Reset(error.to_string()))?;
        Ok("integrated-073-reset-v1".to_owned())
    }

    async fn preload(&self) -> Result<PreloadOutcome, TargetError> {
        for sequence in 0..KEY_CARDINALITY {
            self.direct
                .put_encoded(
                    &format!("direct-{sequence:04}"),
                    self.payload.clone(),
                    CacheOptions::new().tag(format!("tag-{}", sequence % TAG_CARDINALITY)),
                )
                .await
                .map_err(|error| TargetError::Preload(error.to_string()))?;
        }
        Ok(PreloadOutcome {
            operations: KEY_CARDINALITY,
            state_digest: "integrated-073-preload-256-v1".to_owned(),
        })
    }

    async fn state_digest(&self) -> Result<String, TargetError> {
        Ok(format!(
            "integrated-073-entries-{}",
            self.direct.diagnostics().await.estimated_entries
        ))
    }

    async fn execute(&self, request: TargetRequest) -> TargetOutcome {
        let surface = Surface::from_sequence(request.sequence);
        let counter = &self.counters[surface as usize];
        if self.measured.load(Ordering::Acquire) {
            counter.attempted.fetch_add(1, Ordering::Relaxed);
        }
        match self.operation(surface, request.sequence).await {
            Ok(()) => {
                if self.measured.load(Ordering::Acquire) {
                    counter.success.fetch_add(1, Ordering::Relaxed);
                }
                TargetOutcome::Success
            }
            Err(_) => {
                if self.measured.load(Ordering::Acquire) {
                    counter.incomplete.fetch_add(1, Ordering::Relaxed);
                }
                TargetOutcome::Error
            }
        }
    }
}

#[derive(Serialize)]
struct ResourceDelta {
    available: bool,
    cpu_seconds: Option<f64>,
    cpu_seconds_per_completed_operation: Option<f64>,
    rss_before_bytes: Option<u64>,
    rss_after_bytes: Option<u64>,
    peak_rss_after_bytes: Option<u64>,
    anonymous_pss_before_bytes: Option<u64>,
    anonymous_pss_after_bytes: Option<u64>,
    file_pss_before_bytes: Option<u64>,
    file_pss_after_bytes: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize)]
struct ProcessResources {
    available: bool,
    cpu_seconds: f64,
    rss_bytes: u64,
    peak_rss_bytes: u64,
    anonymous_pss_bytes: u64,
    file_pss_bytes: u64,
    minor_faults: u64,
    major_faults: u64,
    threads: u64,
    file_descriptors: u64,
}

impl ProcessResources {
    fn unavailable() -> Self {
        Self {
            available: false,
            cpu_seconds: 0.0,
            rss_bytes: 0,
            peak_rss_bytes: 0,
            anonymous_pss_bytes: 0,
            file_pss_bytes: 0,
            minor_faults: 0,
            major_faults: 0,
            threads: 0,
            file_descriptors: 0,
        }
    }

    #[cfg(target_os = "linux")]
    fn combine(self, other: Self) -> Self {
        Self {
            available: self.available && other.available,
            cpu_seconds: self.cpu_seconds + other.cpu_seconds,
            rss_bytes: self.rss_bytes + other.rss_bytes,
            peak_rss_bytes: self.peak_rss_bytes + other.peak_rss_bytes,
            anonymous_pss_bytes: self.anonymous_pss_bytes + other.anonymous_pss_bytes,
            file_pss_bytes: self.file_pss_bytes + other.file_pss_bytes,
            minor_faults: self.minor_faults + other.minor_faults,
            major_faults: self.major_faults + other.major_faults,
            threads: self.threads + other.threads,
            file_descriptors: self.file_descriptors + other.file_descriptors,
        }
    }
}

#[derive(Serialize)]
struct ResourceCheckpoint {
    schema_version: u32,
    sequence: u64,
    kind: &'static str,
    elapsed_seconds: f64,
    unix_time_milliseconds: u128,
    completed_operations: u64,
    events_received: u64,
    owner_reconciled: bool,
    surfaces: BTreeMap<&'static str, SurfaceReceipt>,
    resources: ProcessResources,
}

#[derive(Serialize)]
struct DurableReceipt {
    attempted: u64,
    success: u64,
    budget_rejections: u64,
    reclaimed_bytes: u64,
    reopen_verified: bool,
    corruption_rejected: bool,
}

#[derive(Serialize)]
struct Receipt {
    schema_version: u32,
    release: &'static str,
    profile_id: String,
    role: String,
    source_sha: String,
    offered_rate_per_second: u64,
    operations: u64,
    warmup_operations: u64,
    weights_percent: [u64; 6],
    daemon_cpu_set: String,
    loadgen_cpu_set: String,
    observation: OpenLoopObservation,
    surfaces: BTreeMap<&'static str, SurfaceReceipt>,
    resources: ResourceDelta,
    events_received: u64,
    reconciliation_exact: bool,
    management_truth_zero: bool,
    durable: DurableReceipt,
    checkpoint_interval_seconds: u64,
    post_work_idle_seconds: u64,
    checkpoint_count: u64,
    final_checkpoint_present: bool,
    promotable: bool,
}

fn completed_operations(surfaces: &BTreeMap<&'static str, SurfaceReceipt>) -> u64 {
    surfaces.values().map(|surface| surface.success).sum()
}

fn checkpoint(
    sequence: u64,
    kind: &'static str,
    started: Instant,
    target: &MixedTarget,
    resources: ProcessResources,
    owner_reconciled: bool,
) -> ResourceCheckpoint {
    let surfaces = target.surface_receipts();
    checkpoint_from_state(
        sequence,
        kind,
        started,
        surfaces,
        target.events.load(Ordering::Acquire),
        resources,
        owner_reconciled,
    )
}

fn checkpoint_from_state(
    sequence: u64,
    kind: &'static str,
    started: Instant,
    surfaces: BTreeMap<&'static str, SurfaceReceipt>,
    events_received: u64,
    resources: ProcessResources,
    owner_reconciled: bool,
) -> ResourceCheckpoint {
    ResourceCheckpoint {
        schema_version: 1,
        sequence,
        kind,
        elapsed_seconds: started.elapsed().as_secs_f64(),
        unix_time_milliseconds: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time after epoch")
            .as_millis(),
        completed_operations: completed_operations(&surfaces),
        events_received,
        owner_reconciled,
        surfaces,
        resources,
    }
}

fn append_checkpoint(path: &Path, checkpoint: &ResourceCheckpoint) -> Result<(), Box<dyn Error>> {
    let mut output = OpenOptions::new().create(true).append(true).open(path)?;
    serde_json::to_writer(&mut output, checkpoint)?;
    output.write_all(b"\n")?;
    output.flush()?;
    Ok(())
}

fn spawn_checkpoint_sampler(
    path: PathBuf,
    target: Arc<MixedTarget>,
    self_pid: u32,
    daemon_pid: u32,
    interval: Duration,
    started: Instant,
    allow_unavailable_resources: bool,
) -> (mpsc::Sender<()>, thread::JoinHandle<Result<u64, String>>) {
    let (stop_tx, stop_rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        let mut sequence = 1_u64;
        loop {
            match stop_rx.recv_timeout(interval) {
                Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(sequence - 1),
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    let resources = match process_resources(self_pid, daemon_pid) {
                        Ok(resources) => resources,
                        Err(_) if allow_unavailable_resources => ProcessResources::unavailable(),
                        Err(error) => return Err(error.to_string()),
                    };
                    append_checkpoint(
                        &path,
                        &checkpoint(
                            sequence,
                            "periodic-work",
                            started,
                            &target,
                            resources,
                            false,
                        ),
                    )
                    .map_err(|error| error.to_string())?;
                    println!(
                        "LONG073_HEARTBEAT sequence={sequence} elapsed_seconds={:.3}",
                        started.elapsed().as_secs_f64()
                    );
                    sequence += 1;
                }
            }
        }
    });
    (stop_tx, handle)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let options = Options::parse()?;
    let daemon = Daemon::start(&options.server_binary, &options.daemon_cpu_set)?;
    let daemon_pid = daemon.child.id();
    let target = MixedTarget::new(&daemon).await?;
    target.preload().await?;
    for sequence in 0..options.warmup_operations {
        if target.execute(TargetRequest { sequence }).await != TargetOutcome::Success {
            return Err("warmup failed".into());
        }
    }
    target
        .wait_for_events(Surface::Hc2.expected_count(options.warmup_operations))
        .await?;
    target.events.store(0, Ordering::Release);
    target.begin_measurement();
    let measurement_started = Instant::now();
    let before = process_resources(std::process::id(), daemon_pid).ok();
    if before.is_none() && !options.allow_unavailable_resources {
        return Err("combined resources unavailable".into());
    }
    let mut checkpoint_count = 0_u64;
    let sampler = if let Some(path) = options.checkpoint_output.as_ref() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let resources = before.unwrap_or_else(ProcessResources::unavailable);
        append_checkpoint(
            path,
            &checkpoint(
                0,
                "pre-work",
                measurement_started,
                &target,
                resources,
                false,
            ),
        )?;
        checkpoint_count = 1;
        Some(spawn_checkpoint_sampler(
            path.clone(),
            Arc::clone(&target),
            std::process::id(),
            daemon_pid,
            Duration::from_secs(options.checkpoint_interval_seconds),
            measurement_started,
            options.allow_unavailable_resources,
        ))
    } else {
        None
    };
    let observation = run_open_loop(
        Arc::clone(&target),
        &OpenLoopConfig {
            offered_rate_per_second: options.rate,
            operations: options.operations,
            highest_trackable_latency: Duration::from_secs(5),
            significant_figures: 3,
            p999_min_samples: 5_000,
            drain_timeout: Duration::from_secs(10),
        },
    )
    .await?;
    if let Some((stop, handle)) = sampler {
        let _ = stop.send(());
        checkpoint_count += handle
            .join()
            .map_err(|_| "resource checkpoint sampler panicked")?
            .map_err(|error| format!("resource checkpoint sampler failed: {error}"))?;
    }
    let after = process_resources(std::process::id(), daemon_pid).ok();
    if after.is_none() && !options.allow_unavailable_resources {
        return Err("combined resources unavailable after workload".into());
    }
    if observation.started != observation.offered
        || observation.completed != observation.started
        || observation.successes != observation.completed
        || observation.errors != 0
        || observation.timeouts != 0
        || observation.rejections != 0
        || !observation.backlog_drained
    {
        return Err("incomplete open-loop outcomes".into());
    }
    let mut surfaces = target.surface_receipts();
    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("HOST073") {
        let hc2 = surfaces.get_mut("hc2").expect("HC2 receipt");
        hc2.success = hc2.success.saturating_sub(1);
        hc2.incomplete = hc2.incomplete.saturating_add(1);
    }
    if surfaces.values().map(|value| value.attempted).sum::<u64>() != options.operations
        || surfaces.values().any(|value| {
            value.success != value.attempted
                || value.rejected != 0
                || value.timeout != 0
                || value.late != 0
                || value.incomplete != 0
        })
    {
        return Err(format!("{CANARY_MARKER}: incomplete per-surface outcomes").into());
    }
    target.direct.flush().await?;
    let reconciliation_exact = target.direct.reconcile_memory_footprint().await?.matched;
    if !reconciliation_exact {
        return Err("direct reconciliation failed".into());
    }
    let durable = durable_companion()?;
    let expected_events = Surface::Hc2.expected_count(options.operations);
    target.wait_for_events(expected_events).await?;
    let events_received = target.events.load(Ordering::Acquire);
    let hc2_success = surfaces["hc2"].success;
    if events_received != expected_events || events_received != hc2_success {
        return Err("HC2 event accounting mismatch".into());
    }
    if let Some(path) = options.checkpoint_output.as_ref() {
        let resources = after.unwrap_or_else(ProcessResources::unavailable);
        append_checkpoint(
            path,
            &checkpoint_from_state(
                checkpoint_count,
                "final-work",
                measurement_started,
                surfaces.clone(),
                events_received,
                resources,
                false,
            ),
        )?;
        checkpoint_count += 1;
    }
    target.hc2.close();
    drop(target);
    daemon.wait_for_hc2_owners_zero().await?;
    if options.post_work_idle_seconds > 0 {
        tokio::time::sleep(Duration::from_secs(options.post_work_idle_seconds)).await;
    }
    let final_checkpoint_present = if let Some(path) = options.checkpoint_output.as_ref() {
        if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("LONG073-MISSING-FINAL") {
            false
        } else {
            let resources = match process_resources(std::process::id(), daemon_pid) {
                Ok(resources) => resources,
                Err(_) if options.allow_unavailable_resources => ProcessResources::unavailable(),
                Err(error) => return Err(error),
            };
            append_checkpoint(
                path,
                &checkpoint_from_state(
                    checkpoint_count,
                    "post-idle-reconciled",
                    measurement_started,
                    surfaces.clone(),
                    events_received,
                    resources,
                    true,
                ),
            )?;
            checkpoint_count += 1;
            true
        }
    } else {
        false
    };
    let resources = match (before, after) {
        (Some(before), Some(after)) => ResourceDelta {
            available: true,
            cpu_seconds: Some((after.cpu_seconds - before.cpu_seconds).max(0.0)),
            cpu_seconds_per_completed_operation: Some(
                (after.cpu_seconds - before.cpu_seconds).max(0.0) / observation.completed as f64,
            ),
            rss_before_bytes: Some(before.rss_bytes),
            rss_after_bytes: Some(after.rss_bytes),
            peak_rss_after_bytes: Some(after.peak_rss_bytes.max(before.peak_rss_bytes)),
            anonymous_pss_before_bytes: Some(before.anonymous_pss_bytes),
            anonymous_pss_after_bytes: Some(after.anonymous_pss_bytes),
            file_pss_before_bytes: Some(before.file_pss_bytes),
            file_pss_after_bytes: Some(after.file_pss_bytes),
        },
        _ => ResourceDelta {
            available: false,
            cpu_seconds: None,
            cpu_seconds_per_completed_operation: None,
            rss_before_bytes: None,
            rss_after_bytes: None,
            peak_rss_after_bytes: None,
            anonymous_pss_before_bytes: None,
            anonymous_pss_after_bytes: None,
            file_pss_before_bytes: None,
            file_pss_after_bytes: None,
        },
    };
    let receipt = Receipt {
        schema_version: 1,
        release: "0.73",
        profile_id: options.profile_id,
        role: options.role,
        source_sha: options.source_sha,
        offered_rate_per_second: options.rate,
        operations: options.operations,
        warmup_operations: options.warmup_operations,
        weights_percent: [35, 30, 15, 10, 5, 5],
        daemon_cpu_set: options.daemon_cpu_set,
        loadgen_cpu_set: options.loadgen_cpu_set,
        observation,
        surfaces,
        resources,
        events_received,
        reconciliation_exact,
        management_truth_zero: true,
        durable,
        checkpoint_interval_seconds: options.checkpoint_interval_seconds,
        post_work_idle_seconds: options.post_work_idle_seconds,
        checkpoint_count,
        final_checkpoint_present,
        promotable: false,
    };
    if let Some(parent) = options.output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&options.output, serde_json::to_vec_pretty(&receipt)?)?;
    daemon.shutdown().await?;
    Ok(())
}

fn durable_companion() -> Result<DurableReceipt, Box<dyn Error>> {
    let root = temp_root("durable");
    let corrupt_root = temp_root("corrupt");
    let mut store = DurableValueStore::open_with_budget(&root, 2 * 1024 * 1024)?;
    let record = |version, fill| {
        ReplicatedValueRecord::value(
            PartitionId::new(1),
            version,
            ClusterEpoch::new(1),
            vec![fill; PAYLOAD_BYTES],
        )
    };
    let mut attempted = 0;
    for index in 0..500 {
        store.upsert(
            format!("key-{}", index % KEY_CARDINALITY),
            record(index + 1, index as u8),
        )?;
        attempted += 1;
    }
    for index in 0..250 {
        if store
            .get(&format!("key-{}", index % KEY_CARDINALITY))?
            .is_none()
        {
            return Err("durable read miss".into());
        }
        attempted += 1;
    }
    let mut tracker = TombstoneTracker::new(TombstoneBudget::new(512, 2 * 1024 * 1024));
    for index in 0..100 {
        let key = format!("key-{index}");
        store.tombstone(
            &key,
            PartitionId::new(1),
            2_000 + index,
            ClusterEpoch::new(2),
        )?;
        tracker.admit(&key, 2_000 + index, 128, Some(ClusterEpoch::new(2)));
        attempted += 1;
    }
    let reclaimed_bytes = store
        .collect_tombstone_garbage(&mut tracker, ClusterEpoch::new(2), KEY_CARDINALITY as usize)?
        .reclaimed_bytes;
    attempted += 1;
    store.flush()?;
    drop(store);
    let reopened = reopen(&root, 2 * 1024 * 1024)?;
    for index in 100..248 {
        if reopened.get(&format!("key-{index}"))?.is_none() {
            return Err("durable reopen miss".into());
        }
        attempted += 1;
    }
    let mut corrupt = DurableValueStore::open_with_budget(&corrupt_root, 16 * 1024)?;
    corrupt.upsert("bad", record(1, 0x44))?;
    corrupt.put_raw_record_for_test("bad", b"invalid")?;
    let corruption_rejected = corrupt.get("bad").is_err();
    attempted += 1;
    drop(corrupt);
    drop(reopened);
    fs::remove_dir_all(root)?;
    fs::remove_dir_all(corrupt_root)?;
    if attempted != 1_000 || reclaimed_bytes == 0 || !corruption_rejected {
        return Err("durable companion accounting failed".into());
    }
    Ok(DurableReceipt {
        attempted,
        success: attempted,
        budget_rejections: 0,
        reclaimed_bytes,
        reopen_verified: true,
        corruption_rejected,
    })
}

fn reopen(path: &Path, budget: u64) -> Result<DurableValueStore, Box<dyn Error>> {
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

fn hex_key(prefix: &str, index: u64) -> String {
    format!("{prefix}-{index:04}")
        .bytes()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(target_os = "linux")]
fn process_resources(self_pid: u32, daemon_pid: u32) -> Result<ProcessResources, Box<dyn Error>> {
    fn one(pid: u32) -> Result<ProcessResources, Box<dyn Error>> {
        let stat = fs::read_to_string(format!("/proc/{pid}/stat"))?;
        let after = stat
            .rsplit_once(')')
            .ok_or("invalid proc stat")?
            .1
            .split_whitespace()
            .collect::<Vec<_>>();
        let ticks = after[11].parse::<f64>()? + after[12].parse::<f64>()?;
        let minor_faults = after[7].parse::<u64>()?;
        let major_faults = after[9].parse::<u64>()?;
        let ticks_per_second = unsafe { libc::sysconf(libc::_SC_CLK_TCK) } as f64;
        let status = fs::read_to_string(format!("/proc/{pid}/status"))?;
        let kib = |name: &str| -> Result<u64, Box<dyn Error>> {
            Ok(status
                .lines()
                .find_map(|line| line.strip_prefix(name))
                .and_then(|value| value.split_whitespace().next())
                .ok_or("missing proc status")?
                .parse::<u64>()?
                * 1024)
        };
        let count = |name: &str| -> Result<u64, Box<dyn Error>> {
            Ok(status
                .lines()
                .find_map(|line| line.strip_prefix(name))
                .and_then(|value| value.split_whitespace().next())
                .ok_or("missing proc status count")?
                .parse::<u64>()?)
        };
        let smaps = fs::read_to_string(format!("/proc/{pid}/smaps_rollup"))?;
        let smaps_kib = |name: &str| -> Result<u64, Box<dyn Error>> {
            Ok(smaps
                .lines()
                .find_map(|line| line.strip_prefix(name))
                .and_then(|value| value.split_whitespace().next())
                .ok_or("missing smaps rollup field")?
                .parse::<u64>()?
                * 1024)
        };
        let file_descriptors = fs::read_dir(format!("/proc/{pid}/fd"))?.count() as u64;
        Ok(ProcessResources {
            available: true,
            cpu_seconds: ticks / ticks_per_second,
            rss_bytes: kib("VmRSS:")?,
            peak_rss_bytes: kib("VmHWM:")?,
            anonymous_pss_bytes: smaps_kib("Pss_Anon:")?,
            file_pss_bytes: smaps_kib("Pss_File:")?,
            minor_faults,
            major_faults,
            threads: count("Threads:")?,
            file_descriptors,
        })
    }
    Ok(one(self_pid)?.combine(one(daemon_pid)?))
}

#[cfg(not(target_os = "linux"))]
fn process_resources(_self_pid: u32, _daemon_pid: u32) -> Result<ProcessResources, Box<dyn Error>> {
    Err("combined resources require Linux".into())
}

struct Options {
    profile_id: String,
    role: String,
    source_sha: String,
    rate: u64,
    operations: u64,
    warmup_operations: u64,
    daemon_cpu_set: String,
    loadgen_cpu_set: String,
    server_binary: PathBuf,
    output: PathBuf,
    allow_unavailable_resources: bool,
    checkpoint_output: Option<PathBuf>,
    checkpoint_interval_seconds: u64,
    post_work_idle_seconds: u64,
}

impl Options {
    fn parse() -> Result<Self, Box<dyn Error>> {
        let mut values = BTreeMap::new();
        let mut args = std::env::args().skip(1);
        while let Some(name) = args.next() {
            if !name.starts_with("--") {
                return Err(format!("unsupported {name}").into());
            }
            values.insert(
                name.trim_start_matches("--").to_owned(),
                args.next().ok_or("argument value missing")?,
            );
        }
        let checkpoint_output = values.remove("checkpoint-output").map(PathBuf::from);
        let checkpoint_interval_seconds = values
            .remove("checkpoint-interval-seconds")
            .unwrap_or_else(|| "0".to_owned())
            .parse()?;
        let post_work_idle_seconds = values
            .remove("post-work-idle-seconds")
            .unwrap_or_else(|| "0".to_owned())
            .parse()?;
        let mut take = |name: &str| {
            values
                .remove(name)
                .ok_or_else(|| format!("--{name} required"))
        };
        let profile_id = take("profile-id")?;
        let options = Self {
            profile_id,
            role: take("role")?,
            source_sha: take("source-sha")?,
            rate: take("rate")?.parse()?,
            operations: take("operations")?.parse()?,
            warmup_operations: take("warmup-operations")?.parse()?,
            daemon_cpu_set: take("daemon-cpu-set")?,
            loadgen_cpu_set: take("loadgen-cpu-set")?,
            server_binary: PathBuf::from(take("server-binary")?),
            output: PathBuf::from(take("output")?),
            allow_unavailable_resources: take("allow-unavailable-resources")?.parse()?,
            checkpoint_output,
            checkpoint_interval_seconds,
            post_work_idle_seconds,
        };
        let identity_valid = matches!(
            (options.role.as_str(), options.source_sha.as_str()),
            ("I73", I73_SHA) | ("C73", C73_SHA)
        );
        let focused_profile = options.profile_id == FOCUSED_PROFILE_ID;
        let long_profile = options.profile_id == LONG_PROFILE_ID;
        let focused_host_shape = [5_000, 12_000, 17_000].contains(&options.rate)
            && options.operations == options.rate * 10
            && options.warmup_operations == 5_000
            && options.checkpoint_output.is_none()
            && options.checkpoint_interval_seconds == 0
            && options.post_work_idle_seconds == 0;
        let long_host_shape = options.rate == 12_000
            && [259_200_000, 1_036_800_000].contains(&options.operations)
            && options.warmup_operations == 5_000
            && options.checkpoint_output.is_some()
            && options.checkpoint_interval_seconds == 60
            && options.post_work_idle_seconds == 300;
        let profile_shape_valid = (focused_profile
            && (options.allow_unavailable_resources || focused_host_shape))
            || (long_profile
                && options.checkpoint_output.is_some()
                && options.checkpoint_interval_seconds > 0
                && (options.allow_unavailable_resources || long_host_shape));
        if !values.is_empty()
            || !identity_valid
            || !profile_shape_valid
            || options.rate == 0
            || options.operations == 0
            || options.daemon_cpu_set.is_empty()
            || options.loadgen_cpu_set.is_empty()
            || options.daemon_cpu_set == options.loadgen_cpu_set
            || !options.server_binary.is_file()
            || options.output.exists()
            || options
                .checkpoint_output
                .as_ref()
                .is_some_and(|path| path.exists())
        {
            return Err("invalid integrated harness arguments".into());
        }
        Ok(options)
    }
}

#[cfg(target_os = "linux")]
fn daemon_command(server_binary: &Path, daemon_cpu_set: &str) -> Command {
    let mut command = Command::new("taskset");
    command
        .arg("--cpu-list")
        .arg(daemon_cpu_set)
        .arg(server_binary);
    command
}

#[cfg(not(target_os = "linux"))]
fn daemon_command(server_binary: &Path, _daemon_cpu_set: &str) -> Command {
    Command::new(server_binary)
}
