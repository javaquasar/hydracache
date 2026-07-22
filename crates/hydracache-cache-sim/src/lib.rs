use std::collections::{BTreeMap, BTreeSet};

mod digest;
mod trace_catalog;
mod workload;

pub use trace_catalog::{trace_digest, CommittedTrace, TraceCatalogId};
pub use workload::{
    GeneratedKeySchedule, KeyDistribution, KeyScheduleSpec, KEY_SCHEDULE_GENERATOR_VERSION,
};

/// One cache access from a committed trace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceEvent {
    /// Logical timestamp from the trace.
    pub at: u64,
    /// Logical cache key.
    pub key: String,
}

/// Replay result for one policy and trace.
#[derive(Debug, Clone, PartialEq)]
pub struct HitRateReport {
    /// Number of trace events replayed.
    pub accesses: usize,
    /// Number of cache hits.
    pub hits: usize,
    /// Number of cache misses.
    pub misses: usize,
}

impl HitRateReport {
    /// Return `hits / accesses`, or `0.0` for an empty trace.
    pub fn hit_rate(&self) -> f64 {
        if self.accesses == 0 {
            0.0
        } else {
            self.hits as f64 / self.accesses as f64
        }
    }
}

/// Parse the tiny committed trace format: `time key` per line, `#` comments allowed.
pub fn parse_trace(text: &str) -> Result<Vec<TraceEvent>, String> {
    let mut events = Vec::new();
    for (line_index, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let at = parts
            .next()
            .ok_or_else(|| format!("line {} missing timestamp", line_index + 1))?
            .parse::<u64>()
            .map_err(|error| format!("line {} timestamp: {error}", line_index + 1))?;
        let key = parts
            .next()
            .ok_or_else(|| format!("line {} missing key", line_index + 1))?;
        if parts.next().is_some() {
            return Err(format!("line {} has extra columns", line_index + 1));
        }
        events.push(TraceEvent {
            at,
            key: key.to_owned(),
        });
    }
    Ok(events)
}

/// Replay Belady/MIN, the offline optimum for a fixed capacity.
pub fn belady_optimal(trace: &[TraceEvent], capacity: usize) -> HitRateReport {
    if capacity == 0 {
        return report(0, trace.len());
    }
    let mut cache = BTreeSet::<String>::new();
    let mut hits = 0;
    for (index, event) in trace.iter().enumerate() {
        if cache.contains(&event.key) {
            hits += 1;
            continue;
        }
        if cache.len() >= capacity {
            let victim = cache
                .iter()
                .max_by_key(|key| next_use(trace, index + 1, key).unwrap_or(usize::MAX))
                .cloned()
                .expect("capacity > 0 and cache full");
            cache.remove(&victim);
        }
        cache.insert(event.key.clone());
    }
    report(hits, trace.len())
}

/// Replay a named online policy.
pub fn replay_policy(
    trace: &[TraceEvent],
    capacity: usize,
    ttl_ticks: Option<u64>,
    policy: PolicyKind,
) -> HitRateReport {
    let mut state = PolicyState::new(capacity, ttl_ticks, policy);
    for event in trace {
        state.access(event);
    }
    report(state.hits, trace.len())
}

/// Online policies used by the W22 quality gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyKind {
    /// Least recently used baseline.
    Lru,
    /// Least frequently used baseline with oldest tie-break.
    Lfu,
    /// HydraCache test policy: frequency-aware admission with recency decay and TTL.
    Hydra,
    /// Canary fixture: deterministic pseudo-random victim selection.
    Random,
}

#[derive(Debug, Clone)]
struct EntryMeta {
    inserted_at: u64,
    last_seen_at: u64,
    hits: u64,
}

#[derive(Debug, Clone)]
struct PolicyState {
    capacity: usize,
    ttl_ticks: Option<u64>,
    policy: PolicyKind,
    entries: BTreeMap<String, EntryMeta>,
    observed_counts: BTreeMap<String, u64>,
    hits: usize,
}

impl PolicyState {
    fn new(capacity: usize, ttl_ticks: Option<u64>, policy: PolicyKind) -> Self {
        Self {
            capacity,
            ttl_ticks,
            policy,
            entries: BTreeMap::new(),
            observed_counts: BTreeMap::new(),
            hits: 0,
        }
    }

    fn access(&mut self, event: &TraceEvent) {
        let observed = self
            .observed_counts
            .entry(event.key.clone())
            .and_modify(|count| *count = count.saturating_add(1))
            .or_insert(1);
        let observed = *observed;
        self.expire(event.at);
        if let Some(entry) = self.entries.get_mut(&event.key) {
            self.hits += 1;
            entry.hits = entry.hits.saturating_add(1);
            entry.last_seen_at = event.at;
            return;
        }

        if self.capacity == 0 {
            return;
        }
        if self.entries.len() >= self.capacity {
            let victim = self.victim(event);
            if self.policy == PolicyKind::Hydra {
                let victim_score = self
                    .entries
                    .get(&victim)
                    .map(|entry| hydra_score(event.at, entry))
                    .expect("victim must exist");
                if hydra_candidate_score(observed) <= victim_score {
                    return;
                }
            }
            self.entries.remove(&victim);
        }
        self.entries.insert(
            event.key.clone(),
            EntryMeta {
                inserted_at: event.at,
                last_seen_at: event.at,
                hits: 1,
            },
        );
    }

    fn expire(&mut self, now: u64) {
        let Some(ttl) = self.ttl_ticks else {
            return;
        };
        self.entries
            .retain(|_, entry| now.saturating_sub(entry.inserted_at) <= ttl);
    }

    fn victim(&self, event: &TraceEvent) -> String {
        match self.policy {
            PolicyKind::Lru => self
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.last_seen_at)
                .map(|(key, _)| key.clone())
                .expect("cache is full"),
            PolicyKind::Lfu => self
                .entries
                .iter()
                .min_by_key(|(_, entry)| (entry.hits, entry.last_seen_at))
                .map(|(key, _)| key.clone())
                .expect("cache is full"),
            PolicyKind::Hydra => self
                .entries
                .iter()
                .min_by_key(|(_, entry)| hydra_score(event.at, entry))
                .map(|(key, _)| key.clone())
                .expect("cache is full"),
            PolicyKind::Random => {
                let index = stable_index(event.at, &event.key, self.entries.len());
                self.entries
                    .keys()
                    .nth(index)
                    .cloned()
                    .expect("cache is full")
            }
        }
    }
}

fn hydra_score(now: u64, entry: &EntryMeta) -> u64 {
    let age = now.saturating_sub(entry.last_seen_at);
    let recency = 32_u64.saturating_sub(age.min(32));
    let decayed_frequency = entry.hits.saturating_mul(8) / (1 + age / 4);
    decayed_frequency.saturating_add(recency)
}

fn hydra_candidate_score(observed_count: u64) -> u64 {
    observed_count.saturating_mul(4).saturating_add(32)
}

fn next_use(trace: &[TraceEvent], start: usize, key: &str) -> Option<usize> {
    trace[start..]
        .iter()
        .position(|event| event.key == key)
        .map(|offset| start + offset)
}

fn stable_index(at: u64, key: &str, len: usize) -> usize {
    let mut hash = at ^ 0x9e37_79b9_7f4a_7c15;
    for byte in key.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    (hash as usize) % len.max(1)
}

fn report(hits: usize, accesses: usize) -> HitRateReport {
    HitRateReport {
        accesses,
        hits,
        misses: accesses.saturating_sub(hits),
    }
}
