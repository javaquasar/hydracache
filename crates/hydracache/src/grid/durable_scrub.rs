use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::grid::durable_store::DurableValueStore;
use crate::grid::hardening::ValueStoreError;

/// Configuration for scheduled durable value-store scrubbing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DurableScrubConfig {
    /// Maximum raw records decoded in one bounded cycle.
    pub records_per_cycle: usize,
    /// Intended interval between background scrub cycles.
    pub interval: Duration,
}

impl DurableScrubConfig {
    /// Create a scrub config with normalized non-zero bounds.
    pub fn new(records_per_cycle: usize, interval: Duration) -> Self {
        Self {
            records_per_cycle: records_per_cycle.max(1),
            interval: if interval.is_zero() {
                Duration::from_secs(1)
            } else {
                interval
            },
        }
    }
}

impl Default for DurableScrubConfig {
    fn default() -> Self {
        Self::new(128, Duration::from_secs(60))
    }
}

/// Bounded-label durable scrub metrics.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct DurableScrubMetrics {
    /// Counter: `durable_scrub_records_total`.
    pub durable_scrub_records_total: u64,
    /// Counter: `durable_scrub_corruption_total`.
    pub durable_scrub_corruption_total: u64,
    /// Gauge: `durable_scrub_cycle_seconds`.
    pub durable_scrub_cycle_seconds: f64,
    /// Gauge: `durable_scrub_cursor_gauge`.
    pub durable_scrub_cursor_gauge: u64,
}

/// One corrupt durable record found by a scrub cycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurableScrubCorruption {
    /// Cache key whose raw durable bytes failed decode/checksum verification.
    pub key: String,
    /// Loud, operator-facing failure reason.
    pub error: String,
}

/// Report from one bounded scrub cycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurableScrubReport {
    /// Keys decoded during this cycle.
    pub checked_keys: Vec<String>,
    /// Number of records decoded during this cycle.
    pub records_checked: usize,
    /// Number of corrupt records found during this cycle.
    pub corruption_count: usize,
    /// Corruption details, one entry per failed record.
    pub corruptions: Vec<DurableScrubCorruption>,
    /// Cursor to resume from, or `None` when the pass reached the end.
    pub cursor: Option<String>,
    /// Whether this cycle reached the end of the current raw record range.
    pub finished_pass: bool,
    /// Elapsed cycle time.
    pub elapsed: Duration,
}

impl DurableScrubReport {
    /// Return whether this cycle found no corruption.
    pub fn is_clean(&self) -> bool {
        self.corruption_count == 0
    }
}

/// Stateful durable value-store scrubber.
#[derive(Debug, Clone)]
pub struct DurableScrubber {
    config: DurableScrubConfig,
    cursor: Option<String>,
    metrics: DurableScrubMetrics,
}

impl DurableScrubber {
    /// Create a scrubber with the supplied bounded-cycle config.
    pub fn new(config: DurableScrubConfig) -> Self {
        Self {
            config,
            cursor: None,
            metrics: DurableScrubMetrics::default(),
        }
    }

    /// Return the configured background interval.
    pub fn interval(&self) -> Duration {
        self.config.interval
    }

    /// Return the current resume cursor.
    pub fn cursor(&self) -> Option<&str> {
        self.cursor.as_deref()
    }

    /// Return a snapshot of bounded-label metrics.
    pub fn metrics(&self) -> DurableScrubMetrics {
        self.metrics
    }

    /// Run one bounded scrub cycle over raw durable records.
    pub fn scrub_cycle(
        &mut self,
        store: &DurableValueStore,
    ) -> Result<DurableScrubReport, ValueStoreError> {
        let started = Instant::now();
        let limit = self.config.records_per_cycle;
        let mut batch = store.raw_record_batch_after(self.cursor.as_deref(), limit + 1)?;
        let has_more = batch.len() > limit;
        if has_more {
            batch.truncate(limit);
        }

        let mut checked_keys = Vec::with_capacity(batch.len());
        let mut corruptions = Vec::new();
        let mut last_key = None;
        for raw in batch {
            let key = raw.key;
            checked_keys.push(key.clone());
            last_key = Some(key.clone());
            if let Err(error) = DurableValueStore::decode_raw_record(&key, &raw.bytes) {
                corruptions.push(DurableScrubCorruption {
                    key,
                    error: error.to_string(),
                });
            }
        }

        let finished_pass = !has_more;
        self.cursor = if finished_pass { None } else { last_key };
        let elapsed = started.elapsed();
        self.metrics.durable_scrub_records_total = self
            .metrics
            .durable_scrub_records_total
            .saturating_add(checked_keys.len() as u64);
        self.metrics.durable_scrub_corruption_total = self
            .metrics
            .durable_scrub_corruption_total
            .saturating_add(corruptions.len() as u64);
        self.metrics.durable_scrub_cycle_seconds = elapsed.as_secs_f64();
        self.metrics.durable_scrub_cursor_gauge = cursor_gauge(self.cursor.as_deref());

        Ok(DurableScrubReport {
            records_checked: checked_keys.len(),
            corruption_count: corruptions.len(),
            checked_keys,
            corruptions,
            cursor: self.cursor.clone(),
            finished_pass,
            elapsed,
        })
    }
}

impl Default for DurableScrubber {
    fn default() -> Self {
        Self::new(DurableScrubConfig::default())
    }
}

fn cursor_gauge(cursor: Option<&str>) -> u64 {
    let Some(cursor) = cursor else {
        return 0;
    };
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    cursor.as_bytes().iter().fold(OFFSET, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(PRIME)
    })
}
