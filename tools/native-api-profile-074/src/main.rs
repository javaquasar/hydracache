use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use bytes::Bytes;
use hdrhistogram::Histogram;
use hydracache::{CacheOptions, HydraCache};
use hydracache_client_protocol::{
    ClientRequest, ClientRequestEnvelope, ClientResponse, Namespace, StructuredKey,
};
use hydracache_client_transport_axum::{
    ClientIdentity, ClientSurfaceLimits, ClientSurfaceProfileMetrics, ClientSurfaceRetainedState,
    ClientSurfaceState,
};
use hydracache_loadgen::allocation::{measure_allocations, AllocationMeasurement};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const PROFILE_ID: &str = "w1-w6-native-api-profile-074-v2";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ApiPath {
    RawEmbedded,
    TypedEmbedded,
    ClientSurface,
    GetOrInsert,
}

impl ApiPath {
    fn parse(value: &str) -> Result<Self, Box<dyn Error>> {
        match value {
            "raw-embedded" => Ok(Self::RawEmbedded),
            "typed-embedded" => Ok(Self::TypedEmbedded),
            "client-surface" => Ok(Self::ClientSurface),
            "get-or-insert" => Ok(Self::GetOrInsert),
            _ => Err(format!("unsupported surface {value}").into()),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::RawEmbedded => "raw-embedded",
            Self::TypedEmbedded => "typed-embedded",
            Self::ClientSurface => "client-surface",
            Self::GetOrInsert => "get-or-insert",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Operation {
    Get,
    ExpiredGet,
    Put,
    Hit,
    SingleFlight,
}

impl Operation {
    fn parse(value: &str) -> Result<Self, Box<dyn Error>> {
        match value {
            "get" => Ok(Self::Get),
            "expired-get" => Ok(Self::ExpiredGet),
            "put" => Ok(Self::Put),
            "hit" => Ok(Self::Hit),
            "single-flight" => Ok(Self::SingleFlight),
            _ => Err(format!("unsupported operation {value}").into()),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Get => "get",
            Self::ExpiredGet => "expired-get",
            Self::Put => "put",
            Self::Hit => "hit",
            Self::SingleFlight => "single-flight",
        }
    }
}

#[derive(Debug, Clone)]
struct Options {
    source_commit: String,
    surface: ApiPath,
    operation: Operation,
    operations: u64,
    warmup_operations: u64,
    concurrency: u64,
    payload_bytes: usize,
    key_space: u64,
    seed: u64,
    subscriber: bool,
    instrumentation: bool,
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
        let options = Self {
            source_commit: values
                .remove("--source-commit")
                .ok_or("--source-commit is required")?,
            surface: ApiPath::parse(&take(&mut values, "--surface", "raw-embedded"))?,
            operation: Operation::parse(&take(&mut values, "--operation", "get"))?,
            operations: take(&mut values, "--operations", "10000").parse()?,
            warmup_operations: take(&mut values, "--warmup-operations", "1000").parse()?,
            concurrency: take(&mut values, "--concurrency", "1").parse()?,
            payload_bytes: take(&mut values, "--payload-bytes", "256").parse()?,
            key_space: take(&mut values, "--key-space", "4096").parse()?,
            seed: take(&mut values, "--seed", "74").parse()?,
            subscriber: parse_bool(&take(&mut values, "--subscriber", "false"))?,
            instrumentation: parse_bool(&take(&mut values, "--instrumentation", "true"))?,
            output: values.remove("--output").map(PathBuf::from),
        };
        if !values.is_empty() {
            return Err(format!("unknown arguments: {:?}", values.keys()).into());
        }
        options.validate()?;
        Ok(options)
    }

    fn validate(&self) -> Result<(), Box<dyn Error>> {
        if self.operations == 0
            || self.concurrency == 0
            || self.payload_bytes == 0
            || self.key_space == 0
        {
            return Err("operations, concurrency, payload, and key-space must be non-zero".into());
        }
        if self.source_commit.len() != 40
            || !self
                .source_commit
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err("source-commit must be a full 40-character Git SHA".into());
        }
        if !self.operations.is_multiple_of(self.concurrency)
            || !self.warmup_operations.is_multiple_of(self.concurrency)
        {
            return Err("operations and warmup-operations must be divisible by concurrency".into());
        }
        let get_or_insert_operation =
            matches!(self.operation, Operation::Hit | Operation::SingleFlight);
        if (self.surface == ApiPath::GetOrInsert) != get_or_insert_operation {
            return Err(
                "get-or-insert requires hit/single-flight; other surfaces require get/put".into(),
            );
        }
        if self.operation == Operation::ExpiredGet {
            if self.surface != ApiPath::ClientSurface {
                return Err("expired-get requires the client-surface".into());
            }
            if self.key_space < self.operations.max(self.warmup_operations) {
                return Err(
                    "expired-get requires key-space >= max(operations, warmup-operations)".into(),
                );
            }
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ProfileValue {
    bytes: Vec<u8>,
}

#[derive(Clone)]
struct Context {
    cache: HydraCache,
    state: Arc<ClientSurfaceState>,
    identity: ClientIdentity,
    namespace: Namespace,
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
    live_get_hits: u64,
    expired_get_removals: u64,
    missing_gets: u64,
}

#[derive(Debug, Serialize)]
struct EmbeddedReceipt {
    estimated_entries: u64,
    hits: u64,
    misses: u64,
    loads: u64,
    single_flight_joins: u64,
    loader_executions_observed: u64,
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
    payload_bytes: usize,
    key_space: u64,
    seed: u64,
    subscriber_enabled: bool,
    instrumentation_enabled: bool,
    workload_sha256: String,
    exact_result_validation: bool,
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
    client_surface: ClientMetricsReceipt,
    embedded: EmbeddedReceipt,
    unavailable_metrics: [&'static str; 3],
}

struct WorkloadResult {
    histogram: Histogram<u64>,
    elapsed: Duration,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let options = Options::parse()?;
    let capacity = options
        .key_space
        .saturating_mul(options.payload_bytes.saturating_add(256) as u64)
        .saturating_mul(2)
        .max(1);
    let cache = HydraCache::local().max_capacity(capacity).build();
    let state = Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default())?);
    let context = Context {
        cache,
        state: Arc::clone(&state),
        identity: ClientIdentity::new("profile-074", "profile-074")?,
        namespace: Namespace::new("profile-074")?,
    };
    let loader_executions = Arc::new(AtomicU64::new(0));

    if options.operation == Operation::ExpiredGet {
        prepare_expired(&context, &options).await?;
    } else if options.operation != Operation::SingleFlight {
        preload(&context, &options).await?;
    }
    if options.warmup_operations > 0 {
        run_workload(
            context.clone(),
            &options,
            options.warmup_operations,
            Arc::clone(&loader_executions),
        )
        .await?;
        if options.operation == Operation::SingleFlight {
            context.cache.flush().await?;
        } else if options.operation == Operation::ExpiredGet {
            prepare_expired(&context, &options).await?;
        }
        loader_executions.store(0, Ordering::Relaxed);
    }

    let subscriber_task = start_subscriber(&context, &options)?;
    state.reset_profile_metrics();
    state.set_profile_instrumentation_enabled(options.instrumentation);
    let before = process_resources()?;
    let (measured, allocation) = measure_allocations(options.operations, async {
        let cpu_before = process_cpu_seconds()?;
        let workload = run_workload(
            context.clone(),
            &options,
            options.operations,
            Arc::clone(&loader_executions),
        )
        .await?;
        let cpu_seconds = (process_cpu_seconds()? - cpu_before).max(0.0);
        Ok::<_, Box<dyn Error>>((cpu_seconds, workload))
    })
    .await;
    let (cpu_seconds, workload) = measured?;
    let after = process_resources()?;
    state.set_profile_instrumentation_enabled(false);
    if let Some(task) = subscriber_task {
        task.abort();
    }

    let diagnostics = context.cache.diagnostics().await;
    let receipt = build_receipt(
        &options,
        Observations {
            cpu_seconds,
            workload,
            allocation,
            before,
            after,
            client: state.profile_metrics(),
            retained: state.retained_state_for_diagnostics(),
            embedded: EmbeddedReceipt {
                estimated_entries: diagnostics.estimated_entries,
                hits: diagnostics.stats.hits,
                misses: diagnostics.stats.misses,
                loads: diagnostics.stats.loads,
                single_flight_joins: diagnostics.stats.single_flight_joins,
                loader_executions_observed: loader_executions.load(Ordering::Relaxed),
            },
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

fn start_subscriber(
    context: &Context,
    options: &Options,
) -> Result<Option<tokio::task::JoinHandle<()>>, Box<dyn Error>> {
    if !options.subscriber {
        return Ok(None);
    }
    if options.surface == ApiPath::ClientSurface {
        let mut subscriber = context.state.subscribe_mutations(&context.identity)?;
        Ok(Some(tokio::spawn(async move {
            while subscriber.recv().await.is_ok() {}
        })))
    } else {
        let mut subscriber = context.cache.subscribe_mutations();
        Ok(Some(tokio::spawn(async move {
            while subscriber.recv().await.is_ok() {}
        })))
    }
}

struct Observations {
    cpu_seconds: f64,
    workload: WorkloadResult,
    allocation: AllocationMeasurement,
    before: ResourceSnapshot,
    after: ResourceSnapshot,
    client: ClientSurfaceProfileMetrics,
    retained: ClientSurfaceRetainedState,
    embedded: EmbeddedReceipt,
}

fn build_receipt(options: &Options, observations: Observations) -> Receipt {
    let operations = options.operations as f64;
    let client = observations.client;
    Receipt {
        schema_version: 1,
        release: "0.74",
        profile_id: PROFILE_ID,
        source_commit: options.source_commit.clone(),
        binary_sha256: binary_digest().expect("profile binary must remain readable"),
        build_profile: "release",
        tier: "local-quick",
        promotable: false,
        surface: options.surface.name(),
        operation: options.operation.name(),
        operations: options.operations,
        warmup_operations: options.warmup_operations,
        concurrency: options.concurrency,
        payload_bytes: options.payload_bytes,
        key_space: options.key_space,
        seed: options.seed,
        subscriber_enabled: options.subscriber,
        instrumentation_enabled: options.instrumentation,
        workload_sha256: workload_digest(options),
        exact_result_validation: true,
        elapsed_seconds: observations.workload.elapsed.as_secs_f64(),
        goodput_operations_per_second: operations / observations.workload.elapsed.as_secs_f64(),
        latency: LatencyReceipt {
            unit: "logical_operation",
            samples: observations.workload.histogram.len(),
            p50_us: observations.workload.histogram.value_at_quantile(0.50),
            p95_us: observations.workload.histogram.value_at_quantile(0.95),
            p99_us: observations.workload.histogram.value_at_quantile(0.99),
            max_us: observations.workload.histogram.max(),
        },
        cpu_seconds: observations.cpu_seconds,
        cpu_nanoseconds_per_operation: observations.cpu_seconds * 1_000_000_000.0 / operations,
        gross_allocated_bytes: observations.allocation.gross_allocated_bytes,
        gross_allocated_bytes_per_operation: observations
            .allocation
            .gross_allocated_bytes_per_operation,
        rss_before_bytes: observations.before.rss_bytes,
        rss_after_bytes: observations.after.rss_bytes,
        peak_rss_bytes: observations
            .after
            .peak_rss_bytes
            .max(observations.before.peak_rss_bytes),
        retained_rss_delta_bytes: observations.after.rss_bytes as i128
            - observations.before.rss_bytes as i128,
        retained_client_state: observations.retained,
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
            live_get_hits: client.live_get_hits,
            expired_get_removals: client.expired_get_removals,
            missing_gets: client.missing_gets,
        },
        embedded: observations.embedded,
        unavailable_metrics: [
            "per_stage_codec_and_moka_allocations_not_yet_instrumented",
            "tokio_poll_time_requires_dedicated_profiler",
            "allocator_retained_bytes_requires_supported_linux_allocator_profiler",
        ],
    }
}

async fn preload(context: &Context, options: &Options) -> Result<(), Box<dyn Error>> {
    if !matches!(options.operation, Operation::Get | Operation::Hit) {
        return Ok(());
    }
    for sequence in 0..options.key_space {
        put_one(context, options.surface, options, sequence).await?;
    }
    Ok(())
}

async fn prepare_expired(context: &Context, options: &Options) -> Result<(), Box<dyn Error>> {
    for sequence in 0..options.key_space {
        let response = dispatch(
            context,
            sequence,
            ClientRequest::Put {
                ns: context.namespace.clone(),
                key: structured_key(options, sequence),
                value: payload(options, sequence),
                ttl_ms: Some(1),
                dimensions: Vec::new(),
            },
        );
        if response != ClientResponse::Stored {
            return Err("expired-get preload was not stored".into());
        }
    }
    context.state.advance_cache_time_for_tests(1);
    Ok(())
}

async fn run_workload(
    context: Context,
    options: &Options,
    operations: u64,
    loader_executions: Arc<AtomicU64>,
) -> Result<WorkloadResult, Box<dyn Error>> {
    if operations == 0 {
        return Ok(WorkloadResult {
            histogram: Histogram::new_with_bounds(1, 60_000_000, 3)?,
            elapsed: Duration::ZERO,
        });
    }
    if options.operation == Operation::SingleFlight {
        return run_single_flight(context, options, operations, loader_executions).await;
    }
    let per_client = operations / options.concurrency;
    let started = Instant::now();
    let mut tasks = Vec::new();
    for client in 0..options.concurrency {
        let context = context.clone();
        let options = options.clone();
        let loader_executions = Arc::clone(&loader_executions);
        tasks.push(tokio::spawn(async move {
            let mut histogram =
                Histogram::new_with_bounds(1, 60_000_000, 3).map_err(|error| error.to_string())?;
            for offset in 0..per_client {
                let sequence = client * per_client + offset;
                let began = Instant::now();
                execute_one(&context, &options, sequence, &loader_executions).await?;
                histogram
                    .record(elapsed_us(began))
                    .map_err(|error| error.to_string())?;
            }
            Ok::<_, String>(histogram)
        }));
    }
    let mut merged = Histogram::new_with_bounds(1, 60_000_000, 3)?;
    for task in tasks {
        merged.add(&task.await.map_err(|error| error.to_string())??)?;
    }
    Ok(WorkloadResult {
        histogram: merged,
        elapsed: started.elapsed(),
    })
}

async fn run_single_flight(
    context: Context,
    options: &Options,
    operations: u64,
    loader_executions: Arc<AtomicU64>,
) -> Result<WorkloadResult, Box<dyn Error>> {
    let waves = operations / options.concurrency;
    let started = Instant::now();
    let mut histogram = Histogram::new_with_bounds(1, 60_000_000, 3)?;
    for wave in 0..waves {
        let key = cache_key(options, wave);
        let barrier = Arc::new(tokio::sync::Barrier::new(options.concurrency as usize));
        let mut tasks = Vec::new();
        for _ in 0..options.concurrency {
            let cache = context.cache.clone();
            let key = key.clone();
            let barrier = Arc::clone(&barrier);
            let loaders = Arc::clone(&loader_executions);
            let value = profile_value(options, wave);
            tasks.push(tokio::spawn(async move {
                barrier.wait().await;
                let began = Instant::now();
                let expected = value.clone();
                let actual = cache
                    .get_or_insert_with(&key, CacheOptions::new(), move || async move {
                        loaders.fetch_add(1, Ordering::Relaxed);
                        tokio::time::sleep(Duration::from_millis(1)).await;
                        value
                    })
                    .await
                    .map_err(|error| error.to_string())?;
                if actual != expected {
                    return Err("single-flight value mismatch".to_owned());
                }
                Ok::<_, String>(elapsed_us(began))
            }));
        }
        for task in tasks {
            histogram.record(task.await.map_err(|error| error.to_string())??)?;
        }
    }
    if loader_executions.load(Ordering::Relaxed) != waves {
        return Err(format!(
            "single-flight expected {waves} loader executions, observed {}",
            loader_executions.load(Ordering::Relaxed)
        )
        .into());
    }
    Ok(WorkloadResult {
        histogram,
        elapsed: started.elapsed(),
    })
}

async fn execute_one(
    context: &Context,
    options: &Options,
    sequence: u64,
    loader_executions: &Arc<AtomicU64>,
) -> Result<(), String> {
    match (options.surface, options.operation) {
        (
            ApiPath::RawEmbedded | ApiPath::TypedEmbedded | ApiPath::ClientSurface,
            Operation::Put,
        ) => put_one(context, options.surface, options, sequence)
            .await
            .map_err(|error| error.to_string()),
        (ApiPath::RawEmbedded, Operation::Get) => {
            let actual = context
                .cache
                .get_encoded(&cache_key(options, sequence))
                .await
                .map_err(|error| error.to_string())?;
            if actual.as_deref() != Some(payload(options, sequence).as_slice()) {
                return Err("raw embedded value mismatch".to_owned());
            }
            Ok(())
        }
        (ApiPath::TypedEmbedded, Operation::Get) => {
            let actual = context
                .cache
                .get::<ProfileValue>(&cache_key(options, sequence))
                .await
                .map_err(|error| error.to_string())?;
            if actual != Some(profile_value(options, sequence)) {
                return Err("typed embedded value mismatch".to_owned());
            }
            Ok(())
        }
        (ApiPath::ClientSurface, Operation::Get) => {
            let response = dispatch(
                context,
                sequence,
                ClientRequest::Get {
                    ns: context.namespace.clone(),
                    key: structured_key(options, sequence),
                },
            );
            match response {
                ClientResponse::Value { value: Some(value) }
                    if value == payload(options, sequence) =>
                {
                    Ok(())
                }
                _ => Err("client-surface value mismatch".to_owned()),
            }
        }
        (ApiPath::ClientSurface, Operation::ExpiredGet) => {
            let response = dispatch(
                context,
                sequence,
                ClientRequest::Get {
                    ns: context.namespace.clone(),
                    key: structured_key(options, sequence),
                },
            );
            match response {
                ClientResponse::Value { value: None } => Ok(()),
                _ => Err("client-surface expired GET was not a miss".to_owned()),
            }
        }
        (ApiPath::GetOrInsert, Operation::Hit) => {
            let expected = profile_value(options, sequence);
            let fallback = expected.clone();
            let loaders = Arc::clone(loader_executions);
            let actual = context
                .cache
                .get_or_insert_with(
                    &cache_key(options, sequence),
                    CacheOptions::new(),
                    move || {
                        loaders.fetch_add(1, Ordering::Relaxed);
                        async move { fallback }
                    },
                )
                .await
                .map_err(|error| error.to_string())?;
            if actual != expected || loader_executions.load(Ordering::Relaxed) != 0 {
                return Err("get-or-insert hit invoked loader or returned wrong value".to_owned());
            }
            Ok(())
        }
        _ => Err("invalid surface/operation pair escaped validation".to_owned()),
    }
}

async fn put_one(
    context: &Context,
    surface: ApiPath,
    options: &Options,
    sequence: u64,
) -> Result<(), Box<dyn Error>> {
    match surface {
        ApiPath::RawEmbedded => {
            context
                .cache
                .put_encoded(
                    &cache_key(options, sequence),
                    Bytes::from(payload(options, sequence)),
                    CacheOptions::new(),
                )
                .await?;
        }
        ApiPath::TypedEmbedded | ApiPath::GetOrInsert => {
            context
                .cache
                .put(
                    &cache_key(options, sequence),
                    profile_value(options, sequence),
                    CacheOptions::new(),
                )
                .await?;
        }
        ApiPath::ClientSurface => {
            let response = dispatch(
                context,
                sequence,
                ClientRequest::Put {
                    ns: context.namespace.clone(),
                    key: structured_key(options, sequence),
                    value: payload(options, sequence),
                    ttl_ms: None,
                    dimensions: Vec::new(),
                },
            );
            if response != ClientResponse::Stored {
                return Err("client-surface PUT was not stored".into());
            }
        }
    }
    Ok(())
}

fn dispatch(context: &Context, sequence: u64, request: ClientRequest) -> ClientResponse {
    context
        .state
        .dispatch_verified_request(
            &context.identity,
            ClientRequestEnvelope::new(format!("native-074-{sequence}"), request),
        )
        .result
        .expect("profile requests are valid")
}

fn cache_key(options: &Options, sequence: u64) -> String {
    format!(
        "native-074:{:016x}:{:016x}",
        options.seed,
        sequence % options.key_space
    )
}

fn structured_key(options: &Options, sequence: u64) -> StructuredKey {
    StructuredKey::new(vec![cache_key(options, sequence)]).expect("generated key is valid")
}

fn profile_value(options: &Options, sequence: u64) -> ProfileValue {
    ProfileValue {
        bytes: payload(options, sequence),
    }
}

fn payload(options: &Options, sequence: u64) -> Vec<u8> {
    let logical = sequence % options.key_space;
    let marker = options.seed.wrapping_add(logical).to_le_bytes();
    (0..options.payload_bytes)
        .map(|index| marker[index % marker.len()])
        .collect()
}

fn elapsed_us(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_nanos().saturating_add(999) / 1_000)
        .unwrap_or(u64::MAX)
        .clamp(1, 60_000_000)
}

fn workload_digest(options: &Options) -> String {
    let mut digest = Sha256::new();
    digest.update(PROFILE_ID.as_bytes());
    digest.update(options.surface.name().as_bytes());
    digest.update(options.operation.name().as_bytes());
    for value in [
        options.operations,
        options.warmup_operations,
        options.concurrency,
        options.payload_bytes as u64,
        options.key_space,
        options.seed,
        options.subscriber as u64,
    ] {
        digest.update(value.to_le_bytes());
    }
    for sequence in 0..options.operations {
        let key = cache_key(options, sequence);
        let value = payload(options, sequence);
        digest.update((key.len() as u64).to_le_bytes());
        digest.update(key.as_bytes());
        digest.update((value.len() as u64).to_le_bytes());
        digest.update(value);
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

    fn options(surface: ApiPath, operation: Operation) -> Options {
        Options {
            source_commit: "0123456789abcdef0123456789abcdef01234567".to_owned(),
            surface,
            operation,
            operations: 8,
            warmup_operations: 0,
            concurrency: 2,
            payload_bytes: 32,
            key_space: 8,
            seed: 74,
            subscriber: false,
            instrumentation: true,
            output: None,
        }
    }

    #[test]
    fn digest_separates_raw_typed_and_client_surfaces() {
        let raw = options(ApiPath::RawEmbedded, Operation::Get);
        let typed = options(ApiPath::TypedEmbedded, Operation::Get);
        let client = options(ApiPath::ClientSurface, Operation::Get);
        assert_ne!(workload_digest(&raw), workload_digest(&typed));
        assert_ne!(workload_digest(&typed), workload_digest(&client));
    }

    #[tokio::test]
    async fn five_paths_validate_real_results() {
        for (surface, operation) in [
            (ApiPath::RawEmbedded, Operation::Get),
            (ApiPath::TypedEmbedded, Operation::Put),
            (ApiPath::ClientSurface, Operation::Get),
            (ApiPath::ClientSurface, Operation::ExpiredGet),
            (ApiPath::GetOrInsert, Operation::Hit),
        ] {
            let options = options(surface, operation);
            let context = Context {
                cache: HydraCache::local().build(),
                state: Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default()).unwrap()),
                identity: ClientIdentity::new("test", "test").unwrap(),
                namespace: Namespace::new("test").unwrap(),
            };
            if operation == Operation::ExpiredGet {
                prepare_expired(&context, &options).await.unwrap();
            } else {
                preload(&context, &options).await.unwrap();
            }
            let result = run_workload(
                context,
                &options,
                options.operations,
                Arc::new(AtomicU64::new(0)),
            )
            .await
            .unwrap();
            assert_eq!(result.histogram.len(), options.operations);
        }
    }
}
