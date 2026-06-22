use super::*;

/// Reason why the admission bridge ignored a discovered candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClusterAdmissionIgnoreReason {
    /// The candidate already matches authoritative metadata.
    AlreadyCurrent,
    /// The candidate role is not admitted by this bridge configuration.
    RoleDisabled,
    /// Local cache roles are never admitted into a cluster control plane.
    LocalRole,
}

/// Reason why the admission bridge rejected a discovered candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClusterAdmissionRejectReason {
    /// The candidate generation is older than authoritative metadata.
    StaleGeneration {
        /// Existing accepted generation.
        existing: ClusterGeneration,
        /// Attempted generation.
        attempted: ClusterGeneration,
    },
    /// The control plane returned an admission error.
    AdmissionError(String),
}

/// Event emitted by a cluster admission bridge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClusterAdmissionBridgeEvent {
    /// A discovery candidate was observed by the bridge.
    CandidateSeen(ClusterCandidate),
    /// A candidate was admitted by the control plane.
    CandidateAdmitted(ClusterMember),
    /// A candidate did not require a control-plane write.
    CandidateIgnored {
        /// Ignored candidate.
        candidate: ClusterCandidate,
        /// Ignore reason.
        reason: ClusterAdmissionIgnoreReason,
    },
    /// A candidate was rejected before or during admission.
    CandidateRejected {
        /// Rejected candidate.
        candidate: ClusterCandidate,
        /// Rejection reason.
        reason: ClusterAdmissionRejectReason,
    },
    /// The bridge loop stopped.
    BridgeStopped,
}

/// Lightweight counters for a cluster admission bridge.
///
/// # Example
///
/// ```rust
/// use hydracache::{
///     ClusterAdmissionBridgeDiagnostics, ClusterAdmissionBridgeEvent,
///     ClusterAdmissionIgnoreReason, ClusterCandidate,
/// };
///
/// let mut diagnostics = ClusterAdmissionBridgeDiagnostics::default();
/// let candidate = ClusterCandidate::client("client-a");
///
/// diagnostics.record_event(&ClusterAdmissionBridgeEvent::CandidateSeen(candidate.clone()));
/// diagnostics.record_event(&ClusterAdmissionBridgeEvent::CandidateIgnored {
///     candidate,
///     reason: ClusterAdmissionIgnoreReason::AlreadyCurrent,
/// });
///
/// assert_eq!(diagnostics.candidates_seen, 1);
/// assert_eq!(diagnostics.total_decisions(), 1);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClusterAdmissionBridgeDiagnostics {
    /// Number of candidate snapshots observed.
    pub candidates_seen: u64,
    /// Number of candidates admitted.
    pub candidates_admitted: u64,
    /// Number of candidates ignored without writing metadata.
    pub candidates_ignored: u64,
    /// Number of candidates rejected as stale or invalid.
    pub candidates_rejected: u64,
    /// Number of admission attempts that returned an error.
    pub admission_failures: u64,
    /// Last candidate node id observed by the bridge.
    pub last_candidate: Option<ClusterNodeId>,
    /// Last admitted node id.
    pub last_admitted: Option<ClusterNodeId>,
    /// Last error message, if any.
    pub last_error: Option<String>,
}

impl ClusterAdmissionBridgeDiagnostics {
    /// Return the total number of terminal bridge decisions.
    pub fn total_decisions(&self) -> u64 {
        self.candidates_admitted
            .saturating_add(self.candidates_ignored)
            .saturating_add(self.candidates_rejected)
    }

    /// Return whether the bridge has observed at least one candidate.
    pub fn has_seen_candidates(&self) -> bool {
        self.candidates_seen > 0
    }

    /// Return whether the bridge admitted at least one candidate.
    pub fn has_admissions(&self) -> bool {
        self.candidates_admitted > 0
    }

    /// Return whether the bridge reported any rejection or failure.
    pub fn has_issues(&self) -> bool {
        self.candidates_rejected > 0 || self.admission_failures > 0
    }

    /// Update counters from a bridge event.
    pub fn record_event(&mut self, event: &ClusterAdmissionBridgeEvent) {
        match event {
            ClusterAdmissionBridgeEvent::CandidateSeen(candidate) => {
                self.candidates_seen = self.candidates_seen.saturating_add(1);
                self.last_candidate = Some(candidate.node_id.clone());
            }
            ClusterAdmissionBridgeEvent::CandidateAdmitted(member) => {
                self.candidates_admitted = self.candidates_admitted.saturating_add(1);
                self.last_admitted = Some(member.node_id.clone());
            }
            ClusterAdmissionBridgeEvent::CandidateIgnored { candidate, .. } => {
                self.candidates_ignored = self.candidates_ignored.saturating_add(1);
                self.last_candidate = Some(candidate.node_id.clone());
            }
            ClusterAdmissionBridgeEvent::CandidateRejected { candidate, reason } => {
                self.candidates_rejected = self.candidates_rejected.saturating_add(1);
                self.last_candidate = Some(candidate.node_id.clone());
                if let ClusterAdmissionRejectReason::AdmissionError(error) = reason {
                    self.admission_failures = self.admission_failures.saturating_add(1);
                    self.last_error = Some(error.clone());
                }
            }
            ClusterAdmissionBridgeEvent::BridgeStopped => {}
        }
    }
}

/// Polling behavior for [`ClusterAdmissionBridge`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClusterAdmissionBridgeConfig {
    /// How often the background task should poll discovery candidates.
    pub poll_interval: Duration,
    /// Whether client candidates should be admitted.
    pub admit_clients: bool,
    /// Whether member candidates should be admitted.
    pub admit_members: bool,
}

impl ClusterAdmissionBridgeConfig {
    /// Return config with a custom polling interval.
    pub fn poll_interval(mut self, poll_interval: Duration) -> Self {
        self.poll_interval = poll_interval;
        self
    }

    /// Enable or disable client admission.
    pub fn admit_clients(mut self, admit_clients: bool) -> Self {
        self.admit_clients = admit_clients;
        self
    }

    /// Enable or disable member admission.
    pub fn admit_members(mut self, admit_members: bool) -> Self {
        self.admit_members = admit_members;
        self
    }

    pub(super) fn normalized_poll_interval(self) -> Duration {
        if self.poll_interval.is_zero() {
            Duration::from_millis(1)
        } else {
            self.poll_interval
        }
    }
}

impl Default for ClusterAdmissionBridgeConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_secs(1),
            admit_clients: true,
            admit_members: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ClusterAdmissionSnapshot {
    generation: ClusterGeneration,
    role: ClusterRole,
}

#[derive(Debug)]
struct ClusterAdmissionBridgeState {
    admitted: BTreeMap<ClusterNodeId, ClusterAdmissionSnapshot>,
    events: Vec<ClusterAdmissionBridgeEvent>,
    diagnostics: ClusterAdmissionBridgeDiagnostics,
    lifecycle: ClusterLifecycleDiagnostics,
}

impl Default for ClusterAdmissionBridgeState {
    fn default() -> Self {
        Self {
            admitted: BTreeMap::new(),
            events: Vec::new(),
            diagnostics: ClusterAdmissionBridgeDiagnostics::default(),
            lifecycle: ClusterLifecycleDiagnostics::idle("cluster-admission-bridge"),
        }
    }
}

#[derive(Debug)]
struct ClusterAdmissionBridgeInner {
    discovery: Arc<dyn ClusterDiscovery>,
    control_plane: Arc<dyn ClusterControlPlane>,
    config: ClusterAdmissionBridgeConfig,
    state: Mutex<ClusterAdmissionBridgeState>,
    run_lock: tokio::sync::Mutex<()>,
}

/// Polls discovery candidates and admits them into an authoritative control plane.
///
/// The bridge is the seam between gossip-style discovery and Raft-style
/// metadata. Discovery can be eventually consistent and noisy; the bridge keeps
/// a local admission snapshot so repeated polls do not rewrite the same
/// generation, and only the control plane decides whether a candidate is truly
/// accepted.
///
/// # Example
///
/// ```rust
/// use std::sync::Arc;
///
/// use hydracache::{
///     ClusterAdmissionBridge, ClusterCandidate, InMemoryCluster,
///     InMemoryClusterDiscovery,
/// };
///
/// # #[tokio::main]
/// # async fn main() -> hydracache::CacheResult<()> {
/// let discovery = Arc::new(InMemoryClusterDiscovery::new());
/// let control_plane = Arc::new(InMemoryCluster::new("orders"));
/// let bridge = ClusterAdmissionBridge::new(discovery.clone(), control_plane.clone());
///
/// discovery.announce(ClusterCandidate::member("member-a"));
/// assert_eq!(bridge.run_once().await, 1);
/// assert_eq!(control_plane.members().len(), 1);
/// assert_eq!(bridge.diagnostics().candidates_admitted, 1);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct ClusterAdmissionBridge {
    inner: Arc<ClusterAdmissionBridgeInner>,
}

impl ClusterAdmissionBridge {
    /// Create a bridge with default polling behavior.
    pub fn new(
        discovery: Arc<dyn ClusterDiscovery>,
        control_plane: Arc<dyn ClusterControlPlane>,
    ) -> Self {
        Self::with_config(
            discovery,
            control_plane,
            ClusterAdmissionBridgeConfig::default(),
        )
    }

    /// Create a bridge with explicit polling behavior.
    pub fn with_config(
        discovery: Arc<dyn ClusterDiscovery>,
        control_plane: Arc<dyn ClusterControlPlane>,
        config: ClusterAdmissionBridgeConfig,
    ) -> Self {
        Self {
            inner: Arc::new(ClusterAdmissionBridgeInner {
                discovery,
                control_plane,
                config,
                state: Mutex::new(ClusterAdmissionBridgeState::default()),
                run_lock: tokio::sync::Mutex::new(()),
            }),
        }
    }

    /// Return this bridge config.
    pub fn config(&self) -> ClusterAdmissionBridgeConfig {
        self.inner.config
    }

    /// Return a point-in-time diagnostics snapshot.
    pub fn diagnostics(&self) -> ClusterAdmissionBridgeDiagnostics {
        self.inner
            .state
            .lock()
            .expect("cluster admission bridge state poisoned")
            .diagnostics
            .clone()
    }

    /// Return lifecycle diagnostics for the background polling loop.
    ///
    /// `run_once` does not change lifecycle status. The lifecycle snapshot
    /// describes only the optional background task started by [`Self::start`].
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::sync::Arc;
    ///
    /// use hydracache::{
    ///     ClusterAdmissionBridge, ClusterLifecycleStatus, InMemoryCluster,
    ///     InMemoryClusterDiscovery,
    /// };
    ///
    /// let discovery = Arc::new(InMemoryClusterDiscovery::new());
    /// let control_plane = Arc::new(InMemoryCluster::new("orders"));
    /// let bridge = ClusterAdmissionBridge::new(discovery, control_plane);
    ///
    /// assert_eq!(
    ///     bridge.lifecycle_diagnostics().status,
    ///     ClusterLifecycleStatus::Idle,
    /// );
    /// ```
    pub fn lifecycle_diagnostics(&self) -> ClusterLifecycleDiagnostics {
        self.inner
            .state
            .lock()
            .expect("cluster admission bridge state poisoned")
            .lifecycle
            .clone()
    }

    /// Return bridge events recorded so far.
    pub fn events(&self) -> Vec<ClusterAdmissionBridgeEvent> {
        self.inner
            .state
            .lock()
            .expect("cluster admission bridge state poisoned")
            .events
            .clone()
    }

    /// Poll discovery once and try to admit every latest candidate snapshot.
    ///
    /// The return value is the number of candidate snapshots processed.
    pub async fn run_once(&self) -> usize {
        let _guard = self.inner.run_lock.lock().await;
        let candidates = self.inner.discovery.candidates();
        let processed = candidates.len();
        for candidate in candidates {
            self.admit_candidate(candidate).await;
        }
        processed
    }

    /// Start a background polling loop.
    ///
    /// Use [`ClusterAdmissionBridgeHandle::shutdown`] to stop the loop
    /// gracefully. Dropping the handle also asks the task to stop, but does not
    /// wait for it.
    pub fn start(&self) -> ClusterAdmissionBridgeHandle {
        let bridge = self.clone();
        let (shutdown, mut shutdown_rx) = tokio::sync::watch::channel(false);
        self.record_lifecycle_start();
        let task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(bridge.config().normalized_poll_interval());
            loop {
                tokio::select! {
                    changed = shutdown_rx.changed() => {
                        if changed.is_err() || *shutdown_rx.borrow() {
                            bridge.record_event(ClusterAdmissionBridgeEvent::BridgeStopped);
                            bridge.record_lifecycle_graceful_stop();
                            break;
                        }
                    }
                    _ = interval.tick() => {
                        bridge.run_once().await;
                    }
                }
            }
        });

        ClusterAdmissionBridgeHandle {
            bridge: self.clone(),
            shutdown: Some(shutdown),
            task: Some(task),
        }
    }

    async fn admit_candidate(&self, candidate: ClusterCandidate) {
        self.record_event(ClusterAdmissionBridgeEvent::CandidateSeen(
            candidate.clone(),
        ));

        if let Some(event) = self.pre_admission_event(&candidate) {
            self.record_event(event);
            return;
        }

        let result = match candidate.role {
            ClusterRole::Member => {
                self.inner
                    .control_plane
                    .join_member(candidate.clone())
                    .await
            }
            ClusterRole::Client => {
                self.inner
                    .control_plane
                    .join_client(candidate.clone())
                    .await
            }
            ClusterRole::Local => unreachable!("local candidates are ignored before admission"),
        };

        match result {
            Ok(member) => self.record_admitted(member),
            Err(error) => self.record_event(ClusterAdmissionBridgeEvent::CandidateRejected {
                candidate,
                reason: ClusterAdmissionRejectReason::AdmissionError(error.to_string()),
            }),
        }
    }

    fn pre_admission_event(
        &self,
        candidate: &ClusterCandidate,
    ) -> Option<ClusterAdmissionBridgeEvent> {
        let ignore_reason = match candidate.role {
            ClusterRole::Local => Some(ClusterAdmissionIgnoreReason::LocalRole),
            ClusterRole::Client if !self.inner.config.admit_clients => {
                Some(ClusterAdmissionIgnoreReason::RoleDisabled)
            }
            ClusterRole::Member if !self.inner.config.admit_members => {
                Some(ClusterAdmissionIgnoreReason::RoleDisabled)
            }
            ClusterRole::Client | ClusterRole::Member => None,
        };
        if let Some(reason) = ignore_reason {
            return Some(ClusterAdmissionBridgeEvent::CandidateIgnored {
                candidate: candidate.clone(),
                reason,
            });
        }

        let state = self
            .inner
            .state
            .lock()
            .expect("cluster admission bridge state poisoned");
        let existing = state.admitted.get(&candidate.node_id)?;

        if existing.generation > candidate.generation {
            return Some(ClusterAdmissionBridgeEvent::CandidateRejected {
                candidate: candidate.clone(),
                reason: ClusterAdmissionRejectReason::StaleGeneration {
                    existing: existing.generation,
                    attempted: candidate.generation,
                },
            });
        }

        if existing.generation == candidate.generation && existing.role == candidate.role {
            return Some(ClusterAdmissionBridgeEvent::CandidateIgnored {
                candidate: candidate.clone(),
                reason: ClusterAdmissionIgnoreReason::AlreadyCurrent,
            });
        }

        None
    }

    fn record_admitted(&self, member: ClusterMember) {
        let mut state = self
            .inner
            .state
            .lock()
            .expect("cluster admission bridge state poisoned");
        state.admitted.insert(
            member.node_id.clone(),
            ClusterAdmissionSnapshot {
                generation: member.generation,
                role: member.role,
            },
        );
        let event = ClusterAdmissionBridgeEvent::CandidateAdmitted(member);
        state.diagnostics.record_event(&event);
        state.events.push(event);
    }

    fn record_event(&self, event: ClusterAdmissionBridgeEvent) {
        let mut state = self
            .inner
            .state
            .lock()
            .expect("cluster admission bridge state poisoned");
        state.diagnostics.record_event(&event);
        state.events.push(event);
    }

    fn record_lifecycle_start(&self) {
        self.inner
            .state
            .lock()
            .expect("cluster admission bridge state poisoned")
            .lifecycle
            .record_start();
    }

    fn record_lifecycle_shutdown_requested(&self) {
        self.inner
            .state
            .lock()
            .expect("cluster admission bridge state poisoned")
            .lifecycle
            .record_shutdown_requested();
    }

    fn record_lifecycle_graceful_stop(&self) {
        self.inner
            .state
            .lock()
            .expect("cluster admission bridge state poisoned")
            .lifecycle
            .record_graceful_stop();
    }

    fn record_lifecycle_failure(&self, error: impl Into<String>) {
        self.inner
            .state
            .lock()
            .expect("cluster admission bridge state poisoned")
            .lifecycle
            .record_failure(error);
    }
}

/// Handle for a background [`ClusterAdmissionBridge`] polling task.
#[must_use]
#[derive(Debug)]
pub struct ClusterAdmissionBridgeHandle {
    bridge: ClusterAdmissionBridge,
    shutdown: Option<tokio::sync::watch::Sender<bool>>,
    task: Option<tokio::task::JoinHandle<()>>,
}

impl ClusterAdmissionBridgeHandle {
    /// Ask the polling task to stop and wait until it exits.
    pub async fn shutdown(mut self) {
        self.bridge.record_lifecycle_shutdown_requested();
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(true);
        }
        if let Some(task) = self.task.take() {
            if let Err(error) = task.await {
                self.bridge.record_lifecycle_failure(error.to_string());
            }
        }
    }
}

impl Drop for ClusterAdmissionBridgeHandle {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            self.bridge.record_lifecycle_shutdown_requested();
            let _ = shutdown.send(true);
        }
    }
}
