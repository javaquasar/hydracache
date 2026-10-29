//! Runtime-only typed helper coordination. Cross-partition writes retain ordered per-key receipts;
//! they do not form a transaction and backups install decisions rather than reevaluate conditions.
use super::*;
use crate::value_plane_cluster_admission_075::ClusterLane;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClusterHelperCommand {
    VersionBulk {
        operation: VersionBulkOperation,
        entries: Vec<VersionBulkEntry>,
    },
    QueryOutcomes {
        identities: Vec<MutationIdentity>,
        max_bytes: usize,
    },
    RetireOutcomes {
        client: String,
        through: u64,
    },
    ExpireStep {
        max_work: usize,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterHelperRequest {
    pub command: ClusterHelperCommand,
    pub identity: MutationIdentity,
    pub now: u64,
    pub deadline: u64,
}
impl ClusterHelperRequest {
    fn credit_key(&self) -> [u8; 32] {
        let mut hash = Sha256::new();
        hash.update(self.request_digest());
        hash.update((self.identity.client.len() as u64).to_le_bytes());
        hash.update(self.identity.client.as_bytes());
        hash.update(self.identity.sequence.to_le_bytes());
        hash.finalize().into()
    }
    pub fn new(identity: MutationIdentity, command: ClusterHelperCommand) -> Self {
        Self {
            command,
            identity,
            now: 0,
            deadline: u64::MAX,
        }
    }
    pub fn lane(&self) -> ClusterLane {
        match self.command {
            ClusterHelperCommand::RetireOutcomes { .. }
            | ClusterHelperCommand::ExpireStep { .. } => ClusterLane::Repair,
            _ => ClusterLane::Data,
        }
    }
    pub fn keys(&self) -> Vec<Vec<u8>> {
        match &self.command {
            ClusterHelperCommand::VersionBulk { entries, .. } => {
                entries.iter().map(|entry| entry.key.clone()).collect()
            }
            _ => vec![Vec::new()],
        }
    }
    pub fn request_digest(&self) -> [u8; 32] {
        struct Hash(Sha256);
        impl fmt::Write for Hash {
            fn write_str(&mut self, value: &str) -> fmt::Result {
                self.0.update(value.as_bytes());
                Ok(())
            }
        }
        let mut hash = Hash(Sha256::new());
        fmt::Write::write_fmt(&mut hash, format_args!("cluster_helper:{:?}", self.command))
            .expect("hash writer");
        hash.0.finalize().into()
    }
    fn payload(&self, maximum: usize) -> Result<Vec<u8>, ServiceClusterError> {
        bounded_payload(&self.command, maximum)
    }
}
fn bounded_payload(
    value: &impl fmt::Debug,
    maximum: usize,
) -> Result<Vec<u8>, ServiceClusterError> {
    struct Writer {
        bytes: Vec<u8>,
        maximum: usize,
    }
    impl fmt::Write for Writer {
        fn write_str(&mut self, value: &str) -> fmt::Result {
            if self
                .bytes
                .len()
                .checked_add(value.len())
                .is_none_or(|n| n > self.maximum)
            {
                return Err(fmt::Error);
            }
            self.bytes.extend_from_slice(value.as_bytes());
            Ok(())
        }
    }
    let mut writer = Writer {
        bytes: Vec::new(),
        maximum,
    };
    fmt::Write::write_fmt(&mut writer, format_args!("{value:?}"))
        .map_err(|_| MapServiceError::ResourceBudget("helper_transport"))?;
    Ok(writer.bytes)
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterHelperPreflight {
    pub keys: Vec<Vec<u8>>,
    pub partitions: Vec<u32>,
    pub request_bytes: usize,
    pub growth: usize,
    pub replica_bytes: usize,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClusterHelperResult {
    Versions(Box<ServiceResult>),
    Outcomes(Vec<OperationOutcomeQuery>),
    Retirement(RetirementReceipt),
    Expiry(ExpiryProgress),
    NotApplied,
    OutcomeUnknown,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterHelperExecution {
    pub result: ClusterHelperResult,
    pub replayed: bool,
    pub backup_proved: bool,
    pub routes: Vec<ServicePartitionAssignment>,
    pub failure: Option<ServiceClusterError>,
}

struct Group {
    partition: u32,
    journal: bool,
    command: LocalHelperCommand,
    identity: MutationIdentity,
    indices: Vec<usize>,
    retirement_clients: Vec<String>,
}
fn retirement_digest(clients: &[String], generation: u64, through: u64) -> [u8; 32] {
    Sha256::digest(format!("retire-domains:{clients:?}:{generation}:{through}").as_bytes()).into()
}
fn uncertain(command: &ClusterHelperCommand, known: &ServiceResult) -> ClusterHelperResult {
    if let ClusterHelperCommand::VersionBulk { entries, .. } = command {
        let mut result = ServiceResult::new("partial");
        result.number = entries.len() as u64;
        result.items = entries
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                known
                    .items
                    .iter()
                    .find(|item| item.index == index)
                    .cloned()
                    .unwrap_or_else(|| ServiceItem {
                        index,
                        key: entry.key.clone(),
                        kind: "outcome_unknown".into(),
                        value: None,
                        version: None,
                        remaining_ttl: None,
                    })
            })
            .collect();
        ClusterHelperResult::Versions(Box::new(result))
    } else {
        ClusterHelperResult::OutcomeUnknown
    }
}
fn child_identity(identity: &MutationIdentity, partition: u32, journal: bool) -> MutationIdentity {
    let digest = Sha256::digest(
        format!("cluster-helper:{}:{partition}:{journal}", identity.client).as_bytes(),
    );
    let mut client = String::from("helper:");
    for byte in digest {
        use fmt::Write;
        write!(&mut client, "{byte:02x}").expect("string writer");
    }
    MutationIdentity::new(client, identity.sequence)
}
fn decode(
    command: &ClusterHelperCommand,
    result: ServiceResult,
) -> Result<ClusterHelperResult, ServiceClusterError> {
    Ok(match result.kind.as_str() {
        "outcome_unknown" => ClusterHelperResult::OutcomeUnknown,
        "not_applied" => ClusterHelperResult::NotApplied,
        _ => match command {
            ClusterHelperCommand::VersionBulk { .. } => {
                ClusterHelperResult::Versions(Box::new(result))
            }
            ClusterHelperCommand::RetireOutcomes { through, .. } => {
                ClusterHelperResult::Retirement(RetirementReceipt {
                    removed: result
                        .number
                        .try_into()
                        .map_err(|_| ServiceClusterError::InvalidBound)?,
                    through: *through,
                })
            }
            ClusterHelperCommand::ExpireStep { .. } => {
                ClusterHelperResult::Expiry(ExpiryProgress {
                    examined: result
                        .number
                        .try_into()
                        .map_err(|_| ServiceClusterError::InvalidBound)?,
                    expired: result
                        .version
                        .unwrap_or(0)
                        .try_into()
                        .map_err(|_| ServiceClusterError::InvalidBound)?,
                    pending: result.flag,
                })
            }
            _ => return Err(ServiceClusterError::InvalidBound),
        },
    })
}

impl ServiceClusterHarness {
    fn helper_transfer_credits(
        &self,
        request: &ClusterHelperRequest,
        root_maximum: usize,
    ) -> Result<BTreeMap<u32, [usize; 2]>, ServiceClusterError> {
        let mut credits = BTreeMap::<u32, [usize; 2]>::new();
        for group in self.helper_groups(request) {
            let service = self.service(
                &self.assignments[group.partition as usize].route.owner,
                group.partition,
                group.journal,
            )?;
            let credit = if let ClusterHelperCommand::RetireOutcomes { through, .. } =
                &request.command
            {
                let pending = service.preflight_retire_client_domains(
                    &group.retirement_clients,
                    self.generation,
                    *through,
                    &group.identity,
                    retirement_digest(&group.retirement_clients, self.generation, *through),
                    [0, 0],
                )?;
                if pending == 0 {
                    0
                } else {
                    group
                        .retirement_clients
                        .iter()
                        .try_fold(4_096usize, |sum, client| {
                            sum.checked_add(service.retirement_delta_credit(client, *through)?)
                                .ok_or(MapServiceError::Capacity)
                        })?
                }
            } else {
                service.local_helper_delta_credit(&group.command, &group.identity, request.now)?
            };
            let held = &mut credits.entry(group.partition).or_default()[usize::from(group.journal)];
            *held = held
                .checked_add(credit)
                .ok_or(ServiceClusterError::InvalidBound)?;
        }
        let root = self.coordinator_for(&request.identity);
        let credit = &mut credits.entry(root).or_default()[1];
        *credit = credit
            .checked_add(
                root_maximum
                    .checked_mul(16)
                    .and_then(|n| n.checked_add(2_048))
                    .ok_or(ServiceClusterError::InvalidBound)?,
            )
            .ok_or(ServiceClusterError::InvalidBound)?;
        Ok(credits)
    }
    pub fn validate_cluster_helper(
        &self,
        request: &ClusterHelperRequest,
    ) -> Result<usize, ServiceClusterError> {
        self.ensure_namespace()?;
        if identity_blank(&request.identity.client)
            || request.identity.client.len() > 128
            || request.identity.sequence == 0
        {
            return Err(MapServiceError::InvalidArgument.into());
        }
        let mut bytes = 256usize;
        match &request.command {
            ClusterHelperCommand::VersionBulk { entries, operation } => {
                if entries.is_empty()
                    || entries.len()
                        > self
                            .bounds
                            .max_bulk_items
                            .min(self.service_limits.max_batch)
                {
                    return Err(MapServiceError::InvalidArgument.into());
                }
                let mut seen = BTreeSet::new();
                for entry in entries {
                    if !seen.insert(&entry.key)
                        || entry.key.len() > self.collection.max_key_bytes
                        || entry
                            .value
                            .as_ref()
                            .is_some_and(|v| v.len() > self.collection.max_value_bytes)
                        || (*operation == VersionBulkOperation::Replace) != entry.value.is_some()
                        || *operation == VersionBulkOperation::SetTtl
                            && entry.ttl == ServiceTtl::Preserve
                        || matches!(entry.ttl, ServiceTtl::ExpireAfter(0))
                    {
                        return Err(MapServiceError::InvalidArgument.into());
                    }
                    bytes = bytes
                        .checked_add(64)
                        .and_then(|n| n.checked_add(entry.key.len()))
                        .and_then(|n| n.checked_add(entry.value.as_ref().map_or(0, Vec::len)))
                        .ok_or(ServiceClusterError::InvalidBound)?;
                }
            }
            ClusterHelperCommand::QueryOutcomes {
                identities,
                max_bytes,
            } => {
                if identities.is_empty()
                    || identities.len() > self.service_limits.max_batch
                    || *max_bytes == 0
                    || *max_bytes > self.service_limits.query_bytes
                {
                    return Err(MapServiceError::InvalidArgument.into());
                }
                for id in identities {
                    if identity_blank(&id.client) || id.client.len() > 128 || id.sequence == 0 {
                        return Err(MapServiceError::InvalidArgument.into());
                    }
                    bytes = bytes
                        .checked_add(48 + id.client.len())
                        .ok_or(ServiceClusterError::InvalidBound)?;
                }
            }
            ClusterHelperCommand::RetireOutcomes { client, through } => {
                if identity_blank(client) || client.len() > 128 || *through == 0 {
                    return Err(MapServiceError::InvalidArgument.into());
                }
                if client == &request.identity.client && request.identity.sequence <= *through {
                    return Err(MapServiceError::PendingOperation.into());
                }
                bytes += client.len();
            }
            ClusterHelperCommand::ExpireStep { max_work } => {
                if *max_work == 0 || *max_work > self.collection.max_scan_items {
                    return Err(MapServiceError::InvalidArgument.into());
                }
            }
        }
        if bytes > self.bounds.max_frame_bytes.min(8 * 1_024 * 1_024) {
            return Err(MapServiceError::ResourceBudget("helper_request").into());
        }
        Ok(bytes)
    }
    fn helper_groups(&self, request: &ClusterHelperRequest) -> Vec<Group> {
        let mut groups = Vec::new();
        match &request.command {
            ClusterHelperCommand::VersionBulk { operation, entries } => {
                let mut partitions = BTreeMap::<u32, (Vec<VersionBulkEntry>, Vec<usize>)>::new();
                for (index, entry) in entries.iter().enumerate() {
                    let group = partitions
                        .entry(self.partition_for(&entry.key))
                        .or_default();
                    group.0.push(entry.clone());
                    group.1.push(index);
                }
                for (partition, (entries, indices)) in partitions {
                    groups.push(Group {
                        partition,
                        journal: false,
                        command: LocalHelperCommand::VersionBulk {
                            operation: *operation,
                            entries,
                        },
                        identity: child_identity(&request.identity, partition, false),
                        indices,
                        retirement_clients: Vec::new(),
                    });
                }
            }
            ClusterHelperCommand::RetireOutcomes { client, through } => {
                for partition in 0..self.bounds.partitions {
                    for journal in [false, true] {
                        let target = MutationIdentity::new(client, *through);
                        let retirement_clients = vec![
                            if journal {
                                client.clone()
                            } else {
                                data_identity(&target, partition).client
                            },
                            child_identity(&target, partition, journal).client,
                        ];
                        groups.push(Group {
                            partition,
                            journal,
                            command: LocalHelperCommand::RetireOutcomes {
                                client: retirement_clients[0].clone(),
                                generation: self.generation,
                                through: *through,
                            },
                            identity: child_identity(&request.identity, partition, journal),
                            indices: Vec::new(),
                            retirement_clients,
                        });
                    }
                }
            }
            ClusterHelperCommand::ExpireStep { max_work } => {
                let partitions = self.bounds.partitions as usize;
                let start = (request.identity.sequence % partitions as u64) as usize;
                for offset in 0..partitions {
                    let quota = max_work / partitions + usize::from(offset < max_work % partitions);
                    if quota != 0 {
                        let partition = ((start + offset) % partitions) as u32;
                        groups.push(Group {
                            partition,
                            journal: false,
                            command: LocalHelperCommand::ExpireStep { max_work: quota },
                            identity: child_identity(&request.identity, partition, false),
                            indices: Vec::new(),
                            retirement_clients: Vec::new(),
                        });
                    }
                }
            }
            _ => {}
        }
        groups
    }

    pub fn cluster_helper_preflight(
        &self,
        request: &ClusterHelperRequest,
    ) -> Result<ClusterHelperPreflight, ServiceClusterError> {
        let request_bytes = self.validate_cluster_helper(request)?;
        let mut partitions = BTreeSet::new();
        let mut growth = 0usize;
        let mut response = 64usize;
        let mut replica_bytes = request_bytes.saturating_mul(8).saturating_add(2_048);
        if let ClusterHelperCommand::QueryOutcomes {
            identities,
            max_bytes,
        } = &request.command
        {
            let mut total = 0usize;
            for identity in identities {
                let p = self.coordinator_for(identity);
                partitions.insert(p);
                let owner = &self.assignments[p as usize].route.owner;
                total = total
                    .checked_add(
                        self.service(owner, p, true)?
                            .outcome_query_response_bytes(std::slice::from_ref(identity))?,
                    )
                    .ok_or(ServiceClusterError::InvalidBound)?;
            }
            if total > *max_bytes {
                return Err(MapServiceError::ResourceBudget("query_response").into());
            }
            return Ok(ClusterHelperPreflight {
                keys: request.keys(),
                partitions: partitions.into_iter().collect(),
                request_bytes,
                growth: total,
                replica_bytes: replica_bytes.saturating_add(total),
            });
        }
        let root = self.coordinator_for(&request.identity);
        partitions.insert(root);
        let journal = self.service(&self.assignments[root as usize].route.owner, root, true)?;
        let completed = matches!(
            journal.recover_external_operation(&request.identity, request.request_digest())?,
            ServiceRecovery::Completed(_)
        );
        let root_maximum = match &request.command {
            ClusterHelperCommand::VersionBulk { entries, .. } => {
                64 + entries
                    .iter()
                    .map(|entry| 64 + entry.key.len())
                    .sum::<usize>()
            }
            _ => 64,
        };
        let root_credit = journal.external_reservation_credit(
            &request.identity,
            request.request_digest(),
            root_maximum,
        )?;
        for group in self.helper_groups(request) {
            partitions.insert(group.partition);
            let service = self.service(
                &self.assignments[group.partition as usize].route.owner,
                group.partition,
                group.journal,
            )?;
            if !completed {
                let (result, change) = if let ClusterHelperCommand::RetireOutcomes {
                    through, ..
                } = &request.command
                {
                    (
                        64,
                        service.preflight_retire_client_domains(
                            &group.retirement_clients,
                            self.generation,
                            *through,
                            &group.identity,
                            retirement_digest(&group.retirement_clients, self.generation, *through),
                            if group.journal && group.partition == root {
                                root_credit
                            } else {
                                [0, 0]
                            },
                        )?,
                    )
                } else {
                    let (_, result, change) = service.local_helper_preflight(
                        &group.command,
                        &group.identity,
                        request.now,
                    )?;
                    (result, change)
                };
                response = response
                    .checked_add(result.saturating_sub(64))
                    .ok_or(ServiceClusterError::InvalidBound)?;
                growth = growth
                    .checked_add(change.saturating_mul(2))
                    .ok_or(ServiceClusterError::InvalidBound)?;
            }
            replica_bytes = replica_bytes
                .checked_add(self.partition_replica_bound(group.partition)?)
                .and_then(|n| n.checked_add(response.saturating_mul(16)))
                .ok_or(ServiceClusterError::InvalidBound)?;
        }
        if response > self.max_response_bytes.min(8 * 1_024 * 1_024) {
            return Err(MapServiceError::ResourceBudget("helper_response").into());
        }
        growth = growth.saturating_add(
            response
                .saturating_add(request.identity.client.len())
                .saturating_add(48)
                .saturating_mul(2),
        );
        replica_bytes = replica_bytes
            .checked_add(self.partition_replica_bound(root)?)
            .and_then(|n| n.checked_add(response.saturating_mul(16)))
            .ok_or(ServiceClusterError::InvalidBound)?;
        let transfer_growth = if completed {
            0
        } else {
            self.preflight_transfer_credits(
                request.credit_key(),
                &self.helper_transfer_credits(request, root_maximum)?,
            )?
        };
        Ok(ClusterHelperPreflight {
            keys: request.keys(),
            partitions: partitions.into_iter().collect(),
            request_bytes,
            growth: growth
                .saturating_add(replica_bytes)
                .saturating_add(transfer_growth),
            replica_bytes,
        })
    }

    pub fn execute_cluster_helper_guarded(
        &mut self,
        entry: &str,
        request: ClusterHelperRequest,
        call: ServiceClusterCall,
        guard: Arc<dyn ServiceMessageGuard>,
    ) -> Result<ClusterHelperExecution, ServiceClusterError> {
        let preflight = self.cluster_helper_preflight(&request)?;
        let previous = self.runtime.invocation.replace(runtime_hooks::Invocation {
            digest: request.request_digest(),
            identity: Some(request.identity.clone()),
            now: request.now,
            deadline: request.deadline,
        });
        let mut result = self.with_message_guard(guard, |cluster| {
            cluster.execute_cluster_helper(entry, &request, &call, &preflight)
        });
        if let Ok(execution) = &mut result {
            match bounded_payload(&execution.result, self.bounds.max_frame_bytes) {
                Err(error) => result = Err(error),
                Ok(payload) => {
                    let partition = execution.routes.first().map_or(0, |route| route.partition);
                    if !matches!(self.transmit(partition, "helper_response", payload, call.faults.response), Ok(delivered) if delivered > 0)
                    {
                        if matches!(request.command, ClusterHelperCommand::QueryOutcomes { .. }) {
                            result = Err(ServiceClusterError::TransportTimeout);
                        } else {
                            execution.result = ClusterHelperResult::OutcomeUnknown;
                            execution.failure = Some(ServiceClusterError::TransportTimeout);
                        }
                    }
                }
            }
        }
        self.runtime.invocation = previous;
        self.capture_online_changes();
        self.signal_subscriptions();
        result
    }
    fn execute_cluster_helper(
        &mut self,
        entry: &str,
        request: &ClusterHelperRequest,
        call: &ServiceClusterCall,
        preflight: &ClusterHelperPreflight,
    ) -> Result<ClusterHelperExecution, ServiceClusterError> {
        self.require_live(entry)?;
        if self.node_is_draining(entry) {
            return Err(ServiceClusterError::NodeUnavailable);
        }
        let payload = request.payload(self.bounds.max_frame_bytes)?;
        if let ClusterHelperCommand::QueryOutcomes { identities, .. } = &request.command {
            if request.now >= request.deadline {
                return Ok(ClusterHelperExecution {
                    result: ClusterHelperResult::NotApplied,
                    replayed: false,
                    backup_proved: false,
                    routes: Vec::new(),
                    failure: None,
                });
            }
            let mut outcomes = Vec::with_capacity(identities.len());
            let mut routes = Vec::new();
            for id in identities {
                let p = self.coordinator_for(id);
                let route = self.route(entry, p, call)?;
                self.authorize_message(
                    ServiceMessageKind::Request,
                    entry,
                    &route.owner,
                    p,
                    None,
                    &payload,
                )?;
                if self.transmit(p, "helper_query", payload.clone(), call.faults.data.request)? == 0
                {
                    return Err(ServiceClusterError::TransportTimeout);
                }
                outcomes.push(
                    self.service(&route.owner, p, true)?
                        .query_operation_outcome(id)?,
                );
                routes.push(route);
            }
            return Ok(ClusterHelperExecution {
                result: ClusterHelperResult::Outcomes(outcomes),
                replayed: false,
                backup_proved: preflight.partitions.iter().all(|p| self.backup_proved(*p)),
                routes,
                failure: None,
            });
        }
        let root = self.coordinator_for(&request.identity);
        let route = self.route(entry, root, call)?;
        self.authorize_message(
            ServiceMessageKind::Request,
            entry,
            &route.owner,
            root,
            None,
            &payload,
        )?;
        let journal = self.service(&route.owner, root, true)?;
        let recovered =
            journal.recover_external_operation(&request.identity, request.request_digest())?;
        let mut routes = vec![route.clone()];
        if let ServiceRecovery::Completed(result) = recovered {
            let proved = self
                .replicate(root, true, call.faults.coordinator)
                .unwrap_or(false)
                && preflight.partitions.iter().all(|p| self.backup_proved(*p));
            return Ok(ClusterHelperExecution {
                result: if proved {
                    decode(&request.command, *result)?
                } else {
                    uncertain(&request.command, &result)
                },
                replayed: true,
                backup_proved: proved,
                routes,
                failure: (!proved).then_some(ServiceClusterError::ReplicaNotProved),
            });
        }
        let pending = matches!(recovered, ServiceRecovery::Pending);
        if request.now >= request.deadline {
            return Ok(ClusterHelperExecution {
                result: if pending {
                    ClusterHelperResult::OutcomeUnknown
                } else {
                    ClusterHelperResult::NotApplied
                },
                replayed: pending,
                backup_proved: false,
                routes,
                failure: None,
            });
        }
        let root_maximum = match &request.command {
            ClusterHelperCommand::VersionBulk { entries, .. } => {
                64 + entries
                    .iter()
                    .map(|entry| 64 + entry.key.len())
                    .sum::<usize>()
            }
            _ => 64,
        };
        let credits = self.helper_transfer_credits(request, root_maximum)?;
        self.reserve_transfer_credits(request.credit_key(), &credits)?;
        if self.transmit(
            root,
            "helper_root_request",
            payload.clone(),
            call.faults.coordinator.request,
        )? == 0
        {
            return Ok(ClusterHelperExecution {
                result: ClusterHelperResult::NotApplied,
                replayed: false,
                backup_proved: false,
                routes,
                failure: None,
            });
        }
        let maximum = match &request.command {
            ClusterHelperCommand::VersionBulk { entries, .. } => {
                64 + entries
                    .iter()
                    .map(|entry| 64 + entry.key.len())
                    .sum::<usize>()
            }
            _ => 64,
        };
        let mut combined = ServiceResult::new(match request.command {
            ClusterHelperCommand::VersionBulk { .. } => "bulk",
            ClusterHelperCommand::RetireOutcomes { .. } => "helper_retired",
            _ => "helper_expiry",
        });
        journal.admit_external_operation(
            request.identity.clone(),
            request.request_digest(),
            maximum,
        )?;
        self.assignments[root as usize].required_journal = journal.journal_replica_stamp()?;
        if !self.replicate(root, true, call.faults.coordinator)? {
            return Ok(ClusterHelperExecution {
                result: uncertain(&request.command, &combined),
                replayed: pending,
                backup_proved: false,
                routes,
                failure: Some(ServiceClusterError::ReplicaNotProved),
            });
        }
        combined.version =
            matches!(request.command, ClusterHelperCommand::ExpireStep { .. }).then_some(0);
        for group in self.helper_groups(request) {
            let route = match self.route(entry, group.partition, call) {
                Ok(route) => route,
                Err(error) => {
                    return Ok(ClusterHelperExecution {
                        result: uncertain(&request.command, &combined),
                        replayed: pending,
                        backup_proved: false,
                        routes,
                        failure: Some(error),
                    })
                }
            };
            routes.push(route.clone());
            let faults = call
                .faults
                .partitions
                .get(&group.partition)
                .copied()
                .unwrap_or(call.faults.data);
            if let Err(error) = self.authorize_message(
                ServiceMessageKind::Request,
                entry,
                &route.owner,
                group.partition,
                None,
                &payload,
            ) {
                return Ok(ClusterHelperExecution {
                    result: uncertain(&request.command, &combined),
                    replayed: pending,
                    backup_proved: false,
                    routes,
                    failure: Some(error),
                });
            }
            if self.transmit(
                group.partition,
                "helper_owner_request",
                payload.clone(),
                faults.request,
            )? == 0
            {
                return Ok(ClusterHelperExecution {
                    result: uncertain(&request.command, &combined),
                    replayed: pending,
                    backup_proved: false,
                    routes,
                    failure: Some(ServiceClusterError::TransportTimeout),
                });
            }
            let service = self.service(&route.owner, group.partition, group.journal)?;
            let result =
                if let ClusterHelperCommand::RetireOutcomes { through, .. } = &request.command {
                    LocalHelperResult::Retirement(service.retire_client_domains(
                        &group.retirement_clients,
                        self.generation,
                        *through,
                        group.identity,
                        retirement_digest(&group.retirement_clients, self.generation, *through),
                    )?)
                } else {
                    service.execute_local_helper(
                        &group.command,
                        group.identity,
                        request.now,
                        request.deadline,
                    )?
                };
            let stamp = if group.journal {
                service.journal_replica_stamp()?
            } else {
                service.replica_stamp()?
            };
            if group.journal {
                self.assignments[group.partition as usize].required_journal = stamp;
            } else {
                self.assignments[group.partition as usize].required_data = stamp;
            }
            match self.replicate(group.partition, group.journal, faults) {
                Ok(true) => {}
                result => {
                    return Ok(ClusterHelperExecution {
                        result: uncertain(&request.command, &combined),
                        replayed: pending,
                        backup_proved: false,
                        routes,
                        failure: Some(
                            result
                                .err()
                                .unwrap_or(ServiceClusterError::ReplicaNotProved),
                        ),
                    })
                }
            }
            match result {
                LocalHelperResult::Versions(mut result) => {
                    for item in &mut result.items {
                        item.index = group.indices[item.index];
                    }
                    combined.items.extend(result.items);
                }
                LocalHelperResult::Retirement(result) => combined.number += result.removed as u64,
                LocalHelperResult::Expiry(result) => {
                    combined.number += result.examined as u64;
                    combined.version = Some(combined.version.unwrap_or(0) + result.expired as u64);
                }
                _ => return Err(ServiceClusterError::InvalidBound),
            }
        }
        if matches!(request.command, ClusterHelperCommand::ExpireStep { .. }) {
            for p in 0..self.bounds.partitions {
                combined.flag |= self
                    .service(&self.assignments[p as usize].route.owner, p, false)?
                    .expiry_backlog(request.now)?;
            }
        }
        if matches!(request.command, ClusterHelperCommand::VersionBulk { .. }) {
            combined.items.sort_by_key(|item| item.index);
            combined.number = combined.items.len() as u64;
        }
        journal.complete_external_operation(
            &request.identity,
            request.request_digest(),
            &combined,
        )?;
        self.assignments[root as usize].required_journal = journal.journal_replica_stamp()?;
        let proved = self.replicate(root, true, call.faults.coordinator)?;
        Ok(ClusterHelperExecution {
            result: if proved {
                decode(&request.command, combined)?
            } else {
                uncertain(&request.command, &combined)
            },
            replayed: pending,
            backup_proved: proved,
            routes,
            failure: (!proved).then_some(ServiceClusterError::ReplicaNotProved),
        })
    }
}
