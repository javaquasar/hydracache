//! Read-only receipt/proof observations and the bounded admission seam used by helper futures.
use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelperGroupProgress {
    pub partition: u32,
    pub journal: bool,
    pub identity: MutationIdentity,
    pub indices: Vec<usize>,
    pub outcome: OperationOutcomeQuery,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelperProof {
    pub partition: u32,
    pub journal: bool,
    pub required: ReplicaStamp,
    pub acknowledged: Option<ReplicaStamp>,
    pub owner_live: bool,
    pub backup_live: bool,
    pub owner_matches: bool,
    pub backup_matches: bool,
    pub proved: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterHelperProgress {
    pub identity: MutationIdentity,
    pub original_digest: [u8; 32],
    pub root_partition: u32,
    pub root: OperationOutcomeQuery,
    pub groups: Vec<HelperGroupProgress>,
    pub proofs: Vec<HelperProof>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClusterHelperRecovery {
    Completed(ClusterHelperExecution),
    Pending,
    Unknown,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClusterHelperReservation {
    Reserved,
    Recovered(ClusterHelperExecution),
    NotApplied,
}

fn outcome(recovered: ServiceRecovery) -> OperationOutcomeQuery {
    match recovered {
        ServiceRecovery::Unknown => OperationOutcomeQuery::Unknown,
        ServiceRecovery::Pending => OperationOutcomeQuery::Pending,
        ServiceRecovery::Completed(result) => OperationOutcomeQuery::Completed(result),
    }
}
fn group_digest(group: &Group, generation: u64, command: &ClusterHelperCommand) -> [u8; 32] {
    if let ClusterHelperCommand::RetireOutcomes { through, .. } = command {
        retirement_digest(&group.retirement_clients, generation, *through)
    } else {
        typed_helper_digest(&group.command)
    }
}

impl ServiceClusterHarness {
    /// Proof membership borrows keys and work quotas; it never owns entry values.
    pub(super) fn helper_domains(
        &self,
        request: &ClusterHelperRequest,
    ) -> Result<BTreeSet<(u32, bool)>, ServiceClusterError> {
        let mut domains = BTreeSet::from([(self.coordinator_for(&request.identity), true)]);
        match &request.command {
            ClusterHelperCommand::VersionBulk { entries, .. } => {
                domains.extend(
                    entries
                        .iter()
                        .map(|entry| (self.partition_for(&entry.key), false)),
                );
            }
            ClusterHelperCommand::RetireOutcomes { .. } => {
                for partition in 0..self.bounds.partitions {
                    domains.insert((partition, false));
                    domains.insert((partition, true));
                }
            }
            ClusterHelperCommand::ExpireStep { max_work } => {
                let count = self.bounds.partitions as usize;
                let start = (request.identity.sequence % count as u64) as usize;
                for offset in 0..count {
                    if max_work / count + usize::from(offset < max_work % count) != 0 {
                        domains.insert((((start + offset) % count) as u32, false));
                    }
                }
            }
            _ => return Err(MapServiceError::InvalidArgument.into()),
        }
        Ok(domains)
    }

    fn helper_proof(
        &self,
        partition: u32,
        journal: bool,
    ) -> Result<HelperProof, ServiceClusterError> {
        let assignment = &self.assignments[partition as usize];
        let required = if journal {
            assignment.required_journal
        } else {
            assignment.required_data
        };
        let acknowledged = if journal {
            assignment.backup_journal
        } else {
            assignment.backup_data
        };
        let owner_live = self.node_is_live(&assignment.route.owner)?;
        let backup_live = self.node_is_live(&assignment.route.backup)?;
        let owner_matches = self
            .service(&assignment.route.owner, partition, journal)?
            .replica_matches_readonly(required)?;
        let backup_matches = self
            .service(&assignment.route.backup, partition, journal)?
            .replica_matches_readonly(required)?;
        Ok(HelperProof {
            partition,
            journal,
            required,
            acknowledged,
            owner_live,
            backup_live,
            owner_matches,
            backup_matches,
            proved: owner_live
                && backup_live
                && owner_matches
                && backup_matches
                && acknowledged == Some(required),
        })
    }

    pub fn helper_progress_bytes(
        &self,
        original: &ClusterHelperRequest,
        original_digest: [u8; 32],
        maximum_bytes: usize,
    ) -> Result<usize, ServiceClusterError> {
        self.validate_cluster_helper(original)?;
        if !original.is_mutating()
            || maximum_bytes == 0
            || maximum_bytes > self.service_limits.query_bytes
        {
            return Err(MapServiceError::InvalidArgument.into());
        }
        if original.request_digest() != original_digest {
            return Err(MapServiceError::from(
                crate::value_plane_collection_075::CollectionError::MutationIdentityConflict,
            )
            .into());
        }
        let root = self.coordinator_for(&original.identity);
        let route = &self.assignments[root as usize].route;
        let root_bytes = self
            .service(&route.owner, root, true)?
            .retained_operation_bytes(&original.identity, original_digest)?;
        let mut bytes = 256usize
            .checked_add(original.identity.client.len())
            .and_then(|n| n.checked_add(root_bytes))
            .ok_or(ServiceClusterError::InvalidBound)?;
        let mut domains = BTreeSet::from([(root, true)]);
        for group in self.helper_groups(original) {
            let route = &self.assignments[group.partition as usize].route;
            let stored = self
                .service(&route.owner, group.partition, group.journal)?
                .retained_operation_bytes(
                    &group.identity,
                    group_digest(&group, self.generation, &original.command),
                )?;
            bytes = bytes
                .checked_add(128)
                .and_then(|n| n.checked_add(group.identity.client.len()))
                .and_then(|n| n.checked_add(group.indices.len().checked_mul(8)?))
                .and_then(|n| n.checked_add(stored))
                .ok_or(ServiceClusterError::InvalidBound)?;
            domains.insert((group.partition, group.journal));
        }
        bytes = bytes
            .checked_add(
                domains
                    .len()
                    .checked_mul(192)
                    .ok_or(ServiceClusterError::InvalidBound)?,
            )
            .ok_or(ServiceClusterError::InvalidBound)?;
        if bytes > maximum_bytes {
            return Err(MapServiceError::ResourceBudget("helper_progress_response").into());
        }
        Ok(bytes)
    }

    pub fn helper_progress(
        &self,
        original: &ClusterHelperRequest,
        original_digest: [u8; 32],
        maximum_bytes: usize,
    ) -> Result<ClusterHelperProgress, ServiceClusterError> {
        self.helper_progress_bytes(original, original_digest, maximum_bytes)?;
        let root_partition = self.coordinator_for(&original.identity);
        let root_route = &self.assignments[root_partition as usize].route;
        let root = outcome(
            self.service(&root_route.owner, root_partition, true)?
                .recover_external_operation(&original.identity, original_digest)?,
        );
        let mut groups = Vec::new();
        let mut domains = BTreeSet::from([(root_partition, true)]);
        for group in self.helper_groups(original) {
            let route = &self.assignments[group.partition as usize].route;
            let result = self
                .service(&route.owner, group.partition, group.journal)?
                .recover_external_operation(
                    &group.identity,
                    group_digest(&group, self.generation, &original.command),
                )?;
            domains.insert((group.partition, group.journal));
            groups.push(HelperGroupProgress {
                partition: group.partition,
                journal: group.journal,
                identity: group.identity,
                indices: group.indices,
                outcome: outcome(result),
            });
        }
        let proofs = domains
            .into_iter()
            .map(|(partition, journal)| self.helper_proof(partition, journal))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ClusterHelperProgress {
            identity: original.identity.clone(),
            original_digest,
            root_partition,
            root,
            groups,
            proofs,
        })
    }

    pub fn recover_cluster_helper(
        &self,
        request: &ClusterHelperRequest,
    ) -> Result<ClusterHelperRecovery, ServiceClusterError> {
        self.validate_cluster_helper(request)?;
        if !request.is_mutating() {
            return Ok(ClusterHelperRecovery::Unknown);
        }
        let root = self.coordinator_for(&request.identity);
        let route = &self.assignments[root as usize].route;
        Ok(
            match self
                .service(&route.owner, root, true)?
                .recover_external_operation(&request.identity, request.request_digest())?
            {
                ServiceRecovery::Unknown => ClusterHelperRecovery::Unknown,
                ServiceRecovery::Pending => ClusterHelperRecovery::Pending,
                ServiceRecovery::Completed(result) => {
                    let domains = self.helper_domains(request)?;
                    let proved =
                        domains
                            .iter()
                            .try_fold(true, |proved, (partition, journal)| {
                                Ok::<_, ServiceClusterError>(
                                    proved && self.helper_proof(*partition, *journal)?.proved,
                                )
                            })?;
                    let mut routes = vec![route.clone()];
                    for partition in domains
                        .into_iter()
                        .map(|(partition, _)| partition)
                        .collect::<BTreeSet<_>>()
                    {
                        if partition != root {
                            routes.push(self.assignments[partition as usize].route.clone());
                        }
                    }
                    ClusterHelperRecovery::Completed(ClusterHelperExecution {
                        result: if proved {
                            decode(&request.command, *result)?
                        } else {
                            uncertain(&request.command, &result)
                        },
                        replayed: true,
                        backup_proved: proved,
                        routes,
                        failure: (!proved).then_some(ServiceClusterError::ReplicaNotProved),
                    })
                }
            },
        )
    }

    pub fn reserve_cluster_helper_guarded(
        &mut self,
        entry: &str,
        request: &ClusterHelperRequest,
        call: &ServiceClusterCall,
        guard: Arc<dyn ServiceMessageGuard>,
    ) -> Result<ClusterHelperReservation, ServiceClusterError> {
        self.cluster_helper_preflight(request)?;
        if !request.is_mutating() {
            return Err(MapServiceError::InvalidArgument.into());
        }
        if let ClusterHelperRecovery::Completed(result) = self.recover_cluster_helper(request)? {
            return Ok(ClusterHelperReservation::Recovered(result));
        }
        let previous = self.runtime.invocation.replace(runtime_hooks::Invocation {
            digest: request.request_digest(),
            identity: Some(request.identity.clone()),
            now: request.now,
            deadline: request.deadline,
        });
        let result = self.with_message_guard(guard, |cluster| {
            cluster.require_live(entry)?;
            if cluster.node_is_draining(entry) {
                return Err(ServiceClusterError::NodeUnavailable);
            }
            let root = cluster.coordinator_for(&request.identity);
            let route = cluster.route(entry, root, call)?;
            let journal = cluster.service(&route.owner, root, true)?;
            let pending = matches!(
                journal.recover_external_operation(&request.identity, request.request_digest())?,
                ServiceRecovery::Pending
            );
            if request.now >= request.deadline {
                return Ok(if pending {
                    ClusterHelperReservation::Reserved
                } else {
                    ClusterHelperReservation::NotApplied
                });
            }
            let payload = request.payload(cluster.bounds.max_frame_bytes)?;
            cluster.authorize_message(
                ServiceMessageKind::Request,
                entry,
                &route.owner,
                root,
                None,
                &payload,
            )?;
            let maximum = match &request.command {
                ClusterHelperCommand::VersionBulk { entries, .. } => {
                    64 + entries
                        .iter()
                        .map(|entry| 64 + entry.key.len())
                        .sum::<usize>()
                }
                _ => 64,
            };
            cluster.reserve_transfer_credits(
                request.credit_key(),
                &cluster.helper_transfer_credits(request, maximum)?,
            )?;
            if cluster.transmit(
                root,
                "helper_reserve_request",
                payload,
                call.faults.coordinator.request,
            )? == 0
            {
                return if pending {
                    Err(ServiceClusterError::TransportTimeout)
                } else {
                    Ok(ClusterHelperReservation::NotApplied)
                };
            }
            journal.admit_external_operation(
                request.identity.clone(),
                request.request_digest(),
                maximum,
            )?;
            cluster.assignments[root as usize].required_journal =
                journal.journal_replica_stamp()?;
            if !cluster.replicate(root, true, call.faults.coordinator)? {
                return Err(ServiceClusterError::ReplicaNotProved);
            }
            Ok(ClusterHelperReservation::Reserved)
        });
        self.runtime.invocation = previous;
        self.capture_online_changes();
        self.signal_subscriptions();
        result
    }

    pub fn cancel_cluster_helper(
        &mut self,
        request: &ClusterHelperRequest,
    ) -> Result<bool, ServiceClusterError> {
        let result = (|| {
            self.validate_cluster_helper(request)?;
            if !request.is_mutating() {
                return Err(MapServiceError::InvalidArgument.into());
            }
            let root = self.coordinator_for(&request.identity);
            let route = self.assignments[root as usize].route.clone();
            self.require_live(&route.owner)?;
            let journal = self.service(&route.owner, root, true)?;
            if !matches!(
                journal.recover_external_operation(&request.identity, request.request_digest())?,
                ServiceRecovery::Pending
            ) {
                return Ok(false);
            }
            journal.cancel_admitted_identity(&request.identity)?;
            self.assignments[root as usize].required_journal = journal.journal_replica_stamp()?;
            if !self.replicate(root, true, ServiceLinkFaults::default())? {
                return Err(ServiceClusterError::ReplicaNotProved);
            }
            Ok(true)
        })();
        self.capture_online_changes();
        self.signal_subscriptions();
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Allow;
    impl ServiceMessageGuard for Allow {
        fn authorize(&self, _: &ServiceMessageContext) -> Result<(), ServiceClusterError> {
            Ok(())
        }
    }
    struct Deny;
    impl ServiceMessageGuard for Deny {
        fn authorize(&self, _: &ServiceMessageContext) -> Result<(), ServiceClusterError> {
            Err(ServiceClusterError::Authorization("injected"))
        }
    }
    fn cluster() -> ServiceClusterHarness {
        ServiceClusterHarness::new(
            &["a", "b", "c"],
            ServiceClusterBounds {
                partitions: 1,
                ..ServiceClusterBounds::default()
            },
            CollectionBounds::default(),
            ServiceLimits::default(),
            1,
        )
        .unwrap()
    }
    fn request() -> ClusterHelperRequest {
        ClusterHelperRequest::new(
            MutationIdentity::new("progress", 1),
            ClusterHelperCommand::ExpireStep { max_work: 1 },
        )
    }
    fn observed(
        cluster: &ServiceClusterHarness,
    ) -> (
        ServiceStatistics,
        ServiceResourceUsage,
        VecDeque<ServiceClusterTrace>,
        u64,
        u64,
        u64,
    ) {
        (
            cluster.aggregate_statistics(),
            cluster.aggregate_resource_usage(),
            cluster.trace().clone(),
            cluster.logical_time(),
            cluster.max_revision(),
            cluster.sequence,
        )
    }

    #[test]
    fn helper_progress_reads_unknown_pending_completed_receipts_and_proofs_without_effects() {
        let mut cluster = cluster();
        let original = request();
        let before = observed(&cluster);
        let unknown = cluster
            .helper_progress(&original, original.request_digest(), 16_384)
            .unwrap();
        assert_eq!(unknown.root, OperationOutcomeQuery::Unknown);
        assert!(unknown
            .groups
            .iter()
            .all(|group| group.outcome == OperationOutcomeQuery::Unknown));
        assert_eq!(observed(&cluster), before);
        assert_eq!(
            cluster
                .reserve_cluster_helper_guarded(
                    "c",
                    &original,
                    &ServiceClusterCall::default(),
                    Arc::new(Allow)
                )
                .unwrap(),
            ClusterHelperReservation::Reserved
        );
        let before = observed(&cluster);
        let pending = cluster
            .helper_progress(&original, original.request_digest(), 16_384)
            .unwrap();
        assert_eq!(pending.root, OperationOutcomeQuery::Pending);
        assert!(pending
            .proofs
            .iter()
            .any(|proof| proof.journal && proof.proved));
        assert!(pending
            .groups
            .iter()
            .all(|group| group.outcome == OperationOutcomeQuery::Unknown));
        assert_eq!(observed(&cluster), before);
        cluster.set_node_live("a", false).unwrap();
        let before = observed(&cluster);
        let offline = cluster
            .helper_progress(&original, original.request_digest(), 16_384)
            .unwrap();
        assert!(offline
            .proofs
            .iter()
            .all(|proof| !proof.owner_live && !proof.proved));
        assert_eq!(observed(&cluster), before);
        cluster.set_node_live("a", true).unwrap();
        let completed = cluster
            .execute_cluster_helper_guarded(
                "c",
                original.clone(),
                ServiceClusterCall::default(),
                Arc::new(Allow),
            )
            .unwrap();
        assert!(completed.backup_proved);
        let before = observed(&cluster);
        let progress = cluster
            .helper_progress(&original, original.request_digest(), 16_384)
            .unwrap();
        assert!(matches!(progress.root, OperationOutcomeQuery::Completed(_)));
        assert!(progress
            .groups
            .iter()
            .all(|group| matches!(group.outcome, OperationOutcomeQuery::Completed(_))));
        assert!(progress.proofs.iter().all(|proof| proof.proved));
        assert!(
            matches!(cluster.recover_cluster_helper(&original).unwrap(), ClusterHelperRecovery::Completed(result) if result.backup_proved && result.result == completed.result)
        );
        assert_eq!(observed(&cluster), before);
        let query = ClusterHelperRequest::new(
            MutationIdentity::new("query", 1),
            ClusterHelperCommand::QueryProgress {
                original: Box::new(original.clone()),
                original_digest: original.request_digest(),
                max_bytes: 16_384,
            },
        );
        let queried = cluster
            .execute_cluster_helper_guarded(
                "not-a-node",
                query,
                ServiceClusterCall::default(),
                Arc::new(Deny),
            )
            .unwrap();
        assert!(
            matches!(queried.result, ClusterHelperResult::Progress(progress) if *progress == cluster.helper_progress(&original, original.request_digest(),16_384).unwrap())
        );
        assert_eq!(observed(&cluster), before);
    }

    #[test]
    fn helper_progress_bounds_digest_scope_and_pending_drop_never_invent_absence() {
        let mut cluster = cluster();
        let original = request();
        assert!(matches!(
            cluster.reserve_cluster_helper_guarded(
                "c",
                &original,
                &ServiceClusterCall::default(),
                Arc::new(Deny)
            ),
            Err(ServiceClusterError::Authorization("injected"))
        ));
        assert_eq!(cluster.aggregate_namespace_owners().pending, 0);
        let mut drop_request = ServiceClusterCall::default();
        drop_request.faults.coordinator.request = LoopbackFault::Drop;
        assert_eq!(
            cluster
                .reserve_cluster_helper_guarded("c", &original, &drop_request, Arc::new(Allow))
                .unwrap(),
            ClusterHelperReservation::NotApplied
        );
        assert_eq!(cluster.aggregate_namespace_owners().pending, 0);
        cluster
            .reserve_cluster_helper_guarded(
                "c",
                &original,
                &ServiceClusterCall::default(),
                Arc::new(Allow),
            )
            .unwrap();
        assert_eq!(
            cluster
                .reserve_cluster_helper_guarded("c", &original, &drop_request, Arc::new(Allow))
                .unwrap_err(),
            ServiceClusterError::TransportTimeout
        );
        assert_eq!(cluster.aggregate_namespace_owners().pending, 2);
        let before = observed(&cluster);
        assert!(
            matches!(cluster.helper_progress(&original, [7; 32],16_384), Err(ServiceClusterError::Model(error)) if error.kind() == "mutation_identity_conflict")
        );
        assert!(matches!(
            cluster.helper_progress(&original, original.request_digest(), 1),
            Err(ServiceClusterError::Model(MapServiceError::ResourceBudget(
                "helper_progress_response"
            )))
        ));
        let mut changed = original.clone();
        changed.command = ClusterHelperCommand::ExpireStep { max_work: 2 };
        changed.deadline = 0;
        assert!(
            matches!(cluster.helper_progress(&changed, changed.request_digest(),16_384), Err(ServiceClusterError::Model(error)) if error.kind() == "mutation_identity_conflict")
        );
        assert!(
            matches!(cluster.cancel_cluster_helper(&changed), Err(ServiceClusterError::Model(error)) if error.kind() == "mutation_identity_conflict")
        );
        assert_eq!(observed(&cluster), before);
        let read = ClusterHelperRequest::new(
            MutationIdentity::new("read", 1),
            ClusterHelperCommand::EntryViews {
                keys: vec![b"a".to_vec()],
                max_bytes: 1_024,
            },
        );
        let nested = ClusterHelperRequest::new(
            MutationIdentity::new("nested", 1),
            ClusterHelperCommand::QueryProgress {
                original: Box::new(read),
                original_digest: [0; 32],
                max_bytes: 1_024,
            },
        );
        assert_eq!(
            cluster.validate_cluster_helper(&nested).unwrap_err(),
            ServiceClusterError::Model(MapServiceError::InvalidArgument)
        );
        assert!(cluster.cancel_cluster_helper(&original).unwrap());
        let terminal = cluster
            .helper_progress(&original, original.request_digest(), 16_384)
            .unwrap();
        assert!(
            matches!(terminal.root, OperationOutcomeQuery::Completed(result) if result.kind == "outcome_unknown")
        );
        assert_eq!(cluster.aggregate_namespace_owners().pending, 0);
        let _ = cluster.fence_namespace_deferred();
        assert!(cluster
            .helper_progress(&original, original.request_digest(), 16_384)
            .is_err());
    }

    #[test]
    fn helper_recovery_uses_only_borrowed_affected_domains_and_exact_backup_proofs() {
        let mut cluster = ServiceClusterHarness::new(
            &["a", "b", "c"],
            ServiceClusterBounds::default(),
            CollectionBounds::default(),
            ServiceLimits::default(),
            1,
        )
        .unwrap();
        let identity = (0..1_000)
            .map(|index| MutationIdentity::new(format!("borrowed-{index}"), 1))
            .find(|identity| cluster.coordinator_for(identity) == 0)
            .unwrap();
        let key = (0..1_000u32)
            .map(|index| index.to_le_bytes().to_vec())
            .find(|key| cluster.partition_for(key) == 2)
            .unwrap();
        let original = ClusterHelperRequest::new(
            identity.clone(),
            ClusterHelperCommand::VersionBulk {
                operation: VersionBulkOperation::Replace,
                entries: vec![VersionBulkEntry {
                    key,
                    token: VersionToken {
                        namespace_generation: 1,
                        map_id: 0,
                        version: 0,
                    },
                    value: Some(vec![91; 256 * 1_024]),
                    ttl: ServiceTtl::Preserve,
                }],
            },
        );
        assert_eq!(
            cluster.helper_domains(&original).unwrap(),
            BTreeSet::from([(0, true), (2, false)])
        );
        let completed = cluster
            .execute_cluster_helper_guarded(
                "c",
                original.clone(),
                ServiceClusterCall::default(),
                Arc::new(Allow),
            )
            .unwrap();
        assert!(completed.backup_proved);
        let before = observed(&cluster);
        assert!(
            matches!(cluster.recover_cluster_helper(&original).unwrap(), ClusterHelperRecovery::Completed(result) if result.backup_proved && result.result == completed.result)
        );
        assert_eq!(observed(&cluster), before);

        // Unrelated pending replication cannot hide this helper's already-proved receipt.
        cluster.assignments[1].backup_data = None;
        assert!(
            matches!(cluster.recover_cluster_helper(&original).unwrap(), ClusterHelperRecovery::Completed(result) if result.backup_proved)
        );
        cluster.assignments[2].backup_data = None;
        let before = observed(&cluster);
        assert!(
            matches!(cluster.recover_cluster_helper(&original).unwrap(), ClusterHelperRecovery::Completed(result) if !result.backup_proved && result.failure == Some(ServiceClusterError::ReplicaNotProved))
        );
        assert_eq!(observed(&cluster), before);

        let retire = ClusterHelperRequest::new(
            identity.clone(),
            ClusterHelperCommand::RetireOutcomes {
                client: "retired".into(),
                through: 1,
            },
        );
        assert_eq!(
            cluster.helper_domains(&retire).unwrap(),
            BTreeSet::from([
                (0, false),
                (0, true),
                (1, false),
                (1, true),
                (2, false),
                (2, true)
            ])
        );
        let mut expire = ClusterHelperRequest::new(
            identity.clone(),
            ClusterHelperCommand::ExpireStep { max_work: 1 },
        );
        assert_eq!(
            cluster.helper_domains(&expire).unwrap(),
            BTreeSet::from([(0, true), (1, false)])
        );
        expire.command = ClusterHelperCommand::ExpireStep { max_work: 4 };
        assert_eq!(
            cluster.helper_domains(&expire).unwrap(),
            BTreeSet::from([(0, false), (0, true), (1, false), (2, false)])
        );
        let read = ClusterHelperRequest::new(
            identity,
            ClusterHelperCommand::EntryViews {
                keys: vec![b"key".to_vec()],
                max_bytes: 1_024,
            },
        );
        assert_eq!(
            cluster.helper_domains(&read).unwrap_err(),
            ServiceClusterError::Model(MapServiceError::InvalidArgument)
        );
        assert_eq!(
            cluster.recover_cluster_helper(&read).unwrap(),
            ClusterHelperRecovery::Unknown
        );
    }
}
