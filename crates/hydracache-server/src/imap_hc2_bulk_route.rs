//! One fenced, bounded subrequest per HC/2 bulk owner.

use super::*;
use crate::imap_value_decision::WriteOp;
use crate::imap_value_plane::MAX_VALUE_BYTES;
use hydracache_client_protocol::ClientResponse;
use hydracache_cluster_transport_axum::{
    ClusterOpaqueMessage, MAX_CLUSTER_MESSAGE_HTTP_BODY_BYTES,
};
use std::collections::BTreeMap;

pub(super) const MAX_OWNER_BATCHES: usize = 8;
const MAX_ITEMS: usize = 256;
const MAX_RESULT_VALUE_BYTES: usize = 4 * 1024 * 1024;
const BATCH_SCHEMA: u16 = 1;
const BATCH_BUDGET: Duration = Duration::from_secs(10);

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OwnerBatchItem {
    pub index: usize,
    pub partition: PartitionId,
    pub envelope: ClientRequestEnvelope,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OwnerBatchCall {
    batch_schema: u16,
    pub(crate) epoch: ClusterEpoch,
    source: PartitionMember,
    pub(crate) target: PartitionMember,
    principal: String,
    tenant: String,
    expires_at_ms: u64,
    pub(crate) items: Vec<OwnerBatchItem>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OwnerBatchReply {
    fingerprint: [u8; 32],
    outcomes: Result<Vec<BatchExecution>, ClientErrorEnvelope>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub(crate) enum BatchState {
    Confirmed,
    NotApplied,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct BatchExecution {
    pub outcome: ClientBackendOutcome,
    pub applied: Option<AppliedVersion>,
    pub state: BatchState,
}

impl BatchExecution {
    pub fn not_applied(error: ClientErrorEnvelope) -> Self {
        Self {
            outcome: Err(error),
            applied: None,
            state: BatchState::NotApplied,
        }
    }

    fn unknown() -> Self {
        Self {
            outcome: Err(outcome_unknown()),
            applied: None,
            state: BatchState::Unknown,
        }
    }
}

impl<S: RaftLogStore> RoutedValueBackend<S> {
    pub(crate) fn group_batch(
        &self,
        principal: &str,
        tenant: &str,
        expires_at_ms: u64,
        envelopes: Vec<ClientRequestEnvelope>,
    ) -> Result<(Vec<OwnerBatchCall>, Vec<OwnerBatchItem>), ClientErrorEnvelope> {
        self.authorize(principal, tenant)?;
        let snapshot = self
            .owner
            .raft
            .partition_snapshot()
            .ok_or_else(unavailable)?;
        if !self.owner.replica.roster_compatible(&snapshot) {
            return Err(unavailable());
        }
        let mut placements = BTreeMap::new();
        let mut groups = BTreeMap::new();
        let mut unassigned = Vec::new();
        for (index, envelope) in envelopes.into_iter().enumerate() {
            let partition = self.batch_partition(tenant, &envelope)?;
            let owner = placements.entry(partition).or_insert_with(|| {
                self.owner
                    .replica
                    .placement(&snapshot, partition)
                    .map(|placement| placement.primary)
            });
            let item = OwnerBatchItem {
                index,
                partition,
                envelope,
            };
            if let Some(target) = owner {
                let group = groups
                    .entry(target.node.clone())
                    .or_insert_with(|| OwnerBatchCall {
                        batch_schema: BATCH_SCHEMA,
                        epoch: snapshot.epoch,
                        source: self.local_member().clone(),
                        target: target.clone(),
                        principal: principal.to_owned(),
                        tenant: tenant.to_owned(),
                        expires_at_ms,
                        items: Vec::new(),
                    });
                group.items.push(item);
            } else {
                unassigned.push(item);
            }
        }
        Ok((groups.into_values().collect(), unassigned))
    }

    pub(crate) async fn call_batch(&self, call: &OwnerBatchCall) -> Vec<BatchExecution> {
        let refused =
            |error: ClientErrorEnvelope| vec![BatchExecution::not_applied(error); call.items.len()];
        let unknown = || vec![BatchExecution::unknown(); call.items.len()];
        let Ok(body) = serde_json::to_vec(call) else {
            return refused(batch_too_large());
        };
        let wire = ClusterOpaqueMessage::new(
            call.source.node.to_string(),
            call.target.node.to_string(),
            call.epoch.value(),
            body,
        );
        if !wire
            .encode_json()
            .is_ok_and(|body| body.len() <= MAX_CLUSTER_MESSAGE_HTTP_BODY_BYTES)
        {
            return refused(batch_too_large());
        }
        let Some(deadline) = batch_deadline(call.expires_at_ms) else {
            return refused(deadline_error());
        };
        self.observe_route(call.source != call.target);
        if call.source == call.target {
            return match self
                .run_batch(call.source.node.as_str(), call.epoch.value(), call)
                .await
            {
                Ok(outcomes) => outcomes,
                Err(error) => refused(error),
            };
        }
        let Some(transport) = &self.owner.value_transport else {
            return refused(unavailable());
        };
        let Ok(Ok(reply)) = timeout_at(deadline, transport.call_owner_batch(call)).await else {
            return unknown();
        };
        if reply.fingerprint != hash(call) {
            return unknown();
        }
        match reply.outcomes {
            Ok(outcomes) if outcomes.len() == call.items.len() => outcomes,
            Ok(_) => unknown(),
            Err(error) => refused(error),
        }
    }

    pub(crate) async fn receive_owner_batch(
        &self,
        peer: &str,
        outer_epoch: u64,
        call: &OwnerBatchCall,
    ) -> OwnerBatchReply {
        let outcomes = if call.source == call.target {
            Err(unavailable())
        } else {
            self.run_batch(peer, outer_epoch, call).await
        };
        OwnerBatchReply {
            fingerprint: hash(call),
            outcomes,
        }
    }

    async fn run_batch(
        &self,
        peer: &str,
        outer_epoch: u64,
        call: &OwnerBatchCall,
    ) -> Result<Vec<BatchExecution>, ClientErrorEnvelope> {
        self.authorize(&call.principal, &call.tenant)?;
        let _admission = self.admit_owner()?;
        if call.batch_schema != BATCH_SCHEMA
            || call.items.is_empty()
            || call.items.len() > MAX_ITEMS
        {
            return Err(batch_too_large());
        }
        let _permit = self.owner_batches.try_acquire().map_err(|_| {
            if let Some(operations) = self.operations() {
                operations.backpressure();
            }
            ClientErrorEnvelope::new(
                ClientErrorCode::RateLimited,
                true,
                "owner batch admission exhausted",
            )
        })?;
        let snapshot = self
            .owner
            .raft
            .partition_snapshot()
            .ok_or_else(unavailable)?;
        if outer_epoch != call.epoch.value()
            || snapshot.epoch != call.epoch
            || !self.owner.replica.roster_compatible(&snapshot)
            || call.source.node.as_str() != peer
            || &call.target != self.local_member()
            || !snapshot.members.iter().any(|member| {
                member.node_id == call.source.node && member.generation == call.source.generation
            })
        {
            self.observe_stale_route();
            return Err(unavailable());
        }
        let mut previous = None;
        for item in &call.items {
            if item.index >= MAX_ITEMS
                || previous.is_some_and(|index| index >= item.index)
                || self.batch_partition(&call.tenant, &item.envelope)? != item.partition
                || !self
                    .owner
                    .replica
                    .placement(&snapshot, item.partition)
                    .is_some_and(|placement| placement.primary == call.target)
            {
                return Err(unavailable());
            }
            previous = Some(item.index);
        }
        let deadline = batch_deadline(call.expires_at_ms).ok_or_else(deadline_error)?;
        let read_only = call
            .items
            .iter()
            .all(|item| matches!(item.envelope.request, ClientRequest::Get { .. }));
        let mut reads = BTreeMap::new();
        let mut remaining = MAX_RESULT_VALUE_BYTES;
        let mut outcomes = Vec::with_capacity(call.items.len());
        for item in &call.items {
            if Instant::now() >= deadline {
                outcomes.push(BatchExecution::not_applied(deadline_error()));
                continue;
            }
            let request = &item.envelope.request;
            let read = match request {
                ClientRequest::Get { key, .. } => Some(key.segments()),
                _ => None,
            };
            let cached = read.and_then(|key| reads.get(key)).cloned();
            let mut result = if let Some(cached) = cached {
                cached
            } else {
                let execute =
                    || self.execute_proven(&call.tenant, &item.envelope, deadline, call.epoch);
                let mut result = execute().await;
                if read.is_some()
                    && Instant::now() < deadline
                    && result.as_ref().is_err_and(|error| error.retryable)
                {
                    result = execute().await;
                }
                match result {
                    Ok((outcome, applied)) => BatchExecution {
                        outcome: Ok(outcome),
                        applied,
                        state: BatchState::Confirmed,
                    },
                    Err(error)
                        if read.is_none()
                            && error.code == ClientErrorCode::BackendUnavailable
                            && !error.retryable =>
                    {
                        BatchExecution::unknown()
                    }
                    Err(error) => BatchExecution::not_applied(error),
                }
            };
            if let Ok(ClientResponse::Value { value }) = &result.outcome {
                let bytes = value.as_ref().map_or(0, Vec::len);
                if bytes <= remaining {
                    remaining -= bytes;
                    if read_only {
                        if let Some(key) = read {
                            reads.entry(key).or_insert_with(|| result.clone());
                        }
                    }
                } else {
                    result.outcome = Err(batch_too_large());
                }
            }
            outcomes.push(result);
        }
        Ok(outcomes)
    }

    fn batch_partition(
        &self,
        tenant: &str,
        envelope: &ClientRequestEnvelope,
    ) -> Result<PartitionId, ClientErrorEnvelope> {
        envelope.validate_protocol()?;
        let (namespace, key) = self.operation_key(&envelope.request).ok_or_else(unsupported)?;
        if !matches!(envelope.request, ClientRequest::Get { .. })
            && !WriteOp::request(&envelope.request)
                .is_some_and(|(_, _, op)| op.fits(self.max_value_bytes.min(MAX_VALUE_BYTES)))
        {
            return Err(batch_too_large());
        }
        if key
            .segments()
            .first()
            .is_none_or(|key| key.len() > 60 * 1024)
            || envelope
                .idempotency_key
                .as_ref()
                .is_some_and(|id| id.is_empty() || id.len() > 128)
        {
            return Err(batch_too_large());
        }
        self.owner
            .replica
            .route_partition(tenant, namespace, key)
            .ok_or_else(unsupported)
    }
}

fn batch_deadline(expires_at_ms: u64) -> Option<Instant> {
    let remaining = expires_at_ms.checked_sub(unix_time_ms())?;
    (remaining > 0).then(|| Instant::now() + Duration::from_millis(remaining).min(BATCH_BUDGET))
}

fn batch_too_large() -> ClientErrorEnvelope {
    ClientErrorEnvelope::new(
        ClientErrorCode::TooLarge,
        false,
        "owner batch exceeds its byte or item bound",
    )
}
