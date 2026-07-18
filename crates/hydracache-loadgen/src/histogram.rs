use std::time::Duration;

use hdrhistogram::Histogram;
use serde::{Deserialize, Serialize};

/// Bounded-memory latency histogram with explicit overflow accounting.
#[derive(Debug, Clone)]
pub struct LatencyHistogram {
    inner: Histogram<u64>,
    highest_trackable_us: u64,
    overflow_count: u64,
}

/// Canonical percentile projection stored in a performance report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LatencySummary {
    pub samples: u64,
    pub p50_us: Option<u64>,
    pub p90_us: Option<u64>,
    pub p99_us: Option<u64>,
    pub p999_us: Option<u64>,
    pub p999_min_samples: u64,
    pub p999_reportable: bool,
    pub max_us: Option<u64>,
    pub overflow_count: u64,
}

impl LatencyHistogram {
    /// Create a histogram with a fixed highest value and 1-5 significant figures.
    pub fn new(highest_trackable: Duration, significant_figures: u8) -> Result<Self, String> {
        let highest_trackable_us = duration_us_ceil(highest_trackable).max(1);
        let inner = Histogram::<u64>::new_with_bounds(1, highest_trackable_us, significant_figures)
            .map_err(|error| error.to_string())?;
        Ok(Self {
            inner,
            highest_trackable_us,
            overflow_count: 0,
        })
    }

    /// Record queue-inclusive latency. Values beyond the configured range are clamped and loud.
    pub fn record(&mut self, latency: Duration) {
        let value = duration_us_ceil(latency).max(1);
        if value > self.highest_trackable_us {
            self.overflow_count = self.overflow_count.saturating_add(1);
            self.inner
                .record(self.highest_trackable_us)
                .expect("configured highest value is recordable");
        } else {
            self.inner
                .record(value)
                .expect("bounded latency is recordable");
        }
    }

    /// Record an already-normalized microsecond value.
    pub fn record_us(&mut self, latency_us: u64) {
        self.record(Duration::from_micros(latency_us));
    }

    /// Number of samples retained by the bounded histogram.
    pub fn len(&self) -> u64 {
        self.inner.len()
    }

    /// Whether no latency samples have been recorded.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Project canonical percentiles and suppress p999 below its declared sample floor.
    pub fn summary(&self, p999_min_samples: u64) -> LatencySummary {
        let samples = self.inner.len();
        let percentile =
            |quantile| (!self.inner.is_empty()).then(|| self.inner.value_at_quantile(quantile));
        let p999_reportable = samples >= p999_min_samples;
        LatencySummary {
            samples,
            p50_us: percentile(0.50),
            p90_us: percentile(0.90),
            p99_us: percentile(0.99),
            p999_us: p999_reportable.then(|| self.inner.value_at_quantile(0.999)),
            p999_min_samples,
            p999_reportable,
            max_us: percentile(1.0),
            overflow_count: self.overflow_count,
        }
    }
}

fn duration_us_ceil(duration: Duration) -> u64 {
    let micros = duration.as_nanos().saturating_add(999) / 1_000;
    u64::try_from(micros).unwrap_or(u64::MAX)
}
