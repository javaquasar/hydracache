use super::*;

/// Metadata key used by members to advertise their peer-fetch base URL.
///
/// The value is a base URL such as `http://127.0.0.1:3000`, not the full
/// peer-fetch route. Transport adapters append their own route path so one
/// advertised endpoint can stay stable across route-versioning changes.
pub const CLUSTER_PEER_FETCH_BASE_URL_METADATA_KEY: &str = "hydracache.peer_fetch.base_url";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterPeerFetchRequest {
    /// Owner member expected to serve this request.
    pub owner: ClusterNodeId,
    /// Logical cache key requested from the owner.
    pub key: String,
    /// Optional owner generation observed by the caller.
    pub generation: Option<ClusterGeneration>,
}

/// Requested owner generation did not match the current owner generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClusterPeerFetchGenerationMismatch {
    /// Generation observed by the caller when it resolved ownership.
    pub requested: ClusterGeneration,
    /// Current generation known by the owner or transport.
    pub current: ClusterGeneration,
}

impl ClusterPeerFetchRequest {
    /// Create a new peer-fetch request.
    pub fn new(owner: impl Into<ClusterNodeId>, key: impl Into<String>) -> Self {
        Self {
            owner: owner.into(),
            key: key.into(),
            generation: None,
        }
    }

    /// Attach the owner generation observed by the caller.
    pub fn generation(mut self, generation: ClusterGeneration) -> Self {
        self.generation = Some(generation);
        self
    }

    /// Return whether this request carries an observed owner generation.
    pub fn has_generation(&self) -> bool {
        self.generation.is_some()
    }

    /// Return whether this request can be served by `current` owner generation.
    pub fn matches_generation(&self, current: ClusterGeneration) -> bool {
        self.generation_mismatch(current).is_none()
    }

    /// Return mismatch details when the observed owner generation is stale.
    pub fn generation_mismatch(
        &self,
        current: ClusterGeneration,
    ) -> Option<ClusterPeerFetchGenerationMismatch> {
        match self.generation {
            Some(requested) if requested != current => {
                Some(ClusterPeerFetchGenerationMismatch { requested, current })
            }
            _ => None,
        }
    }
}

/// Response returned by a peer-fetch implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterPeerFetchResponse {
    /// Owner member that served or attempted to serve the request.
    pub owner: ClusterNodeId,
    /// Logical cache key requested from the owner.
    pub key: String,
    /// Encoded cache value, when the owner had it.
    pub value: Option<Bytes>,
}

impl ClusterPeerFetchResponse {
    /// Create a cache-hit response.
    pub fn hit(owner: impl Into<ClusterNodeId>, key: impl Into<String>, value: Bytes) -> Self {
        Self {
            owner: owner.into(),
            key: key.into(),
            value: Some(value),
        }
    }

    /// Create a cache-miss response.
    pub fn miss(owner: impl Into<ClusterNodeId>, key: impl Into<String>) -> Self {
        Self {
            owner: owner.into(),
            key: key.into(),
            value: None,
        }
    }

    /// Return whether the owner returned a value.
    pub fn is_hit(&self) -> bool {
        self.value.is_some()
    }

    /// Return whether the owner did not have the requested value.
    pub fn is_miss(&self) -> bool {
        self.value.is_none()
    }
}

/// Transport-neutral peer-fetch seam for future owner-side value loading.
#[async_trait::async_trait]
pub trait ClusterPeerFetch: Send + Sync {
    /// Fetch an encoded value from the requested owner.
    async fn fetch(&self, request: ClusterPeerFetchRequest) -> Result<ClusterPeerFetchResponse>;
}

/// In-memory peer-fetch implementation for tests, demos, and sandbox reports.
#[derive(Debug, Clone, Default)]
pub struct InMemoryPeerFetch {
    state: Arc<Mutex<InMemoryPeerFetchState>>,
}

#[derive(Debug, Default)]
struct InMemoryPeerFetchState {
    values: BTreeMap<(ClusterNodeId, String), Bytes>,
    hits: u64,
    misses: u64,
}

/// Point-in-time counters for an [`InMemoryPeerFetch`] registry.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ClusterPeerFetchDiagnostics {
    /// Number of stored owner/key values.
    pub stored_values: usize,
    /// Number of fetch requests that returned a value.
    pub hits: u64,
    /// Number of fetch requests that did not find a value.
    pub misses: u64,
}

impl ClusterPeerFetchDiagnostics {
    /// Return total fetch requests observed by this registry.
    pub fn total_requests(&self) -> u64 {
        self.hits.saturating_add(self.misses)
    }

    /// Return the hit ratio when at least one request has been observed.
    pub fn hit_ratio(&self) -> Option<f64> {
        let total = self.total_requests();
        if total == 0 {
            None
        } else {
            Some(self.hits as f64 / total as f64)
        }
    }
}

impl InMemoryPeerFetch {
    /// Create an empty in-memory peer-fetch registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Store an encoded value for an owner/key pair.
    pub fn put(
        &self,
        owner: impl Into<ClusterNodeId>,
        key: impl Into<String>,
        value: impl Into<Bytes>,
    ) {
        self.state
            .lock()
            .expect("peer fetch state poisoned")
            .values
            .insert((owner.into(), key.into()), value.into());
    }

    /// Remove an encoded value for an owner/key pair.
    pub fn remove(&self, owner: &ClusterNodeId, key: &str) -> Option<Bytes> {
        self.state
            .lock()
            .expect("peer fetch state poisoned")
            .values
            .remove(&(owner.clone(), key.to_owned()))
    }

    /// Return the number of stored owner/key values.
    pub fn len(&self) -> usize {
        self.state
            .lock()
            .expect("peer fetch state poisoned")
            .values
            .len()
    }

    /// Return whether no values are stored.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Return current in-memory peer-fetch diagnostics.
    pub fn diagnostics(&self) -> ClusterPeerFetchDiagnostics {
        let state = self.state.lock().expect("peer fetch state poisoned");
        ClusterPeerFetchDiagnostics {
            stored_values: state.values.len(),
            hits: state.hits,
            misses: state.misses,
        }
    }
}

#[async_trait::async_trait]
impl ClusterPeerFetch for InMemoryPeerFetch {
    async fn fetch(&self, request: ClusterPeerFetchRequest) -> Result<ClusterPeerFetchResponse> {
        let mut state = self.state.lock().expect("peer fetch state poisoned");
        let value = state
            .values
            .get(&(request.owner.clone(), request.key.clone()))
            .cloned();
        if value.is_some() {
            state.hits = state.hits.saturating_add(1);
        } else {
            state.misses = state.misses.saturating_add(1);
        }

        Ok(ClusterPeerFetchResponse {
            owner: request.owner,
            key: request.key,
            value,
        })
    }
}
