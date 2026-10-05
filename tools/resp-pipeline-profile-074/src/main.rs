use std::collections::BTreeMap;
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use hdrhistogram::Histogram;
use hydracache_client_transport_axum::{
    ClientSurfaceLimits, ClientSurfaceProfileMetrics, ClientSurfaceRetainedState,
    ClientSurfaceState,
};
use hydracache_loadgen::allocation::{measure_allocations, AllocationMeasurement};
use hydracache_redis_compat::{RedisListenerConfig, RedisPipelineMetrics, RedisRespServer};
use serde::Serialize;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::net::{TcpListener, TcpStream};

const PROFILE_ID: &str = "w1-w3-resp-pipeline-profile-074-v3";
const W9C_READY_FILE: &str = "HYDRACACHE_W9C_READY_FILE";
const W9C_GO_FILE: &str = "HYDRACACHE_W9C_GO_FILE";
const W9C_RUSAGE_FILE: &str = "HYDRACACHE_W9C_RUSAGE_FILE";
const W9C_GATE_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Transport {
    Duplex,
    Tcp,
}

impl Transport {
    fn parse(value: &str) -> Result<Self, Box<dyn Error>> {
        match value {
            "duplex" => Ok(Self::Duplex),
            "tcp" => Ok(Self::Tcp),
            _ => Err(format!("unsupported transport {value}").into()),
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Duplex => "duplex",
            Self::Tcp => "tcp",
        }
    }

    const fn surface(self) -> &'static str {
        match self {
            Self::Duplex => "resp-api-in-process-duplex",
            Self::Tcp => "resp-api-loopback-tcp",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Operation {
    Get,
    Set,
    Mget,
    Mset,
    Del,
    Exists,
}

impl Operation {
    fn parse(value: &str) -> Result<Self, Box<dyn Error>> {
        match value.to_ascii_lowercase().as_str() {
            "get" => Ok(Self::Get),
            "set" => Ok(Self::Set),
            "mget" => Ok(Self::Mget),
            "mset" => Ok(Self::Mset),
            "del" => Ok(Self::Del),
            "exists" => Ok(Self::Exists),
            _ => Err(format!("unsupported operation {value}").into()),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Get => "get",
            Self::Set => "set",
            Self::Mget => "mget",
            Self::Mset => "mset",
            Self::Del => "del",
            Self::Exists => "exists",
        }
    }

    fn needs_preload(self) -> bool {
        matches!(self, Self::Get | Self::Mget | Self::Del | Self::Exists)
    }
}

#[derive(Debug, Clone)]
struct Options {
    source_commit: String,
    operation: Operation,
    operations: u64,
    warmup_operations: u64,
    concurrency: u64,
    pipeline: u64,
    batch_size: usize,
    payload_bytes: usize,
    key_space: u64,
    seed: u64,
    instrumentation: bool,
    transport: Transport,
    output: Option<PathBuf>,
}

impl Options {
    fn parse() -> Result<Self, Box<dyn Error>> {
        let mut values = BTreeMap::new();
        let mut args = std::env::args().skip(1);
        while let Some(name) = args.next() {
            if !name.starts_with("--") {
                return Err(format!("unsupported argument {name}").into());
            }
            let value = args
                .next()
                .ok_or_else(|| format!("missing value after {name}"))?;
            values.insert(name, value);
        }
        let option = Self {
            source_commit: values
                .remove("--source-commit")
                .ok_or("--source-commit is required")?,
            operation: Operation::parse(&take(&mut values, "--operation", "get"))?,
            operations: take(&mut values, "--operations", "10000").parse()?,
            warmup_operations: take(&mut values, "--warmup-operations", "1000").parse()?,
            concurrency: take(&mut values, "--concurrency", "1").parse()?,
            pipeline: take(&mut values, "--pipeline", "1").parse()?,
            batch_size: take(&mut values, "--batch-size", "8").parse()?,
            payload_bytes: take(&mut values, "--payload-bytes", "256").parse()?,
            key_space: take(&mut values, "--key-space", "4096").parse()?,
            seed: take(&mut values, "--seed", "74").parse()?,
            instrumentation: parse_bool(&take(&mut values, "--instrumentation", "true"))?,
            transport: Transport::parse(&take(&mut values, "--transport", "duplex"))?,
            output: values.remove("--output").map(PathBuf::from),
        };
        if !values.is_empty() {
            return Err(format!("unknown arguments: {:?}", values.keys()).into());
        }
        option.validate()?;
        Ok(option)
    }

    fn validate(&self) -> Result<(), Box<dyn Error>> {
        if self.operations == 0
            || self.concurrency == 0
            || self.pipeline == 0
            || self.batch_size == 0
            || self.payload_bytes == 0
            || self.key_space == 0
        {
            return Err("operations, concurrency, pipeline, batch size, payload, and key space must be non-zero".into());
        }
        if self.source_commit.len() != 40
            || !self
                .source_commit
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err("source-commit must be a full 40-character Git SHA".into());
        }
        let quantum = self.concurrency.saturating_mul(self.pipeline);
        if !self.operations.is_multiple_of(quantum)
            || !self.warmup_operations.is_multiple_of(quantum)
        {
            return Err(format!(
                "operations and warmup-operations must be divisible by concurrency * pipeline ({quantum})"
            )
            .into());
        }
        if self.operation == Operation::Del
            && self.key_space
                < self
                    .operations
                    .max(self.warmup_operations)
                    .saturating_mul(self.batch_size as u64)
        {
            return Err("DEL requires key-space >= max(operations, warmup-operations) * batch-size for exact one-hit semantics".into());
        }
        Ok(())
    }
}

fn take(values: &mut BTreeMap<String, String>, name: &str, default: &str) -> String {
    values.remove(name).unwrap_or_else(|| default.to_owned())
}

fn parse_bool(value: &str) -> Result<bool, Box<dyn Error>> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("expected true or false, got {value}").into()),
    }
}

#[derive(Debug, Serialize)]
struct LatencyReceipt {
    unit: &'static str,
    samples: u64,
    p50_us: u64,
    p95_us: u64,
    p99_us: u64,
    max_us: u64,
}

#[derive(Debug, Serialize)]
struct RespMetricsReceipt {
    read_calls: u64,
    input_bytes: u64,
    input_buffer_high_water_bytes: u64,
    decoded_commands: u64,
    parser_consumed_bytes: u64,
    input_compactions: u64,
    input_compaction_moved_bytes: u64,
    input_compaction_moved_bytes_per_operation: f64,
    translation_contexts: u64,
    request_id_bytes: u64,
    script_cache_entries_cloned: u64,
    output_frames: u64,
    output_bytes: u64,
    output_buffer_high_water_bytes: u64,
    write_calls: u64,
    flush_calls: u64,
}

#[derive(Debug, Serialize)]
struct ClientMetricsReceipt {
    dispatches: u64,
    clock_reads: u64,
    expiry_sweep_checks: u64,
    expiry_sweeps_claimed: u64,
    store_lock_acquisitions: u64,
    store_lock_wait_nanoseconds: u64,
    store_lock_hold_nanoseconds: u64,
    store_lock_wait_nanoseconds_per_operation: f64,
    store_lock_hold_nanoseconds_per_operation: f64,
}

#[derive(Debug, Serialize)]
struct SocketIoReceipt {
    available: bool,
    poll_write_attempts: u64,
    poll_write_ready: u64,
    poll_write_pending: u64,
    requested_write_bytes: u64,
    written_bytes: u64,
    short_writes: u64,
    poll_flush_attempts: u64,
    poll_flush_pending: u64,
}

#[derive(Debug, Serialize)]
struct Receipt {
    schema_version: u32,
    release: &'static str,
    profile_id: &'static str,
    source_commit: String,
    binary_sha256: String,
    build_profile: &'static str,
    tier: &'static str,
    promotable: bool,
    surface: &'static str,
    operation: &'static str,
    operations: u64,
    warmup_operations: u64,
    concurrency: u64,
    pipeline: u64,
    batch_size: usize,
    payload_bytes: usize,
    key_space: u64,
    seed: u64,
    instrumentation_enabled: bool,
    transport: &'static str,
    workload_sha256: String,
    exact_response_validation: bool,
    elapsed_seconds: f64,
    goodput_operations_per_second: f64,
    latency: LatencyReceipt,
    cpu_seconds: f64,
    cpu_nanoseconds_per_operation: f64,
    gross_allocated_bytes: u64,
    gross_allocated_bytes_per_operation: f64,
    rss_before_bytes: u64,
    rss_after_bytes: u64,
    peak_rss_bytes: u64,
    retained_rss_delta_bytes: i128,
    retained_client_state: ClientSurfaceRetainedState,
    resp: RespMetricsReceipt,
    client_surface: ClientMetricsReceipt,
    socket_io: SocketIoReceipt,
    unavailable_metrics: [&'static str; 4],
}

struct WorkloadResult {
    histogram: Histogram<u64>,
    elapsed: Duration,
    socket_io: SocketIoMetrics,
}

#[derive(Debug, Clone, Copy, Default)]
struct SocketIoMetrics {
    available: bool,
    poll_write_attempts: u64,
    poll_write_ready: u64,
    poll_write_pending: u64,
    requested_write_bytes: u64,
    written_bytes: u64,
    short_writes: u64,
    poll_flush_attempts: u64,
    poll_flush_pending: u64,
}

impl SocketIoMetrics {
    fn merge(&mut self, other: Self) {
        self.available |= other.available;
        self.poll_write_attempts = self
            .poll_write_attempts
            .saturating_add(other.poll_write_attempts);
        self.poll_write_ready = self.poll_write_ready.saturating_add(other.poll_write_ready);
        self.poll_write_pending = self
            .poll_write_pending
            .saturating_add(other.poll_write_pending);
        self.requested_write_bytes = self
            .requested_write_bytes
            .saturating_add(other.requested_write_bytes);
        self.written_bytes = self.written_bytes.saturating_add(other.written_bytes);
        self.short_writes = self.short_writes.saturating_add(other.short_writes);
        self.poll_flush_attempts = self
            .poll_flush_attempts
            .saturating_add(other.poll_flush_attempts);
        self.poll_flush_pending = self
            .poll_flush_pending
            .saturating_add(other.poll_flush_pending);
    }
}

#[derive(Debug, Default)]
struct SocketIoCounters {
    poll_write_attempts: AtomicU64,
    poll_write_ready: AtomicU64,
    poll_write_pending: AtomicU64,
    requested_write_bytes: AtomicU64,
    written_bytes: AtomicU64,
    short_writes: AtomicU64,
    poll_flush_attempts: AtomicU64,
    poll_flush_pending: AtomicU64,
}

impl SocketIoCounters {
    fn snapshot(&self) -> SocketIoMetrics {
        SocketIoMetrics {
            available: true,
            poll_write_attempts: self.poll_write_attempts.load(Ordering::Relaxed),
            poll_write_ready: self.poll_write_ready.load(Ordering::Relaxed),
            poll_write_pending: self.poll_write_pending.load(Ordering::Relaxed),
            requested_write_bytes: self.requested_write_bytes.load(Ordering::Relaxed),
            written_bytes: self.written_bytes.load(Ordering::Relaxed),
            short_writes: self.short_writes.load(Ordering::Relaxed),
            poll_flush_attempts: self.poll_flush_attempts.load(Ordering::Relaxed),
            poll_flush_pending: self.poll_flush_pending.load(Ordering::Relaxed),
        }
    }
}

struct ProfiledTcpStream {
    inner: TcpStream,
    counters: Arc<SocketIoCounters>,
}

impl ProfiledTcpStream {
    fn new(inner: TcpStream, counters: Arc<SocketIoCounters>) -> Self {
        Self { inner, counters }
    }
}

impl AsyncRead for ProfiledTcpStream {
    fn poll_read(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_read(context, buffer)
    }
}

impl AsyncWrite for ProfiledTcpStream {
    fn poll_write(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let this = self.get_mut();
        this.counters
            .poll_write_attempts
            .fetch_add(1, Ordering::Relaxed);
        this.counters
            .requested_write_bytes
            .fetch_add(buffer.len() as u64, Ordering::Relaxed);
        match Pin::new(&mut this.inner).poll_write(context, buffer) {
            Poll::Ready(Ok(written)) => {
                this.counters
                    .poll_write_ready
                    .fetch_add(1, Ordering::Relaxed);
                this.counters
                    .written_bytes
                    .fetch_add(written as u64, Ordering::Relaxed);
                if written < buffer.len() {
                    this.counters.short_writes.fetch_add(1, Ordering::Relaxed);
                }
                Poll::Ready(Ok(written))
            }
            Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
            Poll::Pending => {
                this.counters
                    .poll_write_pending
                    .fetch_add(1, Ordering::Relaxed);
                Poll::Pending
            }
        }
    }

    fn poll_flush(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        let this = self.get_mut();
        this.counters
            .poll_flush_attempts
            .fetch_add(1, Ordering::Relaxed);
        match Pin::new(&mut this.inner).poll_flush(context) {
            Poll::Pending => {
                this.counters
                    .poll_flush_pending
                    .fetch_add(1, Ordering::Relaxed);
                Poll::Pending
            }
            result => result,
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_shutdown(context)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let options = Options::parse()?;
    let state = Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default())?);
    let server = Arc::new(RedisRespServer::new(
        Arc::clone(&state),
        RedisListenerConfig::default(),
    )?);

    if options.operation.needs_preload() {
        preload(
            &server,
            &options,
            options.warmup_operations.max(options.operations),
        )
        .await?;
    }
    if options.warmup_operations > 0 {
        run_workload(Arc::clone(&server), &options, options.warmup_operations).await?;
        if options.operation == Operation::Del {
            preload(
                &server,
                &options,
                options.warmup_operations.max(options.operations),
            )
            .await?;
        }
    }

    server.reset_pipeline_metrics();
    state.reset_profile_metrics();
    server.set_pipeline_instrumentation_enabled(options.instrumentation);
    state.set_profile_instrumentation_enabled(options.instrumentation);
    wait_for_w9c_trace_gate()?;
    let w9c_before = optional_w9c_rusage()?;
    let resource_before = process_resources()?;
    let (workload_result, allocation) = measure_allocations(options.operations, async {
        let cpu_before = process_cpu_seconds()?;
        let result = run_workload(Arc::clone(&server), &options, options.operations).await;
        let cpu = (process_cpu_seconds()? - cpu_before).max(0.0);
        Ok::<_, Box<dyn Error>>((cpu, result?))
    })
    .await;
    let (workload, workload_result) = workload_result?;
    let w9c_after = optional_w9c_rusage()?;
    let resource_after = process_resources()?;
    server.set_pipeline_instrumentation_enabled(false);
    state.set_profile_instrumentation_enabled(false);
    write_w9c_rusage(w9c_before, w9c_after)?;

    let receipt = build_receipt(
        &options,
        Observations {
            cpu_seconds: workload,
            workload: workload_result,
            allocation,
            resource_before,
            resource_after,
            resp: server.pipeline_metrics(),
            client: state.profile_metrics(),
            retained: state.retained_state_for_diagnostics(),
        },
    );
    let json = serde_json::to_vec_pretty(&receipt)?;
    if let Some(output) = &options.output {
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(output, &json)?;
    }
    println!("{}", String::from_utf8(json)?);
    Ok(())
}

fn wait_for_w9c_trace_gate() -> Result<(), Box<dyn Error>> {
    let ready = std::env::var_os(W9C_READY_FILE).map(PathBuf::from);
    let go = std::env::var_os(W9C_GO_FILE).map(PathBuf::from);
    let (ready, go) = match (ready, go) {
        (None, None) => return Ok(()),
        (Some(ready), Some(go)) => (ready, go),
        _ => return Err("W9c trace gate requires both ready and go files".into()),
    };
    if go.exists() {
        return Err("W9c go file must not pre-exist".into());
    }
    if let Some(parent) = ready.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut marker = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&ready)?;
    writeln!(marker, "w9c-ready-v1 pid={}", std::process::id())?;
    marker.sync_all()?;

    let deadline = Instant::now() + W9C_GATE_TIMEOUT;
    loop {
        match fs::symlink_metadata(&go) {
            Ok(metadata) if metadata.file_type().is_file() && metadata.len() <= 64 => return Ok(()),
            Ok(_) => return Err("W9c go marker must be a bounded regular file".into()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        if Instant::now() >= deadline {
            return Err("timed out waiting for W9c trace go marker".into());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize)]
struct W9cRusageSnapshot {
    user_cpu_microseconds: u64,
    system_cpu_microseconds: u64,
    minor_page_faults: u64,
    major_page_faults: u64,
    voluntary_context_switches: u64,
    involuntary_context_switches: u64,
}

impl W9cRusageSnapshot {
    fn delta(self, after: Self) -> Self {
        Self {
            user_cpu_microseconds: after
                .user_cpu_microseconds
                .saturating_sub(self.user_cpu_microseconds),
            system_cpu_microseconds: after
                .system_cpu_microseconds
                .saturating_sub(self.system_cpu_microseconds),
            minor_page_faults: after
                .minor_page_faults
                .saturating_sub(self.minor_page_faults),
            major_page_faults: after
                .major_page_faults
                .saturating_sub(self.major_page_faults),
            voluntary_context_switches: after
                .voluntary_context_switches
                .saturating_sub(self.voluntary_context_switches),
            involuntary_context_switches: after
                .involuntary_context_switches
                .saturating_sub(self.involuntary_context_switches),
        }
    }
}

#[derive(Serialize)]
struct W9cRusageReceipt {
    schema_version: &'static str,
    source: &'static str,
    measurement_only: bool,
    delta: W9cRusageSnapshot,
}

fn optional_w9c_rusage() -> Result<Option<W9cRusageSnapshot>, Box<dyn Error>> {
    if std::env::var_os(W9C_RUSAGE_FILE).is_none() {
        return Ok(None);
    }
    process_rusage().map(Some)
}

fn write_w9c_rusage(
    before: Option<W9cRusageSnapshot>,
    after: Option<W9cRusageSnapshot>,
) -> Result<(), Box<dyn Error>> {
    let output = std::env::var_os(W9C_RUSAGE_FILE).map(PathBuf::from);
    match (output, before, after) {
        (None, None, None) => Ok(()),
        (Some(output), Some(before), Some(after)) => write_create_new_json(
            &output,
            &W9cRusageReceipt {
                schema_version: "hydracache-w9c-rusage-v1",
                source: "getrusage-RUSAGE_SELF",
                measurement_only: true,
                delta: before.delta(after),
            },
        ),
        _ => Err("W9c rusage capture was only partially configured".into()),
    }
}

fn write_create_new_json(path: &Path, value: &impl Serialize) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(value)?;
    let mut output = OpenOptions::new().write(true).create_new(true).open(path)?;
    output.write_all(&bytes)?;
    output.write_all(b"\n")?;
    output.sync_all()?;
    Ok(())
}

#[cfg(unix)]
fn process_rusage() -> Result<W9cRusageSnapshot, Box<dyn Error>> {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
    // SAFETY: getrusage initializes the supplied structure on success.
    if unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    // SAFETY: the successful call above initialized `usage`.
    let usage = unsafe { usage.assume_init() };
    let micros = |time: libc::timeval| {
        (time.tv_sec as u64)
            .saturating_mul(1_000_000)
            .saturating_add(time.tv_usec as u64)
    };
    Ok(W9cRusageSnapshot {
        user_cpu_microseconds: micros(usage.ru_utime),
        system_cpu_microseconds: micros(usage.ru_stime),
        minor_page_faults: usage.ru_minflt as u64,
        major_page_faults: usage.ru_majflt as u64,
        voluntary_context_switches: usage.ru_nvcsw as u64,
        involuntary_context_switches: usage.ru_nivcsw as u64,
    })
}

#[cfg(not(unix))]
fn process_rusage() -> Result<W9cRusageSnapshot, Box<dyn Error>> {
    Err("W9c rusage capture is Linux/Unix-only".into())
}

struct Observations {
    cpu_seconds: f64,
    workload: WorkloadResult,
    allocation: AllocationMeasurement,
    resource_before: ResourceSnapshot,
    resource_after: ResourceSnapshot,
    resp: RedisPipelineMetrics,
    client: ClientSurfaceProfileMetrics,
    retained: ClientSurfaceRetainedState,
}

fn build_receipt(options: &Options, observations: Observations) -> Receipt {
    let Observations {
        cpu_seconds,
        workload,
        allocation,
        resource_before: before,
        resource_after: after,
        resp,
        client,
        retained,
    } = observations;
    let operations = options.operations as f64;
    Receipt {
        schema_version: 1,
        release: "0.74",
        profile_id: PROFILE_ID,
        source_commit: options.source_commit.clone(),
        binary_sha256: binary_digest().expect("profile binary must remain readable"),
        build_profile: "release",
        tier: "local-quick",
        promotable: false,
        surface: options.transport.surface(),
        operation: options.operation.name(),
        operations: options.operations,
        warmup_operations: options.warmup_operations,
        concurrency: options.concurrency,
        pipeline: options.pipeline,
        batch_size: options.batch_size,
        payload_bytes: options.payload_bytes,
        key_space: options.key_space,
        seed: options.seed,
        instrumentation_enabled: options.instrumentation,
        transport: options.transport.name(),
        workload_sha256: workload_digest(options),
        exact_response_validation: true,
        elapsed_seconds: workload.elapsed.as_secs_f64(),
        goodput_operations_per_second: operations / workload.elapsed.as_secs_f64(),
        latency: LatencyReceipt {
            unit: "pipeline_batch_round_trip",
            samples: workload.histogram.len(),
            p50_us: workload.histogram.value_at_quantile(0.50),
            p95_us: workload.histogram.value_at_quantile(0.95),
            p99_us: workload.histogram.value_at_quantile(0.99),
            max_us: workload.histogram.max(),
        },
        cpu_seconds,
        cpu_nanoseconds_per_operation: cpu_seconds * 1_000_000_000.0 / operations,
        gross_allocated_bytes: allocation.gross_allocated_bytes,
        gross_allocated_bytes_per_operation: allocation.gross_allocated_bytes_per_operation,
        rss_before_bytes: before.rss_bytes,
        rss_after_bytes: after.rss_bytes,
        peak_rss_bytes: after.peak_rss_bytes.max(before.peak_rss_bytes),
        retained_rss_delta_bytes: after.rss_bytes as i128 - before.rss_bytes as i128,
        retained_client_state: retained,
        resp: RespMetricsReceipt {
            read_calls: resp.read_calls,
            input_bytes: resp.input_bytes,
            input_buffer_high_water_bytes: resp.input_buffer_high_water_bytes,
            decoded_commands: resp.decoded_commands,
            parser_consumed_bytes: resp.parser_consumed_bytes,
            input_compactions: resp.input_compactions,
            input_compaction_moved_bytes: resp.input_compaction_moved_bytes,
            input_compaction_moved_bytes_per_operation: resp.input_compaction_moved_bytes as f64
                / operations,
            translation_contexts: resp.translation_contexts,
            request_id_bytes: resp.request_id_bytes,
            script_cache_entries_cloned: resp.script_cache_entries_cloned,
            output_frames: resp.output_frames,
            output_bytes: resp.output_bytes,
            output_buffer_high_water_bytes: resp.output_buffer_high_water_bytes,
            write_calls: resp.write_calls,
            flush_calls: resp.flush_calls,
        },
        client_surface: ClientMetricsReceipt {
            dispatches: client.dispatches,
            clock_reads: client.clock_reads,
            expiry_sweep_checks: client.expiry_sweep_checks,
            expiry_sweeps_claimed: client.expiry_sweeps_claimed,
            store_lock_acquisitions: client.store_lock_acquisitions,
            store_lock_wait_nanoseconds: client.store_lock_wait_nanoseconds,
            store_lock_hold_nanoseconds: client.store_lock_hold_nanoseconds,
            store_lock_wait_nanoseconds_per_operation: client.store_lock_wait_nanoseconds as f64
                / operations,
            store_lock_hold_nanoseconds_per_operation: client.store_lock_hold_nanoseconds as f64
                / operations,
        },
        socket_io: SocketIoReceipt {
            available: workload.socket_io.available,
            poll_write_attempts: workload.socket_io.poll_write_attempts,
            poll_write_ready: workload.socket_io.poll_write_ready,
            poll_write_pending: workload.socket_io.poll_write_pending,
            requested_write_bytes: workload.socket_io.requested_write_bytes,
            written_bytes: workload.socket_io.written_bytes,
            short_writes: workload.socket_io.short_writes,
            poll_flush_attempts: workload.socket_io.poll_flush_attempts,
            poll_flush_pending: workload.socket_io.poll_flush_pending,
        },
        unavailable_metrics: [
            "kernel_write_syscalls_unavailable_without_elevated_etw_or_linux_strace",
            "kernel_flush_syscalls_unavailable_without_elevated_etw_or_linux_strace",
            "pending_write_duration_not_yet_instrumented",
            "per_stage_allocations_and_copies_not_yet_instrumented",
        ],
    }
}

async fn preload(
    server: &Arc<RedisRespServer>,
    options: &Options,
    operations: u64,
) -> Result<(), Box<dyn Error>> {
    let mut preload_options = options.clone();
    preload_options.operation = if options.batch_size > 1
        && matches!(
            options.operation,
            Operation::Mget | Operation::Del | Operation::Exists
        ) {
        Operation::Mset
    } else {
        Operation::Set
    };
    run_workload(Arc::clone(server), &preload_options, operations).await?;
    Ok(())
}

async fn run_workload(
    server: Arc<RedisRespServer>,
    options: &Options,
    operations: u64,
) -> Result<WorkloadResult, Box<dyn Error>> {
    if operations == 0 {
        return Ok(WorkloadResult {
            histogram: Histogram::new_with_bounds(1, 60_000_000, 3)?,
            elapsed: Duration::ZERO,
            socket_io: SocketIoMetrics::default(),
        });
    }
    let per_client = operations / options.concurrency;
    let started = Instant::now();
    let mut tasks = Vec::new();
    for client_index in 0..options.concurrency {
        let server = Arc::clone(&server);
        let options = options.clone();
        tasks.push(tokio::spawn(async move {
            run_client(server, &options, client_index * per_client, per_client).await
        }));
    }
    let mut merged = Histogram::new_with_bounds(1, 60_000_000, 3)?;
    let mut socket_io = SocketIoMetrics::default();
    for task in tasks {
        let (histogram, client_socket_io) = task.await.map_err(|error| error.to_string())??;
        merged.add(&histogram)?;
        socket_io.merge(client_socket_io);
    }
    Ok(WorkloadResult {
        histogram: merged,
        elapsed: started.elapsed(),
        socket_io,
    })
}

async fn run_client(
    server: Arc<RedisRespServer>,
    options: &Options,
    sequence_start: u64,
    operations: u64,
) -> Result<(Histogram<u64>, SocketIoMetrics), String> {
    match options.transport {
        Transport::Duplex => {
            let capacity = (options.pipeline as usize)
                .saturating_mul(options.batch_size)
                .saturating_mul(options.payload_bytes.saturating_add(256))
                .clamp(64 * 1024, 8 * 1024 * 1024);
            let (client, server_io) = tokio::io::duplex(capacity);
            let serve = tokio::spawn(async move { server.serve_connection(server_io).await });
            let histogram = exchange_pipeline(client, options, sequence_start, operations).await?;
            let result = serve.await.map_err(|error| error.to_string())?;
            result.map_err(|error| error.to_string())?;
            Ok((histogram, SocketIoMetrics::default()))
        }
        Transport::Tcp => {
            let listener = TcpListener::bind("127.0.0.1:0")
                .await
                .map_err(|error| error.to_string())?;
            let address = listener.local_addr().map_err(|error| error.to_string())?;
            let counters = Arc::new(SocketIoCounters::default());
            let server_counters = Arc::clone(&counters);
            let serve = tokio::spawn(async move {
                let (server_io, _) = listener.accept().await?;
                server_io.set_nodelay(true)?;
                server
                    .serve_connection(ProfiledTcpStream::new(server_io, server_counters))
                    .await
                    .map_err(std::io::Error::other)
            });
            let client = TcpStream::connect(address)
                .await
                .map_err(|error| error.to_string())?;
            client
                .set_nodelay(true)
                .map_err(|error| error.to_string())?;
            let histogram = exchange_pipeline(client, options, sequence_start, operations).await?;
            let result = serve.await.map_err(|error| error.to_string())?;
            result.map_err(|error| error.to_string())?;
            Ok((histogram, counters.snapshot()))
        }
    }
}

async fn exchange_pipeline<S>(
    mut client: S,
    options: &Options,
    sequence_start: u64,
    operations: u64,
) -> Result<Histogram<u64>, String>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut histogram =
        Histogram::new_with_bounds(1, 60_000_000, 3).map_err(|error| error.to_string())?;
    let batches = operations / options.pipeline;
    for batch in 0..batches {
        let mut request = Vec::new();
        let mut expected = Vec::new();
        for offset in 0..options.pipeline {
            let sequence = sequence_start + batch * options.pipeline + offset;
            let exchange = exchange_bytes(options, sequence);
            request.extend_from_slice(&exchange.0);
            expected.extend_from_slice(&exchange.1);
        }
        let began = Instant::now();
        client
            .write_all(&request)
            .await
            .map_err(|error| error.to_string())?;
        client.flush().await.map_err(|error| error.to_string())?;
        let mut actual = vec![0; expected.len()];
        client
            .read_exact(&mut actual)
            .await
            .map_err(|error| error.to_string())?;
        let elapsed_us = u64::try_from(began.elapsed().as_nanos().saturating_add(999) / 1_000)
            .unwrap_or(u64::MAX)
            .clamp(1, 60_000_000);
        histogram
            .record(elapsed_us)
            .map_err(|error| error.to_string())?;
        if actual != expected {
            return Err(format!(
                "response mismatch for {} at logical operation {}",
                options.operation.name(),
                sequence_start + batch * options.pipeline
            ));
        }
    }
    client.shutdown().await.map_err(|error| error.to_string())?;
    Ok(histogram)
}

fn exchange_bytes(options: &Options, sequence: u64) -> (Vec<u8>, Vec<u8>) {
    let key_count = if matches!(
        options.operation,
        Operation::Mget | Operation::Mset | Operation::Del | Operation::Exists
    ) {
        options.batch_size
    } else {
        1
    };
    let keys = (0..key_count)
        .map(|index| key(options, sequence, index))
        .collect::<Vec<_>>();
    let values = (0..key_count)
        .map(|index| value(options, sequence, index))
        .collect::<Vec<_>>();
    match options.operation {
        Operation::Get => (
            command(&[b"GET".as_slice(), keys[0].as_slice()]),
            bulk(&values[0]),
        ),
        Operation::Set => (
            command(&[b"SET".as_slice(), keys[0].as_slice(), values[0].as_slice()]),
            b"+OK\r\n".to_vec(),
        ),
        Operation::Mget => {
            let mut arguments = vec![b"MGET".as_slice()];
            arguments.extend(keys.iter().map(Vec::as_slice));
            let mut response = format!("*{}\r\n", values.len()).into_bytes();
            for value in &values {
                response.extend_from_slice(&bulk(value));
            }
            (command(&arguments), response)
        }
        Operation::Mset => {
            let mut arguments = vec![b"MSET".as_slice()];
            for (key, value) in keys.iter().zip(&values) {
                arguments.push(key);
                arguments.push(value);
            }
            (command(&arguments), b"+OK\r\n".to_vec())
        }
        Operation::Del => {
            let mut arguments = vec![b"DEL".as_slice()];
            arguments.extend(keys.iter().map(Vec::as_slice));
            (
                command(&arguments),
                format!(":{}\r\n", keys.len()).into_bytes(),
            )
        }
        Operation::Exists => {
            let mut arguments = vec![b"EXISTS".as_slice()];
            arguments.extend(keys.iter().map(Vec::as_slice));
            (
                command(&arguments),
                format!(":{}\r\n", keys.len()).into_bytes(),
            )
        }
    }
}

fn key(options: &Options, sequence: u64, batch_index: usize) -> Vec<u8> {
    let logical = logical_key(options, sequence, batch_index);
    format!("hc074:{:016x}:{logical:016x}", options.seed).into_bytes()
}

fn value(options: &Options, sequence: u64, batch_index: usize) -> Vec<u8> {
    let logical = logical_key(options, sequence, batch_index);
    let marker = options.seed.wrapping_add(logical).to_le_bytes();
    (0..options.payload_bytes)
        .map(|index| marker[index % marker.len()])
        .collect()
}

fn logical_key(options: &Options, sequence: u64, batch_index: usize) -> u64 {
    let width = if matches!(
        options.operation,
        Operation::Mget | Operation::Mset | Operation::Del | Operation::Exists
    ) {
        options.batch_size as u64
    } else {
        1
    };
    sequence
        .saturating_mul(width)
        .saturating_add(batch_index as u64)
        % options.key_space
}

fn command(arguments: &[&[u8]]) -> Vec<u8> {
    let mut output = format!("*{}\r\n", arguments.len()).into_bytes();
    for argument in arguments {
        output.extend_from_slice(format!("${}\r\n", argument.len()).as_bytes());
        output.extend_from_slice(argument);
        output.extend_from_slice(b"\r\n");
    }
    output
}

fn bulk(value: &[u8]) -> Vec<u8> {
    let mut output = format!("${}\r\n", value.len()).into_bytes();
    output.extend_from_slice(value);
    output.extend_from_slice(b"\r\n");
    output
}

fn workload_digest(options: &Options) -> String {
    let mut digest = Sha256::new();
    digest.update(PROFILE_ID.as_bytes());
    digest.update(options.operation.name().as_bytes());
    for value in [
        options.operations,
        options.warmup_operations,
        options.concurrency,
        options.pipeline,
        options.batch_size as u64,
        options.payload_bytes as u64,
        options.key_space,
        options.seed,
    ] {
        digest.update(value.to_le_bytes());
    }
    for sequence in 0..options.operations {
        let (request, expected) = exchange_bytes(options, sequence);
        digest.update((request.len() as u64).to_le_bytes());
        digest.update(request);
        digest.update((expected.len() as u64).to_le_bytes());
        digest.update(expected);
    }
    format!("sha256:{:x}", digest.finalize())
}

fn binary_digest() -> Result<String, Box<dyn Error>> {
    let executable = std::env::current_exe()?;
    let bytes = fs::read(executable)?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

#[derive(Clone, Copy)]
struct ResourceSnapshot {
    rss_bytes: u64,
    peak_rss_bytes: u64,
}

#[cfg(windows)]
fn process_resources() -> Result<ResourceSnapshot, Box<dyn Error>> {
    use std::mem::size_of;
    use windows_sys::Win32::System::ProcessStatus::{
        GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    let mut counters: PROCESS_MEMORY_COUNTERS = unsafe { std::mem::zeroed() };
    counters.cb = size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
    // SAFETY: `counters` is writable and advertises its exact initialized size.
    if unsafe { GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, counters.cb) } == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(ResourceSnapshot {
        rss_bytes: counters.WorkingSetSize as u64,
        peak_rss_bytes: counters.PeakWorkingSetSize as u64,
    })
}

#[cfg(unix)]
fn process_resources() -> Result<ResourceSnapshot, Box<dyn Error>> {
    let status = fs::read_to_string("/proc/self/status")?;
    let value = |name: &str| -> Result<u64, Box<dyn Error>> {
        let kib = status
            .lines()
            .find_map(|line| line.strip_prefix(name))
            .and_then(|value| value.split_whitespace().next())
            .ok_or_else(|| format!("{name} is unavailable"))?
            .parse::<u64>()?;
        Ok(kib * 1024)
    };
    Ok(ResourceSnapshot {
        rss_bytes: value("VmRSS:")?,
        peak_rss_bytes: value("VmHWM:")?,
    })
}

#[cfg(windows)]
fn process_cpu_seconds() -> Result<f64, Box<dyn Error>> {
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};

    let mut creation = FILETIME::default();
    let mut exit = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    // SAFETY: all FILETIME pointers refer to writable values for the current process.
    if unsafe {
        GetProcessTimes(
            GetCurrentProcess(),
            &mut creation,
            &mut exit,
            &mut kernel,
            &mut user,
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error().into());
    }
    let ticks =
        |value: FILETIME| ((value.dwHighDateTime as u64) << 32) | value.dwLowDateTime as u64;
    Ok((ticks(kernel) + ticks(user)) as f64 / 10_000_000.0)
}

#[cfg(unix)]
fn process_cpu_seconds() -> Result<f64, Box<dyn Error>> {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
    // SAFETY: getrusage initializes the supplied structure on success.
    if unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    // SAFETY: the successful call above initialized `usage`.
    let usage = unsafe { usage.assume_init() };
    let seconds = |time: libc::timeval| time.tv_sec as f64 + time.tv_usec as f64 / 1_000_000.0;
    Ok(seconds(usage.ru_utime) + seconds(usage.ru_stime))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(operation: Operation) -> Options {
        Options {
            source_commit: "0123456789abcdef0123456789abcdef01234567".to_owned(),
            operation,
            operations: 8,
            warmup_operations: 0,
            concurrency: 1,
            pipeline: 2,
            batch_size: 4,
            payload_bytes: 16,
            key_space: 32,
            seed: 74,
            instrumentation: true,
            transport: Transport::Duplex,
            output: None,
        }
    }

    #[test]
    fn workload_digest_changes_with_semantic_input() {
        let original = options(Operation::Get);
        let mut changed = original.clone();
        changed.seed += 1;
        assert_ne!(workload_digest(&original), workload_digest(&changed));
    }

    #[test]
    fn all_commands_have_complete_expected_responses() {
        for operation in [
            Operation::Get,
            Operation::Set,
            Operation::Mget,
            Operation::Mset,
            Operation::Del,
            Operation::Exists,
        ] {
            let (request, response) = exchange_bytes(&options(operation), 3);
            assert!(request.ends_with(b"\r\n"));
            assert!(response.ends_with(b"\r\n"));
        }
    }

    #[test]
    fn multi_key_del_requires_a_unique_preloaded_key_for_every_item() {
        let mut del = options(Operation::Del);
        del.batch_size = 4;
        del.key_space = del.operations * del.batch_size as u64;
        del.validate().unwrap();
        del.key_space -= 1;
        assert!(del.validate().is_err());
    }

    #[test]
    fn w9c_rusage_delta_is_measurement_local_and_saturating() {
        let before = W9cRusageSnapshot {
            user_cpu_microseconds: 10,
            system_cpu_microseconds: 20,
            minor_page_faults: 30,
            major_page_faults: 40,
            voluntary_context_switches: 50,
            involuntary_context_switches: 60,
        };
        let after = W9cRusageSnapshot {
            user_cpu_microseconds: 15,
            system_cpu_microseconds: 18,
            minor_page_faults: 37,
            major_page_faults: 44,
            voluntary_context_switches: 59,
            involuntary_context_switches: 72,
        };
        let delta = before.delta(after);
        assert_eq!(delta.user_cpu_microseconds, 5);
        assert_eq!(delta.system_cpu_microseconds, 0);
        assert_eq!(delta.minor_page_faults, 7);
        assert_eq!(delta.major_page_faults, 4);
        assert_eq!(delta.voluntary_context_switches, 9);
        assert_eq!(delta.involuntary_context_switches, 12);
    }

    #[tokio::test]
    async fn duplex_and_tcp_reconcile_the_same_commands_and_responses() {
        for transport in [Transport::Duplex, Transport::Tcp] {
            let mut options = options(Operation::Set);
            options.transport = transport;
            let state = Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap());
            let server =
                Arc::new(RedisRespServer::new(state, RedisListenerConfig::default()).unwrap());
            server.set_pipeline_instrumentation_enabled(true);
            let result = run_workload(Arc::clone(&server), &options, options.operations)
                .await
                .unwrap();
            assert_eq!(result.histogram.len(), 4);
            let metrics = server.pipeline_metrics();
            assert_eq!(metrics.decoded_commands, options.operations);
            assert_eq!(metrics.output_frames, options.operations);
            match transport {
                Transport::Duplex => assert!(!result.socket_io.available),
                Transport::Tcp => {
                    assert!(result.socket_io.available);
                    assert!(result.socket_io.poll_write_attempts > 0);
                    assert!(result.socket_io.poll_write_attempts <= options.operations);
                    assert!(result.socket_io.poll_write_ready > 0);
                    assert!(result.socket_io.poll_write_ready <= options.operations);
                    assert!(result.socket_io.written_bytes > 0);
                    assert!(
                        result.socket_io.requested_write_bytes >= result.socket_io.written_bytes
                    );
                }
            }
        }
    }
}
