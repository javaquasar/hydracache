use hydracache::{
    ClusterEpoch, ClusterMember, ClusterReplicationStrategy, InMemoryCluster, PartitionId,
    PartitionPromotion, PartitionReplicaIdentity, PartitionReplicaRepair, PartitionValuePrefix,
    PrimaryRebalanceCommand, PrimaryRebalanceWave, RendezvousClusterOwnership,
};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn allows_transfer(
    wave: Option<&PrimaryRebalanceWave>,
    id: Option<ClusterEpoch>,
    partition: PartitionId,
    source: &PartitionReplicaIdentity,
    target: &PartitionReplicaIdentity,
) -> bool {
    match wave.and_then(|wave| wave.head().map(|head| (wave.id, head))) {
        Some((active, head)) => {
            id == Some(active)
                && partition == head.partition
                && source == &head.source
                && target == &head.target
        }
        None => id.is_none(),
    }
}

pub(super) fn allows_seed(
    wave: Option<&PrimaryRebalanceWave>,
    repair: &PartitionReplicaRepair,
) -> bool {
    !wave
        .and_then(PrimaryRebalanceWave::head)
        .is_some_and(|head| head.recovery)
        && allows_transfer(
            wave,
            repair.wave,
            repair.partition,
            &repair.owner,
            &repair.target,
        )
}

pub(super) fn placement(
    partition: PartitionId,
    backups: usize,
    members: &[ClusterMember],
    promotion: Option<&PartitionPromotion>,
    repair: Option<&PartitionReplicaRepair>,
) -> Option<(PartitionReplicaIdentity, Vec<PartitionReplicaIdentity>)> {
    if partition.value() >= 271 || !(1..=2).contains(&backups) {
        return None;
    }
    if let Some(promotion) = promotion {
        return (backups == promotion.backups.len())
            .then(|| (promotion.new_owner.clone(), promotion.backups.clone()));
    }
    if let Some(repair) = repair {
        let active = repair.repaired_backups()?;
        return (backups == active.len()).then(|| (repair.owner.clone(), active));
    }
    let ranked = RendezvousClusterOwnership.replicas_for_key(
        &format!("partition-{}", partition.value()),
        members,
        backups + 1,
    )?;
    let identify = |id| {
        members
            .iter()
            .find(|member| member.node_id == id && member.is_member())
            .map(|member| PartitionReplicaIdentity {
                node: id,
                generation: member.generation,
            })
    };
    let owner = identify(ranked.primary)?;
    let replicas = ranked
        .backups
        .into_iter()
        .map(identify)
        .collect::<Option<Vec<_>>>()?;
    Some((owner, replicas))
}

pub(super) fn allows_handoff(
    wave: Option<&PrimaryRebalanceWave>,
    plan: &PartitionPromotion,
    members: &[ClusterMember],
    promotion: Option<&PartitionPromotion>,
    repair: Option<&PartitionReplicaRepair>,
) -> bool {
    if !allows_transfer(
        wave,
        plan.wave,
        plan.partition,
        &plan.old_owner,
        &plan.new_owner,
    ) {
        return false;
    }
    let Some(wave) = wave.filter(|wave| wave.head().is_some()) else {
        return true;
    };
    let Some((owner, mut backups)) = placement(
        plan.partition,
        wave.sync_backups,
        members,
        promotion,
        repair,
    ) else {
        return false;
    };
    let Some(slot) = backups.iter_mut().find(|backup| **backup == plan.new_owner) else {
        return false;
    };
    *slot = owner.clone();
    plan.old_owner == owner && plan.backups == backups
}

pub(super) fn apply_control(
    cluster: &InMemoryCluster,
    wave: &mut Option<PrimaryRebalanceWave>,
    prefixes: &BTreeMap<PartitionId, PartitionValuePrefix>,
    promotions: &BTreeMap<PartitionId, PartitionPromotion>,
    repairs: &BTreeMap<PartitionId, PartitionReplicaRepair>,
    command: &PrimaryRebalanceCommand,
) {
    match command {
        PrimaryRebalanceCommand::Start {
            id,
            scope,
            sync_backups,
            tasks,
        } => {
            if wave.as_ref().is_some_and(|wave| wave.head().is_some())
                || tasks.is_empty()
                || tasks.len() > PrimaryRebalanceWave::MAX_TASKS
            {
                return;
            }
            let epoch = cluster.epoch();
            let members = cluster.members();
            let admit = |id: &PartitionReplicaIdentity| {
                members
                    .iter()
                    .any(|member| member.node_id == id.node && member.generation == id.generation)
            };
            let mut seen = BTreeSet::new();
            if !tasks.iter().all(|task| {
                seen.insert(task.partition)
                    && prefixes.contains_key(&task.partition)
                    && task.source.node != task.target.node
                    && admit(&task.target)
                    && placement(
                        task.partition,
                        *sync_backups,
                        &members,
                        promotions.get(&task.partition),
                        repairs.get(&task.partition),
                    )
                    .is_some_and(|(owner, backups)| {
                        owner == task.source
                            && if task.recovery {
                                *sync_backups == 1 && backups.contains(&task.target)
                            } else {
                                admit(&task.source)
                            }
                    })
            }) || !cluster.apply_partition_epoch(epoch, *id)
            {
                return;
            }
            *wave = Some(PrimaryRebalanceWave {
                id: *id,
                scope: *scope,
                sync_backups: *sync_backups,
                tasks: tasks.clone(),
                completed: 0,
                cancelled: false,
            });
        }
        PrimaryRebalanceCommand::Cancel { id, at_epoch } => {
            if let Some(active) = wave
                .as_mut()
                .filter(|wave| wave.id == *id && wave.head().is_some())
            {
                if let Some(next) = at_epoch.value().checked_add(1).map(ClusterEpoch::new) {
                    if cluster.apply_partition_epoch(*at_epoch, next) {
                        active.cancelled = true;
                    }
                }
            }
        }
    }
}
