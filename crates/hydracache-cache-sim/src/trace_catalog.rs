use crate::digest::sha256_hex;
use crate::{parse_trace, TraceEvent};

/// Stable identifiers for the tiny committed W22 trace catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TraceCatalogId {
    /// General locality fixture used for the Belady comparison.
    Standard,
    /// Frequency-skew fixture historically called `skewed_zipfian`.
    SkewedZipfian,
    /// Recency plus TTL fixture.
    RecencyTtl,
}

impl TraceCatalogId {
    /// Every committed trace in stable catalog order.
    pub const ALL: [Self; 3] = [Self::Standard, Self::SkewedZipfian, Self::RecencyTtl];

    /// Stable catalog name recorded in evidence.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::SkewedZipfian => "skewed_zipfian",
            Self::RecencyTtl => "recency_ttl",
        }
    }

    /// Exact committed source bytes, reused rather than copied by consumers.
    pub const fn source(self) -> &'static str {
        match self {
            Self::Standard => include_str!("../traces/standard.trace"),
            Self::SkewedZipfian => include_str!("../traces/skewed_zipfian.trace"),
            Self::RecencyTtl => include_str!("../traces/recency_ttl.trace"),
        }
    }

    /// Parse the committed source and bind both its exact bytes and logical event order.
    pub fn load(self) -> Result<CommittedTrace, String> {
        let source = self.source();
        let events = parse_trace(source)?;
        Ok(CommittedTrace {
            id: self,
            source_digest: sha256_hex(source.as_bytes()),
            event_digest: trace_digest(&events),
            events,
        })
    }
}

impl std::fmt::Display for TraceCatalogId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One parsed committed trace with exact-source and order-sensitive identities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedTrace {
    /// Stable catalog id.
    pub id: TraceCatalogId,
    /// Parsed accesses in the exact committed order.
    pub events: Vec<TraceEvent>,
    /// SHA-256 of the exact committed source, including comments and whitespace.
    pub source_digest: String,
    /// SHA-256 of the canonical ordered `(timestamp, key)` event stream.
    pub event_digest: String,
}

/// Hash the logical trace with length prefixes so order and field boundaries are unambiguous.
pub fn trace_digest(events: &[TraceEvent]) -> String {
    let mut canonical = Vec::with_capacity(events.len().saturating_mul(24));
    canonical.extend_from_slice(b"hydracache-trace-events-v1\0");
    canonical.extend_from_slice(&(events.len() as u64).to_le_bytes());
    for event in events {
        canonical.extend_from_slice(&event.at.to_le_bytes());
        canonical.extend_from_slice(&(event.key.len() as u64).to_le_bytes());
        canonical.extend_from_slice(event.key.as_bytes());
    }
    sha256_hex(&canonical)
}
