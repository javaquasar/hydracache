//! Authenticated runtime composition over the existing actual service-cluster owner graph.
//! It owns no additional collection, protocol identity, or production capability.

use std::sync::{Arc, Mutex};

use crate::value_plane_cluster_admission_075::{
    ClusterAdmissionBounds, ClusterLane, ClusterPermit,
};
use crate::value_plane_cluster_auth_075::{
    AuthFault, ClusterAuthConfig, RuntimeAuthentication, RuntimeMember,
};
use crate::value_plane_collection_075::{CollectionBounds, ScanProjection};
use crate::value_plane_security_075::{SecurityBounds, TrustEpochWindow};
use crate::value_plane_service_075::*;
use crate::value_plane_service_cluster_075::*;
use crate::value_plane_service_registry_075::{AuthenticatedNamespace, NamespaceAddress};

#[derive(Debug, Clone)]
pub struct ClusterRuntimeConfig {
    pub cluster: String,
    pub members: Vec<RuntimeMember>,
    pub entry_node: String,
    pub bounds: ServiceClusterBounds,
    pub admission: ClusterAdmissionBounds,
    pub max_replays: usize,
}

#[derive(Debug, Clone, Default)]
pub struct RuntimeCallOptions {
    pub entry_node: Option<String>,
    pub call: ServiceClusterCall,
    pub auth_fault: Option<AuthFault>,
}

#[derive(Clone)]
pub struct ClusterNamespaceRuntime {
    pub(crate) cluster: Arc<Mutex<ServiceClusterHarness>>,
    pub(crate) config: Arc<ClusterRuntimeConfig>,
    pub(crate) authentication: RuntimeAuthentication,
}

impl ClusterNamespaceRuntime {
    pub(crate) fn with_cluster_wakes<T>(
        &self,
        operation: impl FnOnce(&mut ServiceClusterHarness) -> Result<T, ServiceClusterError>,
    ) -> Result<T, ServiceClusterError> {
        let mut cluster = self.cluster.lock().expect("cluster runtime poisoned");
        let result = operation(&mut cluster);
        let wakes = cluster.take_deferred_wakes();
        drop(cluster);
        for wake in wakes {
            wake.wake();
        }
        result
    }
    pub(crate) fn sync_members(&self) -> Result<(), ServiceClusterError> {
        let cluster = self.cluster.lock().unwrap();
        let members = cluster
            .node_ids()
            .into_iter()
            .map(|node| {
                let generation = cluster.node_generation(&node)?;
                Ok((node, generation))
            })
            .collect::<Result<Vec<_>, ServiceClusterError>>()?;
        drop(cluster);
        for (node, generation) in members {
            self.authentication
                .update_member_generation(&node, generation)?;
        }
        Ok(())
    }
    pub fn new(
        config: ClusterRuntimeConfig,
        address: &NamespaceAddress,
        collection: CollectionBounds,
        limits: ServiceLimits,
        security: SecurityBounds,
        trust: TrustEpochWindow,
    ) -> Result<Self, ServiceClusterError> {
        if !config
            .members
            .iter()
            .any(|member| member.node == config.entry_node)
        {
            return Err(ServiceClusterError::InvalidBound);
        }
        let node_ids = config
            .members
            .iter()
            .map(|member| member.node.as_str())
            .collect::<Vec<_>>();
        let cluster = ServiceClusterHarness::new(
            &node_ids,
            config.bounds,
            collection,
            limits,
            address.generation,
        )?;
        let authentication = RuntimeAuthentication::new(
            ClusterAuthConfig {
                cluster: config.cluster.clone(),
                tenant: address.tenant.clone(),
                namespace: address.namespace.clone(),
                generation: address.generation,
                members: config.members.clone(),
                max_replays: config.max_replays,
            },
            security,
            trust,
        )?;
        Ok(Self {
            cluster: Arc::new(Mutex::new(cluster)),
            config: Arc::new(config),
            authentication,
        })
    }

    pub fn execute(
        &self,
        authenticated: &AuthenticatedNamespace,
        permit: &ClusterPermit,
        mut command: ServiceCommand,
        options: Option<&RuntimeCallOptions>,
    ) -> Result<ServiceClusterExecution, ServiceClusterError> {
        authenticated
            .authorize_command(&command)
            .map_err(|_| ServiceClusterError::Authorization("operation_policy"))?;
        self.authentication
            .validate_principal(authenticated, command.now)?;
        if permit.work().lane != ClusterLane::Data
            || permit.work().tenant != authenticated.address().tenant
            || !permit.keys_match(
                &crate::value_plane_service_registry_075::runtime_command_keys(&command),
            )
            || self
                .affected_partitions(&command)
                .iter()
                .any(|partition| !permit.partitions().contains(partition))
        {
            return Err(ServiceClusterError::Authorization("admission_binding"));
        }
        command.deadline = command.deadline.min(authenticated.runtime_proof().4);
        let binding = self.admission_binding(authenticated, &command);
        let _attempt = permit
            .begin_invocation(binding.clone())
            .map_err(|_| ServiceClusterError::Authorization("admission_receipt"))?;
        if self.command_replica_bound(&command)? > permit.work().replica_bytes {
            return Err(ServiceClusterError::Authorization("replica_credit"));
        }
        let options = options.cloned().unwrap_or_default();
        self.sync_members()?;
        let guard = self.authentication.guard_with_receipt(
            authenticated,
            permit.ticket(),
            permit.work().replica_bytes,
            options.auth_fault,
            Some(permit.receipt_digest(&binding)),
        );
        let mut cluster = self.cluster.lock().expect("cluster runtime poisoned");
        let result = cluster.execute_guarded(
            options
                .entry_node
                .as_deref()
                .unwrap_or(&self.config.entry_node),
            command,
            options.call,
            guard,
        );
        let wakes = cluster.take_deferred_wakes();
        drop(cluster);
        for wake in wakes {
            wake.wake();
        }
        result
    }
    pub fn admission_binding(
        &self,
        authenticated: &AuthenticatedNamespace,
        command: &ServiceCommand,
    ) -> crate::value_plane_cluster_admission_075::AdmissionBinding {
        crate::value_plane_cluster_admission_075::AdmissionBinding {
            namespace: authenticated.address().namespace.clone(),
            namespace_generation: authenticated.address().generation,
            request_digest: command.test_request_digest(),
            identity: command.identity.clone(),
            deadline: command.deadline.min(authenticated.runtime_proof().4),
            grant_digest: authenticated.grant_digest(),
        }
    }
    pub fn logical_time(&self) -> u64 {
        self.cluster.lock().unwrap().logical_time()
    }
    pub fn statistics(&self) -> ServiceStatistics {
        self.cluster.lock().unwrap().aggregate_statistics()
    }
    pub fn resource_usage(&self) -> ServiceResourceUsage {
        self.cluster.lock().unwrap().aggregate_resource_usage()
    }
    pub fn namespace_owners(&self) -> NamespaceOwners {
        self.cluster.lock().unwrap().aggregate_namespace_owners()
    }
    pub fn preflight_registry_growth(
        &self,
        command: &ServiceCommand,
    ) -> Result<usize, ServiceClusterError> {
        self.cluster
            .lock()
            .unwrap()
            .preflight_registry_growth(command)
    }
    pub fn partition_for(&self, key: &[u8]) -> u32 {
        self.cluster.lock().unwrap().partition_for(key)
    }
    pub fn affected_partitions(&self, command: &ServiceCommand) -> Vec<u32> {
        let cluster = self.cluster.lock().unwrap();
        let mut partitions = std::collections::BTreeSet::new();
        if !command.entries.is_empty() {
            for entry in &command.entries {
                partitions.insert(cluster.partition_for(&entry.key));
            }
        } else if !command.keys.is_empty() {
            for key in &command.keys {
                partitions.insert(cluster.partition_for(key));
            }
        } else if !matches!(
            command.operation.as_str(),
            "get_all" | "put_all_detailed" | "remove_all_detailed" | "replace_all_detailed"
        ) {
            partitions.insert(cluster.partition_for(&command.key));
        }
        if let Some(identity) = &command.identity {
            if is_mutating_operation(&command.operation)
                || command.operation == "query_operation_outcome"
            {
                partitions.insert(cluster.coordinator_for(identity));
            }
        }
        if partitions.is_empty() {
            partitions.insert(0);
        }
        partitions.into_iter().collect()
    }
    pub fn command_replica_bound(
        &self,
        command: &ServiceCommand,
    ) -> Result<usize, ServiceClusterError> {
        self.cluster.lock().unwrap().command_replica_bound(command)
    }
    pub fn revision(&self) -> u64 {
        self.cluster.lock().unwrap().max_revision()
    }
    pub fn fence_namespace(&self) -> Result<(), ServiceClusterError> {
        let wakes = self.cluster.lock().unwrap().fence_namespace_deferred();
        for wake in wakes {
            wake.wake();
        }
        Ok(())
    }
    pub fn reclaim_namespace_step(
        &self,
        maximum: usize,
    ) -> Result<NamespaceReclaimProgress, ServiceClusterError> {
        self.cluster.lock().unwrap().reclaim_namespace_step(maximum)
    }
    pub fn successor(
        &self,
        generation: u64,
        address: &NamespaceAddress,
        collection: CollectionBounds,
        limits: ServiceLimits,
        security: SecurityBounds,
        trust: TrustEpochWindow,
    ) -> Result<Self, ServiceClusterError> {
        if self.namespace_owners().total() != 0 {
            return Err(ServiceClusterError::Model(MapServiceError::ActiveOwners));
        }
        let cluster = self.cluster.lock().unwrap().empty_successor(generation)?;
        let authentication = RuntimeAuthentication::new(
            ClusterAuthConfig {
                cluster: self.config.cluster.clone(),
                tenant: address.tenant.clone(),
                namespace: address.namespace.clone(),
                generation,
                members: self.authentication.current_members(),
                max_replays: self.config.max_replays,
            },
            security,
            trust,
        )?;
        let _ = (collection, limits);
        Ok(Self {
            cluster: Arc::new(Mutex::new(cluster)),
            config: Arc::clone(&self.config),
            authentication,
        })
    }
    pub fn wake_listener_waiters(&self) {
        let wakes = self
            .cluster
            .lock()
            .unwrap()
            .wake_listener_waiters_deferred();
        for wake in wakes {
            wake.wake();
        }
    }
    pub fn validate_rotation(
        &self,
        next: u64,
        members: &[RuntimeMember],
    ) -> Result<(), ServiceClusterError> {
        self.authentication.validate_rotation(next, members)
    }
    pub fn rotate_credentials(
        &self,
        next: u64,
        now: u64,
        members: &[RuntimeMember],
    ) -> Result<(), ServiceClusterError> {
        self.authentication.rotate_credentials(next, now, members)
    }
    pub fn cancel_pending(&self, command: &ServiceCommand) -> Result<bool, ServiceClusterError> {
        self.cluster.lock().unwrap().cancel_pending(command)
    }
    pub fn cancel_identity(
        &self,
        identity: &crate::value_plane_model_075::MutationIdentity,
    ) -> Result<bool, ServiceClusterError> {
        self.cluster
            .lock()
            .unwrap()
            .cancel_pending_identity(identity)
    }
    pub fn reserve_operation(
        &self,
        _identity: crate::value_plane_model_075::MutationIdentity,
        _command: &ServiceCommand,
    ) -> Result<(), ServiceClusterError> {
        Err(ServiceClusterError::UnsupportedScope)
    }
    pub fn reserve_invocation(
        &self,
        authenticated: &AuthenticatedNamespace,
        permit: &ClusterPermit,
        command: &ServiceCommand,
        options: Option<&RuntimeCallOptions>,
    ) -> Result<(), ServiceClusterError> {
        authenticated
            .authorize_command(command)
            .map_err(|_| ServiceClusterError::Authorization("operation_policy"))?;
        self.authentication
            .validate_principal(authenticated, command.now)?;
        if permit.work().lane != ClusterLane::Data
            || permit.work().tenant != authenticated.address().tenant
            || !permit
                .keys_match(&crate::value_plane_service_registry_075::runtime_command_keys(command))
            || self
                .affected_partitions(command)
                .iter()
                .any(|partition| !permit.partitions().contains(partition))
        {
            return Err(ServiceClusterError::Authorization("admission_binding"));
        }
        if self.command_replica_bound(command)? > permit.work().replica_bytes {
            return Err(ServiceClusterError::Authorization("replica_credit"));
        }
        let binding = self.admission_binding(authenticated, command);
        let _attempt = permit
            .begin_invocation(binding.clone())
            .map_err(|_| ServiceClusterError::Authorization("admission_receipt"))?;
        self.sync_members()?;
        let options = options.cloned().unwrap_or_default();
        let guard = self.authentication.guard_with_receipt(
            authenticated,
            permit.ticket(),
            permit.work().replica_bytes,
            options.auth_fault,
            Some(permit.receipt_digest(&binding)),
        );
        self.with_cluster_wakes(|cluster| {
            cluster.reserve_operation_guarded(
                options
                    .entry_node
                    .as_deref()
                    .unwrap_or(&self.config.entry_node),
                command,
                guard,
            )
        })
    }
    pub(crate) fn recover_invocation(
        &self,
        authenticated: &AuthenticatedNamespace,
        command: &ServiceCommand,
    ) -> Result<ServiceRecovery, ServiceClusterError> {
        self.authentication
            .validate_principal(authenticated, command.now)?;
        authenticated
            .authorize_command(command)
            .map_err(|_| ServiceClusterError::Authorization("operation_policy"))?;
        self.cluster.lock().unwrap().recover_invocation(command)
    }
    pub(crate) fn recovered_execution(
        &self,
        command: &ServiceCommand,
        result: ServiceResult,
    ) -> ServiceClusterExecution {
        let partitions = self.affected_partitions(command);
        let cluster = self.cluster.lock().unwrap();
        let proved = partitions
            .iter()
            .all(|partition| cluster.backup_proved(*partition));
        ServiceClusterExecution {
            result,
            replayed: true,
            backup_proved: proved,
            routes: partitions
                .iter()
                .filter_map(|partition| cluster.assignment(*partition))
                .collect(),
            failure: (!proved).then_some(ServiceClusterError::ReplicaNotProved),
        }
    }
    pub fn page_source(
        &self,
        projection: ScanProjection,
        now: u64,
    ) -> Result<ServicePageSource, ServiceClusterError> {
        self.cluster.lock().unwrap().page_source(projection, now)
    }
    pub fn listener_source(
        &self,
        options: ServiceListenerOptions,
    ) -> Result<ServiceListenerSource, ServiceClusterError> {
        self.cluster.lock().unwrap().listener_source(options)
    }
    pub fn resync_listener(
        &self,
        id: u64,
        now: u64,
        maximum_items: usize,
        maximum_bytes: usize,
    ) -> Result<ListenerSnapshot, ServiceClusterError> {
        self.cluster
            .lock()
            .unwrap()
            .resync_listener(id, now, maximum_items, maximum_bytes)
    }
    pub fn cluster_listener_source(
        &self,
        authenticated: &AuthenticatedNamespace,
        options: ClusterSubscriptionOptions,
    ) -> Result<ClusterListenerSource, ServiceClusterError> {
        authenticated
            .authorize_listener(&options.listener)
            .map_err(|_| ServiceClusterError::Authorization("operation_policy"))?;
        self.authentication
            .validate_principal(authenticated, self.logical_time())?;
        ClusterListenerSource::new(Arc::clone(&self.cluster), options)
    }
    pub fn preflight_cluster_resync(
        &self,
        now: u64,
        maximum_bytes: usize,
    ) -> Result<usize, ServiceClusterError> {
        let cluster = self.cluster.lock().unwrap();
        let mut observation = ServiceCommand::point("get", b"");
        observation.now = now;
        maximum_bytes
            .checked_add(
                cluster
                    .preflight_registry_growth(&observation)?
                    .checked_mul(cluster.partition_count() as usize)
                    .ok_or(ServiceClusterError::InvalidBound)?,
            )
            .ok_or(ServiceClusterError::InvalidBound)
    }
    pub fn resync_cluster_listener(
        &self,
        authenticated: &AuthenticatedNamespace,
        id: u64,
        now: u64,
        maximum_items: usize,
        maximum_bytes: usize,
    ) -> Result<ClusterSubscriptionSnapshot, ServiceClusterError> {
        authenticated
            .authorize(
                crate::value_plane_security_075::GrantOperation::Listen,
                &[],
                true,
            )
            .map_err(|_| ServiceClusterError::Authorization("operation_policy"))?;
        authenticated
            .authorize(
                crate::value_plane_security_075::GrantOperation::Read,
                &[],
                true,
            )
            .map_err(|_| ServiceClusterError::Authorization("operation_policy"))?;
        self.authentication.validate_principal(authenticated, now)?;
        let mut cluster = self.cluster.lock().unwrap();
        let result = cluster.resync_subscription(id, now, maximum_items, maximum_bytes);
        let wakes = cluster.take_deferred_wakes();
        drop(cluster);
        for wake in wakes {
            wake.wake();
        }
        result
    }
}
