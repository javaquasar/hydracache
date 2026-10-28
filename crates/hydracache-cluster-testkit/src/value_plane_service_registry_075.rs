//! Local-only authenticated namespace routing and admission over real unified services.
//!
//! No value plane is copied here. Registry quotas count the service's actual owners,
//! queued command payload, and conservative pre-dispatch growth reservations. Namespace
//! identity slots remain bounded and retain their generation across local recreation.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, Weak};
use std::task::{Context, Poll};

use crate::value_plane_admission_075::{
    AdmissionError, AdmissionLimits, DataWork, FairAdmissionQueue, NamespaceError,
    NamespaceLifecycle, NamespacePhase, ReclamationOwners, ReclamationWatermarks, WorkItem,
};
use crate::value_plane_cluster_admission_075::{
    ClusterAdmission, ClusterAdmissionError, ClusterAdmissionObservations, ClusterLane,
    ClusterPermit, ClusterWork,
};
use crate::value_plane_cluster_health_075::{MaintenanceResult, RuntimeHealth, RuntimeMaintenance};
use crate::value_plane_cluster_runtime_075::{
    ClusterNamespaceRuntime, ClusterRuntimeConfig, RuntimeCallOptions,
};
use crate::value_plane_collection_075::{CollectionBounds, ScanProjection};
use crate::value_plane_model_075::{MutationIdentity, MutationOperation, TtlDirective};
use crate::value_plane_security_075::{
    validate_route, GrantKeys, GrantOperation, GrantRights, NamespaceGrant, RouteClaims,
    SecurityBounds, SecurityError, TrustEpochWindow,
};
use crate::value_plane_service_075::{
    ListenerSnapshot, MapServiceError, NamespaceOwners, ServiceCommand, ServiceEvent, ServiceItem,
    ServiceLimits, ServiceListenerOptions, ServiceListenerSource, ServicePageSource,
    ServiceResourceUsage, ServiceResult, ServiceStatistics, ServiceTtl, UnifiedIMapService,
};
use crate::value_plane_service_cluster_075::{
    ClusterListenerSource, ClusterSubscriptionEvent, ClusterSubscriptionOptions,
    ClusterSubscriptionSnapshot, ServiceClusterError, ServiceClusterExecution,
};
use crate::value_plane_stream_075::{BatchPoll, BoundedStreamSource};
use crate::value_plane_surface_075::{
    IngressPrincipal, SurfaceId, SurfaceOperation, VerifiedMapCommand,
};

static NEXT_REGISTRY: AtomicU64 = AtomicU64::new(1);

#[path = "value_plane_service_registry_invocation_075.rs"]
mod invocation;
pub use invocation::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegistryBounds {
    pub max_tenants: usize,
    pub max_maps: usize,
    pub max_maps_per_tenant: usize,
    pub max_identifier_bytes: usize,
    pub max_metadata_bytes: usize,
    pub max_owned_bytes: usize,
    pub max_tenant_owned_bytes: usize,
    /// Data command payload; the safety lane separately owns at most 128 bytes per slot.
    pub max_queued_bytes: usize,
    pub max_command_bytes: usize,
    pub max_cleanup_work: usize,
    pub partitions: u32,
    pub admission: AdmissionLimits,
}

impl Default for RegistryBounds {
    fn default() -> Self {
        Self {
            max_tenants: 16,
            max_maps: 128,
            max_maps_per_tenant: 16,
            max_identifier_bytes: 128,
            max_metadata_bytes: 128 * 1024,
            max_owned_bytes: 128 * 1024 * 1024,
            max_tenant_owned_bytes: 64 * 1024 * 1024,
            max_queued_bytes: 8 * 1024 * 1024,
            max_command_bytes: 1024 * 1024,
            max_cleanup_work: 256,
            partitions: 17,
            admission: AdmissionLimits {
                total: 256,
                per_tenant: 64,
                per_partition: 16,
                safety: 16,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    InvalidBound(&'static str),
    Security(SecurityError),
    IdentitySubstitution(&'static str),
    WrongRegistry,
    UnknownNamespace,
    RegistryGone,
    DispatcherBusy,
    SafetyLaneRequired,
    Namespace(NamespaceError),
    Admission(AdmissionError),
    Service(MapServiceError),
    Cluster(ServiceClusterError),
    ClusterAdmission(ClusterAdmissionError),
    Quota {
        dimension: &'static str,
        maximum: usize,
        actual: usize,
    },
}

impl fmt::Display for RegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}
impl std::error::Error for RegistryError {}
impl From<SecurityError> for RegistryError {
    fn from(error: SecurityError) -> Self {
        Self::Security(error)
    }
}
impl From<NamespaceError> for RegistryError {
    fn from(error: NamespaceError) -> Self {
        Self::Namespace(error)
    }
}
impl From<AdmissionError> for RegistryError {
    fn from(error: AdmissionError) -> Self {
        Self::Admission(error)
    }
}
impl From<MapServiceError> for RegistryError {
    fn from(error: MapServiceError) -> Self {
        Self::Service(error)
    }
}
impl From<ServiceClusterError> for RegistryError {
    fn from(error: ServiceClusterError) -> Self {
        Self::Cluster(error)
    }
}
impl From<ClusterAdmissionError> for RegistryError {
    fn from(error: ClusterAdmissionError) -> Self {
        Self::ClusterAdmission(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NamespaceAddress {
    pub tenant: String,
    pub namespace: String,
    pub generation: u64,
}

/// Verification copies the bounded namespace identity once; later request fields cannot
/// replace it. Trust epoch, deadline, topology, and generation are checked again on use.
#[derive(Debug, Clone)]
pub struct AuthenticatedNamespace {
    grant: Option<Arc<NamespaceGrant>>,
    address: NamespaceAddress,
    trust_epoch: u64,
    topology_epoch: u64,
    deadline: u64,
    proxy_hops: u8,
    envelope_bytes: usize,
    key_bytes: usize,
    value_bytes: usize,
}

impl AuthenticatedNamespace {
    pub fn verify(
        principal: &IngressPrincipal,
        verified: &VerifiedMapCommand,
        claims: &RouteClaims<'_>,
        bounds: SecurityBounds,
        trust: TrustEpochWindow,
        now: u64,
    ) -> Result<Self, RegistryError> {
        validate_route(claims, bounds, trust, now)?;
        if principal.tenant != verified.tenant || verified.tenant != claims.authenticated_tenant {
            return Err(RegistryError::IdentitySubstitution("tenant"));
        }
        if verified.namespace.is_empty() || verified.tenant.is_empty() {
            return Err(RegistryError::IdentitySubstitution("namespace"));
        }
        if verified.namespace_generation == 0
            || verified.namespace_generation != claims.actual_namespace_generation
        {
            return Err(RegistryError::IdentitySubstitution("generation"));
        }
        if verified
            .tenant
            .len()
            .saturating_add(verified.namespace.len())
            > bounds.max_envelope_bytes
        {
            return Err(SecurityError::EnvelopeTooLarge {
                limit: bounds.max_envelope_bytes,
                actual: verified
                    .tenant
                    .len()
                    .saturating_add(verified.namespace.len()),
            }
            .into());
        }
        let deadline = now
            .checked_add(claims.deadline_remaining_ticks)
            .ok_or(RegistryError::Security(SecurityError::DeadlineExpired))?;
        Ok(Self {
            grant: None,
            address: NamespaceAddress {
                tenant: verified.tenant.clone(),
                namespace: verified.namespace.clone(),
                generation: verified.namespace_generation,
            },
            trust_epoch: claims.trust_epoch,
            topology_epoch: claims.actual_topology_epoch,
            deadline,
            proxy_hops: claims.proxy_hops,
            envelope_bytes: claims.encoded_envelope_bytes,
            key_bytes: claims.key_bytes,
            value_bytes: claims.value_bytes,
        })
    }
    pub fn address(&self) -> &NamespaceAddress {
        &self.address
    }
    /// The caller supplying this immutable grant is the local trusted policy resolver.
    pub fn with_grant(mut self, grant: NamespaceGrant) -> Result<Self, RegistryError> {
        if !grant.matches(
            &self.address.tenant,
            &self.address.namespace,
            self.address.generation,
        ) || self.grant.is_some()
        {
            return Err(SecurityError::PermissionDenied.into());
        }
        self.grant = Some(Arc::new(grant));
        Ok(self)
    }
    pub fn attenuate(&self, rights: GrantRights, keys: GrantKeys) -> Result<Self, RegistryError> {
        let mut next = self.clone();
        next.grant = Some(Arc::new(
            self.grant
                .as_ref()
                .ok_or(SecurityError::PermissionDenied)?
                .attenuate(rights, keys)?,
        ));
        Ok(next)
    }
    pub fn grant_digest(&self) -> Option<[u8; 32]> {
        self.grant.as_ref().map(|grant| grant.digest())
    }
    pub fn policy_epoch(&self) -> Option<u64> {
        self.grant.as_ref().map(|grant| grant.epoch())
    }
    pub(crate) fn authorize_command(&self, command: &ServiceCommand) -> Result<(), RegistryError> {
        let Some(grant) = &self.grant else {
            return Ok(());
        };
        let operation = match command.operation.as_str() {
            "entry_listener" | "remove_entry_listener" => GrantOperation::Listen,
            "clear_detailed" | "destroy" | "evict_all_detailed" => GrantOperation::Admin,
            op if crate::value_plane_service_075::is_mutating_operation(op) => {
                GrantOperation::Write
            }
            _ => GrantOperation::Read,
        };
        if matches!(
            command.operation.as_str(),
            "get_and_put"
                | "get_and_remove"
                | "replace_if_present"
                | "conditional_replace"
                | "conditional_remove"
                | "replace_if_version"
                | "put_if_absent"
                | "remove_all_detailed"
        ) {
            grant.authorize(GrantOperation::Read, &runtime_command_keys(command), false)?;
        }
        if command.operation == "entry_listener" {
            let filter = command.expected.as_ref().unwrap_or(&command.key);
            grant.authorize(operation, std::slice::from_ref(filter), filter.is_empty())?;
            if command.include_value {
                grant.authorize(
                    GrantOperation::Read,
                    std::slice::from_ref(filter),
                    filter.is_empty(),
                )?;
            }
        } else {
            let map_wide = matches!(
                command.operation.as_str(),
                "size"
                    | "is_empty"
                    | "contains_value_bounded"
                    | "key_set_page"
                    | "values_page"
                    | "entry_set_page"
                    | "clear_detailed"
                    | "destroy"
                    | "evict_all_detailed"
                    | "query_operation_outcome"
                    | "local_statistics"
                    | "remove_entry_listener"
            );
            grant.authorize(operation, &runtime_command_keys(command), map_wide)?;
        }
        Ok(())
    }
    pub(crate) fn authorize(
        &self,
        operation: GrantOperation,
        keys: &[Vec<u8>],
        map_wide: bool,
    ) -> Result<(), RegistryError> {
        if let Some(grant) = &self.grant {
            grant.authorize(operation, keys, map_wide)?;
        }
        Ok(())
    }
    pub(crate) fn authorize_listener(
        &self,
        options: &ServiceListenerOptions,
    ) -> Result<(), RegistryError> {
        let keys = std::slice::from_ref(options.exact_key.as_ref().unwrap_or(&options.key_prefix));
        let wide = options.exact_key.is_none() && options.key_prefix.is_empty();
        self.authorize(GrantOperation::Listen, keys, wide)?;
        if options.include_value {
            self.authorize(GrantOperation::Read, keys, wide)?;
        }
        Ok(())
    }
    pub(crate) fn runtime_proof(&self) -> (String, String, u64, u64, u64) {
        (
            self.address.tenant.clone(),
            self.address.namespace.clone(),
            self.address.generation,
            self.trust_epoch,
            self.deadline,
        )
    }
}

#[derive(Debug, Clone)]
pub struct RegistryHandle {
    registry: u64,
    address: NamespaceAddress,
}
impl RegistryHandle {
    pub fn address(&self) -> &NamespaceAddress {
        &self.address
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestTicket(u64);
impl RequestTicket {
    pub const fn id(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamespaceCleanup {
    pub phase: NamespacePhase,
    pub reclaimed: usize,
    pub remaining: usize,
    pub blocked_owners: usize,
    pub queued_requests: usize,
    pub owners: NamespaceOwners,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryResult {
    Command(ServiceResult),
    ClusterCommand(ServiceClusterExecution),
    Maintenance(MaintenanceResult),
    DeleteStarted,
    Cleanup(NamespaceCleanup),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchRecord {
    pub ticket: RequestTicket,
    pub address: NamespaceAddress,
    pub surface: Option<SurfaceId>,
    pub result: Result<RegistryResult, RegistryError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamespaceDiagnostics {
    pub phase: NamespacePhase,
    pub logical_time: u64,
    pub statistics: ServiceStatistics,
    pub resources: ServiceResourceUsage,
    pub owners: NamespaceOwners,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegistryUsage {
    pub tenants: usize,
    pub maps: usize,
    pub metadata_bytes: usize,
    pub owned_bytes: usize,
    pub queued_bytes: usize,
    pub queued_requests: usize,
    pub reserved_growth: usize,
}

struct NamespaceOwner {
    lifecycle: NamespaceLifecycle,
    service: NamespaceBackend,
}
#[derive(Clone)]
enum NamespaceBackend {
    Local(UnifiedIMapService),
    Cluster(ClusterNamespaceRuntime),
}
impl NamespaceBackend {
    fn logical_time(&self) -> u64 {
        match self {
            Self::Local(service) => service.logical_time(),
            Self::Cluster(service) => service.logical_time(),
        }
    }
    fn statistics(&self) -> ServiceStatistics {
        match self {
            Self::Local(service) => service.statistics(),
            Self::Cluster(service) => service.statistics(),
        }
    }
    fn resource_usage(&self) -> ServiceResourceUsage {
        match self {
            Self::Local(service) => service.resource_usage(),
            Self::Cluster(service) => service.resource_usage(),
        }
    }
    fn namespace_owners(&self) -> NamespaceOwners {
        match self {
            Self::Local(service) => service.namespace_owners(),
            Self::Cluster(service) => service.namespace_owners(),
        }
    }
    fn revision(&self) -> u64 {
        match self {
            Self::Local(service) => service.revision(),
            Self::Cluster(service) => service.revision(),
        }
    }
    fn preflight_registry_growth(&self, command: &ServiceCommand) -> Result<usize, RegistryError> {
        match self {
            Self::Local(service) => Ok(service.preflight_registry_growth(command)?),
            Self::Cluster(service) => Ok(service.preflight_registry_growth(command)?),
        }
    }
    fn execute(
        &self,
        command: ServiceCommand,
        authenticated: &AuthenticatedNamespace,
        permit: Option<&ClusterPermit>,
        options: Option<&RuntimeCallOptions>,
    ) -> Result<RegistryResult, RegistryError> {
        match self {
            Self::Local(service) => Ok(RegistryResult::Command(service.execute(command)?)),
            Self::Cluster(service) => Ok(RegistryResult::ClusterCommand(service.execute(
                authenticated,
                permit.ok_or(ServiceClusterError::Authorization("admission_binding"))?,
                command,
                options,
            )?)),
        }
    }
    fn reserve_operation(
        &self,
        identity: MutationIdentity,
        command: &ServiceCommand,
        authenticated: &AuthenticatedNamespace,
        permit: Option<&ClusterPermit>,
    ) -> Result<(), RegistryError> {
        match self {
            Self::Local(service) => Ok(service.reserve_operation(identity, command)?),
            Self::Cluster(service) => {
                if command.identity.as_ref() != Some(&identity) {
                    return Err(MapServiceError::InvalidArgument.into());
                }
                Ok(service.reserve_invocation(
                    authenticated,
                    permit.ok_or(ServiceClusterError::Authorization("admission_binding"))?,
                    command,
                    None,
                )?)
            }
        }
    }
    fn cancel_reserved(&self, identity: &MutationIdentity) -> Result<bool, RegistryError> {
        match self {
            Self::Local(service) => Ok(service.cancel_reserved(identity)?),
            Self::Cluster(service) => Ok(service.cancel_identity(identity)?),
        }
    }
    fn fence_namespace(&self) -> Result<(), RegistryError> {
        match self {
            Self::Local(service) => Ok(service.fence_namespace()?),
            Self::Cluster(service) => Ok(service.fence_namespace()?),
        }
    }
    fn reclaim_namespace_step(
        &self,
        maximum: usize,
    ) -> Result<crate::value_plane_service_075::NamespaceReclaimProgress, RegistryError> {
        match self {
            Self::Local(service) => Ok(service.reclaim_namespace_step(maximum)?),
            Self::Cluster(service) => Ok(service.reclaim_namespace_step(maximum)?),
        }
    }
    fn wake_listener_waiters(&self) {
        match self {
            Self::Local(service) => service.wake_listener_waiters(),
            Self::Cluster(service) => service.wake_listener_waiters(),
        }
    }
    fn page_source(
        &self,
        projection: ScanProjection,
        now: u64,
    ) -> Result<ServicePageSource, RegistryError> {
        match self {
            Self::Local(service) => Ok(ServicePageSource::new(service.clone(), projection, now)?),
            Self::Cluster(service) => Ok(service.page_source(projection, now)?),
        }
    }
    fn listener_source(
        &self,
        options: ServiceListenerOptions,
    ) -> Result<ServiceListenerSource, RegistryError> {
        match self {
            Self::Local(service) => Ok(ServiceListenerSource::new(service.clone(), options)?),
            Self::Cluster(service) => Ok(service.listener_source(options)?),
        }
    }
    fn resync_listener(
        &self,
        listener: u64,
        now: u64,
        maximum_items: usize,
        maximum_bytes: usize,
    ) -> Result<ListenerSnapshot, RegistryError> {
        match self {
            Self::Local(service) => {
                Ok(service.resync_listener(listener, now, maximum_items, maximum_bytes)?)
            }
            Self::Cluster(service) => {
                Ok(service.resync_listener(listener, now, maximum_items, maximum_bytes)?)
            }
        }
    }
    fn partitions(&self, command: &ServiceCommand, fallback: u32) -> Vec<u32> {
        match self {
            Self::Local(_) => vec![partition(command, fallback)],
            Self::Cluster(service) => service.affected_partitions(command),
        }
    }
    fn replica_bound(&self, command: &ServiceCommand) -> Result<usize, RegistryError> {
        match self {
            Self::Local(_) => Ok(0),
            Self::Cluster(service) => Ok(service.command_replica_bound(command)?),
        }
    }
    fn maintenance_growth(&self, operation: &RuntimeMaintenance) -> usize {
        if operation.repair_lane() {
            usage_bytes(self.resource_usage()).saturating_add(512)
        } else {
            0
        }
    }
}
#[derive(Clone)]
enum JobKind {
    Execute {
        cooperative: bool,
        surface: SurfaceId,
        command: ServiceCommand,
        options: Option<RuntimeCallOptions>,
    },
    Reserve(ServiceCommand),
    BeginDelete,
    Reclaim(usize),
    Maintenance(RuntimeMaintenance),
}
#[derive(Clone)]
struct QueuedJob {
    handle: RegistryHandle,
    authenticated: AuthenticatedNamespace,
    kind: JobKind,
    bytes: usize,
    growth: usize,
    admission: Option<u64>,
}
struct RegistryState {
    active_invocations: BTreeMap<u64, (NamespaceAddress, usize, usize)>,
    policy_epoch: Option<u64>,
    id: u64,
    bounds: RegistryBounds,
    collection: CollectionBounds,
    service_limits: ServiceLimits,
    security: SecurityBounds,
    trust: TrustEpochWindow,
    topology_epoch: u64,
    now: u64,
    tenants: BTreeSet<String>,
    maps: BTreeMap<(String, String), NamespaceOwner>,
    queue: FairAdmissionQueue,
    jobs: BTreeMap<u64, QueuedJob>,
    next_request: u64,
    queued_bytes: usize,
    reserved_growth: usize,
    metadata_bytes: usize,
    transient: Option<(String, usize)>,
    cluster_config: Option<ClusterRuntimeConfig>,
    cluster_admission: Option<ClusterAdmission>,
}

#[derive(Clone)]
pub struct UnifiedServiceRegistry {
    state: Arc<Mutex<RegistryState>>,
    dispatching: Arc<Mutex<()>>,
}

impl UnifiedServiceRegistry {
    /// Select strict policy mode, or revoke every older epoch. This cannot revert to legacy mode.
    pub fn advance_policy_epoch(&self, next: u64) -> Result<(), RegistryError> {
        let _dispatch = self
            .dispatching
            .try_lock()
            .map_err(|_| RegistryError::DispatcherBusy)?;
        let mut state = self.state.lock().unwrap();
        if next == 0 || state.policy_epoch.is_some_and(|epoch| next <= epoch) {
            return Err(SecurityError::PolicyEpochMismatch.into());
        }
        let services = state
            .maps
            .values()
            .map(|owner| owner.service.clone())
            .collect::<Vec<_>>();
        if let Some(scheduler) = &state.cluster_admission {
            for job in state.jobs.values() {
                if let Some(ticket) = job.admission {
                    scheduler.cancel(ticket)?;
                }
            }
        }
        state.queue = FairAdmissionQueue::new(state.bounds.admission)?;
        state.jobs.clear();
        state.queued_bytes = state
            .active_invocations
            .values()
            .map(|(_, bytes, _)| *bytes)
            .sum();
        state.reserved_growth = state
            .active_invocations
            .values()
            .map(|(_, _, growth)| *growth)
            .sum();
        state.policy_epoch = Some(next);
        let scheduler = state.cluster_admission.clone();
        for service in &services {
            if let NamespaceBackend::Cluster(runtime) = service {
                runtime.authentication.set_policy_epoch(next)?;
            }
        }
        drop(state);
        drop(_dispatch);
        if let Some(scheduler) = scheduler {
            scheduler.wake_waiters();
        }
        for service in services {
            service.wake_listener_waiters();
        }
        Ok(())
    }
    pub fn new(
        bounds: RegistryBounds,
        collection: CollectionBounds,
        service_limits: ServiceLimits,
        security: SecurityBounds,
        trust: TrustEpochWindow,
    ) -> Result<Self, RegistryError> {
        for (name, value) in [
            ("tenants", bounds.max_tenants),
            ("maps", bounds.max_maps),
            ("maps_per_tenant", bounds.max_maps_per_tenant),
            ("identifier_bytes", bounds.max_identifier_bytes),
            ("metadata_bytes", bounds.max_metadata_bytes),
            ("owned_bytes", bounds.max_owned_bytes),
            ("tenant_owned_bytes", bounds.max_tenant_owned_bytes),
            ("queued_bytes", bounds.max_queued_bytes),
            ("command_bytes", bounds.max_command_bytes),
            ("cleanup_work", bounds.max_cleanup_work),
            ("partitions", bounds.partitions as usize),
        ] {
            if value == 0 || value > isize::MAX as usize {
                return Err(RegistryError::InvalidBound(name));
            }
        }
        for (name, value) in [
            ("security_envelope", security.max_envelope_bytes),
            ("security_key", security.max_key_bytes),
            ("security_value", security.max_value_bytes),
            ("security_hops", usize::from(security.max_proxy_hops)),
            ("security_replay", security.max_replay_entries),
            ("security_audit", security.max_audit_records),
            ("security_overlap", security.trust_overlap_ticks as usize),
        ] {
            if value == 0 {
                return Err(RegistryError::InvalidBound(name));
            }
        }
        // Validate the same core configuration used by every namespace, without retaining
        // an extra authoritative owner or allocating a duplicate value plane.
        UnifiedIMapService::with_limits(collection, 1, service_limits)?;
        let queue = FairAdmissionQueue::new(bounds.admission)?;
        let id = NEXT_REGISTRY
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
                current.checked_add(1)
            })
            .map_err(|_| RegistryError::InvalidBound("registry_identity"))?;
        Ok(Self {
            state: Arc::new(Mutex::new(RegistryState {
                active_invocations: BTreeMap::new(),
                policy_epoch: None,
                id,
                bounds,
                collection,
                service_limits,
                security,
                trust,
                topology_epoch: 1,
                now: 0,
                tenants: BTreeSet::new(),
                maps: BTreeMap::new(),
                queue,
                jobs: BTreeMap::new(),
                next_request: 1,
                queued_bytes: 0,
                reserved_growth: 0,
                metadata_bytes: 0,
                transient: None,
                cluster_config: None,
                cluster_admission: None,
            })),
            dispatching: Arc::new(Mutex::new(())),
        })
    }

    pub fn new_clustered(
        bounds: RegistryBounds,
        collection: CollectionBounds,
        service_limits: ServiceLimits,
        security: SecurityBounds,
        trust: TrustEpochWindow,
        config: ClusterRuntimeConfig,
    ) -> Result<Self, RegistryError> {
        if config.members.len() < 2
            || !config
                .members
                .iter()
                .any(|member| member.node == config.entry_node)
            || config.bounds.partitions == 0
        {
            return Err(ServiceClusterError::InvalidBound.into());
        }
        let admission = ClusterAdmission::new(config.admission)?;
        let registry = Self::new(bounds, collection, service_limits, security, trust)?;
        {
            let mut state = registry.state.lock().expect("registry poisoned");
            state.cluster_config = Some(config);
            state.cluster_admission = Some(admission);
        }
        Ok(registry)
    }

    pub fn open(
        &self,
        authenticated: &AuthenticatedNamespace,
        requested: &NamespaceAddress,
    ) -> Result<RegistryHandle, RegistryError> {
        let mut state = self.state.lock().expect("registry poisoned");
        validate_auth(&state, authenticated, requested)?;
        identifier(&state, &requested.tenant)?;
        identifier(&state, &requested.namespace)?;
        let key = address_key(requested);
        if let Some(owner) = state.maps.get(&key) {
            owner.lifecycle.validate_request(requested.generation)?;
            return Ok(RegistryHandle {
                registry: state.id,
                address: requested.clone(),
            });
        }
        if requested.generation != 1 {
            return Err(NamespaceError::InvalidGeneration.into());
        }
        authenticated.authorize(GrantOperation::Admin, &[], true)?;
        quota(
            "maps",
            state.bounds.max_maps,
            state.maps.len().saturating_add(1),
        )?;
        let tenant_maps = state
            .maps
            .keys()
            .filter(|(tenant, _)| tenant == &requested.tenant)
            .count();
        quota(
            "maps_per_tenant",
            state.bounds.max_maps_per_tenant,
            tenant_maps.saturating_add(1),
        )?;
        let new_tenant = !state.tenants.contains(&requested.tenant);
        quota(
            "tenants",
            state.bounds.max_tenants,
            state.tenants.len().saturating_add(usize::from(new_tenant)),
        )?;
        let metadata = 128usize
            .saturating_add(requested.tenant.len())
            .saturating_add(requested.namespace.len())
            .saturating_add(if new_tenant {
                64 + requested.tenant.len()
            } else {
                0
            });
        quota(
            "metadata_bytes",
            state.bounds.max_metadata_bytes,
            state.metadata_bytes.saturating_add(metadata),
        )?;
        let service = if let Some(config) = &state.cluster_config {
            NamespaceBackend::Cluster(ClusterNamespaceRuntime::new(
                config.clone(),
                requested,
                state.collection,
                state.service_limits,
                state.security,
                state.trust,
            )?)
        } else {
            NamespaceBackend::Local(UnifiedIMapService::with_limits(
                state.collection,
                requested.generation,
                state.service_limits,
            )?)
        };
        if let (Some(epoch), NamespaceBackend::Cluster(runtime)) = (state.policy_epoch, &service) {
            runtime.authentication.set_policy_epoch(epoch)?;
        }
        state.maps.insert(
            key,
            NamespaceOwner {
                lifecycle: NamespaceLifecycle::new(requested.generation)?,
                service,
            },
        );
        state.tenants.insert(requested.tenant.clone());
        state.metadata_bytes += metadata;
        Ok(RegistryHandle {
            registry: state.id,
            address: requested.clone(),
        })
    }

    pub fn enqueue(
        &self,
        handle: &RegistryHandle,
        authenticated: &AuthenticatedNamespace,
        surface: SurfaceId,
        command: ServiceCommand,
    ) -> Result<RequestTicket, RegistryError> {
        if command.operation == "destroy" {
            return Err(RegistryError::SafetyLaneRequired);
        }
        self.enqueue_kind(
            handle,
            authenticated,
            JobKind::Execute {
                cooperative: false,
                surface,
                command,
                options: None,
            },
        )
    }
    pub fn enqueue_cluster(
        &self,
        handle: &RegistryHandle,
        authenticated: &AuthenticatedNamespace,
        surface: SurfaceId,
        command: ServiceCommand,
        options: RuntimeCallOptions,
    ) -> Result<RequestTicket, RegistryError> {
        if command.operation == "destroy" {
            return Err(RegistryError::SafetyLaneRequired);
        }
        {
            let state = self.state.lock().expect("registry poisoned");
            validate_handle(&state, handle, authenticated, false)?;
            if !matches!(
                state.maps[&address_key(&handle.address)].service,
                NamespaceBackend::Cluster(_)
            ) {
                return Err(ServiceClusterError::UnsupportedScope.into());
            }
        }
        self.enqueue_kind(
            handle,
            authenticated,
            JobKind::Execute {
                cooperative: false,
                surface,
                command,
                options: Some(options),
            },
        )
    }

    /// Projection uses the verified DTO only; it never executes its reference-model owner.
    pub fn enqueue_verified(
        &self,
        handle: &RegistryHandle,
        authenticated: &AuthenticatedNamespace,
        verified: &VerifiedMapCommand,
    ) -> Result<RequestTicket, RegistryError> {
        let requested = NamespaceAddress {
            tenant: verified.tenant.clone(),
            namespace: verified.namespace.clone(),
            generation: verified.namespace_generation,
        };
        if requested != handle.address {
            let field = if requested.tenant != handle.address.tenant {
                "tenant"
            } else if requested.namespace != handle.address.namespace {
                "namespace"
            } else {
                "generation"
            };
            return Err(RegistryError::IdentitySubstitution(field));
        }
        let mut command = ServiceCommand::point("get", &verified.key);
        if let SurfaceOperation::Mutation {
            identity,
            operation,
        } = &verified.operation
        {
            command.identity = Some(identity.clone());
            command.operation = match operation {
                MutationOperation::Put { .. } => "put",
                MutationOperation::PutIfAbsent { .. } => "put_if_absent",
                MutationOperation::ReplaceIfPresent { .. } => "replace_if_present",
                MutationOperation::ReplaceIfValue { .. } => "conditional_replace",
                MutationOperation::RemoveIfValue { .. } => "conditional_remove",
                MutationOperation::GetAndPut { .. } => "get_and_put",
                MutationOperation::GetAndRemove => "get_and_remove",
                MutationOperation::Delete => "delete",
            }
            .into();
            match operation {
                MutationOperation::Put { value, ttl }
                | MutationOperation::PutIfAbsent { value, ttl }
                | MutationOperation::ReplaceIfPresent { value, ttl }
                | MutationOperation::GetAndPut { value, ttl } => {
                    command.value = Some(value.clone());
                    command.ttl = service_ttl(*ttl);
                }
                MutationOperation::ReplaceIfValue {
                    expected,
                    value,
                    ttl,
                } => {
                    command.expected = Some(expected.clone());
                    command.value = Some(value.clone());
                    command.ttl = service_ttl(*ttl);
                }
                MutationOperation::RemoveIfValue { expected } => {
                    command.expected = Some(expected.clone())
                }
                MutationOperation::GetAndRemove | MutationOperation::Delete => {}
            }
        }
        self.enqueue(handle, authenticated, verified.surface, command)
    }
    pub fn enqueue_reservation(
        &self,
        handle: &RegistryHandle,
        authenticated: &AuthenticatedNamespace,
        command: ServiceCommand,
    ) -> Result<RequestTicket, RegistryError> {
        if command.operation == "destroy" {
            return Err(RegistryError::SafetyLaneRequired);
        }
        if command.identity.is_none() {
            return Err(MapServiceError::InvalidArgument.into());
        }
        self.enqueue_kind(handle, authenticated, JobKind::Reserve(command))
    }
    pub fn enqueue_delete(
        &self,
        handle: &RegistryHandle,
        authenticated: &AuthenticatedNamespace,
    ) -> Result<RequestTicket, RegistryError> {
        self.enqueue_kind(handle, authenticated, JobKind::BeginDelete)
    }
    pub fn enqueue_reclaim(
        &self,
        handle: &RegistryHandle,
        authenticated: &AuthenticatedNamespace,
        max_work: usize,
    ) -> Result<RequestTicket, RegistryError> {
        self.enqueue_kind(handle, authenticated, JobKind::Reclaim(max_work))
    }
    pub fn enqueue_maintenance(
        &self,
        handle: &RegistryHandle,
        authenticated: &AuthenticatedNamespace,
        operation: RuntimeMaintenance,
    ) -> Result<RequestTicket, RegistryError> {
        {
            let state = self.state.lock().unwrap();
            validate_handle(&state, handle, authenticated, false)?;
            if !matches!(
                state.maps[&address_key(&handle.address)].service,
                NamespaceBackend::Cluster(_)
            ) {
                return Err(ServiceClusterError::UnsupportedScope.into());
            }
        }
        self.enqueue_kind(handle, authenticated, JobKind::Maintenance(operation))
    }
    pub fn cluster_health(
        &self,
        handle: &RegistryHandle,
        authenticated: &AuthenticatedNamespace,
    ) -> Result<RuntimeHealth, RegistryError> {
        let state = self.state.lock().unwrap();
        validate_handle(&state, handle, authenticated, true)?;
        match &state.maps[&address_key(&handle.address)].service {
            NamespaceBackend::Cluster(runtime) => Ok(runtime.health()?),
            _ => Err(ServiceClusterError::UnsupportedScope.into()),
        }
    }
    pub fn cluster_admission_observations(&self) -> Option<ClusterAdmissionObservations> {
        self.state
            .lock()
            .unwrap()
            .cluster_admission
            .as_ref()
            .map(ClusterAdmission::observations)
    }
    pub fn cluster_listener_source(
        &self,
        handle: &RegistryHandle,
        authenticated: &AuthenticatedNamespace,
        options: ClusterSubscriptionOptions,
    ) -> Result<RegistryClusterListenerSource, RegistryError> {
        let _dispatch = self
            .dispatching
            .try_lock()
            .map_err(|_| RegistryError::DispatcherBusy)?;
        let state = self.state.lock().unwrap();
        validate_handle(&state, handle, authenticated, false)?;
        authenticated.authorize_listener(&options.listener)?;
        let runtime = match &state.maps[&address_key(&handle.address)].service {
            NamespaceBackend::Cluster(runtime) => runtime,
            _ => return Err(ServiceClusterError::UnsupportedScope.into()),
        };
        let partitions = if options.listener.exact_key.is_some() {
            1
        } else {
            runtime.config.bounds.partitions as usize
        };
        let max_name = runtime
            .config
            .members
            .iter()
            .map(|member| member.node.len())
            .max()
            .unwrap_or(0);
        let growth = options
            .max_queued_bytes
            .saturating_mul(2)
            .saturating_add(partitions.saturating_mul(128 + 96 + 2 * max_name))
            .saturating_add(128)
            .saturating_add(options.listener.key_prefix.len())
            .saturating_add(options.listener.exact_key.as_ref().map_or(0, Vec::len));
        check_owned_growth(&state, &handle.address.tenant, growth, None)?;
        let inner = runtime.cluster_listener_source(authenticated, options)?;
        Ok(RegistryClusterListenerSource {
            inner,
            guard: stream_guard(&self.state, &self.dispatching, handle, authenticated),
            closed: false,
        })
    }
    pub fn resync_cluster_listener(
        &self,
        handle: &RegistryHandle,
        authenticated: &AuthenticatedNamespace,
        listener: u64,
        maximum_items: usize,
        maximum_bytes: usize,
    ) -> Result<ClusterSubscriptionSnapshot, RegistryError> {
        let _dispatch = self
            .dispatching
            .try_lock()
            .map_err(|_| RegistryError::DispatcherBusy)?;
        let mut state = self.state.lock().unwrap();
        validate_handle(&state, handle, authenticated, false)?;
        authenticated.authorize(GrantOperation::Listen, &[], true)?;
        authenticated.authorize(GrantOperation::Read, &[], true)?;
        let runtime = match &state.maps[&address_key(&handle.address)].service {
            NamespaceBackend::Cluster(runtime) => runtime.clone(),
            _ => return Err(ServiceClusterError::UnsupportedScope.into()),
        };
        let now = state.now;
        let growth = runtime.preflight_cluster_resync(now, maximum_bytes)?;
        check_owned_growth(&state, &handle.address.tenant, growth, None)?;
        state.transient = Some((handle.address.tenant.clone(), growth));
        drop(state);
        let _reservation = TransientReservation {
            state: Arc::downgrade(&self.state),
        };
        let result = runtime.resync_cluster_listener(
            authenticated,
            listener,
            now,
            maximum_items,
            maximum_bytes,
        );
        drop(_reservation);
        drop(_dispatch);
        let scheduler = self.state.lock().unwrap().cluster_admission.clone();
        if let Some(scheduler) = scheduler {
            scheduler.wake_waiters();
        }
        Ok(result?)
    }
    pub fn cancel_cluster_queued(
        &self,
        handle: &RegistryHandle,
        authenticated: &AuthenticatedNamespace,
        ticket: RequestTicket,
    ) -> Result<bool, RegistryError> {
        let mut state = self.state.lock().unwrap();
        validate_handle(&state, handle, authenticated, true)?;
        let Some(job) = state.jobs.get(&ticket.0) else {
            return Ok(false);
        };
        if job.handle.address != handle.address {
            return Err(RegistryError::IdentitySubstitution("namespace"));
        }
        authorize_job(authenticated, &job.kind)?;
        let scheduler = state
            .cluster_admission
            .as_ref()
            .ok_or(ServiceClusterError::UnsupportedScope)?;
        if !scheduler.cancel(job.admission.expect("cluster admission"))? {
            return Ok(false);
        }
        let job = state.jobs.remove(&ticket.0).expect("cancelled queued work");
        state.queued_bytes -= job.bytes;
        state.reserved_growth -= job.growth;
        let scheduler = state.cluster_admission.clone();
        drop(state);
        if let Some(scheduler) = scheduler {
            scheduler.wake_waiters();
        }
        Ok(true)
    }

    fn enqueue_kind(
        &self,
        handle: &RegistryHandle,
        authenticated: &AuthenticatedNamespace,
        mut kind: JobKind,
    ) -> Result<RequestTicket, RegistryError> {
        let mut state = self.state.lock().expect("registry poisoned");
        let safety = matches!(kind, JobKind::BeginDelete | JobKind::Reclaim(_))
            || matches!(&kind, JobKind::Maintenance(operation) if !operation.repair_lane());
        validate_handle(
            &state,
            handle,
            authenticated,
            matches!(kind, JobKind::Reclaim(_)),
        )?;
        authorize_job(authenticated, &kind)?;
        let owner = &state.maps[&address_key(&handle.address)];
        let (bytes, growth, partition) = match &mut kind {
            JobKind::Execute { command, .. } | JobKind::Reserve(command) => {
                command.now = state.now;
                let bytes = command_bytes(command);
                quota("command_bytes", state.bounds.max_command_bytes, bytes)?;
                validate_command_security(command, state.security, bytes)?;
                (
                    bytes,
                    owner.service.preflight_registry_growth(command)?,
                    partition(command, state.bounds.partitions),
                )
            }
            JobKind::BeginDelete => (128, 0, 0),
            JobKind::Reclaim(maximum) => {
                if *maximum == 0
                    || *maximum > state.bounds.max_cleanup_work
                    || *maximum > state.collection.max_scan_items
                {
                    return Err(RegistryError::InvalidBound("cleanup_work"));
                }
                (128, 0, 0)
            }
            JobKind::Maintenance(operation) => {
                let name = match operation {
                    RuntimeMaintenance::Repair { backup, .. } => Some(&*backup),
                    RuntimeMaintenance::SetNodeLive { node, .. }
                    | RuntimeMaintenance::BeginDrain { node }
                    | RuntimeMaintenance::DrainStep { node, .. } => Some(&*node),
                    _ => None,
                };
                if name.is_some_and(|name| name.is_empty() || name.len() > 128) {
                    return Err(RegistryError::InvalidBound("node_identity"));
                }
                (
                    128 + name.map_or(0, |name| name.len()),
                    owner.service.maintenance_growth(operation),
                    operation.partition(),
                )
            }
        };
        let bytes = bytes.saturating_add(
            authenticated
                .grant
                .as_ref()
                .map_or(0, |grant| grant.owned_bytes()),
        );
        quota("command_bytes", state.bounds.max_command_bytes, bytes)?;
        if !safety {
            let data_bytes = state
                .jobs
                .values()
                .filter(|job| matches!(job.kind, JobKind::Execute { .. } | JobKind::Reserve(_)))
                .fold(0usize, |sum, job| sum.saturating_add(job.bytes));
            quota(
                "queued_bytes",
                state.bounds.max_queued_bytes,
                data_bytes.saturating_add(bytes),
            )?;
        }
        if !safety {
            check_owned_growth(&state, &handle.address.tenant, growth, None)?;
        }
        let ticket = state.next_request;
        let next = ticket
            .checked_add(1)
            .ok_or(RegistryError::InvalidBound("request_identity"))?;
        let admission = if let Some(scheduler) = &state.cluster_admission {
            let key = match &kind {
                JobKind::Execute { command, .. } | JobKind::Reserve(command) => command.key.clone(),
                _ => Vec::new(),
            };
            let affected = match &kind {
                JobKind::Execute { command, .. } | JobKind::Reserve(command) => {
                    owner.service.partitions(command, state.bounds.partitions)
                }
                _ => vec![partition],
            };
            let lane = if safety {
                ClusterLane::Safety
            } else if matches!(&kind, JobKind::Maintenance(operation) if operation.repair_lane()) {
                ClusterLane::Repair
            } else {
                ClusterLane::Data
            };
            let replica_bytes =
                match &kind {
                    JobKind::Execute {
                        command,
                        cooperative,
                        ..
                    } => {
                        let bytes = owner.service.replica_bound(command)?;
                        if *cooperative {
                            bytes
                                .checked_mul(2)
                                .ok_or(RegistryError::InvalidBound("invocation_replica_bytes"))?
                        } else {
                            bytes
                        }
                    }
                    JobKind::Reserve(command) => owner
                        .service
                        .replica_bound(command)?
                        .checked_mul(2)
                        .ok_or(RegistryError::InvalidBound("invocation_replica_bytes"))?,
                    JobKind::Maintenance(operation) => match &owner.service {
                        NamespaceBackend::Cluster(runtime) => {
                            runtime.maintenance_replica_bound(operation)?
                        }
                        _ => 0,
                    },
                    _ => 0,
                };
            let primary = if let NamespaceBackend::Cluster(runtime) = &owner.service {
                match &kind {
                    JobKind::Execute { command, .. } | JobKind::Reserve(command) => {
                        if runtime_command_keys(command).is_empty() {
                            affected[0]
                        } else {
                            runtime.partition_for(
                                command
                                    .entries
                                    .first()
                                    .map(|entry| entry.key.as_slice())
                                    .or_else(|| command.keys.first().map(Vec::as_slice))
                                    .unwrap_or(&command.key),
                            )
                        }
                    }
                    _ => partition,
                }
            } else {
                partition
            };
            let resources = match &kind {
                JobKind::Execute { command, .. } | JobKind::Reserve(command) => {
                    runtime_command_keys(command)
                }
                _ => vec![key.clone()],
            };
            Some(
                scheduler.admit_for_resources(
                    ClusterWork {
                        tenant: handle.address.tenant.clone(),
                        partition: primary,
                        key,
                        lane,
                        bytes: bytes
                            .saturating_add(handle.address.tenant.len())
                            .saturating_add(if lane == ClusterLane::Repair {
                                growth.max(replica_bytes)
                            } else {
                                0
                            }),
                        replica_bytes,
                    },
                    &affected,
                    &resources,
                )?,
            )
        } else if safety {
            state.queue.admit_safety(ticket.to_string())?;
            None
        } else {
            state.queue.admit_data(DataWork {
                tenant: handle.address.tenant.clone(),
                partition,
                request_id: ticket,
            })?;
            None
        };
        state.jobs.insert(
            ticket,
            QueuedJob {
                handle: handle.clone(),
                authenticated: authenticated.clone(),
                kind,
                bytes,
                growth,
                admission,
            },
        );
        state.next_request = next;
        state.queued_bytes += bytes;
        state.reserved_growth += growth;
        let scheduler = state.cluster_admission.clone();
        drop(state);
        if let Some(scheduler) = scheduler {
            scheduler.wake_waiters();
        }
        Ok(RequestTicket(ticket))
    }

    pub fn run_next(&self) -> Result<Option<DispatchRecord>, RegistryError> {
        let _dispatch = self
            .dispatching
            .try_lock()
            .map_err(|_| RegistryError::DispatcherBusy)?;
        let (id, job, permit) = {
            let mut state = self.state.lock().expect("registry poisoned");
            let (id, permit) = if let Some(scheduler) = &state.cluster_admission {
                let excluded = state
                    .jobs
                    .values()
                    .filter(|job| {
                        matches!(
                            &job.kind,
                            JobKind::Execute {
                                cooperative: true,
                                ..
                            }
                        )
                    })
                    .filter_map(|job| job.admission)
                    .collect::<Vec<_>>();
                let Some(permit) = scheduler.pop_except(&excluded)? else {
                    return Ok(None);
                };
                let id = state
                    .jobs
                    .iter()
                    .find(|(_, job)| job.admission == Some(permit.ticket()))
                    .map(|(id, _)| *id)
                    .expect("cluster admission ticket");
                (id, Some(permit))
            } else {
                let Some(work) = state.queue.pop() else {
                    return Ok(None);
                };
                let id = match work {
                    WorkItem::Data(work) => work.request_id,
                    WorkItem::Safety { operation } => {
                        operation.parse().expect("registry safety ticket")
                    }
                };
                (id, None)
            };
            (id, state.jobs[&id].clone(), permit)
        };
        // Keep this job's reservation visible while callbacks execute. The metadata mutex
        // is never held while a service can wake downstream code.
        let result = dispatch(&self.state, id, &job, permit.as_ref());
        let mut state = self.state.lock().expect("registry poisoned");
        state.jobs.remove(&id).expect("dispatched registry request");
        state.queued_bytes -= job.bytes;
        state.reserved_growth -= job.growth;
        drop(state);
        drop(_dispatch);
        drop(permit);
        let surface = if let JobKind::Execute { surface, .. } = &job.kind {
            Some(*surface)
        } else {
            None
        };
        Ok(Some(DispatchRecord {
            ticket: RequestTicket(id),
            address: job.handle.address,
            surface,
            result,
        }))
    }

    pub fn diagnostics(
        &self,
        handle: &RegistryHandle,
        authenticated: &AuthenticatedNamespace,
    ) -> Result<NamespaceDiagnostics, RegistryError> {
        let state = self.state.lock().expect("registry poisoned");
        validate_handle(&state, handle, authenticated, true)?;
        let owner = &state.maps[&address_key(&handle.address)];
        Ok(NamespaceDiagnostics {
            phase: owner.lifecycle.phase(),
            logical_time: owner.service.logical_time(),
            statistics: owner.service.statistics(),
            resources: owner.service.resource_usage(),
            owners: owner.service.namespace_owners(),
        })
    }

    pub fn usage(&self) -> RegistryUsage {
        let state = self.state.lock().expect("registry poisoned");
        RegistryUsage {
            tenants: state.tenants.len(),
            maps: state.maps.len(),
            metadata_bytes: state.metadata_bytes,
            owned_bytes: owned_bytes(&state, None),
            queued_bytes: state.queued_bytes,
            queued_requests: state.jobs.len() + state.active_invocations.len(),
            reserved_growth: state
                .reserved_growth
                .saturating_add(state.transient.as_ref().map_or(0, |(_, bytes)| *bytes)),
        }
    }

    pub fn cancel_pending(
        &self,
        handle: &RegistryHandle,
        authenticated: &AuthenticatedNamespace,
        identity: &MutationIdentity,
    ) -> Result<bool, RegistryError> {
        let state = self.state.lock().expect("registry poisoned");
        validate_handle(&state, handle, authenticated, true)?;
        authenticated.authorize(GrantOperation::Write, &[], true)?;
        state.maps[&address_key(&handle.address)]
            .service
            .cancel_reserved(identity)
    }

    pub fn recreate(
        &self,
        handle: &RegistryHandle,
        authenticated: &AuthenticatedNamespace,
    ) -> Result<RegistryHandle, RegistryError> {
        let mut state = self.state.lock().expect("registry poisoned");
        validate_handle(&state, handle, authenticated, true)?;
        authenticated.authorize(GrantOperation::Admin, &[], true)?;
        let key = address_key(&handle.address);
        let owner = &state.maps[&key];
        let actual = owner.service.namespace_owners();
        if actual.total() != 0
            || state
                .jobs
                .values()
                .any(|job| address_key(&job.handle.address) == key)
            || state
                .active_invocations
                .values()
                .any(|(address, _, _)| address_key(address) == key)
        {
            return Err(MapServiceError::ActiveOwners.into());
        }
        let mut lifecycle = owner.lifecycle.clone();
        let generation = lifecycle.recreate()?;
        let service = match &owner.service {
            NamespaceBackend::Local(_) => NamespaceBackend::Local(UnifiedIMapService::with_limits(
                state.collection,
                generation,
                state.service_limits,
            )?),
            NamespaceBackend::Cluster(service) => NamespaceBackend::Cluster(service.successor(
                generation,
                &NamespaceAddress {
                    generation,
                    ..handle.address.clone()
                },
                state.collection,
                state.service_limits,
                state.security,
                state.trust,
            )?),
        };
        if let (Some(epoch), NamespaceBackend::Cluster(runtime)) = (state.policy_epoch, &service) {
            runtime.authentication.set_policy_epoch(epoch)?;
        }
        state
            .maps
            .insert(key, NamespaceOwner { lifecycle, service });
        Ok(RegistryHandle {
            registry: state.id,
            address: NamespaceAddress {
                generation,
                ..handle.address.clone()
            },
        })
    }

    pub fn advance_clock(&self, now: u64) -> Result<(), RegistryError> {
        let mut state = self.state.lock().expect("registry poisoned");
        if now < state.now {
            return Err(MapServiceError::ClockRegressed.into());
        }
        if now == state.now {
            return Ok(());
        }
        state.now = now;
        let services = state
            .maps
            .values()
            .map(|owner| owner.service.clone())
            .collect::<Vec<_>>();
        let scheduler = state.cluster_admission.clone();
        drop(state);
        if let Some(scheduler) = scheduler {
            scheduler.wake_waiters();
        }
        for service in services {
            service.wake_listener_waiters();
        }
        Ok(())
    }
    pub fn rotate_trust(&self, next: u64) -> Result<(), RegistryError> {
        let mut state = self.state.lock().expect("registry poisoned");
        if state.cluster_config.is_some() {
            return Err(ServiceClusterError::Authorization("credential_rotation_required").into());
        }
        let now = state.now;
        state.trust.rotate(next, now)?;
        let services = state
            .maps
            .values()
            .map(|owner| owner.service.clone())
            .collect::<Vec<_>>();
        let scheduler = state.cluster_admission.clone();
        drop(state);
        if let Some(scheduler) = scheduler {
            scheduler.wake_waiters();
        }
        for service in services {
            service.wake_listener_waiters();
        }
        Ok(())
    }
    pub fn rotate_cluster_credentials(
        &self,
        next: u64,
        members: Vec<crate::value_plane_cluster_auth_075::RuntimeMember>,
    ) -> Result<(), RegistryError> {
        let _dispatch = self
            .dispatching
            .try_lock()
            .map_err(|_| RegistryError::DispatcherBusy)?;
        let mut state = self.state.lock().unwrap();
        if state.cluster_config.is_none() {
            return Err(ServiceClusterError::UnsupportedScope.into());
        }
        crate::value_plane_cluster_auth_075::validate_member_rotation(
            &state
                .cluster_config
                .as_ref()
                .expect("cluster config")
                .members,
            &members,
        )?;
        for owner in state.maps.values() {
            if let NamespaceBackend::Cluster(runtime) = &owner.service {
                runtime.validate_rotation(next, &members)?;
            }
        }
        let now = state.now;
        let mut trust = state.trust;
        trust.rotate(next, now)?;
        for owner in state.maps.values() {
            if let NamespaceBackend::Cluster(runtime) = &owner.service {
                runtime.rotate_credentials(next, now, &members)?;
            }
        }
        state.trust = trust;
        state
            .cluster_config
            .as_mut()
            .expect("cluster config")
            .members = members;
        let services = state
            .maps
            .values()
            .map(|owner| owner.service.clone())
            .collect::<Vec<_>>();
        let scheduler = state.cluster_admission.clone();
        drop(state);
        drop(_dispatch);
        if let Some(scheduler) = scheduler {
            scheduler.wake_waiters();
        }
        for service in services {
            service.wake_listener_waiters();
        }
        Ok(())
    }

    pub fn page_source(
        &self,
        handle: &RegistryHandle,
        authenticated: &AuthenticatedNamespace,
        projection: ScanProjection,
    ) -> Result<RegistryPageSource, RegistryError> {
        let _dispatch = self
            .dispatching
            .try_lock()
            .map_err(|_| RegistryError::DispatcherBusy)?;
        let state = self.state.lock().expect("registry poisoned");
        validate_handle(&state, handle, authenticated, false)?;
        authenticated.authorize(GrantOperation::Read, &[], true)?;
        let growth = state.collection.max_response_bytes.saturating_add(64);
        check_owned_growth(&state, &handle.address.tenant, growth, None)?;
        let inner = state.maps[&address_key(&handle.address)]
            .service
            .page_source(projection, state.now)?;
        Ok(RegistryPageSource {
            inner,
            guard: stream_guard(&self.state, &self.dispatching, handle, authenticated),
            closed: false,
        })
    }

    pub fn listener_source(
        &self,
        handle: &RegistryHandle,
        authenticated: &AuthenticatedNamespace,
        options: ServiceListenerOptions,
    ) -> Result<RegistryListenerSource, RegistryError> {
        let _dispatch = self
            .dispatching
            .try_lock()
            .map_err(|_| RegistryError::DispatcherBusy)?;
        let state = self.state.lock().expect("registry poisoned");
        validate_handle(&state, handle, authenticated, false)?;
        authenticated.authorize_listener(&options)?;
        let growth = state
            .collection
            .max_response_bytes
            .saturating_add(192)
            .saturating_add(options.key_prefix.len())
            .saturating_add(options.exact_key.as_ref().map_or(0, Vec::len));
        check_owned_growth(&state, &handle.address.tenant, growth, None)?;
        let inner = state.maps[&address_key(&handle.address)]
            .service
            .listener_source(options)?;
        Ok(RegistryListenerSource {
            inner,
            guard: stream_guard(&self.state, &self.dispatching, handle, authenticated),
            closed: false,
        })
    }

    pub fn resync_listener(
        &self,
        handle: &RegistryHandle,
        authenticated: &AuthenticatedNamespace,
        listener: u64,
        maximum_items: usize,
        maximum_bytes: usize,
    ) -> Result<ListenerSnapshot, RegistryError> {
        let _dispatch = self
            .dispatching
            .try_lock()
            .map_err(|_| RegistryError::DispatcherBusy)?;
        let mut state = self.state.lock().expect("registry poisoned");
        validate_handle(&state, handle, authenticated, false)?;
        authenticated.authorize(GrantOperation::Listen, &[], true)?;
        authenticated.authorize(GrantOperation::Read, &[], true)?;
        let service = state.maps[&address_key(&handle.address)].service.clone();
        let now = state.now;
        // Resync advances expiry before clearing its target listener. Other listener queues
        // can grow even when the snapshot itself is empty, so reserve both owners first.
        let mut observation = ServiceCommand::point("get", b"");
        observation.now = now;
        let growth = maximum_bytes
            .checked_add(service.preflight_registry_growth(&observation)?)
            .ok_or(RegistryError::InvalidBound("resync_growth"))?;
        check_owned_growth(&state, &handle.address.tenant, growth, None)?;
        state.transient = Some((handle.address.tenant.clone(), growth));
        drop(state);
        let _reservation = TransientReservation {
            state: Arc::downgrade(&self.state),
        };
        service.resync_listener(listener, now, maximum_items, maximum_bytes)
    }
}

fn dispatch(
    shared: &Arc<Mutex<RegistryState>>,
    id: u64,
    job: &QueuedJob,
    permit: Option<&ClusterPermit>,
) -> Result<RegistryResult, RegistryError> {
    let mut state = shared.lock().expect("registry poisoned");
    validate_handle(
        &state,
        &job.handle,
        &job.authenticated,
        matches!(job.kind, JobKind::Reclaim(_)),
    )?;
    authorize_job(&job.authenticated, &job.kind)?;
    let key = address_key(&job.handle.address);
    match &job.kind {
        JobKind::Execute { command, .. } | JobKind::Reserve(command) => {
            // Every admitted surface deliberately resolves the very same owner. SurfaceId
            // is provenance only; this dev harness makes no production routing claim.
            let owner = &state.maps[&key];
            let mut command = command.clone();
            command.now = state.now;
            let growth = owner.service.preflight_registry_growth(&command)?;
            check_owned_growth(&state, &job.handle.address.tenant, growth, Some(id))?;
            let service = owner.service.clone();
            drop(state);
            if matches!(job.kind, JobKind::Reserve(_)) {
                let identity = command
                    .identity
                    .clone()
                    .ok_or(MapServiceError::InvalidArgument)?;
                service.reserve_operation(identity, &command, &job.authenticated, permit)?;
                Ok(RegistryResult::Command(ServiceResult::new("reserved")))
            } else {
                let options = if let JobKind::Execute { options, .. } = &job.kind {
                    options.as_ref()
                } else {
                    None
                };
                service.execute(command, &job.authenticated, permit, options)
            }
        }
        JobKind::BeginDelete => {
            let owner = state.maps.get_mut(&key).expect("validated owner");
            owner.lifecycle.begin_delete()?;
            let service = owner.service.clone();
            drop(state);
            service.fence_namespace()?;
            Ok(RegistryResult::DeleteStarted)
        }
        JobKind::Reclaim(maximum) => {
            let owner = &state.maps[&key];
            if owner.lifecycle.phase() == NamespacePhase::Active {
                return Err(NamespaceError::NotDraining.into());
            }
            let service = owner.service.clone();
            drop(state);
            let progress = service.reclaim_namespace_step(*maximum)?;
            let mut state = shared.lock().expect("registry poisoned");
            let queued_requests = state
                .jobs
                .iter()
                .filter(|(queued_id, queued)| {
                    **queued_id != id && address_key(&queued.handle.address) == key
                })
                .count()
                + state
                    .active_invocations
                    .values()
                    .filter(|(address, _, _)| address_key(address) == key)
                    .count();
            let owner = state.maps.get_mut(&key).expect("validated owner");
            let owners = owner.service.namespace_owners();
            if owners.total() == 0
                && queued_requests == 0
                && owner.lifecycle.phase() == NamespacePhase::Draining
            {
                let watermark = owner.service.revision().max(1);
                owner
                    .lifecycle
                    .commit_delete(watermark, ReclamationOwners::default())?;
                // These are process-local completion fences established by the real owner
                // cleanup above, not evidence of replicated or distributed deletion.
                owner.lifecycle.advance_reclamation(ReclamationWatermarks {
                    replica_applied: watermark,
                    listener_cutover: watermark,
                    dedup_expired: watermark,
                    transfer_closed: watermark,
                    tombstone_gc: watermark,
                });
            }
            Ok(RegistryResult::Cleanup(NamespaceCleanup {
                phase: owner.lifecycle.phase(),
                reclaimed: progress.reclaimed,
                remaining: owners.cleanable_records(),
                blocked_owners: owners.blocked_owners(),
                queued_requests,
                owners,
            }))
        }
        JobKind::Maintenance(operation) => {
            let backend = &state.maps[&key].service;
            let growth = backend.maintenance_growth(operation);
            check_owned_growth(&state, &job.handle.address.tenant, growth, Some(id))?;
            let runtime = match backend {
                NamespaceBackend::Cluster(runtime) => runtime.clone(),
                _ => return Err(ServiceClusterError::UnsupportedScope.into()),
            };
            drop(state);
            Ok(RegistryResult::Maintenance(runtime.maintain(
                &job.authenticated,
                permit.ok_or(ServiceClusterError::Authorization("admission_binding"))?,
                operation,
            )?))
        }
    }
}

fn validate_auth(
    state: &RegistryState,
    authenticated: &AuthenticatedNamespace,
    requested: &NamespaceAddress,
) -> Result<(), RegistryError> {
    match (state.policy_epoch, authenticated.policy_epoch()) {
        (None, None) => {}
        (Some(expected), Some(actual)) if expected == actual => {}
        _ => return Err(SecurityError::PolicyEpochMismatch.into()),
    }
    if requested.tenant != authenticated.address.tenant {
        return Err(RegistryError::IdentitySubstitution("tenant"));
    }
    if requested.namespace != authenticated.address.namespace {
        return Err(RegistryError::IdentitySubstitution("namespace"));
    }
    if requested.generation != authenticated.address.generation {
        return Err(RegistryError::IdentitySubstitution("generation"));
    }
    validate_route(
        &RouteClaims {
            authenticated: true,
            authenticated_tenant: &authenticated.address.tenant,
            claimed_tenant: &requested.tenant,
            expected_namespace_generation: requested.generation,
            actual_namespace_generation: authenticated.address.generation,
            expected_topology_epoch: state.topology_epoch,
            actual_topology_epoch: authenticated.topology_epoch,
            proxy_hops: authenticated.proxy_hops,
            deadline_remaining_ticks: authenticated.deadline.saturating_sub(state.now),
            trust_epoch: authenticated.trust_epoch,
            encoded_envelope_bytes: authenticated.envelope_bytes,
            key_bytes: authenticated.key_bytes,
            value_bytes: authenticated.value_bytes,
        },
        state.security,
        state.trust,
        state.now,
    )?;
    Ok(())
}

fn authorize_job(
    authenticated: &AuthenticatedNamespace,
    job: &JobKind,
) -> Result<(), RegistryError> {
    match job {
        JobKind::Execute { command, .. } | JobKind::Reserve(command) => {
            authenticated.authorize_command(command)
        }
        JobKind::BeginDelete | JobKind::Reclaim(_) | JobKind::Maintenance(_) => {
            authenticated.authorize(GrantOperation::Admin, &[], true)
        }
    }
}

fn service_ttl(ttl: TtlDirective) -> ServiceTtl {
    match ttl {
        TtlDirective::Preserve => ServiceTtl::Preserve,
        TtlDirective::Eternal => ServiceTtl::Eternal,
        TtlDirective::ExpireAfter(ticks) => ServiceTtl::ExpireAfter(ticks),
    }
}

fn validate_command_security(
    command: &ServiceCommand,
    security: SecurityBounds,
    bytes: usize,
) -> Result<(), RegistryError> {
    if bytes > security.max_envelope_bytes {
        return Err(SecurityError::EnvelopeTooLarge {
            limit: security.max_envelope_bytes,
            actual: bytes,
        }
        .into());
    }
    for key in std::iter::once(&command.key)
        .chain(&command.keys)
        .chain(command.entries.iter().map(|entry| &entry.key))
        .chain(
            command
                .expected
                .iter()
                .filter(|_| command.operation == "entry_listener"),
        )
    {
        if key.len() > security.max_key_bytes {
            return Err(SecurityError::KeyTooLarge {
                limit: security.max_key_bytes,
                actual: key.len(),
            }
            .into());
        }
    }
    for value in command
        .value
        .iter()
        .chain(
            command
                .expected
                .iter()
                .filter(|_| command.operation != "entry_listener"),
        )
        .chain(command.entries.iter().map(|entry| &entry.value))
    {
        if value.len() > security.max_value_bytes {
            return Err(SecurityError::ValueTooLarge {
                limit: security.max_value_bytes,
                actual: value.len(),
            }
            .into());
        }
    }
    Ok(())
}

fn validate_handle(
    state: &RegistryState,
    handle: &RegistryHandle,
    authenticated: &AuthenticatedNamespace,
    inactive: bool,
) -> Result<(), RegistryError> {
    if handle.registry != state.id {
        return Err(RegistryError::WrongRegistry);
    }
    validate_auth(state, authenticated, &handle.address)?;
    let owner = state
        .maps
        .get(&address_key(&handle.address))
        .ok_or(RegistryError::UnknownNamespace)?;
    if inactive {
        if handle.address.generation != owner.lifecycle.generation() {
            return Err(NamespaceError::StaleGeneration {
                expected: owner.lifecycle.generation(),
                actual: handle.address.generation,
            }
            .into());
        }
    } else {
        owner
            .lifecycle
            .validate_request(handle.address.generation)?;
    }
    Ok(())
}

fn identifier(state: &RegistryState, value: &str) -> Result<(), RegistryError> {
    if value.trim().is_empty() {
        return Err(RegistryError::IdentitySubstitution("empty_identity"));
    }
    quota(
        "identifier_bytes",
        state.bounds.max_identifier_bytes,
        value.len(),
    )
}
fn address_key(address: &NamespaceAddress) -> (String, String) {
    (address.tenant.clone(), address.namespace.clone())
}
fn quota(dimension: &'static str, maximum: usize, actual: usize) -> Result<(), RegistryError> {
    if actual > maximum {
        Err(RegistryError::Quota {
            dimension,
            maximum,
            actual,
        })
    } else {
        Ok(())
    }
}
fn usage_bytes(usage: ServiceResourceUsage) -> usize {
    [
        usage.values,
        usage.outcomes,
        usage.pending,
        usage.listeners,
        usage.streams,
        usage.resync,
    ]
    .into_iter()
    .fold(0usize, usize::saturating_add)
}
fn owned_bytes(state: &RegistryState, tenant: Option<&str>) -> usize {
    state
        .maps
        .iter()
        .filter(|((owner, _), _)| tenant.is_none_or(|tenant| owner == tenant))
        .fold(0usize, |sum, (_, owner)| {
            sum.saturating_add(usage_bytes(owner.service.resource_usage()))
        })
}
fn check_owned_growth(
    state: &RegistryState,
    tenant: &str,
    growth: usize,
    exclude: Option<u64>,
) -> Result<(), RegistryError> {
    let reservations = state.jobs.iter().filter(|(id, _)| exclude != Some(**id));
    let global_reserved = reservations
        .clone()
        .fold(0usize, |sum, (_, job)| sum.saturating_add(job.growth))
        .saturating_add(
            state
                .active_invocations
                .iter()
                .filter(|(id, _)| exclude != Some(**id))
                .map(|(_, (_, _, growth))| *growth)
                .sum::<usize>(),
        );
    let tenant_reserved = reservations
        .filter(|(_, job)| job.handle.address.tenant == tenant)
        .fold(0usize, |sum, (_, job)| sum.saturating_add(job.growth))
        .saturating_add(
            state
                .active_invocations
                .iter()
                .filter(|(id, (address, _, _))| exclude != Some(**id) && address.tenant == tenant)
                .map(|(_, (_, _, growth))| *growth)
                .sum::<usize>(),
        );
    let global_transient = state.transient.as_ref().map_or(0, |(_, bytes)| *bytes);
    let tenant_transient = state
        .transient
        .as_ref()
        .filter(|(owner, _)| owner == tenant)
        .map_or(0, |(_, bytes)| *bytes);
    quota(
        "owned_bytes",
        state.bounds.max_owned_bytes,
        owned_bytes(state, None)
            .saturating_add(global_reserved)
            .saturating_add(global_transient)
            .saturating_add(growth),
    )?;
    quota(
        "tenant_owned_bytes",
        state.bounds.max_tenant_owned_bytes,
        owned_bytes(state, Some(tenant))
            .saturating_add(tenant_reserved)
            .saturating_add(tenant_transient)
            .saturating_add(growth),
    )
}
fn command_bytes(command: &ServiceCommand) -> usize {
    command.entries.iter().fold(
        command.keys.iter().fold(
            128usize
                .saturating_add(command.operation.len())
                .saturating_add(command.key.len())
                .saturating_add(command.value.as_ref().map_or(0, Vec::len))
                .saturating_add(command.expected.as_ref().map_or(0, Vec::len))
                .saturating_add(
                    command
                        .identity
                        .as_ref()
                        .map_or(0, |identity| identity.client.len()),
                ),
            |sum, key| sum.saturating_add(24).saturating_add(key.len()),
        ),
        |sum, entry| {
            sum.saturating_add(48)
                .saturating_add(entry.key.len())
                .saturating_add(entry.value.len())
        },
    )
}
pub(crate) fn runtime_command_keys(command: &ServiceCommand) -> Vec<Vec<u8>> {
    if !command.entries.is_empty() {
        command
            .entries
            .iter()
            .map(|entry| entry.key.clone())
            .collect()
    } else if !command.keys.is_empty() {
        command.keys.clone()
    } else if matches!(
        command.operation.as_str(),
        "get_all" | "put_all_detailed" | "remove_all_detailed" | "replace_all_detailed"
    ) {
        Vec::new()
    } else {
        vec![command.key.clone()]
    }
}
fn partition(command: &ServiceCommand, partitions: u32) -> u32 {
    let key = command
        .entries
        .first()
        .map(|entry| entry.key.as_slice())
        .or_else(|| command.keys.first().map(Vec::as_slice))
        .unwrap_or(&command.key);
    key.iter().fold(0u32, |hash, byte| {
        hash.wrapping_mul(16777619) ^ u32::from(*byte)
    }) % partitions
}

struct TransientReservation {
    state: Weak<Mutex<RegistryState>>,
}
impl Drop for TransientReservation {
    fn drop(&mut self) {
        if let Some(state) = self.state.upgrade() {
            state.lock().expect("registry poisoned").transient = None;
        }
    }
}
struct StreamGuard {
    state: Weak<Mutex<RegistryState>>,
    dispatching: Weak<Mutex<()>>,
    handle: RegistryHandle,
    authenticated: AuthenticatedNamespace,
}
fn stream_guard(
    state: &Arc<Mutex<RegistryState>>,
    dispatching: &Arc<Mutex<()>>,
    handle: &RegistryHandle,
    authenticated: &AuthenticatedNamespace,
) -> StreamGuard {
    StreamGuard {
        state: Arc::downgrade(state),
        dispatching: Arc::downgrade(dispatching),
        handle: handle.clone(),
        authenticated: authenticated.clone(),
    }
}
impl StreamGuard {
    fn validate(&self) -> Result<(), RegistryError> {
        let state = self.state.upgrade().ok_or(RegistryError::RegistryGone)?;
        let state = state.lock().expect("registry poisoned");
        validate_handle(&state, &self.handle, &self.authenticated, false)
    }
    fn poll_with_reservation<T>(&self, poll: impl FnOnce() -> T) -> Result<T, RegistryError> {
        let shared = self.state.upgrade().ok_or(RegistryError::RegistryGone)?;
        let dispatching = self
            .dispatching
            .upgrade()
            .ok_or(RegistryError::RegistryGone)?;
        let _dispatch = dispatching
            .try_lock()
            .map_err(|_| RegistryError::DispatcherBusy)?;
        let mut state = shared.lock().expect("registry poisoned");
        validate_handle(&state, &self.handle, &self.authenticated, false)?;
        let mut observation = ServiceCommand::point("get", b"");
        observation.now = state.now;
        let growth = state.maps[&address_key(&self.handle.address)]
            .service
            .preflight_registry_growth(&observation)?;
        check_owned_growth(&state, &self.handle.address.tenant, growth, None)?;
        state.transient = Some((self.handle.address.tenant.clone(), growth));
        drop(state);
        let _reservation = TransientReservation {
            state: Arc::downgrade(&shared),
        };
        let result = poll();
        drop(_reservation);
        drop(_dispatch);
        let scheduler = shared.lock().unwrap().cluster_admission.clone();
        if let Some(scheduler) = scheduler {
            scheduler.wake_waiters();
        }
        Ok(result)
    }
}

pub struct RegistryPageSource {
    inner: ServicePageSource,
    guard: StreamGuard,
    closed: bool,
}
impl BoundedStreamSource for RegistryPageSource {
    type Item = ServiceItem;
    type Error = RegistryError;
    fn poll_batch(
        &mut self,
        context: &mut Context<'_>,
        maximum: usize,
    ) -> BatchPoll<Self::Item, Self::Error> {
        match self
            .guard
            .poll_with_reservation(|| self.inner.poll_batch(context, maximum))
        {
            Ok(result) => result.map_err(RegistryError::Service),
            Err(error) => Poll::Ready(Err(error)),
        }
    }
    fn validate_buffered(&mut self) -> Result<(), Self::Error> {
        self.guard.validate()?;
        self.inner
            .validate_buffered()
            .map_err(RegistryError::Service)
    }
    fn close(&mut self) {
        if !self.closed {
            self.closed = true;
            self.inner.close();
        }
    }
}
impl Drop for RegistryPageSource {
    fn drop(&mut self) {
        self.close();
    }
}

pub struct RegistryListenerSource {
    inner: ServiceListenerSource,
    guard: StreamGuard,
    closed: bool,
}
impl RegistryListenerSource {
    pub fn registration_id(&self) -> Option<u64> {
        self.inner.registration_id()
    }
}
impl BoundedStreamSource for RegistryListenerSource {
    type Item = ServiceEvent;
    type Error = RegistryError;
    // Dequeue only transfers events into the stream's already-reserved buffer. It cannot
    // increase owned payload, so it remains available while a mutation is being dispatched.
    fn poll_batch(
        &mut self,
        context: &mut Context<'_>,
        maximum: usize,
    ) -> BatchPoll<Self::Item, Self::Error> {
        if let Err(error) = self.guard.validate() {
            return Poll::Ready(Err(error));
        }
        self.inner
            .poll_batch(context, maximum)
            .map_err(RegistryError::Service)
    }
    fn validate_buffered(&mut self) -> Result<(), Self::Error> {
        self.guard.validate()?;
        self.inner
            .validate_buffered()
            .map_err(RegistryError::Service)
    }
    fn take_buffer_reset(&mut self) -> bool {
        self.inner.take_buffer_reset()
    }
    fn close(&mut self) {
        if !self.closed {
            self.closed = true;
            self.inner.close();
        }
    }
}
impl Drop for RegistryListenerSource {
    fn drop(&mut self) {
        self.close();
    }
}

pub struct RegistryClusterListenerSource {
    inner: ClusterListenerSource,
    guard: StreamGuard,
    closed: bool,
}
impl RegistryClusterListenerSource {
    pub fn subscription_id(&self) -> u64 {
        self.inner.subscription_id()
    }
}
impl BoundedStreamSource for RegistryClusterListenerSource {
    type Item = ClusterSubscriptionEvent;
    type Error = RegistryError;
    fn poll_batch(
        &mut self,
        context: &mut Context<'_>,
        maximum: usize,
    ) -> BatchPoll<Self::Item, Self::Error> {
        if let Err(error) = self.guard.validate() {
            return Poll::Ready(Err(error));
        }
        self.inner
            .poll_batch(context, maximum)
            .map_err(RegistryError::Cluster)
    }
    fn validate_buffered(&mut self) -> Result<(), Self::Error> {
        self.guard.validate()?;
        self.inner
            .validate_buffered()
            .map_err(RegistryError::Cluster)
    }
    fn take_buffer_reset(&mut self) -> bool {
        self.inner.take_buffer_reset()
    }
    fn close(&mut self) {
        if !self.closed {
            self.closed = true;
            self.inner.close();
        }
    }
}
impl Drop for RegistryClusterListenerSource {
    fn drop(&mut self) {
        self.close();
    }
}
