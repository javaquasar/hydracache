use hydracache::{
    ClusterCandidate, ClusterGeneration, ClusterMembershipEvent, ClusterNodeId, InMemoryCluster,
};
use proptest::prelude::*;

#[derive(Debug, Clone)]
enum Step {
    Rejoin,
    PublishFromGeneration(u64),
}

fn step_strategy() -> impl Strategy<Value = Step> {
    prop_oneof![
        Just(Step::Rejoin),
        (0_u64..8).prop_map(Step::PublishFromGeneration),
    ]
}

proptest! {
    #[test]
    fn prop_stale_generation_never_publishes(steps in proptest::collection::vec(step_strategy(), 1..32)) {
        let cluster = InMemoryCluster::new("rejoin-property");
        let node_id = ClusterNodeId::from("member-a");
        let mut admitted_generation = ClusterGeneration::new(1);
        cluster
            .join_member(ClusterCandidate::member(node_id.clone()).generation(admitted_generation))
            .unwrap();

        for step in steps {
            match step {
                Step::Rejoin => {
                    admitted_generation = admitted_generation.next();
                    cluster
                        .join_member(ClusterCandidate::member(node_id.clone()).generation(admitted_generation))
                        .unwrap();
                }
                Step::PublishFromGeneration(raw) => {
                    let attempted = ClusterGeneration::new(raw);
                    let accepted = cluster.validate_generation(&node_id, attempted).is_ok();
                    prop_assert_eq!(accepted, attempted == admitted_generation);
                    if attempted < admitted_generation {
                        let events = cluster.events();
                        let has_rejection = events.iter().any(|event| matches!(
                            event,
                            ClusterMembershipEvent::StaleGenerationRejected {
                                attempted: event_attempted,
                                existing,
                                ..
                            } if *event_attempted == attempted && *existing == admitted_generation
                        ));
                        prop_assert_eq!(has_rejection, true);
                    }
                }
            }
        }
    }

    #[test]
    fn prop_rejoin_monotonically_advances_generation(rejoins in proptest::collection::vec(any::<bool>(), 1..32)) {
        let mut generation = ClusterGeneration::new(1);
        let mut observed = vec![generation.value()];

        for should_rejoin in rejoins {
            if should_rejoin {
                generation = generation.next();
                observed.push(generation.value());
            }
        }

        prop_assert!(observed.windows(2).all(|pair| pair[0] < pair[1]));
    }
}

#[test]
fn prop_stale_leave_rejected_targeted_case() {
    let cluster = InMemoryCluster::new("rejoin-property");
    let node_id = ClusterNodeId::from("member-a");
    cluster
        .join_member(
            ClusterCandidate::member(node_id.clone()).generation(ClusterGeneration::new(1)),
        )
        .unwrap();
    cluster
        .join_member(
            ClusterCandidate::member(node_id.clone()).generation(ClusterGeneration::new(2)),
        )
        .unwrap();

    let error = cluster
        .leave(&node_id, ClusterGeneration::new(1))
        .unwrap_err();

    assert!(error.to_string().contains("stale cluster generation"));
    assert!(cluster.events().iter().any(|event| matches!(
        event,
        ClusterMembershipEvent::StaleGenerationRejected {
            attempted,
            existing,
            ..
        } if *attempted == ClusterGeneration::new(1) && *existing == ClusterGeneration::new(2)
    )));
}

#[test]
fn diagnostics_show_generation_and_epoch_movement() {
    let cluster = InMemoryCluster::new("rejoin-property");
    let node_id = ClusterNodeId::from("member-a");
    cluster
        .join_member(
            ClusterCandidate::member(node_id.clone()).generation(ClusterGeneration::new(1)),
        )
        .unwrap();
    let first_epoch = cluster.epoch();
    cluster
        .join_member(ClusterCandidate::member(node_id).generation(ClusterGeneration::new(2)))
        .unwrap();

    let diagnostics = cluster.ownership_diagnostics();
    assert!(cluster.epoch() > first_epoch);
    assert_eq!(diagnostics.stamp, cluster.epoch().value());
}
