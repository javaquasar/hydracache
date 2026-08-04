use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use hydracache::{CacheDiagnostics, CacheOptions, CacheStats, HydraCache};
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;

use crate::target::{PreloadOutcome, Target, TargetError, TargetOutcome, TargetRequest};

const STATE_DIGEST_VERSION: &str = "hydracache-local-target-state-v1";
const PRELOAD_KEY_PREFIX: &str = "w1:preload:";
const MISS_KEY_PREFIX: &str = "w1:miss:";
const LOADER_KEY_PREFIX: &str = "w1:loader:";
const PUT_KEY_PREFIX: &str = "w1:put:";
const PRESSURE_KEY_PREFIX: &str = "w1:pressure:";
const HOT_KEY: &str = "w1:hot";

/// Deterministic operation mix used by the real local-cache target.
///
/// Percentages must add up to 100. Capacity-pressure operations are configured
/// separately and override the selected mixed operation at their interval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalOperationMix {
    pub hit_percent: u8,
    pub miss_percent: u8,
    pub loader_percent: u8,
    pub put_percent: u8,
    pub hot_key_percent: u8,
}

impl LocalOperationMix {
    pub const fn total_percent(self) -> u16 {
        self.hit_percent as u16
            + self.miss_percent as u16
            + self.loader_percent as u16
            + self.put_percent as u16
            + self.hot_key_percent as u16
    }

    fn operation_for(self, sequence: u64) -> LocalOperation {
        let percentile = (sequence % 100) as u16;
        let hit_end = self.hit_percent as u16;
        let miss_end = hit_end + self.miss_percent as u16;
        let loader_end = miss_end + self.loader_percent as u16;
        let put_end = loader_end + self.put_percent as u16;

        if percentile < hit_end {
            LocalOperation::Hit
        } else if percentile < miss_end {
            LocalOperation::Miss
        } else if percentile < loader_end {
            LocalOperation::Loader
        } else if percentile < put_end {
            LocalOperation::Put
        } else {
            LocalOperation::HotKeyLoader
        }
    }
}

impl Default for LocalOperationMix {
    fn default() -> Self {
        Self {
            hit_percent: 40,
            miss_percent: 20,
            loader_percent: 15,
            put_percent: 15,
            hot_key_percent: 10,
        }
    }
}

/// Concrete local-cache path executed for a scheduled request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalOperation {
    Hit,
    Miss,
    Loader,
    Put,
    HotKeyLoader,
    CapacityPressure,
}

/// Configuration for a real, in-process [`HydraCache`] target.
#[derive(Debug, Clone)]
pub struct LocalTargetConfig {
    /// Moka weighted capacity passed through the public HydraCache builder.
    pub max_capacity: u64,
    /// Maximum encoded entry size accepted by HydraCache.
    pub max_entry_bytes: usize,
    /// Number of deterministic entries inserted and verified by `preload`.
    pub preload_entries: u64,
    /// Number of reusable keys in the put path.
    pub key_space: u64,
    /// Payload bytes before codec framing.
    pub payload_bytes: usize,
    pub operation_mix: LocalOperationMix,
    /// Deterministic application-loader cost used by loader characterizations.
    pub loader_delay: Duration,
    /// Test-harness rendezvous: hold the hot-key loader until this many real
    /// cache misses have joined the single-flight operation.
    pub hot_key_expected_miss_waiters: Option<u64>,
    /// Every Nth request uses a unique pressure key. `None` disables the path.
    pub capacity_pressure_every: Option<u64>,
    /// Test-only defect injection included in capacity-pressure latency.
    pub injected_capacity_pressure_delay: Duration,
}

impl LocalTargetConfig {
    /// Validate invariants that would otherwise turn a named path into a lie.
    pub fn validate(&self) -> Result<(), String> {
        if self.max_capacity == 0 {
            return Err("max_capacity must be greater than zero".to_owned());
        }
        if self.max_entry_bytes == 0 {
            return Err("max_entry_bytes must be greater than zero".to_owned());
        }
        if self.payload_bytes == 0 {
            return Err("payload_bytes must be greater than zero".to_owned());
        }
        if self.key_space == 0 {
            return Err("key_space must be greater than zero".to_owned());
        }
        if self.operation_mix.total_percent() != 100 {
            return Err(format!(
                "local operation percentages must total 100, got {}",
                self.operation_mix.total_percent()
            ));
        }
        if self.operation_mix.hit_percent > 0 && self.preload_entries == 0 {
            return Err("a non-zero hit percentage requires at least one preload entry".to_owned());
        }
        if self.capacity_pressure_every == Some(0) {
            return Err("capacity_pressure_every must be greater than zero".to_owned());
        }
        if self.hot_key_expected_miss_waiters == Some(0) {
            return Err("hot_key_expected_miss_waiters must be greater than zero".to_owned());
        }
        Ok(())
    }
}

impl Default for LocalTargetConfig {
    fn default() -> Self {
        Self {
            max_capacity: 1024 * 1024,
            max_entry_bytes: 16 * 1024,
            preload_entries: 256,
            key_space: 1024,
            payload_bytes: 128,
            operation_mix: LocalOperationMix::default(),
            loader_delay: Duration::ZERO,
            hot_key_expected_miss_waiters: None,
            capacity_pressure_every: None,
            injected_capacity_pressure_delay: Duration::ZERO,
        }
    }
}

/// Per-path attempts and normalized outcomes, independent from cache counters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LocalOperationCounts {
    pub hits: u64,
    pub misses: u64,
    pub loaders: u64,
    pub puts: u64,
    pub hot_key_loaders: u64,
    pub capacity_pressure: u64,
    /// Number of loader closures that actually executed; requests may share one.
    pub loader_executions: u64,
    pub successes: u64,
    pub errors: u64,
}

/// One coherent observation of target path counters and HydraCache diagnostics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalTargetSnapshot {
    pub operations: LocalOperationCounts,
    pub diagnostics: CacheDiagnostics,
}

#[derive(Debug, Default)]
struct OperationCounters {
    hits: AtomicU64,
    misses: AtomicU64,
    loaders: AtomicU64,
    puts: AtomicU64,
    hot_key_loaders: AtomicU64,
    capacity_pressure: AtomicU64,
    loader_executions: Arc<AtomicU64>,
    successes: AtomicU64,
    errors: AtomicU64,
}

impl OperationCounters {
    fn reset(&self) {
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
        self.loaders.store(0, Ordering::Relaxed);
        self.puts.store(0, Ordering::Relaxed);
        self.hot_key_loaders.store(0, Ordering::Relaxed);
        self.capacity_pressure.store(0, Ordering::Relaxed);
        self.loader_executions.store(0, Ordering::Relaxed);
        self.successes.store(0, Ordering::Relaxed);
        self.errors.store(0, Ordering::Relaxed);
    }

    fn record_attempt(&self, operation: LocalOperation) {
        let counter = match operation {
            LocalOperation::Hit => &self.hits,
            LocalOperation::Miss => &self.misses,
            LocalOperation::Loader => &self.loaders,
            LocalOperation::Put => &self.puts,
            LocalOperation::HotKeyLoader => &self.hot_key_loaders,
            LocalOperation::CapacityPressure => &self.capacity_pressure,
        };
        counter.fetch_add(1, Ordering::Relaxed);
    }

    fn record_outcome(&self, outcome: TargetOutcome) {
        match outcome {
            TargetOutcome::Success => {
                self.successes.fetch_add(1, Ordering::Relaxed);
            }
            TargetOutcome::Error | TargetOutcome::Rejected | TargetOutcome::Timeout => {
                self.errors.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    fn snapshot(&self) -> LocalOperationCounts {
        LocalOperationCounts {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            loaders: self.loaders.load(Ordering::Relaxed),
            puts: self.puts.load(Ordering::Relaxed),
            hot_key_loaders: self.hot_key_loaders.load(Ordering::Relaxed),
            capacity_pressure: self.capacity_pressure.load(Ordering::Relaxed),
            loader_executions: self.loader_executions.load(Ordering::Relaxed),
            successes: self.successes.load(Ordering::Relaxed),
            errors: self.errors.load(Ordering::Relaxed),
        }
    }
}

/// Real local-cache product surface used by W1 characterization.
///
/// The cache lives behind an `RwLock` so every reset replaces the complete
/// HydraCache instance, including its stats and backend eviction state.
#[derive(Debug)]
pub struct LocalCacheTarget {
    config: LocalTargetConfig,
    cache: RwLock<HydraCache>,
    expected_values: Mutex<BTreeMap<String, Option<Vec<u8>>>>,
    counters: OperationCounters,
}

impl LocalCacheTarget {
    pub fn new(config: LocalTargetConfig) -> Result<Self, TargetError> {
        config.validate().map_err(TargetError::Reset)?;
        let cache = build_cache(&config);
        Ok(Self {
            config,
            cache: RwLock::new(cache),
            expected_values: Mutex::new(BTreeMap::new()),
            counters: OperationCounters::default(),
        })
    }

    pub fn config(&self) -> &LocalTargetConfig {
        &self.config
    }

    /// Return the deterministic path for a scheduled sequence number.
    pub fn operation_for(&self, sequence: u64) -> LocalOperation {
        if self
            .config
            .capacity_pressure_every
            .is_some_and(|interval| sequence % interval == interval - 1)
        {
            LocalOperation::CapacityPressure
        } else {
            self.config.operation_mix.operation_for(sequence)
        }
    }

    /// Expose HydraCache's lightweight counters without replacing them with a
    /// load-generator approximation.
    pub async fn stats(&self) -> CacheStats {
        self.cache.read().await.stats()
    }

    /// Expose a coherent diagnostics snapshot together with target path counts.
    pub async fn snapshot(&self) -> LocalTargetSnapshot {
        let cache = self.cache.read().await.clone();
        LocalTargetSnapshot {
            operations: self.counters.snapshot(),
            diagnostics: cache.diagnostics().await,
        }
    }

    /// Verify that a capacity-pressure candidate was admitted with its exact
    /// deterministic value rather than silently rejected by cache admission.
    pub async fn capacity_pressure_key_present(&self, sequence: u64) -> Result<bool, TargetError> {
        let key = format!("{PRESSURE_KEY_PREFIX}{sequence}");
        let expected = payload_for(sequence, self.config.payload_bytes);
        let cache = self.cache.read().await.clone();
        cache
            .get::<Vec<u8>>(&key)
            .await
            .map(|actual| actual == Some(expected))
            .map_err(|error| TargetError::Measurement(error.to_string()))
    }

    /// Observe a deterministic member of the original preload set. A miss is
    /// valid after eviction; the access still drives the declared popularity
    /// stream through the real cache policy.
    pub async fn observe_preload_key(&self, logical_key: u64) -> Result<bool, TargetError> {
        if self.config.preload_entries == 0 {
            return Err(TargetError::Measurement(
                "capacity lookup requires a non-empty preload set".to_owned(),
            ));
        }
        let key = format!(
            "{PRELOAD_KEY_PREFIX}{}",
            logical_key % self.config.preload_entries
        );
        let cache = self.cache.read().await.clone();
        cache
            .get::<Vec<u8>>(&key)
            .await
            .map(|value| value.is_some())
            .map_err(|error| TargetError::Measurement(error.to_string()))
    }

    /// Execute one explicit product path, useful for the W1 hit/miss/loader
    /// breakdown. The `Target` implementation selects this path from the mix.
    pub async fn execute_operation(
        &self,
        operation: LocalOperation,
        sequence: u64,
    ) -> TargetOutcome {
        self.counters.record_attempt(operation);
        let cache = self.cache.read().await.clone();
        let outcome = match operation {
            LocalOperation::Hit => self.execute_hit(&cache, sequence).await,
            LocalOperation::Miss => self.execute_miss(&cache, sequence).await,
            LocalOperation::Loader => self.execute_loader(&cache, sequence).await,
            LocalOperation::Put => self.execute_put(&cache, sequence).await,
            LocalOperation::HotKeyLoader => self.execute_hot_key_loader(&cache).await,
            LocalOperation::CapacityPressure => {
                self.execute_capacity_pressure(&cache, sequence).await
            }
        };
        self.counters.record_outcome(outcome);
        outcome
    }

    async fn execute_hit(&self, cache: &HydraCache, sequence: u64) -> TargetOutcome {
        if self.config.preload_entries == 0 {
            return TargetOutcome::Error;
        }
        let index = sequence % self.config.preload_entries;
        let key = format!("{PRELOAD_KEY_PREFIX}{index}");
        let expected = payload_for(index, self.config.payload_bytes);
        match cache.get::<Vec<u8>>(&key).await {
            Ok(Some(actual)) if actual == expected => TargetOutcome::Success,
            Ok(_) | Err(_) => TargetOutcome::Error,
        }
    }

    async fn execute_miss(&self, cache: &HydraCache, sequence: u64) -> TargetOutcome {
        let key = format!("{MISS_KEY_PREFIX}{sequence}");
        match cache.get::<Vec<u8>>(&key).await {
            Ok(None) => TargetOutcome::Success,
            Ok(Some(_)) | Err(_) => TargetOutcome::Error,
        }
    }

    async fn execute_loader(&self, cache: &HydraCache, sequence: u64) -> TargetOutcome {
        let key = format!("{LOADER_KEY_PREFIX}{sequence}");
        let expected = payload_for(sequence, self.config.payload_bytes);
        let loaded = expected.clone();
        let loader_executions = Arc::clone(&self.counters.loader_executions);
        let loader_delay = self.config.loader_delay;
        match cache
            .get_or_insert_with(&key, CacheOptions::new(), move || async move {
                loader_executions.fetch_add(1, Ordering::Relaxed);
                if !loader_delay.is_zero() {
                    tokio::time::sleep(loader_delay).await;
                }
                loaded
            })
            .await
        {
            Ok(actual) if actual == expected => {
                if self.record_expected(key, Some(expected)).is_ok() {
                    TargetOutcome::Success
                } else {
                    TargetOutcome::Error
                }
            }
            Ok(_) | Err(_) => TargetOutcome::Error,
        }
    }

    async fn execute_put(&self, cache: &HydraCache, sequence: u64) -> TargetOutcome {
        let index = sequence % self.config.key_space;
        let key = format!("{PUT_KEY_PREFIX}{index}");
        let value = payload_for(sequence, self.config.payload_bytes);
        match cache.put(&key, value.clone(), CacheOptions::new()).await {
            Ok(()) => {
                // Concurrent puts may legally overwrite the same reusable key.
                // Track presence and hash the value observed at the phase
                // boundary instead of guessing which writer linearized last.
                if self.record_expected(key, None).is_ok() {
                    TargetOutcome::Success
                } else {
                    TargetOutcome::Error
                }
            }
            Err(_) => TargetOutcome::Error,
        }
    }

    async fn execute_hot_key_loader(&self, cache: &HydraCache) -> TargetOutcome {
        let expected = payload_for(u64::MAX, self.config.payload_bytes);
        let loaded = expected.clone();
        let loader_executions = Arc::clone(&self.counters.loader_executions);
        let loader_delay = self.config.loader_delay;
        let expected_waiters = self.config.hot_key_expected_miss_waiters;
        let cache_probe = cache.clone();
        match cache
            .get_or_insert_with(HOT_KEY, CacheOptions::new(), move || async move {
                loader_executions.fetch_add(1, Ordering::Relaxed);
                if let Some(expected_waiters) = expected_waiters {
                    while cache_probe.stats().misses < expected_waiters {
                        tokio::task::yield_now().await;
                    }
                }
                if !loader_delay.is_zero() {
                    tokio::time::sleep(loader_delay).await;
                }
                loaded
            })
            .await
        {
            Ok(actual) if actual == expected => {
                if self
                    .record_expected(HOT_KEY.to_owned(), Some(expected))
                    .is_ok()
                {
                    TargetOutcome::Success
                } else {
                    TargetOutcome::Error
                }
            }
            Ok(_) | Err(_) => TargetOutcome::Error,
        }
    }

    async fn execute_capacity_pressure(&self, cache: &HydraCache, sequence: u64) -> TargetOutcome {
        let key = format!("{PRESSURE_KEY_PREFIX}{sequence}");
        let value = payload_for(sequence, self.config.payload_bytes);
        if cache
            .put(&key, value.clone(), CacheOptions::new())
            .await
            .is_err()
        {
            return TargetOutcome::Error;
        }
        if self.record_expected(key, None).is_err() {
            return TargetOutcome::Error;
        }
        if !self.config.injected_capacity_pressure_delay.is_zero() {
            tokio::time::sleep(self.config.injected_capacity_pressure_delay).await;
        }
        TargetOutcome::Success
    }

    fn record_expected(&self, key: String, value: Option<Vec<u8>>) -> Result<(), ()> {
        self.expected_values
            .lock()
            .map_err(|_| ())?
            .insert(key, value);
        Ok(())
    }

    async fn verified_state_digest(&self, require_all_present: bool) -> Result<String, String> {
        let expected = self
            .expected_values
            .lock()
            .map_err(|_| "expected-value registry is poisoned".to_owned())?
            .clone();
        let cache = self.cache.read().await.clone();
        let diagnostics = cache.diagnostics().await;
        let mut hasher = Sha256::new();
        update_digest_field(&mut hasher, STATE_DIGEST_VERSION.as_bytes());
        update_digest_field(&mut hasher, &self.config.max_capacity.to_le_bytes());
        update_digest_field(
            &mut hasher,
            &(self.config.max_entry_bytes as u64).to_le_bytes(),
        );
        update_digest_field(
            &mut hasher,
            &(self.config.payload_bytes as u64).to_le_bytes(),
        );

        let mut present = 0_u64;
        let mut missing = 0_u64;
        for (key, expected_value) in expected {
            update_digest_field(&mut hasher, key.as_bytes());
            match cache.get::<Vec<u8>>(&key).await {
                Ok(Some(actual))
                    if expected_value
                        .as_ref()
                        .is_none_or(|expected| &actual == expected) =>
                {
                    present = present.saturating_add(1);
                    update_digest_field(&mut hasher, b"present");
                    update_digest_field(&mut hasher, &actual);
                }
                Ok(Some(_)) => {
                    return Err(format!(
                        "state verification found an unexpected value for {key}"
                    ));
                }
                Ok(None) => {
                    missing = missing.saturating_add(1);
                    update_digest_field(&mut hasher, b"missing");
                }
                Err(error) => {
                    return Err(format!("state verification could not read {key}: {error}"));
                }
            }
        }

        if require_all_present && missing > 0 {
            return Err(format!(
                "preload verification found {missing} missing entries"
            ));
        }
        if diagnostics.estimated_entries != present {
            return Err(format!(
                "tracked {present} present entries but HydraCache reports {}",
                diagnostics.estimated_entries
            ));
        }
        update_digest_field(&mut hasher, &present.to_le_bytes());
        update_digest_field(&mut hasher, &missing.to_le_bytes());
        update_digest_field(&mut hasher, &diagnostics.estimated_entries.to_le_bytes());
        Ok(format!(
            "sha256:{}",
            hex_digest(hasher.finalize().as_slice())
        ))
    }
}

#[async_trait]
impl Target for LocalCacheTarget {
    async fn reset(&self) -> Result<String, TargetError> {
        self.config.validate().map_err(TargetError::Reset)?;
        *self.cache.write().await = build_cache(&self.config);
        self.expected_values
            .lock()
            .map_err(|_| TargetError::Reset("expected-value registry is poisoned".to_owned()))?
            .clear();
        self.counters.reset();
        self.verified_state_digest(true)
            .await
            .map_err(TargetError::Reset)
    }

    async fn preload(&self) -> Result<PreloadOutcome, TargetError> {
        let cache = self.cache.read().await.clone();
        for index in 0..self.config.preload_entries {
            let key = format!("{PRELOAD_KEY_PREFIX}{index}");
            let value = payload_for(index, self.config.payload_bytes);
            cache
                .put(&key, value.clone(), CacheOptions::new())
                .await
                .map_err(|error| TargetError::Preload(error.to_string()))?;
            self.record_expected(key, Some(value)).map_err(|()| {
                TargetError::Preload("expected-value registry is poisoned".to_owned())
            })?;
        }
        let state_digest = self
            .verified_state_digest(true)
            .await
            .map_err(TargetError::Preload)?;
        Ok(PreloadOutcome {
            operations: self.config.preload_entries,
            state_digest,
        })
    }

    async fn state_digest(&self) -> Result<String, TargetError> {
        self.verified_state_digest(false)
            .await
            .map_err(TargetError::Warmup)
    }

    async fn execute(&self, request: TargetRequest) -> TargetOutcome {
        self.execute_operation(self.operation_for(request.sequence), request.sequence)
            .await
    }
}

fn build_cache(config: &LocalTargetConfig) -> HydraCache {
    HydraCache::local()
        .max_capacity(config.max_capacity)
        .max_entry_bytes(config.max_entry_bytes)
        .invalidation_node_id("hydracache-loadgen-local-w1")
        .build()
}

fn payload_for(identity: u64, payload_bytes: usize) -> Vec<u8> {
    let mut payload = vec![0xA5; payload_bytes];
    for (slot, byte) in payload.iter_mut().zip(identity.to_le_bytes()) {
        *slot = byte;
    }
    payload
}

fn update_digest_field(hasher: &mut Sha256, value: &[u8]) {
    hasher.update((value.len() as u64).to_le_bytes());
    hasher.update(value);
}

fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn zero_preload_lifecycle_uses_one_state_digest() {
        let target = LocalCacheTarget::new(LocalTargetConfig {
            preload_entries: 0,
            operation_mix: LocalOperationMix {
                hit_percent: 0,
                miss_percent: 0,
                loader_percent: 0,
                put_percent: 0,
                hot_key_percent: 100,
            },
            ..LocalTargetConfig::default()
        })
        .expect("W6 local target must construct");

        let reset = target.reset().await.expect("reset must succeed");
        assert_eq!(target.state_digest().await.unwrap(), reset);

        let preload = target.preload().await.expect("preload must succeed");
        assert_eq!(preload.operations, 0);
        assert_eq!(preload.state_digest, reset);
        assert_eq!(target.state_digest().await.unwrap(), reset);
    }
}
