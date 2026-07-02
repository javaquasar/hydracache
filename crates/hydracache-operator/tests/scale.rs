use hydracache_operator::crd::{sample_spec, HydraCacheCluster};
use hydracache_operator::resources::OwnedResources;
use hydracache_operator::scale::{
    admin_base_url, plan_scale, pod_name, quorum_for, AdminAction, AdminStatus, ScaleObservation,
    REBALANCING_PHASE, SCALE_BLOCKED_CONDITION, SCALE_PROGRESSING_CONDITION, SCALING_PHASE,
};
use k8s_openapi::api::apps::v1::StatefulSetStatus;
use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;

fn cluster(name: &str, replicas: u32) -> HydraCacheCluster {
    let mut spec = sample_spec();
    spec.replicas = replicas;
    let mut cluster = HydraCacheCluster::new(name, spec);
    cluster.metadata.namespace = Some("default".to_owned());
    cluster.metadata.uid = Some(format!("{name}-uid"));
    cluster.metadata.generation = Some(8);
    cluster
}

fn observed(cluster: &HydraCacheCluster, current: u32, ready: u32) -> ScaleObservation {
    let mut owned = OwnedResources::build_with_replicas(cluster, current);
    owned.stateful_set.status = Some(StatefulSetStatus {
        ready_replicas: Some(ready as i32),
        replicas: current as i32,
        ..Default::default()
    });
    ScaleObservation::from_statefulset(cluster, Some(&owned.stateful_set))
}

fn admin_status(leader: Option<String>, reshard_phase: &str) -> AdminStatus {
    AdminStatus {
        leader,
        quorum_ok: true,
        members: 3,
        reshard_phase: reshard_phase.to_owned(),
        draining: false,
    }
}

#[test]
fn scale_up_reshards_onto_new_node_no_loss() {
    let target = cluster("scale-up", 4);
    let creating = plan_scale(&target, &observed(&target, 3, 3));

    assert_eq!(creating.effective_replicas, 4);
    assert_eq!(creating.phase, SCALING_PHASE);
    assert!(creating.admin_actions.is_empty());
    assert_eq!(creating.conditions[0].reason, "ScaleUpCreatingPods");

    let mut ready = observed(&target, 4, 4);
    ready.admin_status = Some(admin_status(None, "idle"));
    let rebalance = plan_scale(&target, &ready);

    assert_eq!(rebalance.effective_replicas, 4);
    assert_eq!(rebalance.phase, REBALANCING_PHASE);
    assert_eq!(
        rebalance.admin_actions,
        vec![AdminAction::Reshard { ordinal: 0 }]
    );
    assert_eq!(rebalance.conditions[0].reason, "ScaleUpReshardRequested");
}

#[test]
fn scale_down_drains_before_removing() {
    let target = cluster("scale-down", 2);
    let plan = plan_scale(&target, &observed(&target, 3, 3));

    assert_eq!(plan.effective_replicas, 3);
    assert_eq!(plan.phase, SCALING_PHASE);
    assert_eq!(
        plan.admin_actions,
        vec![
            AdminAction::Reshard { ordinal: 2 },
            AdminAction::Drain { ordinal: 2 }
        ]
    );
    assert_eq!(plan.conditions[0].type_, SCALE_PROGRESSING_CONDITION);
    assert_eq!(plan.conditions[0].reason, "DrainBeforeRemove");
    assert!(plan.conditions[0].message.contains("scale-down-2"));

    let mut after_drain = observed(&target, 3, 3);
    after_drain.drain_complete_for = Some("scale-down-2".to_owned());
    let removal = plan_scale(&target, &after_drain);
    assert_eq!(removal.effective_replicas, 2);
    assert!(removal.admin_actions.is_empty());
}

#[test]
fn scale_down_of_leader_reelects_first() {
    let target = cluster("leader-down", 2);
    let mut observation = observed(&target, 3, 3);
    observation.admin_status = Some(admin_status(Some("leader-down-2".to_owned()), "idle"));

    let plan = plan_scale(&target, &observation);

    assert_eq!(plan.effective_replicas, 3);
    assert_eq!(plan.conditions[0].type_, SCALE_BLOCKED_CONDITION);
    assert_eq!(plan.conditions[0].reason, "LeaderDrainDeferred");
    assert!(plan.admin_actions.is_empty());
}

#[test]
fn scale_below_quorum_is_refused_loud() {
    let target = cluster("below-quorum", 1);
    let plan = plan_scale(&target, &observed(&target, 3, 3));

    assert_eq!(quorum_for(3), 2);
    assert_eq!(plan.effective_replicas, 3);
    assert_eq!(plan.conditions[0].type_, SCALE_BLOCKED_CONDITION);
    assert_eq!(plan.conditions[0].reason, "ScaleBelowQuorumRefused");
    assert!(plan.conditions[0].message.contains("at least 2 members"));
    assert!(plan.admin_actions.is_empty());
}

#[test]
fn scale_pod_disruption_budget_preserves_quorum_under_node_drain() {
    let pdb_cluster = cluster("pdb", 3);
    let owned = OwnedResources::build(&pdb_cluster);
    let pdb = owned.pod_disruption_budget;
    let spec = pdb.spec.as_ref().unwrap();

    assert_eq!(spec.min_available, Some(IntOrString::Int(2)));
    assert_eq!(
        spec.selector
            .as_ref()
            .unwrap()
            .match_labels
            .as_ref()
            .unwrap()["app.kubernetes.io/instance"],
        "pdb"
    );

    let one = OwnedResources::build(&cluster("pdb-one", 1));
    assert_eq!(
        one.pod_disruption_budget.spec.unwrap().min_available,
        Some(IntOrString::Int(1))
    );
    let two = OwnedResources::build(&cluster("pdb-two", 2));
    assert_eq!(
        two.pod_disruption_budget.spec.unwrap().min_available,
        Some(IntOrString::Int(2))
    );
}

#[test]
fn scale_crash_during_reshard_resumes_or_stays_consistent() {
    let target = cluster("resume", 4);
    let mut observation = observed(&target, 4, 4);
    observation.previous_phase = Some(REBALANCING_PHASE.to_owned());
    observation.admin_status = Some(admin_status(None, "running"));

    let plan = plan_scale(&target, &observation);

    assert_eq!(plan.effective_replicas, 4);
    assert_eq!(plan.phase, REBALANCING_PHASE);
    assert_eq!(
        plan.admin_actions,
        vec![AdminAction::Reshard { ordinal: 0 }]
    );

    observation.admin_status = Some(admin_status(None, "idle"));
    let complete = plan_scale(&target, &observation);
    assert_eq!(complete.effective_replicas, 4);
    assert_eq!(complete.phase, hydracache_operator::controller::READY_PHASE);
    assert!(complete.admin_actions.is_empty());
}

#[test]
fn scale_admin_urls_are_cluster_scoped_to_target_ordinal() {
    assert_eq!(pod_name("demo", 2), "demo-2");
    assert_eq!(
        admin_base_url("ns-a", "demo", 2),
        "http://demo-2.demo-headless.ns-a.svc:9091"
    );
}

#[tokio::test]
async fn scale_kind_lifecycle_smoke_skips_without_kind() {
    if std::env::var("HYDRACACHE_OPERATOR_KIND").as_deref() != Ok("1") {
        eprintln!(
            "skipping kind scale lifecycle smoke: set HYDRACACHE_OPERATOR_KIND=1 with a kind cluster"
        );
        return;
    }

    let client = kube::Client::try_default()
        .await
        .expect("HYDRACACHE_OPERATOR_KIND=1 requires kube config");
    let pods: kube::Api<k8s_openapi::api::core::v1::Pod> = kube::Api::namespaced(client, "default");
    let listed = pods
        .list(&kube::api::ListParams::default().labels(
            "app.kubernetes.io/name=hydracache,app.kubernetes.io/managed-by=hydracache-operator",
        ))
        .await
        .expect("kind cluster should list operator-managed HydraCache pods");
    assert!(
        !listed.items.is_empty(),
        "kind fixture should contain operator-managed HydraCache pods"
    );
}
