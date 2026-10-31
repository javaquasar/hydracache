//! Connects the existing bounded async invocation lifecycle to the unified local IMap dispatcher.

use async_trait::async_trait;

use crate::value_plane_async_075::{
    AsyncIMapFacade, InvocationClock, InvocationFuture, InvocationHandle, InvocationRejection,
};
use crate::value_plane_async_075::{AsyncMapBackend, BackendRecovery, InvocationContext};
use crate::value_plane_model_075::MutationIdentity;
use crate::value_plane_service_075::LocalHelperCredit;
use crate::value_plane_service_075::{
    is_mutating_operation, MapServiceError, ServiceCommand, ServiceRecovery, ServiceResult,
    UnifiedIMapService,
};
pub use crate::value_plane_service_075::{LocalHelperCommand, LocalHelperResult};
use std::sync::Arc;

pub struct UnifiedServiceAsyncBackend {
    pub service: UnifiedIMapService,
}

#[async_trait]
impl AsyncMapBackend for UnifiedServiceAsyncBackend {
    type Command = ServiceCommand;
    type Output = ServiceResult;
    type Error = MapServiceError;

    fn recover(
        &self,
        command: &ServiceCommand,
        identity: &MutationIdentity,
    ) -> Result<BackendRecovery<ServiceResult>, MapServiceError> {
        if !is_mutating_operation(&command.operation) {
            return Ok(BackendRecovery::NotRetained);
        }
        Ok(match self.service.recover_command(command, identity)? {
            ServiceRecovery::Unknown => BackendRecovery::NotRetained,
            ServiceRecovery::Pending => BackendRecovery::Unavailable,
            ServiceRecovery::Completed(result) => BackendRecovery::Retained(*result),
        })
    }

    async fn execute(
        &self,
        command: &ServiceCommand,
        context: InvocationContext,
    ) -> Result<ServiceResult, MapServiceError> {
        context
            .admit()
            .map_err(|_| MapServiceError::InvalidArgument)?;
        let mut command = command.clone();
        command.now = context.now();
        command.deadline = context.deadline;
        if is_mutating_operation(&command.operation) {
            command.identity = Some(context.identity);
        }
        self.service.execute(command)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalHelperStartError {
    Domain(MapServiceError),
    Invocation(InvocationRejection),
}

struct HelperRequest {
    command: LocalHelperCommand,
    _credit: LocalHelperCredit,
}
struct LocalHelperBackend {
    service: UnifiedIMapService,
}

#[async_trait]
impl AsyncMapBackend for LocalHelperBackend {
    type Command = HelperRequest;
    type Output = LocalHelperResult;
    type Error = MapServiceError;
    fn recover(
        &self,
        command: &HelperRequest,
        identity: &MutationIdentity,
    ) -> Result<BackendRecovery<LocalHelperResult>, MapServiceError> {
        self.service
            .recover_local_helper(&command.command, identity)
    }
    async fn execute(
        &self,
        command: &HelperRequest,
        context: InvocationContext,
    ) -> Result<LocalHelperResult, MapServiceError> {
        context
            .admit()
            .map_err(|_| MapServiceError::InvalidArgument)?;
        self.service.execute_local_helper(
            &command.command,
            context.identity.clone(),
            context.now(),
            context.deadline,
        )
    }
}

pub type StartedLocalHelper = Result<
    (
        InvocationHandle,
        InvocationFuture<LocalHelperResult, MapServiceError>,
    ),
    LocalHelperStartError,
>;

/// Lazy typed helpers over the same local owner. Both queued request bytes and maximum result
/// bytes hold aggregate service credit; leased results retain their own credit after delivery.
pub struct LocalHelperAsyncFacade {
    service: UnifiedIMapService,
    facade: AsyncIMapFacade<LocalHelperBackend>,
}
impl LocalHelperAsyncFacade {
    pub fn new(
        service: UnifiedIMapService,
        clock: Arc<InvocationClock>,
        max_inflight: usize,
    ) -> Result<Self, InvocationRejection> {
        Ok(Self {
            facade: AsyncIMapFacade::new(
                Arc::new(LocalHelperBackend {
                    service: service.clone(),
                }),
                clock,
                max_inflight,
            )?,
            service,
        })
    }
    pub fn inflight(&self) -> usize {
        self.facade.inflight()
    }
    pub fn invoke(
        &self,
        identity: MutationIdentity,
        deadline: u64,
        command: LocalHelperCommand,
    ) -> StartedLocalHelper {
        let credit = self
            .service
            .reserve_local_helper_credit(&command)
            .map_err(LocalHelperStartError::Domain)?;
        self.facade
            .invoke(
                identity,
                deadline,
                HelperRequest {
                    command,
                    _credit: credit,
                },
            )
            .map_err(LocalHelperStartError::Invocation)
    }
}
