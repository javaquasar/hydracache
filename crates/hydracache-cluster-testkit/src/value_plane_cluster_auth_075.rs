//! Runtime-only authenticated member envelopes for the actual service-cluster test transport.
//! Credentials use the existing operator-supplied node identity boundary. This defines no
//! production wire identity, cryptographic channel, or mTLS deployment claim.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, Mutex};

use hydracache_cluster_transport_axum::{
    AllowAllAuthorizer, Authorizer, ClusterRoute, NodeCredential, NodeIdentityProvider,
    StaticNodeIdentityProvider,
};
use sha2::{Digest, Sha256};

use crate::value_plane_model_075::MutationIdentity;
use crate::value_plane_security_075::{
    validate_replica_proof, validate_route, ReplayDisposition, ReplayGuard, ReplayIdentity,
    ReplicaProofClaims, RouteClaims, SecurityBounds, TrustEpochWindow,
};
use crate::value_plane_service_075::ReplicaStamp;
use crate::value_plane_service_cluster_075::{
    ServiceClusterError, ServiceMessageContext, ServiceMessageGuard, ServiceMessageKind,
};
use crate::value_plane_service_registry_075::AuthenticatedNamespace;

#[derive(Clone)]
pub struct RuntimeMember {
    pub node: String,
    pub generation: u64,
    pub credential: NodeCredential,
}
impl fmt::Debug for RuntimeMember {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RuntimeMember { identity: [redacted], credential: [redacted] }")
    }
}

#[derive(Debug, Clone)]
pub struct ClusterAuthConfig {
    pub cluster: String,
    pub tenant: String,
    pub namespace: String,
    pub generation: u64,
    pub members: Vec<RuntimeMember>,
    pub max_replays: usize,
}

/// Mutable fields deliberately permit hostile-envelope tests. Verification compares each
/// field against the real transport context and independently authenticated member identity.
#[derive(Clone)]
pub struct RuntimeEnvelope {
    pub kind: ServiceMessageKind,
    pub cluster: String,
    pub tenant: String,
    pub namespace: String,
    pub namespace_generation: u64,
    pub source: String,
    pub target: String,
    pub source_generation: u64,
    pub target_generation: u64,
    pub trust_epoch: u64,
    pub member_trust_epoch: u64,
    pub nonce: u64,
    pub deadline: u64,
    pub partition: u32,
    pub epoch: u64,
    pub request_digest: [u8; 32],
    pub payload_digest: [u8; 32],
    pub payload_bytes: usize,
    pub mutation_identity: Option<MutationIdentity>,
    pub replica_stamp: Option<ReplicaStamp>,
    pub admission: u64,
    pub credential: NodeCredential,
}
impl fmt::Debug for RuntimeEnvelope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RuntimeEnvelope")
            .field("nonce", &self.nonce)
            .field("payload_bytes", &self.payload_bytes)
            .field("identity", &"[redacted]")
            .field("credential", &"[redacted]")
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvelopeFault {
    None,
    Tenant,
    Namespace,
    NamespaceGeneration,
    Cluster,
    SourceGeneration,
    Target,
    Credential,
    RequestDigest,
    MutationIdentity,
    AckDigest,
    Deadline,
}
#[derive(Debug, Clone, Copy)]
pub struct AuthFault {
    pub stage: ServiceMessageKind,
    pub fault: EnvelopeFault,
}

struct MemberIdentity {
    generation: u64,
    provider: Arc<dyn NodeIdentityProvider>,
    credential_epochs: BTreeMap<String, u64>,
}
struct AuthState {
    members: BTreeMap<String, MemberIdentity>,
    replay: ReplayGuard,
    next_nonce: u64,
    trust: TrustEpochWindow,
    failures: BTreeMap<&'static str, u64>,
}
#[derive(Clone)]
pub struct RuntimeAuthentication {
    config: Arc<ClusterAuthConfig>,
    security: SecurityBounds,
    authorizer: Arc<dyn Authorizer>,
    state: Arc<Mutex<AuthState>>,
}

impl RuntimeAuthentication {
    pub fn new(
        config: ClusterAuthConfig,
        security: SecurityBounds,
        trust: TrustEpochWindow,
    ) -> Result<Self, ServiceClusterError> {
        Self::with_authorizer(config, security, trust, Arc::new(AllowAllAuthorizer))
    }
    pub fn with_authorizer(
        config: ClusterAuthConfig,
        security: SecurityBounds,
        trust: TrustEpochWindow,
        authorizer: Arc<dyn Authorizer>,
    ) -> Result<Self, ServiceClusterError> {
        if config.generation == 0
            || config.members.len() < 2
            || config.members.len() > 8
            || config.max_replays == 0
            || [&config.cluster, &config.tenant, &config.namespace]
                .iter()
                .any(|value| value.is_empty() || value.len() > 128)
        {
            return Err(ServiceClusterError::Authorization("configuration"));
        }
        let mut members = BTreeMap::new();
        let mut tokens = std::collections::BTreeSet::new();
        for member in &config.members {
            if member.node.is_empty()
                || member.node.len() > 128
                || member.generation == 0
                || member.credential.key_id.is_empty()
                || member.credential.key_id.len() > 128
                || member.credential.token.is_empty()
                || member.credential.token.len() > 256
                || members.contains_key(&member.node)
                || !tokens.insert(member.credential.token.clone())
            {
                return Err(ServiceClusterError::Authorization("configuration"));
            }
            members.insert(
                member.node.clone(),
                MemberIdentity {
                    generation: member.generation,
                    provider: Arc::new(StaticNodeIdentityProvider::new(
                        member.node.clone(),
                        member.credential.key_id.clone(),
                        member.credential.token.clone(),
                    )),
                    credential_epochs: BTreeMap::from([(
                        member.credential.key_id.clone(),
                        trust.current_epoch(),
                    )]),
                },
            );
        }
        let replay = ReplayGuard::new(config.max_replays)
            .map_err(|_| ServiceClusterError::Authorization("configuration"))?;
        Ok(Self {
            config: Arc::new(config),
            security,
            authorizer,
            state: Arc::new(Mutex::new(AuthState {
                members,
                replay,
                next_nonce: 1,
                trust,
                failures: BTreeMap::new(),
            })),
        })
    }

    pub fn update_member_generation(
        &self,
        node: &str,
        generation: u64,
    ) -> Result<(), ServiceClusterError> {
        let mut state = self.state.lock().expect("member auth poisoned");
        let member = state
            .members
            .get_mut(node)
            .ok_or(ServiceClusterError::Authorization("member"))?;
        if generation < member.generation {
            return Err(ServiceClusterError::Authorization("member_generation"));
        }
        member.generation = generation;
        Ok(())
    }
    pub fn validate_rotation(
        &self,
        next: u64,
        members: &[RuntimeMember],
    ) -> Result<(), ServiceClusterError> {
        let state = self.state.lock().unwrap();
        let mut trust = state.trust;
        trust
            .rotate(next, 0)
            .map_err(|_| ServiceClusterError::Authorization("trust"))?;
        let current = state
            .members
            .iter()
            .map(|(node, member)| RuntimeMember {
                node: node.clone(),
                generation: member.generation,
                credential: member.provider.present(),
            })
            .collect::<Vec<_>>();
        validate_member_rotation(&current, members)
    }
    pub fn current_members(&self) -> Vec<RuntimeMember> {
        self.state
            .lock()
            .unwrap()
            .members
            .iter()
            .map(|(node, member)| RuntimeMember {
                node: node.clone(),
                generation: member.generation,
                credential: member.provider.present(),
            })
            .collect()
    }
    pub fn rotate_credentials(
        &self,
        next: u64,
        now: u64,
        members: &[RuntimeMember],
    ) -> Result<(), ServiceClusterError> {
        let mut state = self.state.lock().unwrap();
        let current = state
            .members
            .iter()
            .map(|(node, member)| RuntimeMember {
                node: node.clone(),
                generation: member.generation,
                credential: member.provider.present(),
            })
            .collect::<Vec<_>>();
        validate_member_rotation(&current, members)?;
        let mut trust = state.trust;
        trust
            .rotate(next, now)
            .map_err(|_| ServiceClusterError::Authorization("trust"))?;
        state.trust = trust;
        for member in members {
            let old = &state.members[&member.node];
            let previous = old.provider.present();
            let previous_epoch = old.credential_epochs[&previous.key_id];
            let provider = StaticNodeIdentityProvider::new(
                member.node.clone(),
                member.credential.key_id.clone(),
                member.credential.token.clone(),
            )
            .with_previous(previous.key_id.clone(), previous.token);
            state.members.insert(
                member.node.clone(),
                MemberIdentity {
                    generation: member.generation,
                    provider: Arc::new(provider),
                    credential_epochs: BTreeMap::from([
                        (previous.key_id, previous_epoch),
                        (member.credential.key_id.clone(), next),
                    ]),
                },
            );
        }
        Ok(())
    }
    pub fn failures(&self) -> BTreeMap<&'static str, u64> {
        self.state
            .lock()
            .expect("member auth poisoned")
            .failures
            .clone()
    }

    pub fn guard(
        &self,
        authenticated: &AuthenticatedNamespace,
        admission: u64,
        credit: usize,
        fault: Option<AuthFault>,
    ) -> Arc<dyn ServiceMessageGuard> {
        let (tenant, namespace, generation, trust_epoch, deadline) = authenticated.runtime_proof();
        Arc::new(AuthenticatedMessageGuard {
            authority: self.clone(),
            tenant,
            namespace,
            generation,
            trust_epoch,
            deadline,
            admission,
            credit,
            fault,
        })
    }

    fn envelope(
        &self,
        context: &ServiceMessageContext,
        principal: &AuthenticatedMessageGuard,
    ) -> Result<RuntimeEnvelope, ServiceClusterError> {
        let mut state = self.state.lock().expect("member auth poisoned");
        let member = state
            .members
            .get(&context.source)
            .ok_or(ServiceClusterError::Authorization("member"))?;
        let credential = member.provider.present();
        let member_trust_epoch = member.credential_epochs[&credential.key_id];
        let nonce = state.next_nonce;
        state.next_nonce = nonce
            .checked_add(1)
            .ok_or(ServiceClusterError::Authorization("nonce_capacity"))?;
        Ok(RuntimeEnvelope {
            kind: context.kind,
            cluster: self.config.cluster.clone(),
            tenant: principal.tenant.clone(),
            namespace: principal.namespace.clone(),
            namespace_generation: principal.generation,
            source: context.source.clone(),
            target: context.target.clone(),
            source_generation: context.source_generation,
            target_generation: context.target_generation,
            trust_epoch: principal.trust_epoch,
            member_trust_epoch,
            nonce,
            deadline: context.deadline.min(principal.deadline),
            partition: context.partition,
            epoch: context.epoch,
            request_digest: context.request_digest,
            payload_digest: context.payload_digest,
            payload_bytes: context.payload_bytes,
            mutation_identity: context.mutation_identity.clone(),
            replica_stamp: context.replica_stamp,
            admission: principal.admission,
            credential,
        })
    }

    pub fn make_envelope(
        &self,
        authenticated: &AuthenticatedNamespace,
        expected: &ServiceMessageContext,
        admission: u64,
    ) -> Result<RuntimeEnvelope, ServiceClusterError> {
        let (tenant, namespace, generation, trust_epoch, deadline) = authenticated.runtime_proof();
        self.envelope(
            expected,
            &AuthenticatedMessageGuard {
                authority: self.clone(),
                tenant,
                namespace,
                generation,
                trust_epoch,
                deadline,
                admission,
                credit: usize::MAX,
                fault: None,
            },
        )
    }
    pub fn validate_principal(
        &self,
        authenticated: &AuthenticatedNamespace,
        now: u64,
    ) -> Result<(), ServiceClusterError> {
        let (tenant, namespace, generation, trust_epoch, deadline) = authenticated.runtime_proof();
        if tenant != self.config.tenant
            || namespace != self.config.namespace
            || generation != self.config.generation
        {
            return Err(ServiceClusterError::Authorization("principal"));
        }
        let state = self.state.lock().unwrap();
        validate_route(
            &RouteClaims {
                authenticated: true,
                authenticated_tenant: &self.config.tenant,
                claimed_tenant: &tenant,
                expected_namespace_generation: self.config.generation,
                actual_namespace_generation: generation,
                expected_topology_epoch: 1,
                actual_topology_epoch: 1,
                proxy_hops: 0,
                deadline_remaining_ticks: deadline.saturating_sub(now),
                trust_epoch,
                encoded_envelope_bytes: 0,
                key_bytes: 0,
                value_bytes: 0,
            },
            self.security,
            state.trust,
            now,
        )
        .map_err(|_| ServiceClusterError::Authorization("route"))
    }

    pub fn verify_envelope(
        &self,
        envelope: &RuntimeEnvelope,
        expected: &ServiceMessageContext,
        admission: u64,
    ) -> Result<ReplayDisposition, ServiceClusterError> {
        let result = self.verify(envelope, expected, admission);
        if let Err(ServiceClusterError::Authorization(reason)) = &result {
            let mut state = self.state.lock().expect("member auth poisoned");
            let count = state.failures.entry(*reason).or_default();
            *count = count.saturating_add(1);
        }
        result
    }
    fn verify(
        &self,
        envelope: &RuntimeEnvelope,
        expected: &ServiceMessageContext,
        admission: u64,
    ) -> Result<ReplayDisposition, ServiceClusterError> {
        let fail = |reason| ServiceClusterError::Authorization(reason);
        if [
            &envelope.cluster,
            &envelope.tenant,
            &envelope.namespace,
            &envelope.source,
            &envelope.target,
        ]
        .iter()
        .any(|value| value.len() > 128)
            || envelope.credential.key_id.len() > 128
            || envelope.credential.token.len() > 256
            || envelope
                .mutation_identity
                .as_ref()
                .is_some_and(|identity| identity.client.len() > 128)
        {
            return Err(fail("envelope_bound"));
        }
        if envelope.cluster != self.config.cluster {
            return Err(fail("cluster"));
        }
        if envelope.tenant != self.config.tenant
            || envelope.namespace != self.config.namespace
            || envelope.namespace_generation != self.config.generation
        {
            return Err(fail("principal"));
        }
        if envelope.source != expected.source
            || envelope.target != expected.target
            || envelope.source_generation != expected.source_generation
            || envelope.target_generation != expected.target_generation
        {
            return Err(fail("member_binding"));
        }
        if envelope.kind != expected.kind
            || envelope.nonce == 0
            || admission == 0
            || envelope.admission != admission
            || envelope.partition != expected.partition
            || envelope.epoch != expected.epoch
            || envelope.request_digest != expected.request_digest
            || envelope.payload_digest != expected.payload_digest
            || envelope.payload_bytes != expected.payload_bytes
            || envelope.mutation_identity != expected.mutation_identity
        {
            return Err(fail("request_binding"));
        }
        let mut state = self.state.lock().expect("member auth poisoned");
        let source = state
            .members
            .get(&envelope.source)
            .ok_or_else(|| fail("member"))?;
        let target = state
            .members
            .get(&envelope.target)
            .ok_or_else(|| fail("member"))?;
        if source.generation != envelope.source_generation
            || target.generation != envelope.target_generation
        {
            return Err(fail("member_generation"));
        }
        let identity = source
            .provider
            .verify(&envelope.credential)
            .map_err(|_| fail("credential"))?;
        if source.credential_epochs.get(&envelope.credential.key_id)
            != Some(&envelope.member_trust_epoch)
            || !state
                .trust
                .accepts(envelope.member_trust_epoch, expected.now)
        {
            return Err(fail("member_trust"));
        }
        let route = if expected.kind == ServiceMessageKind::Request {
            ClusterRoute::OwnerLoad
        } else {
            ClusterRoute::Replicate
        };
        if identity.as_str() != envelope.source || !self.authorizer.allow(&identity, route) {
            return Err(fail("authorization"));
        }
        if envelope.deadline > expected.deadline {
            return Err(fail("deadline_binding"));
        }
        validate_route(
            &RouteClaims {
                authenticated: true,
                authenticated_tenant: &self.config.tenant,
                claimed_tenant: &envelope.tenant,
                expected_namespace_generation: self.config.generation,
                actual_namespace_generation: envelope.namespace_generation,
                expected_topology_epoch: expected.epoch,
                actual_topology_epoch: envelope.epoch,
                proxy_hops: 0,
                deadline_remaining_ticks: envelope.deadline.saturating_sub(expected.now),
                trust_epoch: envelope.trust_epoch,
                encoded_envelope_bytes: envelope.payload_bytes,
                key_bytes: 0,
                value_bytes: 0,
            },
            self.security,
            state.trust,
            expected.now,
        )
        .map_err(|_| fail("route"))?;
        if envelope.replica_stamp != expected.replica_stamp {
            return Err(fail("replica_binding"));
        }
        if let Some(stamp) = expected.replica_stamp {
            validate_replica_proof(ReplicaProofClaims {
                authenticated: true,
                expected_partition: expected.partition,
                partition: envelope.partition,
                expected_epoch: expected.epoch,
                epoch: envelope.epoch,
                expected_version: stamp.revision,
                version: envelope
                    .replica_stamp
                    .expect("checked replica proof")
                    .revision,
                expected_checksum: stamp.digest,
                checksum: envelope
                    .replica_stamp
                    .expect("checked replica proof")
                    .digest,
            })
            .map_err(|_| fail("replica_binding"))?;
        } else if expected.kind == ServiceMessageKind::Ack {
            return Err(fail("replica_binding"));
        }
        let canonical = envelope_digest(envelope, expected.kind);
        state
            .replay
            .admit(
                ReplayIdentity {
                    tenant: self.config.tenant.clone(),
                    client: format!(
                        "{}:{}:{:?}",
                        envelope.source, envelope.source_generation, expected.kind
                    ),
                    request: envelope.nonce,
                },
                &canonical,
            )
            .map_err(|_| fail("replay"))
    }
}

pub fn validate_member_rotation(
    current: &[RuntimeMember],
    members: &[RuntimeMember],
) -> Result<(), ServiceClusterError> {
    if members.len() < 2 || members.len() > 8 || members.len() != current.len() {
        return Err(ServiceClusterError::Authorization("configuration"));
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut tokens = std::collections::BTreeSet::new();
    for member in members {
        let old = current
            .iter()
            .find(|old| old.node == member.node)
            .ok_or(ServiceClusterError::Authorization("member"))?;
        if !seen.insert(&member.node)
            || !tokens.insert(&member.credential.token)
            || member.generation == 0
            || member.generation < old.generation
            || member.credential.key_id.is_empty()
            || member.credential.key_id.len() > 128
            || member.credential.token.is_empty()
            || member.credential.token.len() > 256
            || member.credential.key_id == old.credential.key_id
            || current
                .iter()
                .any(|old| old.credential.token == member.credential.token)
        {
            return Err(ServiceClusterError::Authorization("credential_rotation"));
        }
    }
    Ok(())
}

struct AuthenticatedMessageGuard {
    authority: RuntimeAuthentication,
    tenant: String,
    namespace: String,
    generation: u64,
    trust_epoch: u64,
    deadline: u64,
    admission: u64,
    credit: usize,
    fault: Option<AuthFault>,
}
impl ServiceMessageGuard for AuthenticatedMessageGuard {
    fn authorize(&self, context: &ServiceMessageContext) -> Result<(), ServiceClusterError> {
        if context.payload_bytes > self.credit {
            return Err(ServiceClusterError::Authorization("replica_credit"));
        }
        let mut envelope = self.authority.envelope(context, self)?;
        if let Some(fault) = self.fault.filter(|fault| fault.stage == context.kind) {
            match fault.fault {
                EnvelopeFault::None => {}
                EnvelopeFault::Tenant => envelope.tenant.push('!'),
                EnvelopeFault::Namespace => envelope.namespace.push('!'),
                EnvelopeFault::NamespaceGeneration => {
                    envelope.namespace_generation = envelope.namespace_generation.wrapping_add(1)
                }
                EnvelopeFault::Cluster => envelope.cluster.push('!'),
                EnvelopeFault::SourceGeneration => {
                    envelope.source_generation = envelope.source_generation.wrapping_add(1)
                }
                EnvelopeFault::Target => envelope.target.push('!'),
                EnvelopeFault::Credential => envelope.credential.token.push('!'),
                EnvelopeFault::RequestDigest => envelope.request_digest[0] ^= 1,
                EnvelopeFault::MutationIdentity => {
                    envelope.mutation_identity = Some(MutationIdentity::new("forged", 1))
                }
                EnvelopeFault::AckDigest => {
                    if let Some(stamp) = envelope.replica_stamp.as_mut() {
                        stamp.digest[0] ^= 1;
                    }
                }
                EnvelopeFault::Deadline => envelope.deadline = context.now,
            }
        }
        self.authority
            .verify_envelope(&envelope, context, self.admission)
            .map(|_| ())
    }
}
fn envelope_digest(envelope: &RuntimeEnvelope, kind: ServiceMessageKind) -> [u8; 32] {
    let mut hash = Sha256::new();
    for value in [
        &envelope.cluster,
        &envelope.tenant,
        &envelope.namespace,
        &envelope.source,
        &envelope.target,
    ] {
        hash.update((value.len() as u64).to_be_bytes());
        hash.update(value.as_bytes());
    }
    hash.update(
        format!(
            "{:?}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{:?}:{:?}",
            kind,
            envelope.namespace_generation,
            envelope.source_generation,
            envelope.target_generation,
            envelope.trust_epoch,
            envelope.member_trust_epoch,
            envelope.deadline,
            envelope.partition,
            envelope.epoch,
            envelope.admission,
            envelope.mutation_identity,
            envelope.replica_stamp
        )
        .as_bytes(),
    );
    hash.update(envelope.request_digest);
    hash.update(envelope.payload_digest);
    hash.update((envelope.payload_bytes as u64).to_be_bytes());
    hash.finalize().into()
}
