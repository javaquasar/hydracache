//! Scoped maintenance over existing transfer owners. Restart creates a fresh runtime attempt ID.
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransferScope {
    pub transfer: u64,
    pub partition: u32,
    pub source_epoch: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferDiagnostic {
    pub scope: TransferScope,
    pub source: String,
    pub target: String,
    pub purpose: OnlineTransferPurpose,
    pub phase: OnlineTransferPhase,
    pub pending_deltas: usize,
    pub remaining_chunks: usize,
    pub unacknowledged: usize,
    pub held_bytes: usize,
    pub staged_bytes: usize,
    pub pending_owners: usize,
    pub restart_reason: Option<&'static str>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferControlProgress {
    pub previous: TransferScope,
    pub removed: bool,
    pub current: Option<TransferDiagnostic>,
}

impl ServiceClusterHarness {
    fn validate_transfer_scope(
        &self,
        scope: TransferScope,
    ) -> Result<&OnlineTransferState, ServiceClusterError> {
        if scope.transfer == 0
            || scope.partition >= self.bounds.partitions
            || scope.source_epoch == 0
        {
            return Err(ServiceClusterError::InvalidBound);
        }
        let state = self
            .runtime
            .transfers
            .get(&scope.transfer)
            .ok_or(ServiceClusterError::UnknownTransfer)?;
        if state.manifest.partition != scope.partition
            || state.manifest.source_epoch != scope.source_epoch
        {
            return Err(ServiceClusterError::StaleEpoch);
        }
        Ok(state)
    }

    pub fn transfer_diagnostic(
        &self,
        scope: TransferScope,
    ) -> Result<TransferDiagnostic, ServiceClusterError> {
        let state = self.validate_transfer_scope(scope)?;
        let pending_owners = self
            .service(&state.manifest.source, scope.partition, false)?
            .namespace_owners()
            .pending
            .saturating_add(
                self.service(&state.manifest.source, scope.partition, true)?
                    .namespace_owners()
                    .pending,
            );
        Ok(TransferDiagnostic {
            scope,
            source: state.manifest.source.clone(),
            target: state.manifest.target.clone(),
            purpose: state.purpose,
            phase: state.phase,
            pending_deltas: state.deltas.len(),
            remaining_chunks: state
                .components
                .values()
                .map(|part| {
                    part.manifest
                        .chunk_count
                        .saturating_sub(part.accepted.len())
                })
                .sum(),
            unacknowledged: usize::from(state.unacked_chunk.is_some())
                + usize::from(state.unacked_delta.is_some()),
            held_bytes: state
                .component_credit(TransferComponent::Data)
                .saturating_add(state.component_credit(TransferComponent::Journal)),
            staged_bytes: state.resident_bytes(),
            pending_owners,
            restart_reason: state.restart_reason,
        })
    }

    pub fn transfer_diagnostics(
        &self,
        maximum: usize,
    ) -> Result<Vec<TransferDiagnostic>, ServiceClusterError> {
        if maximum == 0
            || maximum > self.runtime.limits.max_transfers
            || self.runtime.transfers.len() > maximum
        {
            return Err(MapServiceError::ResourceBudget("transfer_diagnostics").into());
        }
        self.runtime
            .transfers
            .values()
            .map(|state| {
                self.transfer_diagnostic(TransferScope {
                    transfer: state.manifest.id,
                    partition: state.manifest.partition,
                    source_epoch: state.manifest.source_epoch,
                })
            })
            .collect()
    }

    pub fn abort_transfer_scoped(
        &mut self,
        scope: TransferScope,
    ) -> Result<TransferControlProgress, ServiceClusterError> {
        self.validate_transfer_scope(scope)?;
        self.abort_online_transfer(scope.transfer)?;
        Ok(TransferControlProgress {
            previous: scope,
            removed: false,
            current: Some(self.transfer_diagnostic(scope)?),
        })
    }

    pub fn cleanup_transfer_scoped(
        &mut self,
        scope: TransferScope,
    ) -> Result<TransferControlProgress, ServiceClusterError> {
        if scope.transfer == 0
            || scope.partition >= self.bounds.partitions
            || scope.source_epoch == 0
        {
            return Err(ServiceClusterError::InvalidBound);
        }
        if !self.runtime.transfers.contains_key(&scope.transfer) {
            return Ok(TransferControlProgress {
                previous: scope,
                removed: false,
                current: None,
            });
        }
        self.validate_transfer_scope(scope)?;
        let removed = self.cleanup_online_transfer(scope.transfer)?;
        Ok(TransferControlProgress {
            previous: scope,
            removed,
            current: None,
        })
    }

    pub fn restart_transfer_scoped(
        &mut self,
        scope: TransferScope,
        expected_current_epoch: u64,
        target: &str,
        options: OnlineTransferOptions,
    ) -> Result<TransferControlProgress, ServiceClusterError> {
        self.ensure_namespace()?;
        let state = self.validate_transfer_scope(scope)?;
        if !matches!(
            state.phase,
            OnlineTransferPhase::Aborted | OnlineTransferPhase::RestartRequired
        ) {
            return Err(ServiceClusterError::TransferState);
        }
        let purpose = state.purpose;
        let route = self
            .assignments
            .get(scope.partition as usize)
            .ok_or(ServiceClusterError::InvalidBound)?;
        if route.route.epoch != expected_current_epoch {
            return Err(ServiceClusterError::StaleEpoch);
        }
        self.require_live(target)?;
        self.require_live(&route.route.owner)?;
        let original = self
            .runtime
            .transfers
            .remove(&scope.transfer)
            .expect("validated scoped terminal transfer");
        let started = self.begin_online_transfer_for(
            scope.partition,
            target,
            options,
            purpose,
            original.resident_bytes(),
        );
        match started {
            Ok(transfer) => {
                let manifest = self.online_transfer_manifest(transfer)?;
                let scope_new = TransferScope {
                    transfer,
                    partition: manifest.partition,
                    source_epoch: manifest.source_epoch,
                };
                Ok(TransferControlProgress {
                    previous: scope,
                    removed: true,
                    current: Some(self.transfer_diagnostic(scope_new)?),
                })
            }
            Err(error) => {
                self.runtime.transfers.insert(scope.transfer, original);
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restart_reserves_the_retained_rollback_owner_before_allocating_a_new_snapshot() {
        let mut cluster = ServiceClusterHarness::new(
            &["a", "b", "c"],
            ServiceClusterBounds {
                partitions: 1,
                ..ServiceClusterBounds::default()
            },
            CollectionBounds::default(),
            ServiceLimits::default(),
            1,
        )
        .unwrap();
        let old = cluster
            .begin_online_transfer(0, "c", OnlineTransferOptions::default())
            .unwrap();
        let scope = TransferScope {
            transfer: old,
            partition: 0,
            source_epoch: 1,
        };
        cluster.abort_transfer_scoped(scope).unwrap();
        let mut fresh_bytes = 384;
        for journal in [false, true] {
            let source = cluster.service("a", 0, journal).unwrap();
            let (image, cursor) = source.preflight_replica_snapshot_with_cursor().unwrap();
            fresh_bytes +=
                image + cursor + source.replica_transport_size(8 * 1_024 * 1_024).unwrap();
        }
        cluster.runtime.limits.max_transfer_bytes = fresh_bytes;
        let options = OnlineTransferOptions {
            max_snapshot_bytes: fresh_bytes,
            max_delta_bytes: fresh_bytes,
            ..OnlineTransferOptions::default()
        };
        let before = (
            cluster.aggregate_resource_usage(),
            cluster.transfer_diagnostic(scope).unwrap(),
            cluster.runtime.next_transfer,
        );
        assert_eq!(
            cluster
                .restart_transfer_scoped(scope, 1, "c", options)
                .unwrap_err(),
            ServiceClusterError::Model(MapServiceError::ResourceBudget("transfer_staging"))
        );
        assert_eq!(
            (
                cluster.aggregate_resource_usage(),
                cluster.transfer_diagnostic(scope).unwrap(),
                cluster.runtime.next_transfer
            ),
            before
        );
        assert!(cluster.cleanup_transfer_scoped(scope).unwrap().removed);
        let replacement = cluster.begin_online_transfer(0, "c", options).unwrap();
        assert!(cluster.runtime.transfers[&replacement].resident_bytes() <= fresh_bytes);
        cluster.abort_online_transfer(replacement).unwrap();
        cluster.cleanup_online_transfer(replacement).unwrap();
        assert_eq!(cluster.aggregate_namespace_owners().resync_leases, 0);
    }
}
