use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use tokio::sync::Mutex;

use crate::stats::StatsCounters;

/// Per-key loader circuit-breaker policy.
///
/// The policy is disabled by default so the ordinary healthy-key fast path is
/// unchanged unless an application explicitly opts in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadBreakerPolicy {
    failure_threshold: u32,
    initial_backoff: Duration,
    max_backoff: Duration,
    max_tracked_keys: usize,
    inactive_entry_ttl: Duration,
}

const DEFAULT_MAX_TRACKED_KEYS: usize = 4_096;
const DEFAULT_INACTIVE_ENTRY_TTL: Duration = Duration::from_secs(300);

impl LoadBreakerPolicy {
    /// Create a disabled breaker policy.
    pub const fn disabled() -> Self {
        Self {
            failure_threshold: 0,
            initial_backoff: Duration::ZERO,
            max_backoff: Duration::ZERO,
            max_tracked_keys: DEFAULT_MAX_TRACKED_KEYS,
            inactive_entry_ttl: DEFAULT_INACTIVE_ENTRY_TTL,
        }
    }

    /// Create an enabled breaker policy.
    ///
    /// `failure_threshold` is normalized to at least `2`, so a transient single
    /// failure cannot open the breaker. Backoff durations are normalized to a
    /// non-zero initial backoff and a max backoff at least as large as initial.
    pub fn new(failure_threshold: u32, initial_backoff: Duration, max_backoff: Duration) -> Self {
        let initial_backoff = normalize_backoff(initial_backoff);
        Self {
            failure_threshold: failure_threshold.max(2),
            initial_backoff,
            max_backoff: max_backoff.max(initial_backoff),
            max_tracked_keys: DEFAULT_MAX_TRACKED_KEYS,
            inactive_entry_ttl: DEFAULT_INACTIVE_ENTRY_TTL,
        }
    }

    /// Set the maximum number of per-key breaker records retained by a cache.
    ///
    /// The budget is normalized to at least one. At capacity, the least
    /// recently active closed record is evicted. Open breakers are never
    /// evicted merely to admit a new key; if every slot is open, the new key is
    /// left untracked until an existing breaker recovers.
    pub fn max_tracked_keys(mut self, max_tracked_keys: usize) -> Self {
        self.max_tracked_keys = max_tracked_keys.max(1);
        self
    }

    /// Set how long a closed failure record may remain inactive.
    ///
    /// The duration is normalized to at least one millisecond. Open and
    /// half-open breakers are retained regardless of this timeout.
    pub fn inactive_entry_ttl(mut self, inactive_entry_ttl: Duration) -> Self {
        self.inactive_entry_ttl = normalize_backoff(inactive_entry_ttl);
        self
    }

    /// Return whether this policy is enabled.
    pub fn is_enabled(&self) -> bool {
        self.failure_threshold > 0
    }

    /// Return the consecutive failure threshold.
    pub fn failure_threshold(&self) -> u32 {
        self.failure_threshold
    }

    /// Return the first open-breaker backoff.
    pub fn initial_backoff(&self) -> Duration {
        self.initial_backoff
    }

    /// Return the maximum open-breaker backoff.
    pub fn max_backoff(&self) -> Duration {
        self.max_backoff
    }

    /// Return the maximum number of retained per-key breaker records.
    pub fn max_tracked_keys_limit(&self) -> usize {
        self.max_tracked_keys
    }

    /// Return the retention timeout for inactive closed failure records.
    pub fn inactive_entry_ttl_limit(&self) -> Duration {
        self.inactive_entry_ttl
    }
}

impl Default for LoadBreakerPolicy {
    fn default() -> Self {
        Self::disabled()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LoadBreakerDecision {
    Allow,
    Reject,
}

/// Per-key breaker registry shared by a cache instance.
#[derive(Debug)]
pub(crate) struct LoadBreakerRegistry {
    policy: LoadBreakerPolicy,
    entries: Mutex<HashMap<String, LoadBreaker>>,
}

impl LoadBreakerRegistry {
    pub(crate) fn new(policy: LoadBreakerPolicy) -> Self {
        Self {
            policy,
            entries: Mutex::new(HashMap::new()),
        }
    }

    pub(crate) fn disabled() -> Self {
        Self::new(LoadBreakerPolicy::disabled())
    }

    pub(crate) async fn before_load(
        &self,
        key: &str,
        stats: &StatsCounters,
    ) -> LoadBreakerDecision {
        if !self.policy.is_enabled() {
            return LoadBreakerDecision::Allow;
        }

        let mut entries = self.entries.lock().await;
        prune_inactive_entries(&mut entries, Instant::now(), self.policy.inactive_entry_ttl);
        let Some(entry) = entries.get_mut(key) else {
            return LoadBreakerDecision::Allow;
        };
        let Some(opened_at) = entry.opened_at else {
            return LoadBreakerDecision::Allow;
        };
        if entry.half_open {
            stats
                .load_breaker_rejected_total
                .fetch_add(1, Ordering::Relaxed);
            return LoadBreakerDecision::Reject;
        }
        if opened_at.elapsed() < entry.backoff {
            stats
                .load_breaker_rejected_total
                .fetch_add(1, Ordering::Relaxed);
            return LoadBreakerDecision::Reject;
        }

        entry.half_open = true;
        stats
            .load_breaker_half_open_total
            .fetch_add(1, Ordering::Relaxed);
        LoadBreakerDecision::Allow
    }

    pub(crate) async fn after_load_result(&self, key: &str, success: bool, stats: &StatsCounters) {
        if !self.policy.is_enabled() {
            return;
        }

        let mut entries = self.entries.lock().await;
        let now = Instant::now();
        prune_inactive_entries(&mut entries, now, self.policy.inactive_entry_ttl);
        if success {
            if entries
                .remove(key)
                .and_then(|entry| entry.opened_at)
                .is_some()
            {
                stats
                    .load_breaker_recovered_total
                    .fetch_add(1, Ordering::Relaxed);
            }
            return;
        }

        if !entries.contains_key(key) && entries.len() >= self.policy.max_tracked_keys {
            let eviction_candidate = entries
                .iter()
                .filter(|(_, entry)| entry.opened_at.is_none())
                .min_by_key(|(_, entry)| entry.last_activity)
                .map(|(key, _)| key.clone());
            if let Some(eviction_candidate) = eviction_candidate {
                entries.remove(&eviction_candidate);
            } else {
                stats
                    .load_breaker_saturated_total
                    .fetch_add(1, Ordering::Relaxed);
                return;
            }
        }

        let entry = entries
            .entry(key.to_owned())
            .or_insert_with(|| LoadBreaker {
                failures: 0,
                opened_at: None,
                backoff: self.policy.initial_backoff,
                half_open: false,
                last_activity: now,
            });
        entry.last_activity = now;
        entry.failures = entry.failures.saturating_add(1);
        if entry.opened_at.is_some() {
            entry.opened_at = Some(now);
            entry.backoff = double_backoff(entry.backoff, self.policy.max_backoff);
            entry.half_open = false;
            stats
                .load_breaker_open_total
                .fetch_add(1, Ordering::Relaxed);
            return;
        }
        if entry.failures >= self.policy.failure_threshold {
            entry.opened_at = Some(now);
            entry.backoff = self.policy.initial_backoff;
            entry.half_open = false;
            stats
                .load_breaker_open_total
                .fetch_add(1, Ordering::Relaxed);
        }
    }

    #[cfg(test)]
    async fn retained_state(&self) -> LoadBreakerRetainedState {
        let entries = self.entries.lock().await;
        LoadBreakerRetainedState {
            entries: entries.len(),
            open_entries: entries
                .values()
                .filter(|entry| entry.opened_at.is_some())
                .count(),
            key_capacity_bytes: entries.keys().map(String::capacity).sum(),
        }
    }
}

impl Default for LoadBreakerRegistry {
    fn default() -> Self {
        Self::disabled()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LoadBreaker {
    failures: u32,
    opened_at: Option<Instant>,
    backoff: Duration,
    half_open: bool,
    last_activity: Instant,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LoadBreakerRetainedState {
    entries: usize,
    open_entries: usize,
    key_capacity_bytes: usize,
}

fn prune_inactive_entries(
    entries: &mut HashMap<String, LoadBreaker>,
    now: Instant,
    inactive_entry_ttl: Duration,
) {
    entries.retain(|_, entry| {
        entry.opened_at.is_some()
            || now.saturating_duration_since(entry.last_activity) < inactive_entry_ttl
    });
}

fn normalize_backoff(backoff: Duration) -> Duration {
    if backoff.is_zero() {
        Duration::from_millis(1)
    } else {
        backoff
    }
}

fn double_backoff(current: Duration, max_backoff: Duration) -> Duration {
    current.saturating_mul(2).min(max_backoff)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy(max_tracked_keys: usize) -> LoadBreakerPolicy {
        LoadBreakerPolicy::new(2, Duration::from_millis(50), Duration::from_secs(1))
            .max_tracked_keys(max_tracked_keys)
    }

    #[test]
    fn policy_retention_limits_are_normalized_and_observable() {
        let policy = policy(0).inactive_entry_ttl(Duration::ZERO);

        assert_eq!(policy.max_tracked_keys_limit(), 1);
        assert_eq!(policy.inactive_entry_ttl_limit(), Duration::from_millis(1));
    }

    #[tokio::test]
    async fn unique_one_shot_failures_stay_within_entry_budget() {
        let registry = LoadBreakerRegistry::new(policy(32));
        let stats = StatsCounters::default();

        for index in 0..10_000 {
            registry
                .after_load_result(&format!("key-{index}"), false, &stats)
                .await;
        }

        let retained = registry.retained_state().await;
        assert_eq!(retained.entries, 32);
        assert_eq!(retained.open_entries, 0);
        assert!(retained.key_capacity_bytes > 0);
    }

    #[tokio::test]
    async fn success_releases_breaker_ownership() {
        let registry = LoadBreakerRegistry::new(policy(4));
        let stats = StatsCounters::default();
        registry.after_load_result("key", false, &stats).await;
        assert_eq!(registry.retained_state().await.entries, 1);

        registry.after_load_result("key", true, &stats).await;

        assert_eq!(registry.retained_state().await.entries, 0);
    }

    #[tokio::test]
    async fn saturation_never_evicts_an_open_breaker() {
        let registry = LoadBreakerRegistry::new(policy(2));
        let stats = StatsCounters::default();
        for key in ["poison-a", "poison-b"] {
            registry.after_load_result(key, false, &stats).await;
            registry.after_load_result(key, false, &stats).await;
        }

        registry.after_load_result("new-key", false, &stats).await;

        let retained = registry.retained_state().await;
        assert_eq!(retained.entries, 2);
        assert_eq!(retained.open_entries, 2);
        assert_eq!(
            stats.load_breaker_saturated_total.load(Ordering::Relaxed),
            1
        );
        assert_eq!(
            registry.before_load("poison-a", &stats).await,
            LoadBreakerDecision::Reject
        );
        assert_eq!(
            registry.before_load("poison-b", &stats).await,
            LoadBreakerDecision::Reject
        );
    }

    #[tokio::test]
    async fn inactive_closed_entries_expire_without_touching_open_breakers() {
        let policy = policy(4).inactive_entry_ttl(Duration::from_millis(1));
        let registry = LoadBreakerRegistry::new(policy);
        let stats = StatsCounters::default();
        registry.after_load_result("closed", false, &stats).await;
        registry.after_load_result("open", false, &stats).await;
        registry.after_load_result("open", false, &stats).await;
        tokio::time::sleep(Duration::from_millis(5)).await;

        assert_eq!(
            registry.before_load("missing", &stats).await,
            LoadBreakerDecision::Allow
        );

        let retained = registry.retained_state().await;
        assert_eq!(retained.entries, 1);
        assert_eq!(retained.open_entries, 1);
    }
}
