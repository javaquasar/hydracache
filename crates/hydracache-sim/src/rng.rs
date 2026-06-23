use rand_chacha::ChaCha8Rng;
use rand_core::{RngCore, SeedableRng};

/// Seeded deterministic RNG used by the simulator.
#[derive(Debug, Clone)]
pub struct SimRng {
    inner: ChaCha8Rng,
}

impl SimRng {
    /// Build a reproducible RNG from a numeric seed.
    pub fn from_seed(seed: u64) -> Self {
        Self {
            inner: ChaCha8Rng::seed_from_u64(seed),
        }
    }

    /// Return the next deterministic `u64`.
    pub fn next_u64(&mut self) -> u64 {
        self.inner.next_u64()
    }

    /// Return a deterministic number in `0..upper`.
    ///
    /// Panics if `upper == 0`; simulator callers should make empty choices
    /// explicit rather than silently picking a sentinel.
    pub fn next_index(&mut self, upper: usize) -> usize {
        assert!(upper > 0, "upper bound must be non-zero");
        (self.next_u64() % upper as u64) as usize
    }

    /// Return true with probability `numerator / denominator`.
    pub fn chance(&mut self, numerator: u64, denominator: u64) -> bool {
        assert!(denominator > 0, "denominator must be non-zero");
        assert!(
            numerator <= denominator,
            "numerator must not exceed denominator"
        );
        self.next_u64() % denominator < numerator
    }
}
