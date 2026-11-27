use super::*;

const AUDIT_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct IMapReplicaDigestQuery {
    pub partition: u32,
    pub epoch: u64,
    pub version: u64,
}

impl IMapReplicaDigestQuery {
    pub fn is_valid(&self) -> bool {
        self.partition < D2_PARTITION_COUNT.get() && self.epoch != 0 && self.version != 0
    }
}

/// Locally verified journal content at one certified IMap partition prefix.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IMapReplicaDigest {
    pub schema_version: u16,
    pub member: PartitionMember,
    pub topology_epoch: u64,
    pub primary: bool,
    pub prefix: PartitionValuePrefix,
    pub image_sha256: [u8; 32],
    pub rows: usize,
    pub retained_outcomes: usize,
}

fn audit_unavailable() -> CacheError {
    CacheError::Backend("IMap replica digest is unavailable at the requested cut".into())
}

impl<S: RaftLogStore> ValueBackend<S> {
    pub(crate) async fn replica_digest(
        &self,
        query: IMapReplicaDigestQuery,
    ) -> CacheResult<IMapReplicaDigest> {
        let replica = Arc::clone(&self.replica);
        let permit = Arc::clone(&replica.audit)
            .try_acquire_owned()
            .map_err(|_| audit_unavailable())?;
        let deadline = Instant::now() + AUDIT_TIMEOUT;
        let (prefix, primary) = self.audit_cut(&query, deadline).await?;
        let (image_sha256, rows, retained_outcomes) = timeout_at(
            deadline,
            tokio::task::spawn_blocking(move || {
                // Keep the admission permit until the storage work ends, even after cancellation.
                let _permit = permit;
                let lane = replica
                    .lane(prefix.partition)
                    .ok_or_else(audit_unavailable)?;
                let _guard = lane.storage.try_lock().map_err(|_| audit_unavailable())?;
                replica.journal.audit_image(prefix, |key| {
                    replica.scope.admits_storage_key(key, prefix.partition)
                })
            }),
        )
        .await
        .map_err(|_| audit_unavailable())?
        .map_err(|_| audit_unavailable())??;
        if self.audit_cut(&query, deadline).await? != (prefix, primary) {
            return Err(audit_unavailable());
        }
        Ok(IMapReplicaDigest {
            schema_version: 1,
            member: self.replica.local.clone(),
            topology_epoch: query.epoch,
            primary,
            prefix,
            image_sha256,
            rows,
            retained_outcomes,
        })
    }

    async fn audit_cut(
        &self,
        query: &IMapReplicaDigestQuery,
        deadline: Instant,
    ) -> CacheResult<(PartitionValuePrefix, bool)> {
        let proof = request_read_index(&self.raft, &self.raft_transport, deadline)
            .await
            .map_err(|_| audit_unavailable())?;
        let partition = PartitionId::new(query.partition);
        proof
            .with_snapshot(|snapshot, _| {
                if Instant::now() >= deadline
                    || snapshot.epoch.value() != query.epoch
                    || !admitted(&snapshot, &self.replica.local)
                    || !self.replica.roster_compatible(&snapshot)
                {
                    return Err(audit_unavailable());
                }
                let placement = self
                    .replica
                    .placement(&snapshot, partition)
                    .ok_or_else(audit_unavailable)?;
                let primary = placement.primary == self.replica.local;
                if !primary && !placement.backups.contains(&self.replica.local) {
                    return Err(audit_unavailable());
                }
                snapshot
                    .value_prefixes
                    .get(&partition)
                    .copied()
                    .filter(|prefix| prefix.version == query.version)
                    .map(|prefix| (prefix, primary))
                    .ok_or_else(audit_unavailable)
            })
            .map_err(|_| audit_unavailable())?
    }
}
