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
}

impl LoadBreakerPolicy {
    /// Create a disabled breaker policy.
    pub const fn disabled() -> Self {
        Self {
            failure_threshold: 0,
            initial_backoff: Duration::ZERO,
            max_backoff: Duration::ZERO,
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
        }
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

        let entry = entries
            .entry(key.to_owned())
            .or_insert_with(|| LoadBreaker {
                failures: 0,
                opened_at: None,
                backoff: self.policy.initial_backoff,
                half_open: false,
            });
        entry.failures = entry.failures.saturating_add(1);
        if entry.opened_at.is_some() {
            entry.opened_at = Some(Instant::now());
            entry.backoff = double_backoff(entry.backoff, self.policy.max_backoff);
            entry.half_open = false;
            stats
                .load_breaker_open_total
                .fetch_add(1, Ordering::Relaxed);
            return;
        }
        if entry.failures >= self.policy.failure_threshold {
            entry.opened_at = Some(Instant::now());
            entry.backoff = self.policy.initial_backoff;
            entry.half_open = false;
            stats
                .load_breaker_open_total
                .fetch_add(1, Ordering::Relaxed);
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
