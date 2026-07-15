use std::collections::BTreeMap;

use hydracache_operator::controller::READY_PHASE;
use hydracache_operator::crd::{sample_spec, HydraCacheCluster, HydraCacheClusterStatus};
use hydracache_operator::resources::OwnedResources;
use hydracache_operator::scale::{
    plan_scale, quorum_for, scale_condition, AdminStatus, ScaleObservation,
    SCALE_PROGRESSING_CONDITION,
};
use hydracache_operator::tls::{
    plan_tls_rotation, plan_tls_secret, TlsPodObservation, TlsRotationObservation,
    TlsSecretObservation, TLS_ROTATION_BLOCKED_CONDITION, TLS_ROTATION_PROGRESSING_CONDITION,
    TLS_SECRET_FINGERPRINT_ANNOTATION,
};
use hydracache_operator::upgrade::{
    plan_upgrade, version_skew_supported, PodObservation, UpgradeObservation, VERSION_ANNOTATION,
};
use k8s_openapi::api::apps::v1::StatefulSetStatus;
use k8s_openapi::api::core::v1::{Container, Pod, PodCondition, PodSpec, PodStatus, Secret};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;

fn cluster(name: &str) -> HydraCacheCluster {
    let mut cluster = HydraCacheCluster::new(name, sample_spec());
    cluster.metadata.namespace = Some("default".to_owned());
    cluster.metadata.uid = Some(format!("{name}-uid"));
    cluster.metadata.generation = Some(12);
    cluster
}

fn admin_status(leader: Option<&str>, quorum_ok: bool) -> AdminStatus {
    AdminStatus {
        leader: leader.map(str::to_owned),
        quorum_ok,
        members: 3,
        voters: 3,
        reshard_phase: "idle".to_owned(),
        draining: false,
    }
}

fn runtime_pod(cluster_name: &str, ordinal: u32, ready: bool) -> Pod {
    Pod {
        metadata: ObjectMeta {
            name: Some(format!("{cluster_name}-{ordinal}")),
            annotations: Some(BTreeMap::from([
                (VERSION_ANNOTATION.to_owned(), "0.56.0".to_owned()),
                (
                    TLS_SECRET_FINGERPRINT_ANNOTATION.to_owned(),
                    "fingerprint-a".to_owned(),
                ),
            ])),
            ..Default::default()
        },
        spec: Some(PodSpec {
            containers: vec![Container {
                name: "hydracache".to_owned(),
                image: Some("repo/hydracache:0.56.0".to_owned()),
                ..Default::default()
            }],
            ..Default::default()
        }),
        status: Some(PodStatus {
            conditions: Some(vec![PodCondition {
                status: if ready { "True" } else { "False" }.to_owned(),
                type_: "Ready".to_owned(),
                ..Default::default()
            }]),
            ..Default::default()
        }),
    }
}

#[test]
fn scale_observation_recovers_resumable_drain_markers_from_kubernetes_state() {
    let mut cluster = cluster("scale-observation");
    cluster.status = Some(HydraCacheClusterStatus {
        phase: "Scaling".to_owned(),
        conditions: vec![
            scale_condition(
                SCALE_PROGRESSING_CONDITION,
                "True",
                "DrainRequested",
                "drain requested for scale-observation-2",
                cluster.metadata.generation,
            ),
            scale_condition(
                SCALE_PROGRESSING_CONDITION,
                "True",
                "DrainComplete",
                "drain complete for scale-observation-1",
                cluster.metadata.generation,
            ),
        ],
        ..Default::default()
    });
    let mut stateful_set = OwnedResources::build(&cluster).stateful_set;
    stateful_set.status = Some(StatefulSetStatus {
        replicas: 3,
        ready_replicas: None,
        ..Default::default()
    });

    let observed = ScaleObservation::from_statefulset(&cluster, Some(&stateful_set));

    assert_eq!(observed.current_replicas, 3);
    assert_eq!(observed.ready_replicas, 3);
    assert_eq!(observed.previous_phase.as_deref(), Some("Scaling"));
    assert_eq!(
        observed.drain_requested_for.as_deref(),
        Some("scale-observation-2")
    );
    assert_eq!(
        observed.drain_complete_for.as_deref(),
        Some("scale-observation-1")
    );
}

#[test]
fn scale_down_waits_for_current_replicas_and_zero_has_no_quorum() {
    let mut cluster = cluster("scale-wait");
    cluster.spec.replicas = 2;
    let plan = plan_scale(
        &cluster,
        &ScaleObservation {
            current_replicas: 3,
            ready_replicas: 2,
            previous_phase: Some(READY_PHASE.to_owned()),
            drain_requested_for: None,
            drain_complete_for: None,
            admin_status: Some(admin_status(Some("scale-wait-0"), true)),
        },
    );

    assert_eq!(plan.conditions[0].reason, "WaitingForCurrentReplicas");
    assert!(plan.admin_actions.is_empty());
    assert_eq!(quorum_for(0), 0);
}

#[test]
fn pod_observations_parse_image_version_readiness_and_tls_fingerprint() {
    let pod = runtime_pod("observed", 2, true);
    let upgrade = PodObservation::from_pod("observed", &pod).unwrap();
    let tls = TlsPodObservation::from_pod("observed", &pod).unwrap();

    assert_eq!(upgrade.name, "observed-2");
    assert_eq!(upgrade.ordinal, 2);
    assert!(upgrade.ready);
    assert!(upgrade.is_current("repo/hydracache:0.56.0", "0.56.0"));
    assert_eq!(tls.ordinal, 2);
    assert!(tls.ready);
    assert_eq!(tls.tls_fingerprint.as_deref(), Some("fingerprint-a"));

    let mut unrelated = pod;
    unrelated.metadata.name = Some("another-cluster-0".to_owned());
    assert!(PodObservation::from_pod("observed", &unrelated).is_none());
    assert!(TlsPodObservation::from_pod("observed", &unrelated).is_none());
}

#[test]
fn tls_secret_string_data_and_incomplete_material_are_distinguished() {
    let complete = Secret {
        metadata: ObjectMeta {
            name: Some("tls".to_owned()),
            ..Default::default()
        },
        string_data: Some(BTreeMap::from([
            ("tls.crt".to_owned(), "cert".to_owned()),
            ("tls.key".to_owned(), "key".to_owned()),
            ("ca.crt".to_owned(), "ca".to_owned()),
        ])),
        ..Default::default()
    };
    let complete = TlsSecretObservation::from_secret("tls", Some(&complete));
    assert!(complete.exists);
    assert!(complete.missing_keys.is_empty());
    assert!(complete.fingerprint.is_some());

    let incomplete = Secret {
        string_data: Some(BTreeMap::from([("tls.crt".to_owned(), "cert".to_owned())])),
        ..Default::default()
    };
    let incomplete = TlsSecretObservation::from_secret("tls", Some(&incomplete));
    let plan = plan_tls_secret(&cluster("tls-incomplete"), &incomplete);
    assert!(plan.blocked);
    assert_eq!(plan.conditions[0].reason, "TlsSecretIncomplete");
    assert!(plan.conditions[0].message.contains("tls.key"));
    assert!(plan.conditions[0].message.contains("ca.crt"));

    let mut disabled = cluster("tls-disabled");
    disabled.spec.tls = None;
    assert!(!plan_tls_secret(&disabled, &TlsSecretObservation::disabled()).blocked);
}

#[test]
fn tls_rotation_blocks_each_unsafe_precondition_and_stays_steady_when_current() {
    let cluster = cluster("tls-edges");
    let secret = TlsSecretObservation {
        name: Some("hydracache-mtls".to_owned()),
        exists: true,
        fingerprint: Some("fingerprint-a".to_owned()),
        missing_keys: Vec::new(),
    };
    let pod = TlsPodObservation::from_pod("tls-edges", &runtime_pod("tls-edges", 0, true)).unwrap();
    let base = |admin_status, ready_replicas, pods| TlsRotationObservation {
        current_replicas: 1,
        ready_replicas,
        admin_status,
        secret: secret.clone(),
        pods,
    };

    let missing_admin = plan_tls_rotation(&cluster, &base(None, 1, vec![pod.clone()]));
    assert_eq!(
        missing_admin.conditions[0].type_,
        TLS_ROTATION_BLOCKED_CONDITION
    );
    assert_eq!(missing_admin.conditions[0].reason, "WaitingForAdminStatus");

    let no_quorum = plan_tls_rotation(
        &cluster,
        &base(Some(admin_status(None, false)), 1, vec![pod.clone()]),
    );
    assert_eq!(
        no_quorum.conditions[0].reason,
        "TlsRotationQuorumUnavailable"
    );

    let mut deleting = pod.clone();
    deleting.deleting = true;
    let waiting = plan_tls_rotation(
        &cluster,
        &base(Some(admin_status(None, true)), 1, vec![deleting]),
    );
    assert_eq!(
        waiting.conditions[0].type_,
        TLS_ROTATION_PROGRESSING_CONDITION
    );
    assert_eq!(waiting.conditions[0].reason, "WaitingForReadyPod");

    let steady = plan_tls_rotation(
        &cluster,
        &base(Some(admin_status(None, true)), 1, vec![pod]),
    );
    assert_eq!(steady.phase, READY_PHASE);
    assert!(steady.conditions.is_empty());
}

#[test]
fn upgrade_observation_blocks_without_admin_and_parses_invalid_versions_loudly() {
    let cluster = cluster("upgrade-edges");
    let pod =
        PodObservation::from_pod("upgrade-edges", &runtime_pod("upgrade-edges", 0, true)).unwrap();
    let plan = plan_upgrade(
        &cluster,
        &UpgradeObservation {
            current_replicas: 1,
            ready_replicas: 1,
            previous_phase: None,
            admin_status: None,
            pods: vec![pod],
        },
    );
    assert_eq!(plan.conditions[0].reason, "WaitingForAdminStatus");
    assert!(!version_skew_supported("not-a-version", "0.56.0"));
    assert!(!version_skew_supported("0.56.0", "also-invalid"));

    let empty = plan_upgrade(
        &cluster,
        &UpgradeObservation {
            current_replicas: 0,
            ready_replicas: 0,
            previous_phase: None,
            admin_status: None,
            pods: Vec::new(),
        },
    );
    assert_eq!(empty.phase, READY_PHASE);
}
