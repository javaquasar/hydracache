//! Owner execution for the mTLS HC/2 member profile.

use std::sync::Arc;
use std::time::Duration;

use hydracache::{CacheError, CacheResult, ClusterEpoch, PartitionId};
use hydracache_client_protocol::{
    ClientErrorCode, ClientErrorEnvelope, ClientRequest, ClientRequestEnvelope, Namespace,
    StructuredKey,
};
use hydracache_client_transport_axum::{
    ClientBackendFuture, ClientBackendOutcome, ClientDataBackend, VerifiedClientRequest,
};
use hydracache_cluster_raft::RaftLogStore;
use serde::{Deserialize, Serialize};
use tokio::time::{timeout_at, Instant};

use crate::config::DistributedHc2Config;
use crate::hc2::unix_time_ms;
use crate::imap_owner_read::unavailable;
use crate::imap_topology::{PartitionMember, PartitionPlacement};
use crate::imap_value_journal::hash;
use crate::imap_value_plane::{AppliedOutcome, AppliedVersion, ValueBackend};
use crate::value_plane_subscription::{EventPage, EventPoll, ListenerView};

const OWNER_CALL_VERSION: u16 = 1;
const OWNER_BUDGET: Duration = Duration::from_millis(2_000);

#[path = "imap_hc2_bulk_route.rs"]
mod bulk;
pub(crate) use bulk::{BatchExecution, BatchState, OwnerBatchCall, OwnerBatchReply};

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OwnerCall {
    version: u16,
    pub target: PartitionMember,
    pub epoch: ClusterEpoch,
    source: PartitionMember,
    partition: PartitionId,
    principal: String,
    tenant: String,
    expires_at_ms: u64,
    envelope: ClientRequestEnvelope,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OwnerReply {
    fingerprint: [u8; 32],
    pub(crate) outcome: ClientBackendOutcome,
}

pub(crate) struct RoutedValueBackend<S: RaftLogStore> {
    owner: Arc<ValueBackend<S>>,
    profile: DistributedHc2Config,
    principal: String,
    max_value_bytes: usize,
    owner_batches: tokio::sync::Semaphore,
}

impl<S: RaftLogStore> std::fmt::Debug for RoutedValueBackend<S> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RoutedValueBackend")
            .field("owner", &self.owner)
            .finish_non_exhaustive()
    }
}

impl<S: RaftLogStore> RoutedValueBackend<S> {
    pub(crate) fn new(
        owner: Arc<ValueBackend<S>>,
        profile: DistributedHc2Config,
        max_value_bytes: usize,
    ) -> Self {
        let principal = profile.principal();
        Self {
            owner,
            profile,
            principal,
            max_value_bytes,
            owner_batches: tokio::sync::Semaphore::new(bulk::MAX_OWNER_BATCHES),
        }
    }

    pub(crate) fn authorize(
        &self,
        principal: &str,
        tenant: &str,
    ) -> Result<(), ClientErrorEnvelope> {
        if self.principal != principal || self.profile.tenant != tenant {
            return Err(ClientErrorEnvelope::new(
                ClientErrorCode::Unauthorized,
                false,
                "HC/2 IMap scope is not admitted",
            ));
        }
        Ok(())
    }

    pub(crate) fn listener_view(&self) -> Option<ListenerView> {
        self.owner.listener_view()
    }

    pub(crate) fn local_member(&self) -> &PartitionMember {
        self.owner.replica.local_member()
    }

    pub(crate) fn event_scope(&self) -> [u8; 32] {
        self.owner.event_scope()
    }

    pub(crate) async fn fetch_events(&self, request: &EventPoll) -> CacheResult<EventPage> {
        if request.target == *self.local_member() {
            return self
                .owner
                .event_page(request.source.node.as_str(), request.epoch.value(), request)
                .await;
        }
        self.owner
            .value_transport
            .as_ref()
            .ok_or_else(transport_unavailable)?
            .poll_events(request)
            .await
    }

    pub(crate) async fn receive_events(
        &self,
        peer: &str,
        epoch: u64,
        request: &EventPoll,
    ) -> CacheResult<EventPage> {
        self.owner.event_page(peer, epoch, request).await
    }

    fn placement(
        &self,
        tenant: &str,
        request: &ClientRequest,
    ) -> Result<PartitionPlacement, ClientErrorEnvelope> {
        let (ns, key) = operation_key(request).ok_or_else(unsupported)?;
        let partition = self
            .owner
            .replica
            .route_partition(tenant, ns, key)
            .ok_or_else(unsupported)?;
        let view = self
            .owner
            .raft
            .partition_snapshot()
            .ok_or_else(unavailable)?;
        self.owner
            .replica
            .placement(&view, partition)
            .ok_or_else(unavailable)
    }

    async fn execute_at_owner(&self, call: &OwnerCall, deadline: Instant) -> ClientBackendOutcome {
        self.execute_proven(&call.tenant, &call.envelope, deadline, call.epoch)
            .await
            .map(|(response, _)| response)
    }

    async fn execute_proven(
        &self,
        tenant: &str,
        envelope: &ClientRequestEnvelope,
        deadline: Instant,
        epoch: ClusterEpoch,
    ) -> Result<AppliedOutcome, ClientErrorEnvelope> {
        if matches!(envelope.request, ClientRequest::Get { .. }) {
            self.owner
                .read
                .read_at_proven(tenant, &envelope.request, deadline, Some(epoch))
                .await
                .map(|(response, proof)| (response, Some(proof)))
        } else {
            self.owner
                .write_at_proven(
                    tenant,
                    envelope,
                    self.max_value_bytes,
                    deadline,
                    Some(epoch),
                )
                .await
        }
    }

    pub(crate) async fn receive(
        &self,
        peer: &str,
        outer_epoch: u64,
        call: OwnerCall,
    ) -> OwnerReply {
        let fingerprint = hash(&call);
        let outcome = self.receive_checked(peer, outer_epoch, &call).await;
        OwnerReply {
            fingerprint,
            outcome,
        }
    }

    async fn receive_checked(
        &self,
        peer: &str,
        outer_epoch: u64,
        call: &OwnerCall,
    ) -> ClientBackendOutcome {
        self.authorize(&call.principal, &call.tenant)?;
        call.envelope.validate_protocol()?;
        let snapshot = self
            .owner
            .raft
            .partition_snapshot()
            .ok_or_else(unavailable)?;
        let (namespace, key) = operation_key(&call.envelope.request).ok_or_else(unsupported)?;
        let partition = self
            .owner
            .replica
            .route_partition(&call.tenant, namespace, key)
            .ok_or_else(unsupported)?;
        if call.version != OWNER_CALL_VERSION
            || outer_epoch != call.epoch.value()
            || call.source.node.as_str() != peer
            || call.source.node == call.target.node
            || &call.target != self.owner.replica.local_member()
            || !snapshot.members.iter().any(|member| {
                member.node_id == call.source.node && member.generation == call.source.generation
            })
            || call.partition != partition
            || !self
                .owner
                .replica
                .placement(&snapshot, partition)
                .is_some_and(|placement| {
                    placement.epoch == call.epoch && placement.primary == call.target
                })
        {
            return Err(unavailable());
        }
        let remaining = call.expires_at_ms.saturating_sub(unix_time_ms());
        if remaining == 0 {
            return Err(deadline_error());
        }
        let deadline = Instant::now() + Duration::from_millis(remaining).min(OWNER_BUDGET);
        self.execute_at_owner(call, deadline).await
    }
}

impl<S: RaftLogStore + 'static> ClientDataBackend for RoutedValueBackend<S> {
    fn execute<'a>(&'a self, request: VerifiedClientRequest<'a>) -> ClientBackendFuture<'a> {
        Box::pin(async move {
            let now = Instant::now();
            let deadline = request
                .transport_deadline()
                .unwrap_or(now + OWNER_BUDGET)
                .min(now + OWNER_BUDGET);
            let principal = request.identity().client_id().to_owned();
            let tenant = request.identity().tenant().to_owned();
            self.authorize(&principal, &tenant)?;
            let placement = self.placement(&tenant, &request.envelope().request)?;
            let remaining = deadline
                .saturating_duration_since(Instant::now())
                .as_millis() as u64;
            if remaining == 0 {
                return Err(deadline_error());
            }
            let call = OwnerCall {
                version: OWNER_CALL_VERSION,
                target: placement.primary,
                epoch: placement.epoch,
                source: self.owner.replica.local_member().clone(),
                partition: placement.partition,
                principal,
                tenant,
                expires_at_ms: unix_time_ms().saturating_add(remaining),
                envelope: request.into_envelope(),
            };
            if call.target == call.source {
                return self.execute_at_owner(&call, deadline).await;
            }
            let fingerprint = hash(&call);
            let transport = self
                .owner
                .value_transport
                .as_ref()
                .ok_or_else(unavailable)?;
            let reply = timeout_at(deadline, transport.call_owner(&call))
                .await
                .map_err(|_| outcome_unknown())?
                .map_err(|_| outcome_unknown())?;
            if reply.fingerprint != fingerprint {
                return Err(outcome_unknown());
            }
            reply.outcome
        })
    }
}

fn operation_key(request: &ClientRequest) -> Option<(&Namespace, &StructuredKey)> {
    match request {
        ClientRequest::Get { ns, key }
        | ClientRequest::Invalidate { ns, key }
        | ClientRequest::Put {
            ns,
            key,
            ttl_ms: None,
            ..
        }
        | ClientRequest::CompareAndSet { ns, key, .. }
        | ClientRequest::RemoveIfValue { ns, key, .. } => Some((ns, key)),
        _ => None,
    }
}

pub(crate) fn decode_key(key: &StructuredKey) -> Option<Vec<u8>> {
    let [hex] = key.segments() else { return None };
    if hex.is_empty()
        || hex.len() % 2 != 0
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return None;
    }
    (0..hex.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&hex[index..index + 2], 16).ok())
        .collect()
}

pub(crate) fn transport_unavailable() -> CacheError {
    CacheError::Backend("HC/2 IMap owner transport unavailable".into())
}

fn outcome_unknown() -> ClientErrorEnvelope {
    ClientErrorEnvelope::new(
        ClientErrorCode::BackendUnavailable,
        false,
        "HC/2 owner outcome unknown",
    )
}

fn unsupported() -> ClientErrorEnvelope {
    ClientErrorEnvelope::new(
        ClientErrorCode::MalformedFrame,
        false,
        "request is outside the admitted HC/2 IMap scope",
    )
}

fn deadline_error() -> ClientErrorEnvelope {
    ClientErrorEnvelope::new(
        ClientErrorCode::DeadlineExceeded,
        true,
        "HC/2 owner deadline expired",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hc2_keys_are_canonical_binary_segments() {
        let key = |parts: &[&str]| {
            StructuredKey::new(parts.iter().map(ToString::to_string).collect()).unwrap()
        };
        assert_eq!(decode_key(&key(&["00ff61"])), Some(vec![0, 255, b'a']));
        for parts in [&["0"][..], &["AB"], &["00", "ff"], &["zz"]] {
            assert_eq!(decode_key(&key(parts)), None);
        }
    }
}
