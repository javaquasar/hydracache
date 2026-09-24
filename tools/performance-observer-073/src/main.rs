use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use hydracache::{CacheOptions, HydraCache, MemoryInstrumentationMode, MemorySnapshotRequest};
use hydracache_loadgen::allocation::measure_allocations;
use hydracache_loadgen::{
    run_open_loop, OpenLoopConfig, PreloadOutcome, Target, TargetError, TargetOutcome,
    TargetRequest,
};
use serde::Serialize;

const KEY_COUNT: u64 = 4_096;
const TAG_COUNT: u64 = 64;
const PAYLOAD_BYTES: usize = 128;

#[derive(Debug)]
struct ObserverTarget {
    cache: HydraCache,
}

impl ObserverTarget {
    fn new(mode: MemoryInstrumentationMode) -> Self {
        Self {
            cache: HydraCache::local()
                .max_capacity(16 * 1024 * 1024)
                .memory_instrumentation_mode(mode)
                .build(),
        }
    }

    async fn put(&self, sequence: u64, ttl: Option<Duration>) -> Result<(), String> {
        let key = format!("observer-073:key:{}", sequence % KEY_COUNT);
        let tag = format!("observer-073:tag:{}", sequence % TAG_COUNT);
        let mut options = CacheOptions::new().tags(["observer-073", tag.as_str()]);
        if let Some(ttl) = ttl {
            options = options.ttl(ttl);
        }
        self.cache
            .put(&key, vec![(sequence & 0xff) as u8; PAYLOAD_BYTES], options)
            .await
            .map_err(|error| error.to_string())
    }
}

#[async_trait]
impl Target for ObserverTarget {
    async fn reset(&self) -> Result<String, TargetError> {
        self.cache
            .flush()
            .await
            .map_err(|error| TargetError::Reset(error.to_string()))?;
        Ok("observer-073:reset:v1".to_owned())
    }

    async fn preload(&self) -> Result<PreloadOutcome, TargetError> {
        for sequence in 0..KEY_COUNT {
            self.put(sequence, None)
                .await
                .map_err(TargetError::Preload)?;
        }
        Ok(PreloadOutcome {
            operations: KEY_COUNT,
            state_digest: "observer-073:preload-4096:v1".to_owned(),
        })
    }

    async fn state_digest(&self) -> Result<String, TargetError> {
        let diagnostics = self.cache.diagnostics().await;
        Ok(format!(
            "observer-073:entries:{}",
            diagnostics.estimated_entries
        ))
    }

    async fn execute(&self, request: TargetRequest) -> TargetOutcome {
        let sequence = request.sequence;
        let percentile = sequence % 100;
        let result = if percentile < 40 {
            let key = format!("observer-073:key:{}", sequence % KEY_COUNT);
            self.cache
                .get::<Vec<u8>>(&key)
                .await
                .map(|_| ())
                .map_err(|error| error.to_string())
        } else if percentile < 65 {
            self.put(sequence, None).await
        } else if percentile < 80 {
            let key = format!("observer-073:key:{}", sequence % KEY_COUNT);
            match self.cache.remove(&key).await {
                Ok(_) => self.put(sequence, None).await,
                Err(error) => Err(error.to_string()),
            }
        } else if percentile < 90 {
            let tag = format!("observer-073:tag:{}", sequence % TAG_COUNT);
            match self.cache.invalidate_tag(&tag).await {
                Ok(_) => self.put(sequence, None).await,
                Err(error) => Err(error.to_string()),
            }
        } else {
            self.put(sequence, Some(Duration::from_millis(5))).await
        };
        if result.is_ok() {
            TargetOutcome::Success
        } else {
            TargetOutcome::Error
        }
    }
}

#[derive(Debug, Serialize)]
struct Receipt {
    schema_version: u32,
    release: &'static str,
    profile_id: &'static str,
    instrumentation_mode: &'static str,
    offered_rate_per_second: u64,
    operations: u64,
    warmup_operations: u64,
    workload_mix_percent: BTreeMap<&'static str, u64>,
    observation: hydracache_loadgen::OpenLoopObservation,
    gross_allocated_bytes: u64,
    gross_allocated_bytes_per_operation: f64,
    cpu_seconds: f64,
    cpu_seconds_per_operation: f64,
    rss_before_bytes: u64,
    rss_after_bytes: u64,
    peak_rss_bytes: u64,
    reconciliation_exact: bool,
    final_estimated_entries: u64,
    promotable: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let options = Options::parse()?;
    let mode = match options.mode.as_str() {
        "off" => MemoryInstrumentationMode::Off,
        "production" => MemoryInstrumentationMode::Production,
        value => return Err(format!("unsupported --mode {value}").into()),
    };
    let target = Arc::new(ObserverTarget::new(mode));
    target.preload().await?;
    for sequence in 0..options.warmup_operations {
        if target.execute(TargetRequest { sequence }).await != TargetOutcome::Success {
            return Err("warmup operation failed".into());
        }
    }

    let rss_before = process_memory()?;
    let cpu_before = process_cpu_seconds()?;
    let config = OpenLoopConfig {
        offered_rate_per_second: options.rate,
        operations: options.operations,
        highest_trackable_latency: Duration::from_secs(5),
        significant_figures: 3,
        p999_min_samples: 5_000,
        drain_timeout: Duration::from_secs(5),
    };
    let (observation, allocation) = measure_allocations(
        options.operations,
        run_open_loop(Arc::clone(&target), &config),
    )
    .await;
    let observation = observation?;
    let cpu_seconds = (process_cpu_seconds()? - cpu_before).max(0.0);
    let rss_after = process_memory()?;

    let reconciliation_exact = if mode == MemoryInstrumentationMode::Production {
        target.cache.reconcile_memory_footprint().await?.matched
    } else {
        true
    };
    let diagnostics = target.cache.diagnostics().await;
    if observation.started != observation.offered
        || observation.completed != observation.started
        || observation.successes != observation.completed
        || observation.errors != 0
        || observation.timeouts != 0
        || observation.rejections != 0
        || !observation.backlog_drained
        || !reconciliation_exact
    {
        return Err("incomplete outcome accounting or reconciliation failure".into());
    }
    if mode == MemoryInstrumentationMode::Production {
        let barrier = target.cache.memory_snapshot_barrier()?;
        let snapshot = target
            .cache
            .memory_footprint_snapshot(MemorySnapshotRequest::Exact {
                acknowledged_epoch: barrier.epoch,
            })
            .await?;
        if snapshot.consistency != hydracache::MemorySnapshotConsistency::Exact {
            return Err("production memory snapshot was not exact".into());
        }
    }

    let receipt = Receipt {
        schema_version: 1,
        release: "0.73",
        profile_id: "observer-baseline-pilot-073-v1",
        instrumentation_mode: if mode == MemoryInstrumentationMode::Off {
            "off"
        } else {
            "production"
        },
        offered_rate_per_second: options.rate,
        operations: options.operations,
        warmup_operations: options.warmup_operations,
        workload_mix_percent: BTreeMap::from([
            ("get", 40),
            ("tagged_put", 25),
            ("remove_refill", 15),
            ("tag_invalidate_refill", 10),
            ("ttl_put", 10),
        ]),
        observation,
        gross_allocated_bytes: allocation.gross_allocated_bytes,
        gross_allocated_bytes_per_operation: allocation.gross_allocated_bytes_per_operation,
        cpu_seconds,
        cpu_seconds_per_operation: cpu_seconds / options.operations as f64,
        rss_before_bytes: rss_before.0,
        rss_after_bytes: rss_after.0,
        peak_rss_bytes: rss_after.1.max(rss_before.1),
        reconciliation_exact,
        final_estimated_entries: diagnostics.estimated_entries,
        promotable: false,
    };
    if let Some(parent) = options.output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(options.output, serde_json::to_vec_pretty(&receipt)?)?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn process_cpu_seconds() -> Result<f64, Box<dyn Error>> {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
    // SAFETY: getrusage initializes the supplied rusage structure on success.
    if unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    // SAFETY: the successful call above initialized the structure.
    let usage = unsafe { usage.assume_init() };
    let seconds = |time: libc::timeval| time.tv_sec as f64 + time.tv_usec as f64 / 1_000_000.0;
    Ok(seconds(usage.ru_utime) + seconds(usage.ru_stime))
}

#[cfg(not(target_os = "linux"))]
fn process_cpu_seconds() -> Result<f64, Box<dyn Error>> {
    Err("CPU accounting is available only on the Linux reference host".into())
}

#[cfg(target_os = "linux")]
fn process_memory() -> Result<(u64, u64), Box<dyn Error>> {
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
    Ok((value("VmRSS:")?, value("VmHWM:")?))
}

#[cfg(not(target_os = "linux"))]
fn process_memory() -> Result<(u64, u64), Box<dyn Error>> {
    Err("RSS accounting is available only on the Linux reference host".into())
}

struct Options {
    mode: String,
    rate: u64,
    operations: u64,
    warmup_operations: u64,
    output: PathBuf,
}

impl Options {
    fn parse() -> Result<Self, Box<dyn Error>> {
        let mut values = BTreeMap::new();
        let mut args = std::env::args().skip(1);
        while let Some(name) = args.next() {
            if !name.starts_with("--") {
                return Err(format!("unsupported argument {name}").into());
            }
            values.insert(
                name.trim_start_matches("--").to_owned(),
                args.next()
                    .ok_or_else(|| format!("{name} requires a value"))?,
            );
        }
        let mut take = |name: &str| {
            values
                .remove(name)
                .ok_or_else(|| format!("--{name} is required"))
        };
        let options = Self {
            mode: take("mode")?,
            rate: take("rate")?.parse()?,
            operations: take("operations")?.parse()?,
            warmup_operations: take("warmup-operations")?.parse()?,
            output: PathBuf::from(take("output")?),
        };
        if !values.is_empty() || options.rate == 0 || options.operations == 0 {
            return Err("unsupported arguments or zero rate/operations".into());
        }
        Ok(options)
    }
}
