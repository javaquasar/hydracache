use std::collections::BTreeMap;

use hydracache::{
    ActiveActiveAcknowledgement, ActiveActiveConfig, ActiveActiveState, ClusterEndpoints,
    ClusterEpoch, ClusterGeneration, ClusterMember, ClusterNodeId, ClusterRole, GeoBatch,
    IdempotencyKey, NodeTopology, RegionId, RegionLink, ResidencyAuditAction,
    ResidencyFailoverDecision, ResidencyPolicy, ResidencyPolicyEnforcer, ResidencyPolicySet,
    ResidencyRejectionKind, ResidencyRemediationAction, ResidencyValueLocation,
    ZoneAwareReplicationStrategy, RESIDENCY_POLICY_FORMAT_VERSION,
};

fn member(id: &str) -> ClusterMember {
    ClusterMember {
        node_id: ClusterNodeId::from(id),
        generation: ClusterGeneration::default(),
        role: ClusterRole::Member,
        epoch: ClusterEpoch::new(1),
        endpoints: ClusterEndpoints::default(),
        metadata: BTreeMap::new(),
    }
}

fn members(ids: &[&str]) -> Vec<ClusterMember> {
    ids.iter().map(|id| member(id)).collect()
}

fn strategy(replication_factor: usize) -> ZoneAwareReplicationStrategy {
    let topology = BTreeMap::from([
        (ClusterNodeId::from("eu-a"), NodeTopology::new("eu", "az-a")),
        (ClusterNodeId::from("eu-b"), NodeTopology::new("eu", "az-b")),
        (ClusterNodeId::from("us-a"), NodeTopology::new("us", "az-a")),
        (ClusterNodeId::from("us-b"), NodeTopology::new("us", "az-b")),
    ]);
    ZoneAwareReplicationStrategy::new(topology, replication_factor, 2)
}

fn policy_set(allowed_regions: &[&str], min_replicas: usize, epoch: u64) -> ResidencyPolicySet {
    let mut policies = ResidencyPolicySet::new();
    let policy = ResidencyPolicy::new(
        allowed_regions.iter().copied().map(RegionId::from),
        min_replicas,
        ClusterEpoch::new(epoch),
    )
    .unwrap();
    policies
        .commit_namespace_policy("users", policy)
        .expect("commit policy");
    policies
}

fn write(key: &str, value: &[u8], wall: u64) -> hydracache::GeoWrite {
    let config = ActiveActiveConfig::active_active(
        "home",
        ActiveActiveAcknowledgement::BoundedStalenessAccepted,
    )
    .expect("ack");
    let mut state = ActiveActiveState::new(
        config,
        "eu",
        "node-eu",
        64,
        ClusterEpoch::new(3),
        vec![RegionId::from("us")],
    );
    state
        .accept_local_write(key, value.to_vec(), wall)
        .expect("write");
    state.drain_pending().pop().expect("pending")
}

mod residency {
    use super::*;

    #[test]
    fn pinned_value_is_not_placed_outside_allowed_regions() {
        let members = members(&["eu-a", "eu-b", "us-a", "us-b"]);
        let mut enforcer = ResidencyPolicyEnforcer::new(policy_set(&["eu"], 2, 7));

        let replicas = enforcer
            .place_key(&strategy(2), "users", "user:42", &members)
            .expect("placement inside policy");

        assert_eq!(replicas.replicas.copy_count(), 2);
        assert!(replicas
            .topology
            .values()
            .all(|topology| topology.region.as_str() == "eu"));
        assert_eq!(enforcer.metrics().residency_rejected_placement_total, 0);
    }

    #[test]
    fn pinned_value_is_refused_crossing_a_forbidden_link() {
        let mut enforcer = ResidencyPolicyEnforcer::new(policy_set(&["eu"], 1, 7));
        let batch = GeoBatch::new(
            "us",
            vec![write("user:42", b"ada", 10)],
            vec![IdempotencyKey::from("idem:user:42")],
        )
        .expect("batch");
        let mut link = RegionLink::new("us", 1, 1, 4);

        let report =
            link.try_send_with_residency(&batch, &mut enforcer, |_| Some("users".to_owned()));

        assert!(!report.sent);
        assert_eq!(report.checked, 1);
        assert_eq!(report.refused, 1);
        assert_eq!(link.bytes_total(), 0);
        assert_eq!(enforcer.metrics().residency_refused_crossing_total, 1);
    }

    #[test]
    fn unsatisfiable_rf_in_policy_rejects_put_loud() {
        let members = members(&["eu-a", "eu-b", "us-a", "us-b"]);
        let mut enforcer = ResidencyPolicyEnforcer::new(policy_set(&["eu"], 3, 7));

        let error = enforcer
            .place_key(&strategy(3), "users", "user:42", &members)
            .expect_err("rf cannot fit inside eu-only policy");

        assert_eq!(error.kind, ResidencyRejectionKind::RejectPlacement);
        assert!(error.reason.contains("replication factor 3"));
        assert_eq!(enforcer.metrics().residency_rejected_placement_total, 1);
    }

    #[test]
    fn forbidden_region_read_does_not_serve_stale_replica() {
        let mut enforcer = ResidencyPolicyEnforcer::new(policy_set(&["eu"], 1, 7));

        let error = enforcer
            .guard_read(
                "users",
                "user:42",
                &RegionId::from("us"),
                ClusterEpoch::new(7),
            )
            .expect_err("us read must not serve pinned value bytes");

        assert_eq!(error.kind, ResidencyRejectionKind::RejectRead);
        assert_eq!(error.policy_epoch, ClusterEpoch::new(7));
        assert_eq!(enforcer.metrics().residency_rejected_read_total, 1);
    }

    #[test]
    fn policy_epoch_is_enforced_and_reported() {
        let mut enforcer = ResidencyPolicyEnforcer::new(policy_set(&["eu"], 1, 7));

        let error = enforcer
            .guard_read(
                "users",
                "user:42",
                &RegionId::from("eu"),
                ClusterEpoch::new(6),
            )
            .expect_err("stale policy epoch must fail loud");

        assert_eq!(error.kind, ResidencyRejectionKind::StalePolicyEpoch);
        assert_eq!(error.policy_epoch, ClusterEpoch::new(7));
        assert!(error.reason.contains("older than current epoch 7"));
    }

    #[test]
    fn policy_narrowing_evicts_or_marks_existing_out_of_policy_data() {
        let mut enforcer = ResidencyPolicyEnforcer::new(policy_set(&["eu"], 1, 8));

        let report = enforcer.plan_policy_narrowing(vec![
            ResidencyValueLocation::new("users", "user:42", "eu"),
            ResidencyValueLocation::new("users", "user:42", "us"),
        ]);

        assert!(report.has_remediation());
        assert!(report.actions.iter().any(|action| matches!(
            action,
            ResidencyRemediationAction::Evict { region, .. } if region.as_str() == "us"
        )));
        assert!(report.actions.iter().any(|action| matches!(
            action,
            ResidencyRemediationAction::Keep { region, .. } if region.as_str() == "eu"
        )));
        assert_eq!(
            enforcer
                .metrics()
                .residency_policy_narrowing_out_of_policy_total,
            1
        );
    }

    #[test]
    #[ignore = "chaos gate: run with -- --ignored for region failover residency governance"]
    fn residency_holds_under_region_failover() {
        let enforcer = ResidencyPolicyEnforcer::new(policy_set(&["eu"], 1, 8));

        let decision =
            enforcer.choose_failover_home("users", "user:42", vec![RegionId::from("us")]);

        assert!(matches!(
            decision,
            ResidencyFailoverDecision::Degraded { .. }
        ));
    }

    #[test]
    fn residency_violation_is_audited() {
        let mut enforcer = ResidencyPolicyEnforcer::new(policy_set(&["eu"], 1, 7));

        let _ = enforcer.guard_cross_boundary(
            "users",
            "user:42",
            &RegionId::from("eu"),
            &RegionId::from("us"),
        );

        assert_eq!(enforcer.audit_events().len(), 1);
        let event = &enforcer.audit_events()[0];
        assert_eq!(event.action, ResidencyAuditAction::RefuseCrossBoundary);
        assert_eq!(event.namespace, "users");
        assert_eq!(event.key, "user:42");
        assert_eq!(event.policy_epoch, ClusterEpoch::new(7));
    }

    #[test]
    fn residency_include_value_gate_blocks_forbidden_region_bytes() {
        let enforcer = ResidencyPolicyEnforcer::new(policy_set(&["eu"], 1, 7));

        assert!(enforcer.include_value_allowed("users", "user:42", &RegionId::from("eu")));
        assert!(!enforcer.include_value_allowed("users", "user:42", &RegionId::from("us")));
    }

    #[test]
    fn residency_policy_rejects_unknown_future_format() {
        let mut policies = ResidencyPolicySet::new();
        let future_policy =
            ResidencyPolicy::new(vec![RegionId::from("eu")], 1, ClusterEpoch::new(1))
                .unwrap()
                .with_format_version(RESIDENCY_POLICY_FORMAT_VERSION + 1);

        let error = policies
            .commit_namespace_policy("users", future_policy)
            .expect_err("future policy format must fail closed");

        assert!(error.to_string().contains("newer than supported"));
    }
}
