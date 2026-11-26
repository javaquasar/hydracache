//! Conservative cluster-time bounds used by owner-authoritative expiry.

/// A validated provider sample in the common Unix millisecond time domain.
/// Providers withdraw the observation on loss of synchronization, a stale
/// reference, or a discontinuous clock step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeObservation {
    pub now_ms: u64,
    pub uncertainty_ms: u64,
}

/// Trusted local view of cluster time. Absence fences time-dependent work.
pub trait ClusterTimeAuthority: Send + Sync {
    fn observe(&self) -> Option<TimeObservation>;
}

impl TimeObservation {
    /// Inclusive lower and upper bounds, with the admitted uncertainty enforced.
    pub fn bounds(self, maximum_uncertainty_ms: u64) -> Option<(u64, u64)> {
        if self.uncertainty_ms > maximum_uncertainty_ms {
            return None;
        }
        Some((
            self.now_ms.checked_sub(self.uncertainty_ms)?,
            self.now_ms.checked_add(self.uncertainty_ms)?,
        ))
    }
}
