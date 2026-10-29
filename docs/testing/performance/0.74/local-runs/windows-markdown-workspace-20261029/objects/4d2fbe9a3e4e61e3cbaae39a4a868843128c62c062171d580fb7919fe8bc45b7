//! Independent local controls. Timing builds contain no counting allocator.
use std::collections::BTreeMap;
use std::error::Error;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};

use bytes::Bytes;
use hdrhistogram::Histogram;
use hydracache::{CacheOptions, HydraCache};
use hydracache_client_protocol::{
    ClientRequest, ClientRequestEnvelope, ClientResponse, Namespace, StructuredKey,
};
use hydracache_client_transport_axum::{ClientIdentity, ClientSurfaceLimits, ClientSurfaceState};
use hydracache_redis_compat::{RedisListenerConfig, RedisRespServer};
use serde::Serialize;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[cfg(feature = "allocation-profile")]
#[path = "../../resp-scratch-screen-074/src/memory.rs"]
mod memory;
#[cfg(feature = "allocation-profile")]
#[global_allocator]
static ALLOCATOR: memory::Allocator = memory::Allocator;

const PROFILE: &str = "get-owner-local-controls-074-v1";
const SEED: u64 = 740074;
type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

#[derive(Clone, Debug, Serialize)]
struct Workload {
    surface: String,
    operation: String,
    payload: usize,
    concurrency: usize,
    pipeline: usize,
    operations: usize,
    warmup: usize,
    key_space: usize,
    seed: u64,
}
impl Workload {
    fn validate(&self) -> Result<()> {
        if !matches!(
            self.surface.as_str(),
            "embedded" | "client-surface" | "resp-tcp"
        ) || !matches!(self.operation.as_str(), "get" | "set")
            || self.seed != SEED
            || !(1..=1_048_576).contains(&self.payload)
            || ![1, 8].contains(&self.concurrency)
            || ![1, 50].contains(&self.pipeline)
            || (self.surface != "resp-tcp" && self.pipeline != 1)
            || !(1..=1_000_000).contains(&self.operations)
            || !(1..=100_000).contains(&self.warmup)
            || !(1..=64).contains(&self.key_space)
            || self.payload * self.key_space > 64 * 1024 * 1024
            || self.payload * self.pipeline > 16 * 1024 * 1024
            || !self
                .operations
                .is_multiple_of(self.concurrency * self.pipeline)
            || !self.warmup.is_multiple_of(self.concurrency * self.pipeline)
        {
            return Err("unsupported or out-of-budget controls workload".into());
        }
        Ok(())
    }
}

struct Options {
    workload: Workload,
    source: String,
    output: PathBuf,
}
impl Options {
    fn parse() -> Result<Self> {
        let mut values = BTreeMap::new();
        let mut args = std::env::args().skip(1);
        while let Some(key) = args.next() {
            let value = args.next().ok_or("missing argument value")?;
            if values.insert(key, value).is_some() {
                return Err("duplicate argument".into());
            }
        }
        let mut take = |key: &str| values.remove(key).ok_or_else(|| format!("missing {key}"));
        let options = Self {
            source: take("--source")?,
            output: take("--output")?.into(),
            workload: Workload {
                surface: take("--surface")?,
                operation: take("--operation")?,
                payload: take("--payload")?.parse()?,
                concurrency: take("--concurrency")?.parse()?,
                pipeline: take("--pipeline")?.parse()?,
                operations: take("--operations")?.parse()?,
                warmup: take("--warmup")?.parse()?,
                key_space: take("--key-space")?.parse()?,
                seed: take("--seed")?.parse()?,
            },
        };
        if !values.is_empty() {
            return Err("unknown argument".into());
        }
        options.workload.validate()?;
        Ok(options)
    }
}

fn command(parts: &[&[u8]]) -> Vec<u8> {
    let mut output = format!("*{}\r\n", parts.len()).into_bytes();
    for part in parts {
        output.extend_from_slice(format!("${}\r\n", part.len()).as_bytes());
        output.extend_from_slice(part);
        output.extend_from_slice(b"\r\n");
    }
    output
}

// Match the canonical RESP binary-key mapping without exposing a private
// product helper. The TCP semantic fixture verifies interoperability.
fn structured_key(key: &str) -> Result<StructuredKey> {
    let mut encoded = String::from("redis-binary-v1-");
    for byte in key.bytes() {
        use std::fmt::Write;
        write!(&mut encoded, "{byte:02x}")?;
    }
    Ok(StructuredKey::new(vec![encoded])?)
}

struct Entry {
    key: String,
    value: Vec<u8>,
    request: Vec<u8>,
    expected: Vec<u8>,
}
struct Context {
    corpus: Vec<Entry>,
    cache: HydraCache,
    state: Arc<ClientSurfaceState>,
    identity: ClientIdentity,
    namespace: Namespace,
    server: Arc<RedisRespServer>,
}
impl Context {
    async fn new(w: &Workload) -> Result<Self> {
        let mut corpus = Vec::new();
        for index in 0..w.key_space {
            let key = format!("hc074-owner-control:{SEED:016x}:{index:016x}");
            let marker = SEED.wrapping_add(index as u64).to_le_bytes();
            let value: Vec<u8> = (0..w.payload).map(|i| marker[i % 8]).collect();
            let (request, expected) = if w.operation == "get" {
                let mut expected = format!("${}\r\n", value.len()).into_bytes();
                expected.extend_from_slice(&value);
                expected.extend_from_slice(b"\r\n");
                (command(&[b"GET", key.as_bytes()]), expected)
            } else {
                (
                    command(&[b"SET", key.as_bytes(), &value]),
                    b"+OK\r\n".to_vec(),
                )
            };
            corpus.push(Entry {
                key,
                value,
                request,
                expected,
            });
        }
        let state = Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default())?);
        state.set_profile_instrumentation_enabled(false);
        let server = Arc::new(RedisRespServer::new(
            Arc::clone(&state),
            RedisListenerConfig {
                namespace: "default".to_owned(),
                client_id: "owner-controls".to_owned(),
                tenant: "owner-controls".to_owned(),
                ..RedisListenerConfig::default()
            },
        )?);
        server.set_pipeline_instrumentation_enabled(false);
        let context = Self {
            corpus,
            cache: HydraCache::local().max_capacity(128 * 1024 * 1024).build(),
            state,
            server,
            identity: ClientIdentity::new("owner-controls", "owner-controls")?,
            namespace: Namespace::new("default")?,
        };
        for entry in &context.corpus {
            context
                .cache
                .put_encoded(
                    &entry.key,
                    Bytes::copy_from_slice(&entry.value),
                    CacheOptions::new(),
                )
                .await?;
            let result = context.dispatch(ClientRequest::Put {
                ns: context.namespace.clone(),
                key: structured_key(&entry.key)?,
                value: entry.value.clone(),
                ttl_ms: None,
                dimensions: Vec::new(),
            })?;
            if result != ClientResponse::Stored {
                return Err("preload not stored".into());
            }
        }
        Ok(context)
    }
    fn dispatch(&self, request: ClientRequest) -> Result<ClientResponse> {
        self.state
            .dispatch_verified_request(
                &self.identity,
                ClientRequestEnvelope::new("owner-controls", request),
            )
            .result
            .map_err(|error| format!("dispatch error: {error:?}").into())
    }
    async fn operation(&self, w: &Workload, index: usize) -> Result<()> {
        let entry = &self.corpus[index % self.corpus.len()];
        if w.surface == "embedded" {
            if w.operation == "get" {
                let actual = self.cache.get_encoded(&entry.key).await?;
                if actual.as_deref() != Some(entry.value.as_slice()) {
                    return Err("embedded value mismatch".into());
                }
            } else {
                self.cache
                    .put_encoded(
                        &entry.key,
                        Bytes::copy_from_slice(&entry.value),
                        CacheOptions::new(),
                    )
                    .await?;
            }
        } else {
            let key = structured_key(&entry.key)?;
            let request = if w.operation == "get" {
                ClientRequest::Get {
                    ns: self.namespace.clone(),
                    key,
                }
            } else {
                ClientRequest::Put {
                    ns: self.namespace.clone(),
                    key,
                    value: entry.value.clone(),
                    ttl_ms: None,
                    dimensions: Vec::new(),
                }
            };
            let actual = self.dispatch(request)?;
            let valid = match actual {
                ClientResponse::Value { value: Some(value) } => {
                    w.operation == "get" && value == entry.value
                }
                ClientResponse::Stored => w.operation == "set",
                _ => false,
            };
            if !valid {
                return Err("client-surface response mismatch".into());
            }
        }
        Ok(())
    }
    fn digest(&self, w: &Workload) -> Result<String> {
        let mut digest = Sha256::new();
        digest.update(PROFILE);
        digest.update(serde_json::to_vec(w)?);
        for entry in &self.corpus {
            for part in [
                entry.key.as_bytes(),
                &entry.value,
                &entry.request,
                &entry.expected,
            ] {
                digest.update((part.len() as u64).to_le_bytes());
                digest.update(part);
            }
        }
        Ok(format!("sha256:{:x}", digest.finalize()))
    }
}

fn histogram() -> Result<Histogram<u64>> {
    Ok(Histogram::new_with_bounds(1, 60_000_000, 3)?)
}
fn micros(start: Instant) -> u64 {
    start.elapsed().as_micros().clamp(1, 60_000_000) as u64
}

async fn tcp_client(
    context: Arc<Context>,
    w: &Workload,
    client: usize,
    count: usize,
) -> Result<Histogram<u64>> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let server = Arc::clone(&context.server);
    let serve = tokio::spawn(async move {
        let (stream, _) = listener.accept().await?;
        stream.set_nodelay(true)?;
        server
            .serve_connection(stream)
            .await
            .map_err(std::io::Error::other)
    });
    let mut stream = TcpStream::connect(addr).await?;
    stream.set_nodelay(true)?;
    let mut hist = histogram()?;
    let mut request = Vec::new();
    let mut expected = Vec::new();
    let mut actual = Vec::new();
    for batch in 0..count / w.pipeline {
        request.clear();
        expected.clear();
        for offset in 0..w.pipeline {
            let index = (client * count + batch * w.pipeline + offset) % w.key_space;
            request.extend_from_slice(&context.corpus[index].request);
            expected.extend_from_slice(&context.corpus[index].expected);
        }
        actual.resize(expected.len(), 0);
        let started = Instant::now();
        stream.write_all(&request).await?;
        stream.flush().await?;
        stream.read_exact(&mut actual).await?;
        hist.record(micros(started))?;
        if actual != expected {
            return Err("RESP exact bytes mismatch".into());
        }
    }
    stream.shutdown().await?;
    serve.await??;
    Ok(hist)
}

async fn workload(context: Arc<Context>, w: &Workload, count: usize) -> Result<Histogram<u64>> {
    let mut tasks = Vec::new();
    for client in 0..w.concurrency {
        let context = Arc::clone(&context);
        let w = w.clone();
        tasks.push(tokio::spawn(async move {
            let per_client = count / w.concurrency;
            if w.surface == "resp-tcp" {
                return tcp_client(context, &w, client, per_client).await;
            }
            let mut hist = histogram()?;
            for index in 0..per_client {
                let started = Instant::now();
                context.operation(&w, client * per_client + index).await?;
                hist.record(micros(started))?;
                // ClientSurfaceState dispatch is synchronous. Yield prevents one
                // logical client from monopolizing a worker across the entire run.
                if index % 64 == 63 {
                    tokio::task::yield_now().await;
                }
            }
            Ok(hist)
        }));
    }
    let mut merged = histogram()?;
    for task in tasks {
        merged.add(task.await??)?;
    }
    Ok(merged)
}

#[cfg(windows)]
fn cpu_seconds() -> Result<f64> {
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};
    let mut creation: FILETIME = unsafe { std::mem::zeroed() };
    let mut exit = creation;
    let mut kernel = creation;
    let mut user = creation;
    // SAFETY: pseudo process handle and four initialized writable FILETIMEs.
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
    let ticks = |time: FILETIME| ((time.dwHighDateTime as u64) << 32) | time.dwLowDateTime as u64;
    Ok((ticks(kernel) + ticks(user)) as f64 / 10_000_000.0)
}
#[cfg(not(windows))]
fn cpu_seconds() -> Result<f64> {
    Err("this local controls contract requires Windows process CPU accounting".into())
}

fn git(args: &[&str]) -> Result<String> {
    let output = Command::new("git").args(args).output()?;
    if !output.status.success() {
        return Err("Git check failed".into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

async fn placement_gate() -> Result<()> {
    let ready = std::env::var_os("HYDRACACHE_GET_OWNER_CONTROL_READY");
    let go = std::env::var_os("HYDRACACHE_GET_OWNER_CONTROL_GO");
    match (ready, go) {
        (Some(ready), Some(go)) => {
            let go = PathBuf::from(go);
            if go.exists() {
                return Err("stale placement GO file".into());
            }
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(ready)?;
            let started = Instant::now();
            while !go.is_file() {
                if started.elapsed() > Duration::from_secs(30) {
                    return Err("placement gate timeout".into());
                }
                tokio::time::sleep(Duration::from_millis(2)).await;
            }
            Ok(())
        }
        (None, None) => Err("sealed runner placement gate required".into()),
        _ => Err("both placement gate paths are required".into()),
    }
}

#[derive(Serialize)]
struct Receipt {
    profile_id: &'static str,
    source_commit: String,
    source_clean: bool,
    binary_sha256: String,
    get_owner_enabled: bool,
    allocation_profile_enabled: bool,
    instrumentation_enabled: bool,
    promotable: bool,
    exact_result_validation: bool,
    exact_final_values: bool,
    workload: Workload,
    workload_sha256: String,
    runtime_workers: usize,
    latency_unit: &'static str,
    latency_samples: u64,
    p50_us: u64,
    p95_us: u64,
    p99_us: u64,
    elapsed_seconds: f64,
    cpu_seconds: f64,
    cpu_nanoseconds_per_operation: f64,
    goodput_operations_per_second: f64,
    gross_allocated_bytes_per_operation: Option<f64>,
    // Requested-layout counts only. Never allocator active/resident/retained.
    #[cfg(feature = "allocation-profile")]
    allocation: memory::Measurement,
}

#[tokio::main(worker_threads = 2)]
async fn main() -> Result<()> {
    let options = Options::parse()?;
    if options.source != env!("SCREEN_SOURCE_SHA")
        || git(&["rev-parse", "HEAD"])? != options.source
        || !git(&["status", "--porcelain"])?.is_empty()
    {
        return Err("compiled/source/clean identity mismatch".into());
    }
    // Reserve the exact receipt before any work, never overwrite a prior attempt.
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&options.output)?;
    let w = &options.workload;
    let context = Arc::new(Context::new(w).await?);
    let trace = context.digest(w)?;
    placement_gate().await?;
    workload(Arc::clone(&context), w, w.warmup).await?;
    #[cfg(feature = "allocation-profile")]
    let scope = memory::Scope::start();
    let cpu_before = cpu_seconds()?;
    let started = Instant::now();
    let hist = workload(Arc::clone(&context), w, w.operations).await?;
    let elapsed = started.elapsed().as_secs_f64();
    let cpu = cpu_seconds()? - cpu_before;
    #[cfg(feature = "allocation-profile")]
    let allocation = scope.finish();
    // Both independent stores must retain exactly the preloaded values. No
    // expiration/authorization/atomicity claim follows from this no-TTL cohort.
    for index in 0..w.key_space {
        let mut verify = w.clone();
        verify.operation = "get".to_owned();
        verify.surface = "embedded".to_owned();
        context.operation(&verify, index).await?;
        verify.surface = "client-surface".to_owned();
        context.operation(&verify, index).await?;
    }
    let receipt = Receipt {
        profile_id: PROFILE,
        source_commit: options.source,
        source_clean: true,
        binary_sha256: format!(
            "sha256:{:x}",
            Sha256::digest(std::fs::read(std::env::current_exe()?)?)
        ),
        get_owner_enabled: cfg!(feature = "get-owner"),
        allocation_profile_enabled: cfg!(feature = "allocation-profile"),
        instrumentation_enabled: false,
        promotable: false,
        exact_result_validation: true,
        exact_final_values: true,
        workload: w.clone(),
        workload_sha256: trace,
        runtime_workers: 2,
        latency_unit: if w.pipeline == 1 {
            "closed_loop_operation"
        } else {
            "closed_loop_pipeline_batch"
        },
        latency_samples: hist.len(),
        p50_us: hist.value_at_quantile(0.50),
        p95_us: hist.value_at_quantile(0.95),
        p99_us: hist.value_at_quantile(0.99),
        elapsed_seconds: elapsed,
        cpu_seconds: cpu,
        cpu_nanoseconds_per_operation: cpu * 1e9 / w.operations as f64,
        goodput_operations_per_second: w.operations as f64 / elapsed,
        gross_allocated_bytes_per_operation: {
            #[cfg(feature = "allocation-profile")]
            {
                Some(allocation.gross_allocated_bytes as f64 / w.operations as f64)
            }
            #[cfg(not(feature = "allocation-profile"))]
            {
                None
            }
        },
        #[cfg(feature = "allocation-profile")]
        allocation,
    };
    serde_json::to_writer_pretty(&mut output, &receipt)?;
    output.write_all(b"\n")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn cell(surface: &str, operation: &str) -> Workload {
        Workload {
            surface: surface.to_owned(),
            operation: operation.to_owned(),
            payload: 4096,
            concurrency: 8,
            pipeline: 1,
            operations: 80,
            warmup: 8,
            key_space: 8,
            seed: SEED,
        }
    }
    #[test]
    fn bounds_reject_unsupported_or_ambiguous_work() {
        let mut w = cell("embedded", "get");
        w.validate().unwrap();
        w.pipeline = 50;
        assert!(w.validate().is_err());
        w.pipeline = 1;
        w.operations = 81;
        assert!(w.validate().is_err());
        w.operations = 80;
        w.seed += 1;
        assert!(w.validate().is_err());
        w.seed = SEED;
        w.payload = usize::MAX;
        assert!(w.validate().is_err());
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn exact_native_and_unwrapped_tcp_semantics() {
        for surface in ["embedded", "client-surface", "resp-tcp"] {
            for operation in ["get", "set"] {
                let mut w = cell(surface, operation);
                if surface == "resp-tcp" {
                    w.pipeline = 50;
                    w.operations = 400;
                    w.warmup = 400;
                }
                w.validate().unwrap();
                let context = Arc::new(Context::new(&w).await.unwrap());
                let hist = workload(Arc::clone(&context), &w, w.operations)
                    .await
                    .unwrap();
                assert_eq!(hist.len(), (w.operations / w.pipeline) as u64);
                for index in 0..w.key_space {
                    let mut verify = cell("client-surface", "get");
                    verify.key_space = w.key_space;
                    context.operation(&verify, index).await.unwrap();
                }
                assert_eq!(
                    context.state.retained_state_for_diagnostics().store_entries,
                    w.key_space
                );
                assert_eq!(context.state.profile_metrics().dispatches, 0);
            }
        }
    }
    #[tokio::test]
    async fn trace_binds_all_workload_fields_and_not_build_flags() {
        let w = cell("client-surface", "get");
        let context = Context::new(&w).await.unwrap();
        let trace = context.digest(&w).unwrap();
        let mut changed = w.clone();
        changed.operations *= 2;
        assert_ne!(trace, context.digest(&changed).unwrap());
        changed = w.clone();
        changed.operation = "set".to_owned();
        assert_ne!(trace, context.digest(&changed).unwrap());
        assert_eq!(trace, context.digest(&w).unwrap());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn p1_tcp_validates_small_and_large_payload_without_batch_latency() {
        for payload in [256, 1_048_576] {
            let mut w = cell("resp-tcp", "get");
            w.payload = payload;
            w.concurrency = 1;
            w.operations = 4;
            w.warmup = 1;
            w.key_space = 2;
            w.validate().unwrap();
            let context = Arc::new(Context::new(&w).await.unwrap());
            assert_eq!(workload(context, &w, 4).await.unwrap().len(), 4);
        }
    }
}
