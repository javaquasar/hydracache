use hydracache_operator::crd::{sample_spec, HydraCacheCluster};
use hydracache_operator::scale::{AdminAction, AdminStatus};
use hydracache_operator::upgrade::{
    expected_pod_name, plan_upgrade, version_skew_supported, PodObservation, UpgradeObservation,
    UPGRADE_BLOCKED_CONDITION, UPGRADE_FAILED_CONDITION, UPGRADE_PROGRESSING_CONDITION,
    UPGRADE_STEP_TIMEOUT_SECS, UPGRADING_PHASE,
};

fn cluster(name: &str, image: &str, version: &str) -> HydraCacheCluster {
    let mut spec = sample_spec();
    spec.image = image.to_owned();
    spec.version = version.to_owned();
    spec.replicas = 3;
    let mut cluster = HydraCacheCluster::new(name, spec);
    cluster.metadata.namespace = Some("default".to_owned());
    cluster.metadata.uid = Some(format!("{name}-uid"));
    cluster.metadata.generation = Some(9);
    cluster
}

fn admin_status(leader: &str) -> AdminStatus {
    AdminStatus {
        leader: Some(leader.to_owned()),
        quorum_ok: true,
        members: 3,
        voters: 3,
        reshard_phase: "idle".to_owned(),
        draining: false,
    }
}

fn pod(
    cluster_name: &str,
    ordinal: u32,
    image: &str,
    version: &str,
    ready: bool,
) -> PodObservation {
    PodObservation {
        name: expected_pod_name(cluster_name, ordinal),
        ordinal,
        image: Some(image.to_owned()),
        version: Some(version.to_owned()),
        ready,
        deleting: false,
        not_ready_for_seconds: None,
    }
}

fn observed(pods: Vec<PodObservation>, leader: &str) -> UpgradeObservation {
    UpgradeObservation {
        current_replicas: pods.len() as u32,
        ready_replicas: pods.iter().filter(|pod| pod.ready).count() as u32,
        previous_phase: None,
        admin_status: Some(admin_status(leader)),
        pods,
    }
}

#[tokio::test]
async fn rolling_upgrade_keeps_a_leader_and_serves_reads() {
    if std::env::var("HYDRACACHE_OPERATOR_KIND").as_deref() != Ok("1") {
        eprintln!(
            "skipping kind rolling upgrade probe: set HYDRACACHE_OPERATOR_KIND=1 with a kind cluster"
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
        "kind fixture should contain operator-managed HydraCache pods for the zero-downtime probe"
    );
}

#[test]
fn only_one_pod_is_down_at_a_time() {
    let cluster = cluster("upgrade-one", "repo/hydracache:0.56.0", "0.56.0");
    let pods = vec![
        pod("upgrade-one", 0, "repo/hydracache:0.56.0", "0.56.0", true),
        pod("upgrade-one", 1, "repo/hydracache:0.55.0", "0.55.0", false),
        pod("upgrade-one", 2, "repo/hydracache:0.55.0", "0.55.0", true),
    ];

    let plan = plan_upgrade(&cluster, &observed(pods, "upgrade-one-0"));

    assert_eq!(plan.phase, UPGRADING_PHASE);
    assert_eq!(plan.conditions[0].type_, UPGRADE_PROGRESSING_CONDITION);
    assert_eq!(plan.conditions[0].reason, "WaitingForReadyPod");
    assert!(plan.admin_actions.is_empty());
    assert!(plan.delete_pod.is_none());
}

#[test]
fn leader_pod_is_drained_after_reelection_not_before() {
    let cluster = cluster("upgrade-leader", "repo/hydracache:0.56.0", "0.56.0");
    let pods = vec![
        pod(
            "upgrade-leader",
            0,
            "repo/hydracache:0.56.0",
            "0.56.0",
            true,
        ),
        pod(
            "upgrade-leader",
            1,
            "repo/hydracache:0.56.0",
            "0.56.0",
            true,
        ),
        pod(
            "upgrade-leader",
            2,
            "repo/hydracache:0.55.0",
            "0.55.0",
            true,
        ),
    ];

    let before_reelection = plan_upgrade(&cluster, &observed(pods.clone(), "upgrade-leader-2"));
    assert_eq!(
        before_reelection.admin_actions,
        vec![AdminAction::Drain { ordinal: 2 }]
    );
    assert_eq!(
        before_reelection.conditions[0].reason,
        "LeaderReelectionRequested"
    );
    assert!(before_reelection.delete_pod.is_none());

    let after_reelection = plan_upgrade(&cluster, &observed(pods, "upgrade-leader-0"));
    assert_eq!(
        after_reelection.admin_actions,
        vec![AdminAction::Drain { ordinal: 2 }]
    );
    assert_eq!(
        after_reelection.delete_pod.as_deref(),
        Some("upgrade-leader-2")
    );
}

#[test]
fn failed_pod_halts_the_rollout_loud() {
    let cluster = cluster("upgrade-fail", "repo/hydracache:0.56.0", "0.56.0");
    let mut failing = pod("upgrade-fail", 1, "repo/hydracache:0.56.0", "0.56.0", false);
    failing.not_ready_for_seconds = Some(UPGRADE_STEP_TIMEOUT_SECS);
    let pods = vec![
        pod("upgrade-fail", 0, "repo/hydracache:0.56.0", "0.56.0", true),
        failing,
        pod("upgrade-fail", 2, "repo/hydracache:0.55.0", "0.55.0", true),
    ];

    let plan = plan_upgrade(&cluster, &observed(pods, "upgrade-fail-0"));

    assert_eq!(plan.phase, UPGRADING_PHASE);
    assert_eq!(plan.conditions[0].type_, UPGRADE_FAILED_CONDITION);
    assert_eq!(plan.conditions[0].reason, "UpgradePodNotReadyTimeout");
    assert!(plan.admin_actions.is_empty());
    assert!(plan.delete_pod.is_none());
}

#[test]
fn mixed_version_skew_stays_within_the_supported_window() {
    assert!(version_skew_supported("0.55.0", "0.56.0"));
    assert!(version_skew_supported("v0.56.0", "0.55.0"));
    assert!(!version_skew_supported("0.54.0", "0.56.0"));
    assert!(!version_skew_supported("1.55.0", "0.56.0"));

    let cluster = cluster("upgrade-skew", "repo/hydracache:0.56.0", "0.56.0");
    let pods = vec![
        pod("upgrade-skew", 0, "repo/hydracache:0.54.0", "0.54.0", true),
        pod("upgrade-skew", 1, "repo/hydracache:0.54.0", "0.54.0", true),
        pod("upgrade-skew", 2, "repo/hydracache:0.54.0", "0.54.0", true),
    ];
    let plan = plan_upgrade(&cluster, &observed(pods, "upgrade-skew-0"));

    assert_eq!(plan.conditions[0].type_, UPGRADE_BLOCKED_CONDITION);
    assert_eq!(plan.conditions[0].reason, "UnsupportedVersionSkew");
    assert!(plan.admin_actions.is_empty());
    assert!(plan.delete_pod.is_none());
}

#[test]
fn version_revert_rolls_back() {
    let cluster = cluster("upgrade-rollback", "repo/hydracache:0.55.0", "0.55.0");
    let pods = vec![
        pod(
            "upgrade-rollback",
            0,
            "repo/hydracache:0.56.0",
            "0.56.0",
            true,
        ),
        pod(
            "upgrade-rollback",
            1,
            "repo/hydracache:0.56.0",
            "0.56.0",
            true,
        ),
        pod(
            "upgrade-rollback",
            2,
            "repo/hydracache:0.56.0",
            "0.56.0",
            true,
        ),
    ];

    let plan = plan_upgrade(&cluster, &observed(pods, "upgrade-rollback-0"));

    assert_eq!(plan.phase, UPGRADING_PHASE);
    assert_eq!(plan.conditions[0].reason, "PodDrainAndReplace");
    assert_eq!(plan.admin_actions, vec![AdminAction::Drain { ordinal: 2 }]);
    assert_eq!(plan.delete_pod.as_deref(), Some("upgrade-rollback-2"));
}
