//! Development-only backend conformance boundary. These types allocate no wire IDs.
//! A binding checks adapter consistency; it is not a production authentication credential.

use crate::provisional_backend_075::{NamespaceIdentity, PartitionRoute};
use crate::value_plane_model_075::identity_blank;
use crate::value_plane_service_075::{
    is_mutating_operation, MapServiceError, ServiceCommand, ServiceResult, UnifiedIMapService,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationScope {
    ExplicitKeys,
    NamespaceReferenceOnly,
    Handle,
}

macro_rules! operations {
    ($( $variant:ident => ($name:literal, $scope:ident) ),+ $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        pub enum BackendOperation { $( $variant ),+ }
        impl BackendOperation {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];
            pub fn name(self) -> &'static str { match self { $( Self::$variant => $name ),+ } }
            pub fn scope(self) -> OperationScope { match self { $( Self::$variant => OperationScope::$scope ),+ } }
            pub fn parse(name: &str) -> Result<Self, BoundaryError> {
                match name { $( $name => Ok(Self::$variant), )+ _ => Err(BoundaryError::UnsupportedOperation) }
            }
        }
    };
}

operations! {
    Get => ("get", ExplicitKeys), Put => ("put", ExplicitKeys), Delete => ("delete", ExplicitKeys),
    ConditionalReplace => ("conditional_replace", ExplicitKeys), ConditionalRemove => ("conditional_remove", ExplicitKeys),
    ContainsKey => ("contains_key", ExplicitKeys), PutIfAbsent => ("put_if_absent", ExplicitKeys),
    ReplaceIfPresent => ("replace_if_present", ExplicitKeys), GetAndRemove => ("get_and_remove", ExplicitKeys),
    GetAndPut => ("get_and_put", ExplicitKeys), GetAll => ("get_all", ExplicitKeys),
    PutAll => ("put_all_detailed", ExplicitKeys), RemoveAll => ("remove_all_detailed", ExplicitKeys),
    SetTtl => ("set_ttl", ExplicitKeys), RemainingTtl => ("remaining_ttl", ExplicitKeys),
    EntryListener => ("entry_listener", NamespaceReferenceOnly), Size => ("size", NamespaceReferenceOnly),
    IsEmpty => ("is_empty", NamespaceReferenceOnly), ContainsValue => ("contains_value_bounded", NamespaceReferenceOnly),
    Clear => ("clear_detailed", NamespaceReferenceOnly), Destroy => ("destroy", NamespaceReferenceOnly),
    Evict => ("evict", ExplicitKeys), EvictAll => ("evict_all_detailed", NamespaceReferenceOnly),
    KeyPage => ("key_set_page", NamespaceReferenceOnly), ValuePage => ("values_page", NamespaceReferenceOnly),
    EntryPage => ("entry_set_page", NamespaceReferenceOnly), GetOrDefault => ("get_or_default", ExplicitKeys),
    ContainsEntry => ("contains_entry", ExplicitKeys), Set => ("set", ExplicitKeys),
    EntryView => ("get_entry_view", ExplicitKeys), TryPut => ("try_put", ExplicitKeys), TryRemove => ("try_remove", ExplicitKeys),
    ReplaceAll => ("replace_all_detailed", ExplicitKeys), RemoveListener => ("remove_entry_listener", Handle),
    Capability => ("capability_descriptor", NamespaceReferenceOnly), ReplaceVersion => ("replace_if_version", ExplicitKeys),
    RemoveVersion => ("remove_if_version", ExplicitKeys), SetTtlVersion => ("set_ttl_if_version", ExplicitKeys),
    QueryOutcome => ("query_operation_outcome", Handle), Statistics => ("local_statistics", NamespaceReferenceOnly),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendBinding {
    pub namespace: NamespaceIdentity,
    pub route: PartitionRoute,
    pub client: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchMode {
    LocalReference,
    ProductionMember,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoundaryError {
    InvalidBinding,
    ContextMismatch,
    IdentityMismatch,
    UnsupportedOperation,
    DisabledBlockedBy074,
    Service(MapServiceError),
}

/// Owned request: callers cannot replace payload, TTL, identity or deadline after preflight.
#[derive(Debug)]
pub struct PreparedInvocation {
    binding: BackendBinding,
    operation: BackendOperation,
    command: ServiceCommand,
}

impl PreparedInvocation {
    pub fn operation(&self) -> BackendOperation {
        self.operation
    }
    pub fn deadline(&self) -> u64 {
        self.command.deadline
    }
}

pub trait DevelopmentBackendPort {
    fn prepare(
        &self,
        binding: &BackendBinding,
        command: ServiceCommand,
    ) -> Result<PreparedInvocation, BoundaryError>;
    fn execute(&self, invocation: PreparedInvocation) -> Result<ServiceResult, BoundaryError>;
}

pub struct ReferenceBackendPort {
    binding: BackendBinding,
    service: UnifiedIMapService,
}

impl ReferenceBackendPort {
    pub fn new(
        binding: BackendBinding,
        service: UnifiedIMapService,
        mode: DispatchMode,
    ) -> Result<Self, BoundaryError> {
        if mode != DispatchMode::LocalReference {
            return Err(BoundaryError::DisabledBlockedBy074);
        }
        if binding.namespace.tenant == 0
            || binding.namespace.namespace == 0
            || binding.namespace.generation == 0
            || binding.namespace.generation != service.namespace_generation()
            || binding.route.epoch == 0
            || binding.route.owner == 0
            || identity_blank(&binding.client)
            || binding.client.len() > 128
        {
            return Err(BoundaryError::InvalidBinding);
        }
        Ok(Self { binding, service })
    }
}

impl DevelopmentBackendPort for ReferenceBackendPort {
    fn prepare(
        &self,
        binding: &BackendBinding,
        command: ServiceCommand,
    ) -> Result<PreparedInvocation, BoundaryError> {
        if binding != &self.binding {
            return Err(BoundaryError::ContextMismatch);
        }
        let operation = BackendOperation::parse(&command.operation)?;
        if (is_mutating_operation(operation.name()) && command.identity.is_none())
            || command
                .identity
                .as_ref()
                .is_some_and(|id| id.client != binding.client)
        {
            return Err(BoundaryError::IdentityMismatch);
        }
        self.service
            .validate_command(&command)
            .map_err(BoundaryError::Service)?;
        Ok(PreparedInvocation {
            binding: binding.clone(),
            operation,
            command,
        })
    }

    fn execute(&self, invocation: PreparedInvocation) -> Result<ServiceResult, BoundaryError> {
        if invocation.binding != self.binding {
            return Err(BoundaryError::ContextMismatch);
        }
        self.service
            .execute(invocation.command)
            .map_err(BoundaryError::Service)
    }
}
