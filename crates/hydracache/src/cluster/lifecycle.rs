use super::*;

/// Lifecycle state for an embedded cluster component.
///
/// The lifecycle model is diagnostic, not supervisory: HydraCache records what
/// happened, while the application still decides when to start HTTP servers,
/// admission bridges, or other background work.
///
/// # Example
///
/// ```rust
/// use hydracache::{ClusterLifecycleDiagnostics, ClusterLifecycleStatus};
///
/// let mut lifecycle = ClusterLifecycleDiagnostics::idle("admission-bridge");
/// assert_eq!(lifecycle.status, ClusterLifecycleStatus::Idle);
///
/// lifecycle.record_start();
/// assert!(lifecycle.is_running());
///
/// lifecycle.record_shutdown_requested();
/// lifecycle.record_graceful_stop();
/// assert!(lifecycle.is_stopped());
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClusterLifecycleStatus {
    /// The component has not been started yet.
    #[default]
    Idle,
    /// The component is currently running.
    Running,
    /// Shutdown was requested, but the component has not reported completion.
    Stopping,
    /// The component stopped gracefully.
    Stopped,
    /// The component failed.
    Failed,
}

impl ClusterLifecycleStatus {
    /// Return whether this status represents active work.
    pub fn is_running(self) -> bool {
        self == Self::Running
    }

    /// Return whether shutdown was requested and completion is pending.
    pub fn is_stopping(self) -> bool {
        self == Self::Stopping
    }

    /// Return whether this status represents a graceful stop.
    pub fn is_stopped(self) -> bool {
        self == Self::Stopped
    }

    /// Return whether this status represents a failure.
    pub fn has_failed(self) -> bool {
        self == Self::Failed
    }

    /// Return whether no more work is expected for this lifecycle instance.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Stopped | Self::Failed)
    }
}

/// Point-in-time lifecycle diagnostics for an embedded cluster component.
///
/// # Example
///
/// ```rust
/// use hydracache::ClusterLifecycleDiagnostics;
///
/// let mut lifecycle = ClusterLifecycleDiagnostics::idle("peer-fetch-service");
/// lifecycle.record_start();
/// lifecycle.record_failure("listener closed unexpectedly");
///
/// assert!(lifecycle.has_failed());
/// assert_eq!(lifecycle.start_count, 1);
/// assert_eq!(
///     lifecycle.last_error.as_deref(),
///     Some("listener closed unexpectedly"),
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClusterLifecycleDiagnostics {
    /// Stable component name used in diagnostics and sandbox reports.
    pub component: String,
    /// Current lifecycle status.
    pub status: ClusterLifecycleStatus,
    /// Number of recorded starts.
    pub start_count: u64,
    /// Number of graceful stops.
    pub stop_count: u64,
    /// Whether shutdown was requested at least once.
    pub shutdown_requested: bool,
    /// Last lifecycle error, if any.
    pub last_error: Option<String>,
}

impl ClusterLifecycleDiagnostics {
    /// Create an idle lifecycle snapshot for a component.
    pub fn idle(component: impl Into<String>) -> Self {
        Self {
            component: component.into(),
            status: ClusterLifecycleStatus::Idle,
            start_count: 0,
            stop_count: 0,
            shutdown_requested: false,
            last_error: None,
        }
    }

    /// Create a running lifecycle snapshot for a component.
    pub fn running(component: impl Into<String>) -> Self {
        let mut lifecycle = Self::idle(component);
        lifecycle.record_start();
        lifecycle
    }

    /// Record that the component started.
    pub fn record_start(&mut self) {
        self.status = ClusterLifecycleStatus::Running;
        self.start_count = self.start_count.saturating_add(1);
        self.shutdown_requested = false;
        self.last_error = None;
    }

    /// Record that shutdown was requested.
    pub fn record_shutdown_requested(&mut self) {
        self.shutdown_requested = true;
        if !self.status.is_terminal() {
            self.status = ClusterLifecycleStatus::Stopping;
        }
    }

    /// Record a graceful stop.
    pub fn record_graceful_stop(&mut self) {
        self.status = ClusterLifecycleStatus::Stopped;
        self.stop_count = self.stop_count.saturating_add(1);
        self.shutdown_requested = true;
    }

    /// Record a component failure.
    pub fn record_failure(&mut self, error: impl Into<String>) {
        self.status = ClusterLifecycleStatus::Failed;
        self.last_error = Some(error.into());
    }

    /// Return whether this component is currently running.
    pub fn is_running(&self) -> bool {
        self.status.is_running()
    }

    /// Return whether this component is stopping.
    pub fn is_stopping(&self) -> bool {
        self.status.is_stopping()
    }

    /// Return whether this component stopped gracefully.
    pub fn is_stopped(&self) -> bool {
        self.status.is_stopped()
    }

    /// Return whether this component failed.
    pub fn has_failed(&self) -> bool {
        self.status.has_failed()
    }

    /// Return whether this component reached a terminal status.
    pub fn is_terminal(&self) -> bool {
        self.status.is_terminal()
    }
}

/// Error returned by a background cluster component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterComponentError {
    component: &'static str,
    message: String,
}

impl ClusterComponentError {
    /// Create a component error with a stable component name.
    pub fn new(component: &'static str, message: impl Into<String>) -> Self {
        Self {
            component,
            message: message.into(),
        }
    }

    /// Return the stable component name.
    pub fn component(&self) -> &'static str {
        self.component
    }

    /// Return the human-readable error message.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for ClusterComponentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "cluster component '{}' failed: {}",
            self.component, self.message
        )
    }
}

impl std::error::Error for ClusterComponentError {}

/// Uniform lifecycle surface for background cluster components.
#[async_trait::async_trait]
pub trait ClusterComponent: Send + Sync {
    /// Stable name for diagnostics.
    fn name(&self) -> &'static str;

    /// Start background work. Implementations should be idempotent.
    async fn start(&self) -> std::result::Result<(), ClusterComponentError>;

    /// Request a graceful stop. Implementations should be idempotent.
    async fn stop(&self) -> std::result::Result<(), ClusterComponentError>;

    /// Return a point-in-time lifecycle diagnostics snapshot.
    fn diagnostics(&self) -> ClusterLifecycleDiagnostics;

    /// Return the most recent error, if one was recorded.
    fn last_error(&self) -> Option<String>;
}

/// Minimal reusable lifecycle component for adapters that already own work.
#[derive(Debug, Clone)]
pub struct ClusterLifecycleComponent {
    name: &'static str,
    state: Arc<Mutex<ClusterLifecycleDiagnostics>>,
}

impl ClusterLifecycleComponent {
    /// Create an idle component with a stable diagnostic name.
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            state: Arc::new(Mutex::new(ClusterLifecycleDiagnostics::idle(name))),
        }
    }

    /// Record a component failure.
    pub fn fail(&self, message: impl Into<String>) {
        self.state
            .lock()
            .expect("cluster component lifecycle poisoned")
            .record_failure(message);
    }
}

#[async_trait::async_trait]
impl ClusterComponent for ClusterLifecycleComponent {
    fn name(&self) -> &'static str {
        self.name
    }

    async fn start(&self) -> std::result::Result<(), ClusterComponentError> {
        let mut diagnostics = self
            .state
            .lock()
            .expect("cluster component lifecycle poisoned");
        if !diagnostics.is_running() {
            diagnostics.record_start();
        }
        Ok(())
    }

    async fn stop(&self) -> std::result::Result<(), ClusterComponentError> {
        let mut diagnostics = self
            .state
            .lock()
            .expect("cluster component lifecycle poisoned");
        if !diagnostics.is_stopped() {
            diagnostics.record_shutdown_requested();
            diagnostics.record_graceful_stop();
        }
        Ok(())
    }

    fn diagnostics(&self) -> ClusterLifecycleDiagnostics {
        self.state
            .lock()
            .expect("cluster component lifecycle poisoned")
            .clone()
    }

    fn last_error(&self) -> Option<String> {
        self.diagnostics().last_error
    }
}
