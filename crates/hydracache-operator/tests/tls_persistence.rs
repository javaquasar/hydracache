use std::collections::BTreeMap;

use hydracache_operator::crd::{sample_spec, HydraCacheCluster};
use hydracache_operator::persistence::{plan_persistence, PERSISTENCE_BLOCKED_CONDITION};
use hydracache_operator::resources::{cleanup_plan, OwnedResources, DATA_VOLUME};
use hydracache_operator::scale::{AdminAction, AdminStatus};
use hydracache_operator::tls::{
    expected_pod_name, plan_tls_rotation, plan_tls_secret, TlsPodObservation,
    TlsRotationObservation, TlsSecretObservation, TLS_ROTATION_BLOCKED_CONDITION,
    TLS_ROTATION_PROGRESSING_CONDITION, TLS_SECRET_FINGERPRINT_ANNOTATION,
    TLS_SECRET_NAME_ANNOTATION,
};
use k8s_openapi::api::core::v1::Secret;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use k8s_openapi::ByteString;

fn cluster(name: &str) -> HydraCacheCluster {
    let mut cluster = HydraCacheCluster::new(name, sample_spec());
    cluster.metadata.namespace = Some("default".to_owned());
    cluster.metadata.uid = Some(format!("{name}-uid"));
    cluster.metadata.generation = Some(10);
    cluster
}

fn tls_secret(name: &str, generation: &str) -> Secret {
    Secret {
        metadata: ObjectMeta {
            name: Some(name.to_owned()),
            ..Default::default()
        },
        data: Some(BTreeMap::from([
            (
                "tls.crt".to_owned(),
                ByteString(format!("cert-{generation}").into_bytes()),
            ),
            (
                "tls.key".to_owned(),
                ByteString(format!("key-{generation}").into_bytes()),
            ),
            (
                "ca.crt".to_owned(),
                ByteString(format!("ca-{generation}").into_bytes()),
            ),
        ])),
        ..Default::default()
    }
}

fn admin_status(leader: &str) -> AdminStatus {
    AdminStatus {
        leader: Some(leader.to_owned()),
        quorum_ok: true,
        members: 3,
        reshard_phase: "idle".to_owned(),
        draining: false,
    }
}

fn tls_pod(
    cluster_name: &str,
    ordinal: u32,
    fingerprint: Option<&str>,
    ready: bool,
) -> TlsPodObservation {
    TlsPodObservation {
        name: expected_pod_name(cluster_name, ordinal),
        ordinal,
        tls_fingerprint: fingerprint.map(str::to_owned),
        ready,
        deleting: false,
    }
}

fn observed(
    secret: TlsSecretObservation,
    pods: Vec<TlsPodObservation>,
    leader: &str,
) -> TlsRotationObservation {
    TlsRotationObservation {
        current_replicas: pods.len() as u32,
        ready_replicas: pods.iter().filter(|pod| pod.ready).count() as u32,
        admin_status: Some(admin_status(leader)),
        secret,
        pods,
    }
}

#[tokio::test]
async fn cert_rotation_does_not_break_live_mtls_connections() {
    if std::env::var("HYDRACACHE_OPERATOR_KIND").as_deref() != Ok("1") {
        eprintln!(
            "skipping kind mTLS rotation probe: set HYDRACACHE_OPERATOR_KIND=1 with a kind cluster"
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
        "kind fixture should contain operator-managed HydraCache pods for the mTLS probe"
    );
}

#[test]
fn persistent_namespace_survives_pod_reschedule_on_its_pvc() {
    let cluster = cluster("persistent");
    let cleanup = cleanup_plan(&cluster);
    let desired = OwnedResources::build(&cluster);
    let spec = desired.stateful_set.spec.as_ref().unwrap();
    let claims = spec.volume_claim_templates.as_ref().unwrap();
    let claim = &claims[0];
    let retention = spec
        .persistent_volume_claim_retention_policy
        .as_ref()
        .unwrap();

    assert!(!cleanup.delete_pvcs);
    assert_eq!(claim.metadata.name.as_deref(), Some(DATA_VOLUME));
    assert_eq!(
        claim.spec.as_ref().unwrap().storage_class_name.as_deref(),
        Some("standard")
    );
    assert_eq!(
        claim
            .spec
            .as_ref()
            .unwrap()
            .resources
            .as_ref()
            .unwrap()
            .requests
            .as_ref()
            .unwrap()["storage"]
            .0,
        "20Gi"
    );
    assert_eq!(retention.when_deleted.as_deref(), Some("Retain"));
    assert_eq!(retention.when_scaled.as_deref(), Some("Retain"));

    let volumes = spec
        .template
        .spec
        .as_ref()
        .unwrap()
        .volumes
        .as_ref()
        .unwrap();
    assert!(!volumes
        .iter()
        .any(|volume| volume.name == DATA_VOLUME && volume.empty_dir.is_some()));
}

#[test]
fn ram_only_namespace_uses_no_pvc() {
    let mut cluster = cluster("ram-only");
    cluster.spec.persistence = None;
    cluster.spec.tls = None;
    let desired = OwnedResources::build(&cluster);
    let spec = desired.stateful_set.spec.as_ref().unwrap();

    assert!(spec.volume_claim_templates.is_none());
    let pod_spec = spec.template.spec.as_ref().unwrap();
    assert!(pod_spec
        .volumes
        .as_ref()
        .unwrap()
        .iter()
        .any(|volume| volume.name == DATA_VOLUME && volume.empty_dir.is_some()));
    assert!(pod_spec.containers[0]
        .volume_mounts
        .as_ref()
        .unwrap()
        .iter()
        .any(|mount| mount.name == DATA_VOLUME));
}

#[test]
fn persistence_without_storage_class_is_refused_loud() {
    let mut cluster = cluster("missing-storage-class");
    cluster
        .spec
        .persistence
        .as_mut()
        .unwrap()
        .storage_class_name = "   ".to_owned();

    let plan = plan_persistence(&cluster);

    assert!(plan.blocked);
    assert_eq!(plan.conditions[0].type_, PERSISTENCE_BLOCKED_CONDITION);
    assert_eq!(plan.conditions[0].reason, "MissingStorageClass");
    assert_eq!(plan.conditions[0].observed_generation, Some(10));
}

#[test]
fn tls_secret_change_rotates_one_pod_at_a_time() {
    let cluster = cluster("tls-rotate");
    let old_secret = tls_secret("hydracache-mtls", "old");
    let new_secret = tls_secret("hydracache-mtls", "new");
    let old_fingerprint = TlsSecretObservation::from_secret("hydracache-mtls", Some(&old_secret))
        .fingerprint
        .unwrap();
    let new_observation = TlsSecretObservation::from_secret("hydracache-mtls", Some(&new_secret));
    let pods = vec![
        tls_pod("tls-rotate", 0, Some(&old_fingerprint), true),
        tls_pod("tls-rotate", 1, Some(&old_fingerprint), true),
        tls_pod("tls-rotate", 2, Some(&old_fingerprint), true),
    ];

    let plan = plan_tls_rotation(&cluster, &observed(new_observation, pods, "tls-rotate-0"));

    assert_eq!(plan.conditions[0].type_, TLS_ROTATION_PROGRESSING_CONDITION);
    assert_eq!(plan.conditions[0].reason, "TlsPodDrainAndReplace");
    assert_eq!(plan.admin_actions, vec![AdminAction::Drain { ordinal: 2 }]);
    assert_eq!(plan.delete_pod.as_deref(), Some("tls-rotate-2"));
}

#[test]
fn tls_leader_pod_is_rotated_after_reelection_not_before() {
    let cluster = cluster("tls-leader");
    let old_secret = tls_secret("hydracache-mtls", "old");
    let new_secret = tls_secret("hydracache-mtls", "new");
    let old_fingerprint = TlsSecretObservation::from_secret("hydracache-mtls", Some(&old_secret))
        .fingerprint
        .unwrap();
    let new_fingerprint = TlsSecretObservation::from_secret("hydracache-mtls", Some(&new_secret))
        .fingerprint
        .unwrap();
    let new_observation = TlsSecretObservation::from_secret("hydracache-mtls", Some(&new_secret));
    let pods = vec![
        tls_pod("tls-leader", 0, Some(&new_fingerprint), true),
        tls_pod("tls-leader", 1, Some(&new_fingerprint), true),
        tls_pod("tls-leader", 2, Some(&old_fingerprint), true),
    ];

    let before_reelection = plan_tls_rotation(
        &cluster,
        &observed(new_observation.clone(), pods.clone(), "tls-leader-2"),
    );
    assert_eq!(
        before_reelection.admin_actions,
        vec![AdminAction::Drain { ordinal: 2 }]
    );
    assert_eq!(
        before_reelection.conditions[0].reason,
        "TlsLeaderReelectionRequested"
    );
    assert!(before_reelection.delete_pod.is_none());

    let after_reelection =
        plan_tls_rotation(&cluster, &observed(new_observation, pods, "tls-leader-0"));
    assert_eq!(after_reelection.delete_pod.as_deref(), Some("tls-leader-2"));
}

#[test]
fn tls_secret_ref_without_secret_is_refused_loud() {
    let cluster = cluster("missing-secret");
    let missing = TlsSecretObservation::from_secret("hydracache-mtls", None);

    let plan = plan_tls_secret(&cluster, &missing);

    assert!(plan.blocked);
    assert_eq!(plan.conditions[0].type_, TLS_ROTATION_BLOCKED_CONDITION);
    assert_eq!(plan.conditions[0].reason, "TlsSecretMissing");
}

#[test]
fn tls_secret_fingerprint_is_projected_to_pod_template() {
    let cluster = cluster("tls-fingerprint");
    let secret = tls_secret("hydracache-mtls", "current");
    let observation = TlsSecretObservation::from_secret("hydracache-mtls", Some(&secret));
    let desired = OwnedResources::build_with_replicas_and_tls_fingerprint(
        &cluster,
        cluster.spec.replicas,
        observation.fingerprint.as_deref(),
    );
    let annotations = desired
        .stateful_set
        .spec
        .as_ref()
        .unwrap()
        .template
        .metadata
        .as_ref()
        .unwrap()
        .annotations
        .as_ref()
        .unwrap();

    assert_eq!(
        annotations[TLS_SECRET_NAME_ANNOTATION],
        cluster.spec.tls.as_ref().unwrap().secret_name
    );
    assert_eq!(
        annotations[TLS_SECRET_FINGERPRINT_ANNOTATION],
        observation.fingerprint.unwrap()
    );
}
