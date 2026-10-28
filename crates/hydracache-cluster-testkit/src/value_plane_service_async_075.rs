//! Connects the existing bounded async invocation lifecycle to the unified local IMap dispatcher.

use async_trait::async_trait;

use crate::value_plane_async_075::{AsyncMapBackend, BackendRecovery, InvocationContext};
use crate::value_plane_model_075::MutationIdentity;
use crate::value_plane_service_075::{
    is_mutating_operation, MapServiceError, ServiceCommand, ServiceRecovery, ServiceResult,
    UnifiedIMapService,
};

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
