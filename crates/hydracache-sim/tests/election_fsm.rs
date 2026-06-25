use hydracache_sim::{
    cluster_transition, node_transition, ClusterFsm, ClusterFsmAction, ClusterFsmEvent,
    FormationPhase, NodeFsm, NodeFsmState, CLUSTER_TRANSITION_TABLE, NODE_TRANSITION_TABLE,
};

#[test]
fn transition_table_is_total() {
    assert_eq!(NODE_TRANSITION_TABLE.len(), NodeFsmState::ALL.len());
    for row in NODE_TRANSITION_TABLE {
        assert_eq!(row.len(), ClusterFsmEvent::ALL.len());
    }
    assert_eq!(CLUSTER_TRANSITION_TABLE.len(), FormationPhase::ALL.len());
    for row in CLUSTER_TRANSITION_TABLE {
        assert_eq!(row.len(), ClusterFsmEvent::ALL.len());
    }

    for state in NodeFsmState::ALL {
        for event in ClusterFsmEvent::ALL {
            let transition = node_transition(state, event);
            assert!(
                NodeFsmState::ALL.contains(&transition.next),
                "{state:?} + {event:?} produced unknown node state"
            );
        }
    }

    for phase in FormationPhase::ALL {
        for event in ClusterFsmEvent::ALL {
            let transition = cluster_transition(phase, event);
            assert!(
                FormationPhase::ALL.contains(&transition.next),
                "{phase:?} + {event:?} produced unknown cluster phase"
            );
        }
    }
}

#[test]
fn crash_then_rejoin_returns_through_defined_states() {
    let mut node = NodeFsm::new();
    let mut cluster = ClusterFsm::new();

    let sequence = [
        ClusterFsmEvent::Boot,
        ClusterFsmEvent::ElectionTimeout,
        ClusterFsmEvent::VoteQuorum,
        ClusterFsmEvent::Isolate,
        ClusterFsmEvent::Rejoin,
        ClusterFsmEvent::CatchUpComplete,
    ];

    let node_path = sequence
        .iter()
        .map(|event| node.apply(*event).next)
        .collect::<Vec<_>>();
    let cluster_path = sequence
        .iter()
        .map(|event| cluster.apply(*event).next)
        .collect::<Vec<_>>();

    assert_eq!(
        node_path,
        vec![
            NodeFsmState::Joining,
            NodeFsmState::Candidate,
            NodeFsmState::Leader,
            NodeFsmState::Disconnected,
            NodeFsmState::Joining,
            NodeFsmState::Follower,
        ]
    );
    assert_eq!(
        cluster_path,
        vec![
            FormationPhase::Bootstrapping,
            FormationPhase::Electing,
            FormationPhase::Formed,
            FormationPhase::Degraded,
            FormationPhase::CatchingUp,
            FormationPhase::Formed,
        ]
    );
}

#[test]
fn fsm_transition_sequence_is_reproducible() {
    let sequence = [
        ClusterFsmEvent::Boot,
        ClusterFsmEvent::ElectionTimeout,
        ClusterFsmEvent::ElectionTimeout,
        ClusterFsmEvent::VoteQuorum,
        ClusterFsmEvent::AddNode,
        ClusterFsmEvent::RebalanceComplete,
        ClusterFsmEvent::LeaderLost,
    ];

    let left = replay_cluster_sequence(sequence);
    let right = replay_cluster_sequence(sequence);

    assert_eq!(left, right);
    assert_eq!(
        left,
        vec![
            (
                FormationPhase::Bootstrapping,
                0,
                ClusterFsmAction::DiscoverPeers
            ),
            (FormationPhase::Electing, 1, ClusterFsmAction::StartElection),
            (FormationPhase::Electing, 2, ClusterFsmAction::StartElection),
            (FormationPhase::Formed, 2, ClusterFsmAction::BecomeLeader),
            (
                FormationPhase::Rebalancing,
                2,
                ClusterFsmAction::StartRebalance
            ),
            (FormationPhase::Formed, 2, ClusterFsmAction::FinishRebalance),
            (FormationPhase::Electing, 3, ClusterFsmAction::StartElection),
        ]
    );
}

fn replay_cluster_sequence(
    sequence: impl IntoIterator<Item = ClusterFsmEvent>,
) -> Vec<(FormationPhase, u64, ClusterFsmAction)> {
    let mut fsm = ClusterFsm::new();
    sequence
        .into_iter()
        .map(|event| {
            let transition = fsm.apply(event);
            (fsm.phase(), fsm.current_term(), transition.action)
        })
        .collect()
}
