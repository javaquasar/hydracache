use std::collections::VecDeque;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Phi-accrual detector configuration.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PhiAccrualConfig {
    /// Number of heartbeat intervals retained.
    pub window_size: usize,
    /// Suspicion threshold; higher is more conservative.
    pub phi_threshold: f64,
    /// Initial expected interval before enough samples are collected.
    pub initial_interval: Duration,
}

impl Default for PhiAccrualConfig {
    fn default() -> Self {
        Self {
            window_size: 100,
            phi_threshold: 8.0,
            initial_interval: Duration::from_millis(100),
        }
    }
}

impl PhiAccrualConfig {
    /// Create a normalized config.
    pub fn new(window_size: usize, phi_threshold: f64, initial_interval: Duration) -> Self {
        Self {
            window_size: window_size.max(1),
            phi_threshold: if phi_threshold.is_finite() && phi_threshold > 0.0 {
                phi_threshold
            } else {
                8.0
            },
            initial_interval: if initial_interval.is_zero() {
                Duration::from_millis(1)
            } else {
                initial_interval
            },
        }
    }
}

/// Adaptive heartbeat detector that outputs suspicion level, not ownership.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhiAccrualDetector {
    config: PhiAccrualConfig,
    intervals_millis: VecDeque<u64>,
    last_heartbeat_millis: Option<u64>,
    false_suspect_total: u64,
}

impl PhiAccrualDetector {
    /// Create a detector with default config.
    pub fn new() -> Self {
        Self::with_config(PhiAccrualConfig::default())
    }

    /// Create a detector with explicit config.
    pub fn with_config(config: PhiAccrualConfig) -> Self {
        Self {
            config,
            intervals_millis: VecDeque::new(),
            last_heartbeat_millis: None,
            false_suspect_total: 0,
        }
    }

    /// Record one heartbeat at a logical millisecond timestamp.
    pub fn heartbeat(&mut self, now_millis: u64) {
        if let Some(last) = self.last_heartbeat_millis {
            let interval = now_millis.saturating_sub(last).max(1);
            self.intervals_millis.push_back(interval);
            while self.intervals_millis.len() > self.config.window_size {
                self.intervals_millis.pop_front();
            }
        }
        self.last_heartbeat_millis = Some(now_millis);
    }

    /// Return suspicion level at `now_millis`.
    pub fn phi(&self, now_millis: u64) -> f64 {
        let Some(last) = self.last_heartbeat_millis else {
            return 0.0;
        };
        let elapsed = now_millis.saturating_sub(last) as f64;
        let mean = self.mean_interval_millis().max(1.0);
        elapsed / mean
    }

    /// Return whether the peer is considered available.
    pub fn is_available(&self, now_millis: u64) -> bool {
        self.phi(now_millis) < self.config.phi_threshold
    }

    /// Return the current liveness view.
    pub fn liveness(&self, now_millis: u64) -> Liveness {
        let phi = self.phi(now_millis);
        if phi < self.config.phi_threshold {
            Liveness::Up { phi }
        } else {
            Liveness::Suspect { phi }
        }
    }

    /// Record that a suspected peer recovered without a committed outage.
    pub fn record_false_suspect(&mut self) {
        self.false_suspect_total = self.false_suspect_total.saturating_add(1);
    }

    /// Return bounded metrics for this detector.
    pub fn metrics(&self, now_millis: u64) -> FailureDetectorMetrics {
        FailureDetectorMetrics {
            peer_phi_scaled: (self.phi(now_millis) * 1000.0).max(0.0) as u64,
            false_suspect_total: self.false_suspect_total,
        }
    }

    fn mean_interval_millis(&self) -> f64 {
        if self.intervals_millis.is_empty() {
            return self.config.initial_interval.as_millis().max(1) as f64;
        }
        self.intervals_millis.iter().sum::<u64>() as f64 / self.intervals_millis.len() as f64
    }
}

impl Default for PhiAccrualDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Liveness signal that feeds gossip suspicion and repair/handoff gates.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Liveness {
    /// Peer is available at the current threshold.
    Up { phi: f64 },
    /// Peer is suspected; authority still requires committed topology.
    Suspect { phi: f64 },
}

impl Liveness {
    /// Return whether this signal is available.
    pub fn is_up(self) -> bool {
        matches!(self, Self::Up { .. })
    }
}

/// Metrics emitted by the detector.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailureDetectorMetrics {
    /// Current phi scaled by 1000 to keep the aggregate counter integer-shaped.
    pub peer_phi_scaled: u64,
    /// Suspicions later observed to recover without a real outage.
    pub false_suspect_total: u64,
}

/// Return whether handoff/repair may talk to this peer.
pub fn liveness_allows_repair_or_handoff(detector: &PhiAccrualDetector, now_millis: u64) -> bool {
    detector.is_available(now_millis)
}

/// Return whether a liveness signal is allowed to change ownership.
pub fn liveness_allows_ownership_change(
    liveness: Liveness,
    committed_topology_change: bool,
) -> bool {
    matches!(liveness, Liveness::Suspect { .. }) && committed_topology_change
}
