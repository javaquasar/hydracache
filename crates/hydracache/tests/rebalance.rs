use hydracache::{
    diff_effective_maps, ClusterEpoch, ClusterNodeId, EffectiveReplicationMap, PartitionId,
    RebalancePlan, RebalanceTask, RebalanceTaskAck, Replicas,
};

#[test]
fn diff_produces_expected_move_tasks() {
    let partition = PartitionId::new(7);
    let old = EffectiveReplicationMap::new(Replicas::new(
        "member-a",
        vec![ClusterNodeId::from("member-b")],
    ));
    let new = EffectiveReplicationMap::new(Replicas::new(
        "member-c",
        vec![
            ClusterNodeId::from("member-b"),
            ClusterNodeId::from("member-d"),
        ],
    ));

    let tasks = diff_effective_maps(partition, &old, &new);

    assert_eq!(
        tasks,
        vec![
            RebalanceTask::MovePartition {
                partition,
                from: ClusterNodeId::from("member-a"),
                to: ClusterNodeId::from("member-c"),
            },
            RebalanceTask::ReReplicate {
                partition,
                target: ClusterNodeId::from("member-d"),
            },
        ]
    );
}

#[test]
fn replaying_plan_is_idempotent() {
    let partition = PartitionId::new(3);
    let task = RebalanceTask::ReReplicate {
        partition,
        target: ClusterNodeId::from("member-b"),
    };

    let plan = RebalancePlan::new(ClusterEpoch::new(9), vec![task.clone(), task.clone()]);

    assert_eq!(plan.tasks, vec![task]);
}

#[test]
fn under_replication_reported_until_plan_completes() {
    let partition = PartitionId::new(11);
    let plan = RebalancePlan::new(
        ClusterEpoch::new(4),
        vec![
            RebalanceTask::ReReplicate {
                partition,
                target: ClusterNodeId::from("member-b"),
            },
            RebalanceTask::ReReplicate {
                partition,
                target: ClusterNodeId::from("member-c"),
            },
        ],
    );
    let first_ack = RebalanceTaskAck {
        epoch: ClusterEpoch::new(4),
        task: plan.tasks[0].clone(),
    };

    assert_eq!(plan.pending_task_count(&[]), 2);
    assert_eq!(plan.pending_task_count(&[first_ack.clone()]), 1);
    assert!(!plan.is_complete(&[first_ack]));

    let all_acks = plan
        .tasks
        .iter()
        .cloned()
        .map(|task| RebalanceTaskAck {
            epoch: ClusterEpoch::new(4),
            task,
        })
        .collect::<Vec<_>>();
    assert!(plan.is_complete(&all_acks));
    assert_eq!(plan.pending_task_count(&all_acks), 0);
}
