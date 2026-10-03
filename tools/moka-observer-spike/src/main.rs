use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use hydracache_loadgen::allocation::measure_allocations;
use hydracache_loadgen::notification_observer::{
    PublishOutcome, RemovalKind, RemovalObserver, VersionedEntry,
};
use moka::notification::RemovalCause;
use serde::Serialize;

const OPERATIONS: u64 = 1_024;
const REPETITIONS: u64 = 3;

#[derive(Clone, Copy)]
enum Mode {
    Off,
    Listener,
    Observer,
}

impl Mode {
    fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Listener => "listener",
            Self::Observer => "observer",
        }
    }
}

#[derive(Serialize)]
struct Receipt {
    schema_version: &'static str,
    release: &'static str,
    experiment: &'static str,
    operations_per_case: u64,
    repetitions: u64,
    causes_verified: Vec<&'static str>,
    cases: Vec<Case>,
    diagnostic_only: bool,
    product_semantics_eligible: bool,
    promotable: bool,
}

#[derive(Serialize)]
struct Case {
    mode: &'static str,
    operation: &'static str,
    repetition: u64,
    position: u64,
    gross_allocated_bytes: u64,
    gross_allocated_bytes_per_operation: f64,
    elapsed_ns: u64,
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("moka-observer-spike: {error}");
        std::process::exit(2);
    }
}

async fn run() -> Result<(), String> {
    let output = parse_output()?;
    if output.exists() {
        return Err(format!(
            "append-only output already exists: {}",
            output.display()
        ));
    }
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    verify_removal_causes().await?;
    verify_versioned_adapter().await?;
    let mut cases = Vec::with_capacity((REPETITIONS * 6) as usize);
    for repetition in 1..=REPETITIONS {
        for (position, mode) in order(repetition).into_iter().enumerate() {
            cases.push(measure_insert(mode, repetition, position as u64 + 1).await);
            cases.push(measure_remove(mode, repetition, position as u64 + 1).await);
        }
    }
    let receipt = Receipt {
        schema_version: "moka-post-removal-observer-spike-073-v1",
        release: "0.73",
        experiment: "moka-future-listener-vs-observer-key-lock-ablation",
        operations_per_case: OPERATIONS,
        repetitions: REPETITIONS,
        causes_verified: vec!["explicit", "replaced", "expired", "size"],
        cases,
        diagnostic_only: true,
        product_semantics_eligible: false,
        promotable: false,
    };
    std::fs::write(&output, serde_json::to_vec_pretty(&receipt).unwrap())
        .map_err(|error| error.to_string())?;
    println!("moka-observer-spike: OK ({})", output.display());
    Ok(())
}

fn cache(mode: Mode) -> moka::future::Cache<u64, [u8; 64]> {
    let builder = moka::future::Cache::builder().max_capacity(16_384);
    match mode {
        Mode::Off => builder.build(),
        Mode::Listener => builder.eviction_listener(|_, _, _| {}).build(),
        Mode::Observer => builder.post_removal_observer(|_, _, _| {}).build(),
    }
}

async fn measure_insert(mode: Mode, repetition: u64, position: u64) -> Case {
    let cache = cache(mode);
    let started = Instant::now();
    let (_, allocation) = measure_allocations(OPERATIONS, async {
        for key in 0..OPERATIONS {
            cache.insert(key, [key as u8; 64]).await;
        }
        cache.run_pending_tasks().await;
    })
    .await;
    case(
        mode,
        "insert",
        repetition,
        position,
        allocation.gross_allocated_bytes,
        started,
    )
}

async fn measure_remove(mode: Mode, repetition: u64, position: u64) -> Case {
    let cache = cache(mode);
    for key in 0..OPERATIONS {
        cache.insert(key, [key as u8; 64]).await;
    }
    cache.run_pending_tasks().await;
    let started = Instant::now();
    let (_, allocation) = measure_allocations(OPERATIONS, async {
        for key in 0..OPERATIONS {
            cache.invalidate(&key).await;
        }
        cache.run_pending_tasks().await;
    })
    .await;
    case(
        mode,
        "remove",
        repetition,
        position,
        allocation.gross_allocated_bytes,
        started,
    )
}

fn case(
    mode: Mode,
    operation: &'static str,
    repetition: u64,
    position: u64,
    allocated: u64,
    started: Instant,
) -> Case {
    Case {
        mode: mode.name(),
        operation,
        repetition,
        position,
        gross_allocated_bytes: allocated,
        gross_allocated_bytes_per_operation: allocated as f64 / OPERATIONS as f64,
        elapsed_ns: started.elapsed().as_nanos().try_into().unwrap_or(u64::MAX),
    }
}

async fn verify_removal_causes() -> Result<(), String> {
    let causes = Arc::new(Mutex::new(Vec::new()));
    let observed = causes.clone();
    let cache = moka::future::Cache::builder()
        .max_capacity(1)
        .time_to_live(Duration::from_millis(2))
        .post_removal_observer(move |_, _, cause| observed.lock().unwrap().push(cause))
        .build();

    cache.insert(1, [1; 64]).await;
    cache.insert(1, [2; 64]).await;
    cache.invalidate(&1).await;
    cache.insert(2, [2; 64]).await;
    cache.insert(3, [3; 64]).await;
    cache.run_pending_tasks().await;
    cache.insert(4, [4; 64]).await;
    tokio::time::sleep(Duration::from_millis(5)).await;
    let _ = cache.get(&4).await;
    cache.run_pending_tasks().await;

    let causes = causes.lock().unwrap();
    for required in [
        RemovalCause::Explicit,
        RemovalCause::Replaced,
        RemovalCause::Expired,
        RemovalCause::Size,
    ] {
        if !causes.contains(&required) {
            return Err(format!("observer did not receive {required:?}: {causes:?}"));
        }
    }
    Ok(())
}

async fn verify_versioned_adapter() -> Result<(), String> {
    let lifecycle = Arc::new(RemovalObserver::new(8));
    let observed = lifecycle.clone();
    let cache = moka::future::Cache::builder()
        .max_capacity(8)
        .post_removal_observer(move |_, entry: VersionedEntry, cause| {
            let kind = match cause {
                RemovalCause::Explicit => RemovalKind::Explicit,
                RemovalCause::Replaced => RemovalKind::Replaced,
                RemovalCause::Expired => RemovalKind::Expired,
                RemovalCause::Size => RemovalKind::Capacity,
            };
            assert_eq!(observed.publish(entry, kind), PublishOutcome::Accepted);
        })
        .build();

    let old = VersionedEntry::new("key", 41, vec!["blue".to_owned()], 100);
    lifecycle.register(&old);
    cache.insert("key", old).await;
    let new = VersionedEntry::new("key", 42, vec!["blue".to_owned()], 120);
    cache.insert("key", new.clone()).await;
    lifecycle.register(&new);
    lifecycle.drain_available();

    if !lifecycle.contains_membership("blue", "key", 42) {
        return Err("delayed Moka replacement cleanup removed the new membership".to_owned());
    }
    let snapshot = lifecycle
        .exact_snapshot()
        .map_err(|error| format!("versioned adapter snapshot failed: {error:?}"))?;
    if snapshot.retained_bytes != 120 || snapshot.memberships != 1 {
        return Err(format!("versioned adapter mismatch: {snapshot:?}"));
    }
    Ok(())
}

fn order(repetition: u64) -> [Mode; 3] {
    match repetition % 3 {
        1 => [Mode::Off, Mode::Listener, Mode::Observer],
        2 => [Mode::Observer, Mode::Off, Mode::Listener],
        _ => [Mode::Listener, Mode::Observer, Mode::Off],
    }
}

fn parse_output() -> Result<PathBuf, String> {
    let mut args = std::env::args().skip(1);
    match (args.next().as_deref(), args.next(), args.next()) {
        (Some("--output"), Some(path), None) => Ok(PathBuf::from(path)),
        _ => Err("usage: moka-observer-spike --output <new-json-path>".to_owned()),
    }
}
