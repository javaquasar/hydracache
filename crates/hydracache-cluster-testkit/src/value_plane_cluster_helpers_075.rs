//! Authenticated typed helpers use the same permit binding, owner graph and backup proof as core calls.
use super::*;
use crate::value_plane_cluster_admission_075::AdmissionBinding;
use crate::value_plane_security_075::GrantOperation;
pub use crate::value_plane_service_cluster_075::{
    ClusterHelperCommand, ClusterHelperExecution, ClusterHelperPreflight, ClusterHelperProgress,
    ClusterHelperRecovery, ClusterHelperRequest, ClusterHelperReservation, ClusterHelperResult,
};

impl ClusterNamespaceRuntime {
    fn authorize_helper(
        &self,
        authenticated: &AuthenticatedNamespace,
        request: &ClusterHelperRequest,
    ) -> Result<(), ServiceClusterError> {
        self.cluster
            .lock()
            .unwrap()
            .validate_cluster_helper(request)?;
        let keys = request.keys();
        let (operation, map_wide) = match &request.command {
            ClusterHelperCommand::VersionBulk { .. } => (GrantOperation::Write, false),
            ClusterHelperCommand::EntryViews { .. } => (GrantOperation::Read, false),
            ClusterHelperCommand::QueryProgress { original, .. } => (
                GrantOperation::Read,
                !matches!(original.command, ClusterHelperCommand::VersionBulk { .. }),
            ),
            ClusterHelperCommand::QueryOutcomes { .. } => (GrantOperation::Read, true),
            _ => (GrantOperation::Admin, true),
        };
        authenticated
            .authorize(operation, &keys, map_wide)
            .map_err(|_| ServiceClusterError::Authorization("operation_policy"))?;
        self.authentication
            .validate_principal(authenticated, request.now)
    }
    pub fn helper_preflight(
        &self,
        authenticated: &AuthenticatedNamespace,
        request: &ClusterHelperRequest,
    ) -> Result<ClusterHelperPreflight, ServiceClusterError> {
        self.authorize_helper(authenticated, request)?;
        self.cluster
            .lock()
            .unwrap()
            .cluster_helper_preflight(request)
    }

    pub fn execute_helper(
        &self,
        authenticated: &AuthenticatedNamespace,
        permit: &ClusterPermit,
        mut request: ClusterHelperRequest,
        options: Option<&RuntimeCallOptions>,
    ) -> Result<ClusterHelperExecution, ServiceClusterError> {
        request.deadline = request.deadline.min(authenticated.runtime_proof().4);
        let preflight = self.helper_preflight(authenticated, &request)?;
        if permit.work().lane != request.lane()
            || permit.work().tenant != authenticated.address().tenant
            || !permit.keys_match(&preflight.keys)
            || preflight
                .partitions
                .iter()
                .any(|partition| !permit.partitions().contains(partition))
        {
            return Err(ServiceClusterError::Authorization("admission_binding"));
        }
        if preflight.replica_bytes > permit.work().replica_bytes {
            return Err(ServiceClusterError::Authorization("replica_credit"));
        }
        let binding = AdmissionBinding {
            namespace: authenticated.address().namespace.clone(),
            namespace_generation: authenticated.address().generation,
            request_digest: request.request_digest(),
            identity: Some(request.identity.clone()),
            deadline: request.deadline,
            grant_digest: authenticated.grant_digest(),
        };
        let _attempt = permit
            .begin_invocation(binding.clone())
            .map_err(|_| ServiceClusterError::Authorization("admission_receipt"))?;
        if let ClusterHelperCommand::QueryProgress {
            original,
            original_digest,
            max_bytes,
        } = &request.command
        {
            let progress = self.cluster.lock().unwrap().helper_progress(
                original,
                *original_digest,
                *max_bytes,
            )?;
            return Ok(ClusterHelperExecution {
                backup_proved: progress.proofs.iter().all(|proof| proof.proved),
                result: ClusterHelperResult::Progress(Box::new(progress)),
                replayed: false,
                routes: Vec::new(),
                failure: None,
            });
        }
        self.sync_members()?;
        let options = options.cloned().unwrap_or_default();
        let guard = self.authentication.guard_with_receipt(
            authenticated,
            permit.ticket(),
            permit.work().replica_bytes,
            options.auth_fault,
            Some(permit.receipt_digest(&binding)),
        );
        self.with_cluster_wakes(|cluster| {
            cluster.execute_cluster_helper_guarded(
                options
                    .entry_node
                    .as_deref()
                    .unwrap_or(&self.config.entry_node),
                request,
                options.call,
                guard,
            )
        })
    }

    pub fn helper_progress(
        &self,
        authenticated: &AuthenticatedNamespace,
        original: &ClusterHelperRequest,
        original_digest: [u8; 32],
        maximum_bytes: usize,
    ) -> Result<ClusterHelperProgress, ServiceClusterError> {
        self.cluster
            .lock()
            .unwrap()
            .validate_cluster_helper(original)?;
        if !original.is_mutating() {
            return Err(MapServiceError::InvalidArgument.into());
        }
        let mut query = ClusterHelperRequest::new(
            original.identity.clone(),
            ClusterHelperCommand::QueryProgress {
                original: Box::new(original.clone()),
                original_digest,
                max_bytes: maximum_bytes,
            },
        );
        query.now = self.logical_time();
        query.deadline = authenticated.runtime_proof().4;
        self.authorize_helper(authenticated, &query)?;
        self.cluster
            .lock()
            .unwrap()
            .helper_progress(original, original_digest, maximum_bytes)
    }

    pub fn recover_helper_invocation(
        &self,
        authenticated: &AuthenticatedNamespace,
        request: &ClusterHelperRequest,
    ) -> Result<ClusterHelperRecovery, ServiceClusterError> {
        self.authorize_helper(authenticated, request)?;
        self.cluster.lock().unwrap().recover_cluster_helper(request)
    }

    pub fn cancel_helper_invocation(
        &self,
        request: &ClusterHelperRequest,
    ) -> Result<bool, ServiceClusterError> {
        self.with_cluster_wakes(|cluster| cluster.cancel_cluster_helper(request))
    }

    pub fn reserve_helper_invocation(
        &self,
        authenticated: &AuthenticatedNamespace,
        permit: &ClusterPermit,
        request: &ClusterHelperRequest,
        options: Option<&RuntimeCallOptions>,
    ) -> Result<ClusterHelperReservation, ServiceClusterError> {
        if !request.is_mutating() {
            return Err(MapServiceError::InvalidArgument.into());
        }
        self.authorize_helper(authenticated, request)?;
        let mut request = request.clone();
        request.deadline = request.deadline.min(authenticated.runtime_proof().4);
        let preflight = self.helper_preflight(authenticated, &request)?;
        if permit.work().lane != request.lane()
            || permit.work().tenant != authenticated.address().tenant
            || !permit.keys_match(&preflight.keys)
            || preflight
                .partitions
                .iter()
                .any(|partition| !permit.partitions().contains(partition))
        {
            return Err(ServiceClusterError::Authorization("admission_binding"));
        }
        if preflight.replica_bytes > permit.work().replica_bytes {
            return Err(ServiceClusterError::Authorization("replica_credit"));
        }
        let binding = AdmissionBinding {
            namespace: authenticated.address().namespace.clone(),
            namespace_generation: authenticated.address().generation,
            request_digest: request.request_digest(),
            identity: Some(request.identity.clone()),
            deadline: request.deadline,
            grant_digest: authenticated.grant_digest(),
        };
        let _attempt = permit
            .begin_invocation(binding.clone())
            .map_err(|_| ServiceClusterError::Authorization("admission_receipt"))?;
        self.sync_members()?;
        let options = options.cloned().unwrap_or_default();
        let guard = self.authentication.guard_with_receipt(
            authenticated,
            permit.ticket(),
            permit.work().replica_bytes,
            options.auth_fault,
            Some(permit.receipt_digest(&binding)),
        );
        self.with_cluster_wakes(|cluster| {
            cluster.reserve_cluster_helper_guarded(
                options
                    .entry_node
                    .as_deref()
                    .unwrap_or(&self.config.entry_node),
                &request,
                &options.call,
                guard,
            )
        })
    }
}
