use crate::digest::sha256_hex;

/// Version of the deterministic key generator and its canonical digest format.
pub const KEY_SCHEDULE_GENERATOR_VERSION: u32 = 1;

/// Key popularity distribution owned by the shared workload input layer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KeyDistribution {
    /// Every key has equal probability.
    Uniform,
    /// Rank-based Zipfian popularity, where rank zero is hottest.
    Zipfian {
        /// Positive, finite exponent; YCSB-like workloads commonly use `0.99`.
        theta: f64,
    },
}

/// Complete versioned input for a deterministic key schedule.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeyScheduleSpec {
    /// Must match [`KEY_SCHEDULE_GENERATOR_VERSION`].
    pub generator_version: u32,
    /// Seed for the stable SplitMix64 stream.
    pub seed: u64,
    /// Number of logical keys, addressed as `0..key_count`.
    pub key_count: u64,
    /// Number of scheduled operations to generate.
    pub operations: u64,
    /// Popularity distribution.
    pub distribution: KeyDistribution,
}

impl KeyScheduleSpec {
    /// Build a version-1 uniform schedule specification.
    pub const fn uniform(seed: u64, key_count: u64, operations: u64) -> Self {
        Self {
            generator_version: KEY_SCHEDULE_GENERATOR_VERSION,
            seed,
            key_count,
            operations,
            distribution: KeyDistribution::Uniform,
        }
    }

    /// Build a version-1 Zipfian schedule specification.
    pub const fn zipfian(seed: u64, key_count: u64, operations: u64, theta: f64) -> Self {
        Self {
            generator_version: KEY_SCHEDULE_GENERATOR_VERSION,
            seed,
            key_count,
            operations,
            distribution: KeyDistribution::Zipfian { theta },
        }
    }

    /// Validate the contract without allocating workload state.
    pub fn validate(&self) -> Result<(), String> {
        if self.generator_version != KEY_SCHEDULE_GENERATOR_VERSION {
            return Err(format!(
                "unsupported key schedule generator version {}; expected {}",
                self.generator_version, KEY_SCHEDULE_GENERATOR_VERSION
            ));
        }
        if self.key_count == 0 {
            return Err("key schedule key_count must be positive".to_owned());
        }
        if self.operations == 0 {
            return Err("key schedule operations must be positive".to_owned());
        }
        usize::try_from(self.key_count)
            .map_err(|_| "key schedule key_count does not fit this platform".to_owned())?;
        usize::try_from(self.operations)
            .map_err(|_| "key schedule operations do not fit this platform".to_owned())?;
        if let KeyDistribution::Zipfian { theta } = self.distribution {
            if !theta.is_finite() || theta <= 0.0 {
                return Err("Zipfian theta must be positive and finite".to_owned());
            }
        }
        Ok(())
    }

    /// Generate the exact ordered key stream and its canonical digest.
    pub fn generate(&self) -> Result<GeneratedKeySchedule, String> {
        self.validate()?;
        let operation_count = usize::try_from(self.operations)
            .map_err(|_| "key schedule operations do not fit this platform".to_owned())?;
        let mut random = SplitMix64::new(self.seed);
        let mut keys = Vec::with_capacity(operation_count);

        match self.distribution {
            KeyDistribution::Uniform => {
                for _ in 0..operation_count {
                    keys.push(sample_bounded(&mut random, self.key_count));
                }
            }
            KeyDistribution::Zipfian { theta } => {
                let cumulative = zipfian_cumulative(self.key_count, theta)?;
                for _ in 0..operation_count {
                    let draw = random.next_unit_f64();
                    let rank = cumulative.partition_point(|cutoff| *cutoff <= draw);
                    keys.push(rank.min(cumulative.len() - 1) as u64);
                }
            }
        }

        let digest = schedule_digest(self, &keys);
        Ok(GeneratedKeySchedule {
            spec: *self,
            keys,
            digest,
        })
    }
}

/// Materialized deterministic key stream consumed by load generators and simulators.
#[derive(Debug, Clone, PartialEq)]
pub struct GeneratedKeySchedule {
    /// Exact generator inputs.
    pub spec: KeyScheduleSpec,
    /// Ordered logical key ids.
    pub keys: Vec<u64>,
    /// SHA-256 of version, seed, distribution parameters, dimensions, and ordered keys.
    pub digest: String,
}

fn zipfian_cumulative(key_count: u64, theta: f64) -> Result<Vec<f64>, String> {
    let key_count = usize::try_from(key_count)
        .map_err(|_| "Zipfian key_count does not fit this platform".to_owned())?;
    let mut cumulative = Vec::with_capacity(key_count);
    let mut total = 0.0_f64;
    for rank in 1..=key_count {
        total += 1.0 / (rank as f64).powf(theta);
        cumulative.push(total);
    }
    if !total.is_finite() || total <= 0.0 {
        return Err("Zipfian normalization is not finite".to_owned());
    }
    for value in &mut cumulative {
        *value /= total;
    }
    if let Some(last) = cumulative.last_mut() {
        *last = 1.0;
    }
    Ok(cumulative)
}

fn sample_bounded(random: &mut SplitMix64, upper: u64) -> u64 {
    if upper == 1 {
        return 0;
    }
    let acceptance_zone = u64::MAX - (u64::MAX % upper);
    loop {
        let candidate = random.next_u64();
        if candidate < acceptance_zone {
            return candidate % upper;
        }
    }
}

fn schedule_digest(spec: &KeyScheduleSpec, keys: &[u64]) -> String {
    let mut canonical = Vec::with_capacity(keys.len().saturating_mul(8).saturating_add(64));
    canonical.extend_from_slice(b"hydracache-key-schedule-v1\0");
    canonical.extend_from_slice(&spec.generator_version.to_le_bytes());
    canonical.extend_from_slice(&spec.seed.to_le_bytes());
    canonical.extend_from_slice(&spec.key_count.to_le_bytes());
    canonical.extend_from_slice(&spec.operations.to_le_bytes());
    match spec.distribution {
        KeyDistribution::Uniform => canonical.push(0),
        KeyDistribution::Zipfian { theta } => {
            canonical.push(1);
            canonical.extend_from_slice(&theta.to_bits().to_le_bytes());
        }
    }
    canonical.extend_from_slice(&(keys.len() as u64).to_le_bytes());
    for key in keys {
        canonical.extend_from_slice(&key.to_le_bytes());
    }
    sha256_hex(&canonical)
}

/// Stable, compact generator whose algorithm is part of schedule version 1.
#[derive(Debug, Clone, Copy)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }

    fn next_unit_f64(&mut self) -> f64 {
        const SCALE: f64 = 1.0 / ((1_u64 << 53) as f64);
        ((self.next_u64() >> 11) as f64) * SCALE
    }
}
