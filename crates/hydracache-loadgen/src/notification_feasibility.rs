use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::Serialize;

use crate::allocation::{measure_allocations, AllocationMeasurement};

const OPERATIONS: u64 = 1_024;
const CAPACITY: u64 = 16_384;
const REPETITIONS: u64 = 3;

#[derive(Debug, Clone, Serialize)]
pub struct NotificationFeasibilityReceipt {
    pub schema_version: &'static str,
    pub release: &'static str,
    pub experiment: &'static str,
    pub operations_per_case: u64,
    pub repetitions: u64,
    pub cases: Vec<NotificationFeasibilityCase>,
    pub diagnostic_only: bool,
    pub product_semantics_eligible: bool,
    pub promotable: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct NotificationFeasibilityCase {
    pub backend: &'static str,
    pub operation: &'static str,
    pub listener: &'static str,
    pub repetition: u64,
    pub position: u64,
    pub gross_allocated_bytes: u64,
    pub gross_allocated_bytes_per_operation: f64,
    pub elapsed_ns: u64,
}

pub async fn run_and_write(output: &Path) -> Result<NotificationFeasibilityReceipt, String> {
    if output.exists() {
        return Err(format!(
            "append-only feasibility output already exists: {}",
            output.display()
        ));
    }
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    let cases = run_cases().await;
    let receipt = NotificationFeasibilityReceipt {
        schema_version: "hydracache-notification-feasibility-073-v1",
        release: "0.73",
        experiment: "moka-future-vs-sync-noop-listener",
        operations_per_case: OPERATIONS,
        repetitions: REPETITIONS,
        cases,
        diagnostic_only: true,
        product_semantics_eligible: false,
        promotable: false,
    };
    std::fs::write(
        output,
        serde_json::to_vec_pretty(&receipt).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    Ok(receipt)
}

async fn run_cases() -> Vec<NotificationFeasibilityCase> {
    let mut cases = Vec::with_capacity((REPETITIONS * 8) as usize);
    for repetition in 1..=REPETITIONS {
        let order = listener_order(repetition);
        for (position, listener) in order.into_iter().enumerate() {
            let position = position as u64 + 1;
            cases.push(future_insert(listener, repetition, position).await);
            cases.push(future_remove(listener, repetition, position).await);
            cases.push(sync_insert(listener, repetition, position).await);
            cases.push(sync_remove(listener, repetition, position).await);
        }
    }
    cases
}

async fn future_insert(
    listener: bool,
    repetition: u64,
    position: u64,
) -> NotificationFeasibilityCase {
    let cache = future_cache(listener);
    let started = Instant::now();
    let (_, allocation) = measure_allocations(OPERATIONS, async {
        for key in 0..OPERATIONS {
            cache.insert(key, [key as u8; 64]).await;
        }
        cache.run_pending_tasks().await;
    })
    .await;
    case(
        "moka-future",
        "insert",
        listener,
        repetition,
        position,
        allocation,
        started,
    )
}

async fn future_remove(
    listener: bool,
    repetition: u64,
    position: u64,
) -> NotificationFeasibilityCase {
    let cache = future_cache(listener);
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
        "moka-future",
        "remove",
        listener,
        repetition,
        position,
        allocation,
        started,
    )
}

async fn sync_insert(
    listener: bool,
    repetition: u64,
    position: u64,
) -> NotificationFeasibilityCase {
    let cache = sync_cache(listener);
    let started = Instant::now();
    let (_, allocation) = measure_allocations(OPERATIONS, async {
        for key in 0..OPERATIONS {
            cache.insert(key, [key as u8; 64]);
        }
        cache.run_pending_tasks();
    })
    .await;
    case(
        "moka-sync",
        "insert",
        listener,
        repetition,
        position,
        allocation,
        started,
    )
}

async fn sync_remove(
    listener: bool,
    repetition: u64,
    position: u64,
) -> NotificationFeasibilityCase {
    let cache = sync_cache(listener);
    for key in 0..OPERATIONS {
        cache.insert(key, [key as u8; 64]);
    }
    cache.run_pending_tasks();
    let started = Instant::now();
    let (_, allocation) = measure_allocations(OPERATIONS, async {
        for key in 0..OPERATIONS {
            cache.invalidate(&key);
        }
        cache.run_pending_tasks();
    })
    .await;
    case(
        "moka-sync",
        "remove",
        listener,
        repetition,
        position,
        allocation,
        started,
    )
}

fn future_cache(listener: bool) -> moka::future::Cache<u64, [u8; 64]> {
    let builder = moka::future::Cache::builder().max_capacity(CAPACITY);
    if listener {
        builder.eviction_listener(|_key, _value, _cause| {}).build()
    } else {
        builder.build()
    }
}

fn sync_cache(listener: bool) -> moka::sync::Cache<u64, [u8; 64]> {
    let builder = moka::sync::Cache::builder().max_capacity(CAPACITY);
    if listener {
        builder.eviction_listener(|_key, _value, _cause| {}).build()
    } else {
        builder.build()
    }
}

fn case(
    backend: &'static str,
    operation: &'static str,
    listener: bool,
    repetition: u64,
    position: u64,
    allocation: AllocationMeasurement,
    started: Instant,
) -> NotificationFeasibilityCase {
    NotificationFeasibilityCase {
        backend,
        operation,
        listener: if listener { "noop" } else { "off" },
        repetition,
        position,
        gross_allocated_bytes: allocation.gross_allocated_bytes,
        gross_allocated_bytes_per_operation: allocation.gross_allocated_bytes_per_operation,
        elapsed_ns: u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX),
    }
}

fn listener_order(repetition: u64) -> [bool; 2] {
    if repetition & 1 == 0 {
        [true, false]
    } else {
        [false, true]
    }
}

pub fn default_output() -> PathBuf {
    PathBuf::from("target/performance-evidence/0.73/local/notification-feasibility.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn experiment_covers_both_backends_operations_and_listener_states() {
        let cases = run_cases().await;
        assert_eq!(cases.len(), 24);
        for backend in ["moka-future", "moka-sync"] {
            for operation in ["insert", "remove"] {
                for listener in ["off", "noop"] {
                    assert_eq!(
                        cases
                            .iter()
                            .filter(|case| case.backend == backend
                                && case.operation == operation
                                && case.listener == listener)
                            .count(),
                        3
                    );
                }
            }
        }
        assert_eq!(listener_order(1), [false, true]);
        assert_eq!(listener_order(2), [true, false]);
    }
}
