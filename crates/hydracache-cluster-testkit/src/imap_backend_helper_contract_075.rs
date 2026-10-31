//! Local development helper boundary; the configured grant is not a production credential.
use crate::imap_backend_contract_075::{
    BackendBinding, BoundaryError, DispatchMode, ReferenceBackendPort,
};
use crate::value_plane_async_075::{
    InvocationClock, InvocationFuture, InvocationHandle, InvocationRejection,
};
use crate::value_plane_model_075::MutationIdentity;
use crate::value_plane_security_075::{GrantOperation, NamespaceGrant, SecurityError};
use crate::value_plane_service_075::{
    LocalHelperCommand, LocalHelperResult, MapServiceError, UnifiedIMapService,
};
use crate::value_plane_service_async_075::{LocalHelperAsyncFacade, LocalHelperStartError};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HelperBoundaryError {
    Boundary(BoundaryError),
    Security(SecurityError),
    Domain(MapServiceError),
    Invocation(InvocationRejection),
    Start(LocalHelperStartError),
}
impl std::fmt::Display for HelperBoundaryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for HelperBoundaryError {}

/// Owned command plus a captured immutable local grant. No resources are reserved by prepare.
#[derive(Debug)]
pub struct PreparedHelperInvocation {
    owner: Arc<()>,
    binding: BackendBinding,
    grant: [u8; 32],
    identity: MutationIdentity,
    deadline: u64,
    command: LocalHelperCommand,
}
pub type StartedHelperInvocation = Result<
    (
        InvocationHandle,
        InvocationFuture<LocalHelperResult, MapServiceError>,
    ),
    HelperBoundaryError,
>;
pub trait DevelopmentHelperPort {
    fn prepare(
        &self,
        binding: &BackendBinding,
        identity: MutationIdentity,
        deadline: u64,
        command: LocalHelperCommand,
    ) -> Result<PreparedHelperInvocation, HelperBoundaryError>;
    fn invoke(&self, prepared: PreparedHelperInvocation) -> StartedHelperInvocation;
}
pub struct ReferenceHelperPort {
    owner: Arc<()>,
    binding: BackendBinding,
    grant: NamespaceGrant,
    service: UnifiedIMapService,
    clock: Arc<InvocationClock>,
    facade: LocalHelperAsyncFacade,
}
impl ReferenceHelperPort {
    pub fn new(
        binding: BackendBinding,
        grant: NamespaceGrant,
        service: UnifiedIMapService,
        clock: Arc<InvocationClock>,
        max_inflight: usize,
        mode: DispatchMode,
    ) -> Result<Self, HelperBoundaryError> {
        ReferenceBackendPort::new(binding.clone(), service.clone(), mode)
            .map_err(HelperBoundaryError::Boundary)?;
        if !grant.matches(
            &binding.namespace.tenant.to_string(),
            &binding.namespace.namespace.to_string(),
            binding.namespace.generation,
        ) {
            return Err(HelperBoundaryError::Security(
                SecurityError::PermissionDenied,
            ));
        }
        let facade = LocalHelperAsyncFacade::new(service.clone(), Arc::clone(&clock), max_inflight)
            .map_err(HelperBoundaryError::Invocation)?;
        Ok(Self {
            owner: Arc::new(()),
            binding,
            grant,
            service,
            clock,
            facade,
        })
    }
    fn validate(
        &self,
        binding: &BackendBinding,
        identity: &MutationIdentity,
        command: &LocalHelperCommand,
    ) -> Result<(), HelperBoundaryError> {
        if binding != &self.binding {
            return Err(HelperBoundaryError::Boundary(
                BoundaryError::ContextMismatch,
            ));
        }
        if identity.client != binding.client || identity.sequence == 0 {
            return Err(HelperBoundaryError::Boundary(
                BoundaryError::IdentityMismatch,
            ));
        }
        self.service
            .local_helper_preflight(command, identity, self.clock.now())
            .map_err(HelperBoundaryError::Domain)?;
        let (operation, wide) = match command {
            LocalHelperCommand::EntryViews { .. } => (GrantOperation::Read, false),
            LocalHelperCommand::VersionBulk { .. } => (GrantOperation::Write, false),
            LocalHelperCommand::QueryOutcomes { .. } => (GrantOperation::Read, true),
            LocalHelperCommand::ListenerSnapshot { .. } => (GrantOperation::Listen, true),
            _ => (GrantOperation::Admin, true),
        };
        self.grant
            .authorize(operation, &[], wide)
            .map_err(HelperBoundaryError::Security)?;
        let allowed = match command {
            LocalHelperCommand::EntryViews { keys, .. } => {
                keys.iter().all(|key| self.grant.key_allowed(key))
            }
            LocalHelperCommand::VersionBulk { entries, .. } => entries
                .iter()
                .all(|entry| self.grant.key_allowed(&entry.key)),
            _ => true,
        };
        if !allowed {
            return Err(HelperBoundaryError::Security(
                SecurityError::PermissionDenied,
            ));
        }
        Ok(())
    }
}
impl DevelopmentHelperPort for ReferenceHelperPort {
    fn prepare(
        &self,
        binding: &BackendBinding,
        identity: MutationIdentity,
        deadline: u64,
        command: LocalHelperCommand,
    ) -> Result<PreparedHelperInvocation, HelperBoundaryError> {
        self.validate(binding, &identity, &command)?;
        Ok(PreparedHelperInvocation {
            owner: Arc::clone(&self.owner),
            binding: binding.clone(),
            grant: self.grant.digest(),
            identity,
            deadline,
            command,
        })
    }
    fn invoke(&self, prepared: PreparedHelperInvocation) -> StartedHelperInvocation {
        if !Arc::ptr_eq(&self.owner, &prepared.owner) || prepared.grant != self.grant.digest() {
            return Err(HelperBoundaryError::Boundary(
                BoundaryError::ContextMismatch,
            ));
        }
        self.validate(&prepared.binding, &prepared.identity, &prepared.command)?;
        self.facade
            .invoke(prepared.identity, prepared.deadline, prepared.command)
            .map_err(HelperBoundaryError::Start)
    }
}
