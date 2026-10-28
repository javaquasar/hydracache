//! Actual bounded runtime readiness and drain observations. Labels are fixed reasons only.

use crate::value_plane_cluster_runtime_075::ClusterNamespaceRuntime;
use crate::value_plane_service_075::NamespaceOwners;
use crate::value_plane_service_cluster_075::{ServiceClusterError, ServiceLinkFaults};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReadinessReason {
    AuthenticationUnavailable,
    NamespaceFenced,
    Draining,
    OwnerUnavailable,
    BackupUnavailable,
    BackupUnproved,
    TransferPending,
}
impl ReadinessReason {
    pub const fn label(self) -> &'static str {
        match self {
            Self::AuthenticationUnavailable => "authentication_unavailable",
            Self::NamespaceFenced => "namespace_fenced",
            Self::Draining => "draining",
            Self::OwnerUnavailable => "owner_unavailable",
            Self::BackupUnavailable => "backup_unavailable",
            Self::BackupUnproved => "backup_unproved",
            Self::TransferPending => "transfer_pending",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartitionHealth {
    pub partition: u32,
    pub epoch: u64,
    pub owner_live: bool,
    pub backup_live: bool,
    pub backup_proved: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeHealth {
    pub authentication_ready: bool,
    pub replay: crate::value_plane_security_075::ReplayWindowObservations,
    pub production_enabled: bool,
    pub ready: bool,
    pub reasons: Vec<ReadinessReason>,
    pub partitions: Vec<PartitionHealth>,
    pub owners: NamespaceOwners,
    pub authentication_rejections: Vec<(&'static str, u64)>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrainProgress {
    pub moved: usize,
    pub reclaimed: usize,
    pub remaining_roles: usize,
    pub blocked_owners: usize,
    pub complete: bool,
}

#[derive(Debug, Clone)]
pub enum RuntimeMaintenance {
    Repair {
        partition: u32,
        backup: String,
        faults: ServiceLinkFaults,
    },
    Promote {
        partition: u32,
    },
    SetNodeLive {
        node: String,
        live: bool,
    },
    BeginDrain {
        node: String,
    },
    DrainStep {
        node: String,
        maximum_partitions: usize,
        maximum_reclaim: usize,
    },
}
impl RuntimeMaintenance {
    pub fn repair_lane(&self) -> bool {
        matches!(self, Self::Repair { .. } | Self::DrainStep { .. })
    }
    pub fn partition(&self) -> u32 {
        match self {
            Self::Repair { partition, .. } | Self::Promote { partition } => *partition,
            _ => 0,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaintenanceResult {
    Completed,
    Drain(DrainProgress),
}

impl ClusterNamespaceRuntime {
    pub fn health(&self) -> Result<RuntimeHealth, ServiceClusterError> {
        let cluster = self.cluster.lock().unwrap();
        let mut reasons = std::collections::BTreeSet::new();
        let authentication_ready = self.authentication.ready();
        if !authentication_ready {
            reasons.insert(ReadinessReason::AuthenticationUnavailable);
        }
        let mut partitions = Vec::new();
        if cluster.is_namespace_fenced() {
            reasons.insert(ReadinessReason::NamespaceFenced);
        }
        for transfer in cluster.transfer_readiness() {
            if !matches!(
                transfer.phase,
                crate::value_plane_service_cluster_075::OnlineTransferPhase::Committed
                    | crate::value_plane_service_cluster_075::OnlineTransferPhase::Aborted
            ) {
                reasons.insert(ReadinessReason::TransferPending);
            }
        }
        for node in cluster.node_ids() {
            if cluster.node_is_draining(&node)
                && (0..cluster.partition_count()).any(|partition| {
                    cluster
                        .assignment(partition)
                        .is_some_and(|route| route.owner == node || route.backup == node)
                })
            {
                reasons.insert(ReadinessReason::Draining);
            }
        }
        for partition in 0..cluster.partition_count() {
            let route = cluster
                .assignment(partition)
                .ok_or(ServiceClusterError::InvalidBound)?;
            let owner_live = cluster.node_is_live(&route.owner)?;
            let backup_live = cluster.node_is_live(&route.backup)?;
            let backup_proved = cluster.backup_proved(partition);
            if !owner_live {
                reasons.insert(ReadinessReason::OwnerUnavailable);
            }
            if !backup_live {
                reasons.insert(ReadinessReason::BackupUnavailable);
            }
            if !backup_proved {
                reasons.insert(ReadinessReason::BackupUnproved);
            }
            partitions.push(PartitionHealth {
                partition,
                epoch: route.epoch,
                owner_live,
                backup_live,
                backup_proved,
            });
        }
        let owners = cluster.aggregate_namespace_owners();
        drop(cluster);
        Ok(RuntimeHealth {
            authentication_ready,
            replay: self.authentication.replay_observations(),
            production_enabled: false,
            ready: reasons.is_empty(),
            reasons: reasons.into_iter().collect(),
            partitions,
            owners,
            authentication_rejections: self.authentication.failures().into_iter().collect(),
        })
    }
    pub fn set_node_live(&self, node: &str, live: bool) -> Result<(), ServiceClusterError> {
        self.with_cluster_wakes(|cluster| cluster.set_node_live(node, live))
    }
    pub fn promote_backup(&self, partition: u32) -> Result<(), ServiceClusterError> {
        self.with_cluster_wakes(|cluster| cluster.promote_backup(partition).map(|_| ()))
    }
    pub fn repair_backup(
        &self,
        partition: u32,
        node: &str,
        faults: ServiceLinkFaults,
    ) -> Result<(), ServiceClusterError> {
        self.with_cluster_wakes(|cluster| cluster.repair_backup(partition, node, faults))
    }
    pub fn begin_drain(&self, node: &str) -> Result<(), ServiceClusterError> {
        self.with_cluster_wakes(|cluster| cluster.begin_node_drain(node))
    }
    pub fn maintain(
        &self,
        authenticated: &crate::value_plane_service_registry_075::AuthenticatedNamespace,
        permit: &crate::value_plane_cluster_admission_075::ClusterPermit,
        operation: &RuntimeMaintenance,
    ) -> Result<MaintenanceResult, ServiceClusterError> {
        authenticated
            .authorize(
                crate::value_plane_security_075::GrantOperation::Admin,
                &[],
                true,
            )
            .map_err(|_| ServiceClusterError::Authorization("operation_policy"))?;
        let lane = if operation.repair_lane() {
            crate::value_plane_cluster_admission_075::ClusterLane::Repair
        } else {
            crate::value_plane_cluster_admission_075::ClusterLane::Safety
        };
        if permit.work().lane != lane || permit.work().tenant != authenticated.address().tenant {
            return Err(ServiceClusterError::Authorization("admission_binding"));
        }
        self.sync_members()?;
        self.authentication
            .validate_principal(authenticated, self.logical_time())?;
        use sha2::Digest;
        let binding = crate::value_plane_cluster_admission_075::AdmissionBinding {
            namespace: authenticated.address().namespace.clone(),
            namespace_generation: authenticated.address().generation,
            request_digest: sha2::Sha256::digest(format!("{operation:?}").as_bytes()).into(),
            identity: None,
            deadline: authenticated.runtime_proof().4,
            grant_digest: authenticated.grant_digest(),
        };
        let _attempt = permit
            .begin_invocation(binding.clone())
            .map_err(|_| ServiceClusterError::Authorization("admission_receipt"))?;
        let guard = self.authentication.guard_with_receipt(
            authenticated,
            permit.ticket(),
            permit.work().replica_bytes,
            None,
            Some(permit.receipt_digest(&binding)),
        );
        if operation.repair_lane()
            && self.maintenance_replica_bound(operation)? > permit.work().replica_bytes
        {
            return Err(ServiceClusterError::Authorization("replica_credit"));
        }
        match operation {
            RuntimeMaintenance::Repair {
                partition,
                backup,
                faults,
            } => {
                self.with_cluster_wakes(|cluster| {
                    cluster.with_message_guard(guard, |cluster| {
                        cluster.repair_backup(*partition, backup, *faults)
                    })
                })?;
                Ok(MaintenanceResult::Completed)
            }
            RuntimeMaintenance::Promote { partition } => {
                self.with_cluster_wakes(|cluster| {
                    cluster.with_message_guard(guard, |cluster| cluster.promote_backup(*partition))
                })?;
                Ok(MaintenanceResult::Completed)
            }
            RuntimeMaintenance::SetNodeLive { node, live } => {
                self.set_node_live(node, *live)?;
                Ok(MaintenanceResult::Completed)
            }
            RuntimeMaintenance::BeginDrain { node } => {
                self.begin_drain(node)?;
                Ok(MaintenanceResult::Completed)
            }
            RuntimeMaintenance::DrainStep {
                node,
                maximum_partitions,
                maximum_reclaim,
            } => self
                .drain_step_guarded(node, *maximum_partitions, *maximum_reclaim, Some(guard))
                .map(MaintenanceResult::Drain),
        }
    }
    pub fn maintenance_replica_bound(
        &self,
        operation: &RuntimeMaintenance,
    ) -> Result<usize, ServiceClusterError> {
        let cluster = self.cluster.lock().unwrap();
        match operation {
            RuntimeMaintenance::Repair { partition, .. } => {
                cluster.partition_replica_bound(*partition)
            }
            RuntimeMaintenance::DrainStep { .. } => (0..cluster.partition_count())
                .try_fold(0usize, |maximum, partition| {
                    Ok(maximum.max(cluster.partition_replica_bound(partition)?))
                }),
            _ => Ok(0),
        }
    }
    pub fn drain_step(
        &self,
        node: &str,
        maximum_partitions: usize,
        maximum_reclaim: usize,
    ) -> Result<DrainProgress, ServiceClusterError> {
        self.drain_step_guarded(node, maximum_partitions, maximum_reclaim, None)
    }
    fn drain_step_guarded(
        &self,
        node: &str,
        maximum_partitions: usize,
        maximum_reclaim: usize,
        guard: Option<
            std::sync::Arc<dyn crate::value_plane_service_cluster_075::ServiceMessageGuard>,
        >,
    ) -> Result<DrainProgress, ServiceClusterError> {
        if maximum_partitions == 0
            || maximum_partitions > self.config.bounds.partitions as usize
            || maximum_reclaim == 0
        {
            return Err(ServiceClusterError::InvalidBound);
        }
        self.with_cluster_wakes(|cluster| {
            if !cluster.node_is_draining(node) {
                return Err(ServiceClusterError::InvalidBound);
            }
            let mut moved = 0;
            for partition in 0..cluster.partition_count() {
                let mut route = cluster
                    .assignment(partition)
                    .ok_or(ServiceClusterError::InvalidBound)?;
                if route.owner != node && route.backup != node {
                    continue;
                }
                if moved == maximum_partitions {
                    break;
                }
                let candidates = cluster
                    .node_ids()
                    .into_iter()
                    .filter(|candidate| {
                        candidate != node
                            && cluster.node_is_live(candidate).unwrap_or(false)
                            && !cluster.node_is_draining(candidate)
                    })
                    .collect::<Vec<_>>();
                if candidates.len() < 2 {
                    return Err(ServiceClusterError::NodeUnavailable);
                }
                if route.owner == node {
                    let target = candidates
                        .iter()
                        .find(|candidate| **candidate != route.owner)
                        .expect("different live node");
                    route = if let Some(guard) = &guard {
                        cluster.with_message_guard(std::sync::Arc::clone(guard), |cluster| {
                            cluster.rebalance(partition, target, ServiceLinkFaults::default())
                        })?
                    } else {
                        cluster.rebalance(partition, target, ServiceLinkFaults::default())?
                    };
                }
                if route.backup == node {
                    let backup = candidates
                        .iter()
                        .find(|candidate| **candidate != route.owner)
                        .ok_or(ServiceClusterError::NodeUnavailable)?;
                    if let Some(guard) = &guard {
                        cluster.with_message_guard(std::sync::Arc::clone(guard), |cluster| {
                            cluster.repair_backup(partition, backup, ServiceLinkFaults::default())
                        })?;
                    } else {
                        cluster.repair_backup(partition, backup, ServiceLinkFaults::default())?;
                    }
                }
                moved += 1;
            }
            let remaining_roles = (0..cluster.partition_count())
                .filter(|partition| {
                    cluster
                        .assignment(*partition)
                        .is_some_and(|route| route.owner == node || route.backup == node)
                })
                .count();
            let progress = if remaining_roles == 0 {
                Some(cluster.reclaim_drained_node_step(node, maximum_reclaim)?)
            } else {
                None
            };
            let result = DrainProgress {
                moved,
                reclaimed: progress.as_ref().map_or(0, |progress| progress.reclaimed),
                remaining_roles,
                blocked_owners: progress
                    .as_ref()
                    .map_or(0, |progress| progress.blocked_owners),
                complete: remaining_roles == 0
                    && progress.is_some_and(|progress| {
                        progress.remaining == 0 && progress.blocked_owners == 0
                    }),
            };
            Ok(result)
        })
    }
}
