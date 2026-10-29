use std::collections::BTreeMap;
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::Command;
use std::sync::Arc;
use std::task::{Context, Poll};

use hydracache_client_transport_axum::{ClientSurfaceLimits, ClientSurfaceState};
use hydracache_redis_compat::{RedisCommand, RedisListenerConfig, RedisRespServer, RespValue};
use serde::Serialize;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

#[path = "../../resp-scratch-screen-074/src/memory.rs"]
mod memory;
#[global_allocator]
static ALLOCATOR: memory::Allocator = memory::Allocator;
const PROFILE_ID: &str = "get-response-owner-allocation-memory-screen-074-v1";
const SEED: u64 = 740074;

#[derive(Clone, Debug)]
struct Options {
    source: String,
    operation: String,
    payload: usize,
    alternate_payload: usize,
    pipeline: usize,
    batches: usize,
    warmup_batches: usize,
    read_chunk: usize,
    output: PathBuf,
}
impl Options {
    fn parse() -> Result<Self, Box<dyn Error>> {
        let mut values = BTreeMap::new();
        let mut args = std::env::args().skip(1);
        while let Some(name) = args.next() {
            let value = args.next().ok_or("missing argument value")?;
            if values.insert(name, value).is_some() {
                return Err("duplicate argument".into());
            }
        }
        let mut take = |key: &str| values.remove(key).ok_or_else(|| format!("missing {key}"));
        let options = Self {
            source: take("--source")?,
            operation: take("--operation")?,
            payload: take("--payload")?.parse()?,
            alternate_payload: take("--alternate-payload")?.parse()?,
            pipeline: take("--pipeline")?.parse()?,
            batches: take("--batches")?.parse()?,
            warmup_batches: take("--warmup-batches")?.parse()?,
            read_chunk: take("--read-chunk")?.parse()?,
            output: PathBuf::from(take("--output")?),
        };
        if !values.is_empty() {
            return Err("unknown argument".into());
        }
        options.validate()?;
        Ok(options)
    }
    fn validate(&self) -> Result<(), Box<dyn Error>> {
        if !matches!(self.operation.as_str(), "get" | "get-miss" | "set")
            || !(0..=1_048_576).contains(&self.payload)
            || !(0..=1_048_576).contains(&self.alternate_payload)
            || !(1..=50).contains(&self.pipeline)
            || !(1..=2000).contains(&self.batches)
            || !(1..=100).contains(&self.warmup_batches)
            || !(1..=8192).contains(&self.read_chunk)
            || self.payload.max(self.alternate_payload) * self.pipeline > 16 * 1024 * 1024
        {
            return Err("unsupported or out-of-budget local screen workload".into());
        }
        Ok(())
    }
}

fn command(parts: &[&[u8]]) -> Vec<u8> {
    let mut bytes = format!("*{}\r\n", parts.len()).into_bytes();
    for part in parts {
        bytes.extend_from_slice(format!("${}\r\n", part.len()).as_bytes());
        bytes.extend_from_slice(part);
        bytes.extend_from_slice(b"\r\n");
    }
    bytes
}
fn payload(size: usize, key: usize) -> Vec<u8> {
    let marker = SEED.wrapping_add(key as u64).to_le_bytes();
    (0..size)
        .map(|index| marker[index % marker.len()])
        .collect()
}
fn key(index: usize) -> Vec<u8> {
    format!("hc074-get-owner:{SEED:016x}:{index:016x}").into_bytes()
}
struct Corpus {
    request: Vec<u8>,
    expected: Vec<u8>,
    values: Vec<Vec<u8>>,
    keys: Vec<Vec<u8>>,
}
impl Corpus {
    fn build(options: &Options) -> Self {
        let mut corpus = Self {
            request: Vec::new(),
            expected: Vec::new(),
            keys: Vec::new(),
            values: Vec::new(),
        };
        for index in 0..options.pipeline {
            let size = if index % 2 == 0 {
                options.payload
            } else {
                options.alternate_payload
            };
            let key = key(index);
            let value = payload(size, index);
            if options.operation == "get" || options.operation == "get-miss" {
                corpus.request.extend(command(&[b"GET", &key]));
                if options.operation == "get-miss" {
                    corpus.expected.extend_from_slice(b"$-1\r\n");
                } else {
                    corpus
                        .expected
                        .extend(format!("${}\r\n", value.len()).as_bytes());
                    corpus.expected.extend_from_slice(&value);
                    corpus.expected.extend_from_slice(b"\r\n");
                }
            } else {
                corpus.request.extend(command(&[b"SET", &key, &value]));
                corpus.expected.extend_from_slice(b"+OK\r\n");
            }
            corpus.keys.push(key);
            corpus.values.push(value);
        }
        corpus
    }
    fn digest(&self, options: &Options) -> String {
        let mut digest = Sha256::new();
        digest.update(PROFILE_ID);
        for value in [
            SEED,
            options.pipeline as u64,
            options.batches as u64,
            options.warmup_batches as u64,
            options.read_chunk as u64,
        ] {
            digest.update(value.to_le_bytes());
        }
        for bytes in [&self.request, &self.expected] {
            digest.update((bytes.len() as u64).to_le_bytes());
            digest.update(bytes);
        }
        format!("sha256:{:x}", digest.finalize())
    }
}

// The oracle and corpus are immutable, prepared before counting. No output
// collection, hashing, formatting or sampling allocation occurs in IO callbacks.
struct Stream<'a> {
    corpus: &'a Corpus,
    batches: usize,
    read_chunk: usize,
    read_batch: usize,
    read_offset: usize,
    write_batch: usize,
    write_offset: usize,
    writes: u64,
    flushes: u64,
    idle_samples: u64,
    idle_min: u64,
    idle_max: u64,
}
impl<'a> Stream<'a> {
    fn new(corpus: &'a Corpus, batches: usize, read_chunk: usize) -> Self {
        Self {
            corpus,
            batches,
            read_chunk,
            read_batch: 0,
            read_offset: 0,
            write_batch: 0,
            write_offset: 0,
            writes: 0,
            flushes: 0,
            idle_samples: 0,
            idle_min: u64::MAX,
            idle_max: 0,
        }
    }
    fn validate(&self, pipeline: usize) -> Result<(), Box<dyn Error>> {
        if self.read_batch != self.batches
            || self.write_batch != self.batches
            || self.write_offset != 0
            || self.flushes != (pipeline * self.batches) as u64
        {
            return Err("incomplete/extra response, read or flush accounting".into());
        }
        Ok(())
    }
}
impl AsyncRead for Stream<'_> {
    fn poll_read(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let this = self.get_mut();
        if this.read_offset == 0 && this.read_batch > 0 {
            if this.write_batch != this.read_batch || this.write_offset != 0 {
                return Poll::Ready(Err(std::io::Error::other(
                    "next batch read before exact previous response",
                )));
            }
            let live = memory::live();
            this.idle_samples += 1;
            this.idle_min = this.idle_min.min(live);
            this.idle_max = this.idle_max.max(live);
        }
        if this.read_batch == this.batches {
            return Poll::Ready(Ok(()));
        }
        let end = (this.read_offset + buffer.remaining().min(this.read_chunk))
            .min(this.corpus.request.len());
        buffer.put_slice(&this.corpus.request[this.read_offset..end]);
        this.read_offset = end;
        if end == this.corpus.request.len() {
            this.read_batch += 1;
            this.read_offset = 0;
        }
        Poll::Ready(Ok(()))
    }
}
impl AsyncWrite for Stream<'_> {
    fn poll_write(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let this = self.get_mut();
        let end = this.write_offset + bytes.len();
        if this.write_batch >= this.batches
            || end > this.corpus.expected.len()
            || this.corpus.expected[this.write_offset..end] != *bytes
        {
            return Poll::Ready(Err(std::io::Error::other("unexpected response bytes")));
        }
        this.writes += 1;
        this.write_offset = end;
        if end == this.corpus.expected.len() {
            this.write_batch += 1;
            this.write_offset = 0;
        }
        Poll::Ready(Ok(bytes.len()))
    }
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.get_mut().flushes += 1;
        Poll::Ready(Ok(()))
    }
    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[derive(Serialize)]
struct Rss {
    current_bytes: u64,
    lifetime_peak_bytes: u64,
}
#[cfg(windows)]
fn rss() -> Result<Rss, Box<dyn Error>> {
    use windows_sys::Win32::System::{
        ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS},
        Threading::GetCurrentProcess,
    };
    let mut counters = PROCESS_MEMORY_COUNTERS {
        cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        ..Default::default()
    };
    // SAFETY: writable initialized structure and current-process pseudo handle.
    if unsafe { GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, counters.cb) } == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(Rss {
        current_bytes: counters.WorkingSetSize as u64,
        lifetime_peak_bytes: counters.PeakWorkingSetSize as u64,
    })
}
#[cfg(not(windows))]
fn rss() -> Result<Rss, Box<dyn Error>> {
    let status = fs::read_to_string("/proc/self/status")?;
    let field = |name: &str| -> Result<u64, Box<dyn Error>> {
        Ok(status
            .lines()
            .find_map(|line| line.strip_prefix(name))
            .ok_or("RSS field missing")?
            .split_whitespace()
            .next()
            .ok_or("RSS empty")?
            .parse::<u64>()?
            * 1024)
    };
    Ok(Rss {
        current_bytes: field("VmRSS:")?,
        lifetime_peak_bytes: field("VmHWM:")?,
    })
}
fn git(root: &Path, args: &[&str]) -> Result<String, Box<dyn Error>> {
    let output = Command::new("git").current_dir(root).args(args).output()?;
    if !output.status.success() {
        return Err("Git identity check failed".into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

#[derive(Serialize)]
struct Receipt {
    schema_version: u32,
    profile_id: &'static str,
    tier: &'static str,
    promotable: bool,
    source_commit: String,
    source_clean: bool,
    binary_sha256: String,
    get_owner_enabled: bool,
    allocator: &'static str,
    operation: String,
    payload_bytes: usize,
    alternate_payload_bytes: usize,
    pipeline: usize,
    batches: usize,
    operations: usize,
    warmup_batches: usize,
    read_chunk_bytes: usize,
    seed: u64,
    concurrency: u64,
    workload_sha256: String,
    exact_response_validation: bool,
    exact_final_values_and_cardinality: bool,
    measured_dispatches: u64,
    measured_mutations: u64,
    measured_errors: u64,
    allocation: memory::Measurement,
    gross_allocated_bytes_per_operation: f64,
    allocation_calls_per_operation: f64,
    next_read_boundary_live_min_bytes: u64,
    next_read_boundary_live_max_bytes: u64,
    next_read_boundary_samples: u64,
    write_calls: u64,
    flush_calls: u64,
    rss_before: Rss,
    rss_after_close: Rss,
    limitations: [&'static str; 7],
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let options = Options::parse()?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    if options.source != env!("SCREEN_SOURCE_SHA")
        || options.source != git(&root, &["rev-parse", "HEAD"])?
        || !git(
            &root,
            &["status", "--porcelain", "--untracked-files=normal"],
        )?
        .is_empty()
    {
        return Err("binary/source identity mismatch or dirty source; no sample executed".into());
    }
    if options.output.exists() {
        return Err("receipt must not pre-exist".into());
    }
    let corpus = Corpus::build(&options);
    let state = Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default())?);
    let server = RedisRespServer::new(Arc::clone(&state), RedisListenerConfig::default())?;
    for (key, value) in corpus.keys.iter().zip(&corpus.values) {
        if options.operation == "get-miss" {
            continue;
        }
        if server.execute_command(RedisCommand::Set {
            key: key.clone(),
            value: value.clone(),
            options: Vec::new(),
        }) != RespValue::SimpleString("OK")
        {
            return Err("preload failed".into());
        }
    }
    let mut warm = Stream::new(&corpus, options.warmup_batches, options.read_chunk);
    server.serve_connection(&mut warm).await?;
    warm.validate(options.pipeline)?;
    // All corpus/oracle allocations and warm-up are outside the epoch. Endpoint
    // decoding, dispatch, reduction, encoding, writes/flushes and close are inside.
    let mut stream = Stream::new(&corpus, options.batches, options.read_chunk);
    let rss_before = rss()?;
    let dispatches_before = state.dispatch_attempts();
    let mutations_before = state.state_mutations();
    let errors_before = server.metrics().errors;
    let scope = memory::Scope::start();
    let result = server.serve_connection(&mut stream).await;
    let allocation = scope.finish();
    result?;
    stream.validate(options.pipeline)?;
    let measured_dispatches = state.dispatch_attempts() - dispatches_before;
    let measured_mutations = state.state_mutations() - mutations_before;
    let measured_errors = server.metrics().errors - errors_before;
    let operations = options.pipeline * options.batches;
    let expected_mutations = if options.operation == "set" {
        operations as u64
    } else {
        0
    };
    if measured_dispatches != operations as u64
        || measured_mutations != expected_mutations
        || measured_errors != 0
    {
        return Err("dispatch or mutation count mismatch".into());
    }
    let rss_after_close = rss()?;
    for (key, value) in corpus.keys.iter().zip(&corpus.values) {
        let expected = if options.operation == "get-miss" {
            RespValue::Null
        } else {
            RespValue::BulkString(value.clone())
        };
        if server.execute_command(RedisCommand::Get { key: key.clone() }) != expected {
            return Err("final value mismatch".into());
        }
    }
    let expected_entries = if options.operation == "get-miss" {
        0
    } else {
        options.pipeline
    };
    if state.retained_state_for_diagnostics().store_entries != expected_entries {
        return Err("final cardinality mismatch".into());
    }
    let workload_sha256 = corpus.digest(&options);
    let receipt = Receipt {
        schema_version: 1, profile_id: PROFILE_ID, tier: "local-d3a-allocation-memory-screen", promotable: false,
        source_commit: options.source.clone(), source_clean: true,
        binary_sha256: format!("sha256:{:x}", Sha256::digest(fs::read(std::env::current_exe()?)?)),
        get_owner_enabled: cfg!(feature = "get-owner"), allocator: "System-with-tool-only-requested-layout-accounting",
        operation: options.operation, payload_bytes: options.payload, alternate_payload_bytes: options.alternate_payload,
        pipeline: options.pipeline, batches: options.batches, operations, warmup_batches: options.warmup_batches,
        read_chunk_bytes: options.read_chunk, seed: SEED, concurrency: 1,
        workload_sha256, exact_response_validation: true, exact_final_values_and_cardinality: true,
        measured_dispatches, measured_mutations, measured_errors,
        gross_allocated_bytes_per_operation: allocation.gross_allocated_bytes as f64 / operations as f64,
        allocation_calls_per_operation: allocation.successful_allocation_calls as f64 / operations as f64,
        allocation,
        next_read_boundary_live_min_bytes: stream.idle_min, next_read_boundary_live_max_bytes: stream.idle_max,
        next_read_boundary_samples: stream.idle_samples, write_calls: stream.writes, flush_calls: stream.flushes,
        rss_before, rss_after_close,
        limitations: ["profiling allocator overhead is not CPU/goodput/p99 evidence", "scripted plaintext RESP2 IO, not real socket or syscall evidence", "live requested layouts are not allocator active/resident/retained bytes", "realloc hidden overlap and allocator metadata are not counted", "RSS peak is lifetime not window; endpoint snapshots do not establish retention", "next-read checkpoint is owner lifetime, not a timed idle/RSS soak", "native HC1 HC2 embedded ClientSurfaceState and secure/concurrent cohorts still require separate D3 controls"],
    };
    let bytes = serde_json::to_vec_pretty(&receipt)?;
    if let Some(parent) = options.output.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut output = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&options.output)?;
    output.write_all(&bytes)?;
    output.sync_all()?;
    println!(
        "{} {} operations; gross={:.3} bytes/op; peak-above-start={} bytes",
        receipt.operation,
        operations,
        receipt.gross_allocated_bytes_per_operation,
        receipt.allocation.peak_live_above_start_bytes
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn options() -> Options {
        Options {
            source: "test".into(),
            operation: "get".into(),
            payload: 4096,
            alternate_payload: 4096,
            pipeline: 10,
            batches: 2,
            warmup_batches: 1,
            read_chunk: 8192,
            output: PathBuf::new(),
        }
    }
    #[test]
    fn workload_identity_is_variant_independent_and_covers_every_semantic_input() {
        let original = options();
        let digest = Corpus::build(&original).digest(&original);
        for field in 0..6 {
            let mut changed = original.clone();
            match field {
                0 => changed.payload += 1,
                1 => changed.pipeline += 1,
                2 => changed.batches += 1,
                3 => changed.warmup_batches += 1,
                4 => changed.read_chunk -= 1,
                _ => changed.alternate_payload = 256,
            }
            assert_ne!(digest, Corpus::build(&changed).digest(&changed));
        }
    }
    #[test]
    fn screen_budget_and_operation_bounds_fail_loudly() {
        let mut options = options();
        options.validate().unwrap();
        options.batches = 0;
        assert!(options.validate().is_err());
        options.batches = 2001;
        assert!(options.validate().is_err());
        options.batches = 1;
        options.payload = 1048576;
        options.pipeline = 50;
        assert!(options.validate().is_err());
    }
    #[tokio::test(flavor = "current_thread")]
    async fn scripted_oracle_checks_fragmented_reads_mixed_payloads_and_final_flush_count() {
        for operation in ["get", "set"] {
            for read_chunk in [1, 17, 8192] {
                let mut options = options();
                options.operation = operation.into();
                options.alternate_payload = 256;
                let corpus = Corpus::build(&options);
                let state =
                    Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap());
                let server =
                    RedisRespServer::new(Arc::clone(&state), RedisListenerConfig::default())
                        .unwrap();
                for (key, value) in corpus.keys.iter().zip(&corpus.values) {
                    assert_eq!(
                        server.execute_command(RedisCommand::Set {
                            key: key.clone(),
                            value: value.clone(),
                            options: vec![]
                        }),
                        RespValue::SimpleString("OK")
                    );
                }
                let mut stream = Stream::new(&corpus, 2, read_chunk);
                let dispatches_before = state.dispatch_attempts();
                let mutations_before = state.state_mutations();
                server.serve_connection(&mut stream).await.unwrap();
                stream.validate(options.pipeline).unwrap();
                assert_eq!(state.dispatch_attempts() - dispatches_before, 20);
                assert_eq!(
                    state.state_mutations() - mutations_before,
                    if operation == "set" { 20 } else { 0 }
                );
                assert_eq!(server.metrics().errors, 0);
                assert_eq!(stream.writes, 20);
                assert_eq!(stream.flushes, 20);
                assert_eq!(stream.idle_samples, 2);
            }
        }
    }
    #[tokio::test(flavor = "current_thread")]
    async fn empty_and_missing_oracles_have_exact_null_and_empty_shapes() {
        for operation in ["get", "get-miss"] {
            let mut options = options();
            options.operation = operation.into();
            options.payload = 0;
            options.alternate_payload = 0;
            let corpus = Corpus::build(&options);
            let state = Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap());
            let server = RedisRespServer::new(state, RedisListenerConfig::default()).unwrap();
            if operation == "get" {
                for key in &corpus.keys {
                    assert_eq!(
                        server.execute_command(RedisCommand::Set {
                            key: key.clone(),
                            value: vec![],
                            options: vec![]
                        }),
                        RespValue::SimpleString("OK")
                    );
                }
            }
            let mut stream = Stream::new(&corpus, options.batches, options.read_chunk);
            server.serve_connection(&mut stream).await.unwrap();
            stream.validate(options.pipeline).unwrap();
            assert_eq!(stream.writes, 20);
        }
    }
    #[tokio::test(flavor = "current_thread")]
    async fn oracle_rejects_missing_hit_instead_of_counting_failed_work() {
        let options = options();
        let corpus = Corpus::build(&options);
        let state = Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap());
        let server = RedisRespServer::new(state, RedisListenerConfig::default()).unwrap();
        assert!(server
            .serve_connection(Stream::new(&corpus, 1, 8192))
            .await
            .is_err());
    }
}
