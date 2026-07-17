use hydracache_operator::backup::{
    backup_failed_condition, plan_backup, plan_pitr_restore_into_fresh_cluster, BackupObservation,
    PitrRestoreRequest, BACKUP_BLOCKED_CONDITION, BACKUP_FAILED_CONDITION,
    BACKUP_PROGRESSING_CONDITION, RESTORE_BLOCKED_CONDITION, RESTORE_PLANNED_CONDITION,
};
use hydracache_operator::controller::{HEALTHY_HEALTH, READY_PHASE};
use hydracache_operator::crd::{sample_spec, HydraCacheCluster};
use hydracache_operator::health::{
    client_service_routes_only_ready, statefulset_exposes_admin_port,
    statefulset_uses_admin_liveness_probe, statefulset_uses_admission_readiness_gate,
};
use hydracache_operator::resources::OwnedResources;
use hydracache_operator::scale::AdminAction;

fn cluster(name: &str) -> HydraCacheCluster {
    let mut cluster = HydraCacheCluster::new(name, sample_spec());
    cluster.metadata.namespace = Some("default".to_owned());
    cluster.metadata.uid = Some(format!("{name}-uid"));
    cluster.metadata.generation = Some(11);
    cluster
}

fn ready_backup_observation(last_backup: Option<&str>) -> BackupObservation {
    BackupObservation {
        phase: READY_PHASE.to_owned(),
        health: HEALTHY_HEALTH.to_owned(),
        ready_replicas: 3,
        last_backup: last_backup.map(str::to_owned),
    }
}

#[test]
fn scheduled_backup_request_does_not_record_a_durable_artifact() {
    let cluster = cluster("backup-run");
    let plan = plan_backup(&cluster, &ready_backup_observation(None));

    assert_eq!(plan.conditions[0].type_, BACKUP_PROGRESSING_CONDITION);
    assert_eq!(plan.conditions[0].reason, "ScheduledBackupRequested");
    assert_eq!(plan.admin_actions, vec![AdminAction::Backup { ordinal: 0 }]);
    assert!(
        !plan.record_last_backup_on_success,
        "request acceptance is not durable backup completion"
    );

    let steady = plan_backup(
        &cluster,
        &ready_backup_observation(Some("2026-07-02T11:00:00Z")),
    );
    assert!(steady.admin_actions.is_empty());
    assert!(!steady.record_last_backup_on_success);
}

#[test]
fn pitr_restore_into_fresh_cluster_reconciles_with_authority() {
    let cluster = cluster("restore-fresh");
    let request = PitrRestoreRequest {
        manifest_key: "backup-a/manifest.json".to_owned(),
        pitr_key: Some("backup-a/pitr-12.log".to_owned()),
        target_epoch: 12,
    };

    let plan = plan_pitr_restore_into_fresh_cluster(&cluster, &request, 0);

    assert!(plan.restore_allowed);
    assert_eq!(plan.authority_epoch, 12);
    assert_eq!(plan.conditions[0].type_, RESTORE_PLANNED_CONDITION);
    assert_eq!(plan.conditions[0].reason, "PitrRestorePreflightPassed");
    assert!(plan.conditions[0].message.contains("authority epoch 12"));
    assert!(plan.conditions[0]
        .message
        .contains("no live restore sink is wired"));

    let blocked = plan_pitr_restore_into_fresh_cluster(&cluster, &request, 1);
    assert!(!blocked.restore_allowed);
    assert_eq!(blocked.conditions[0].type_, RESTORE_BLOCKED_CONDITION);
    assert_eq!(blocked.conditions[0].reason, "RestoreRequiresFreshCluster");
}

#[test]
fn failed_backup_sets_a_loud_condition() {
    let condition = backup_failed_condition("admin HTTP action rejected with status 409", Some(11));

    assert_eq!(condition.type_, BACKUP_FAILED_CONDITION);
    assert_eq!(condition.status, "True");
    assert_eq!(condition.reason, "AdminBackupFailed");
    assert!(condition.message.contains("409"));
    assert_eq!(condition.observed_generation, Some(11));
}

#[test]
fn unready_pod_is_not_routed() {
    let cluster = cluster("health-route");
    let desired = OwnedResources::build(&cluster);

    assert!(client_service_routes_only_ready(&desired.client_service));
    assert!(statefulset_uses_admission_readiness_gate(
        &desired.stateful_set
    ));
    assert!(statefulset_uses_admin_liveness_probe(&desired.stateful_set));
    assert!(statefulset_exposes_admin_port(&desired.stateful_set));
    assert_ne!(
        desired
            .client_service
            .spec
            .as_ref()
            .unwrap()
            .publish_not_ready_addresses,
        Some(true)
    );
}

#[test]
fn backup_location_is_wired_to_server_env() {
    let cluster = cluster("backup-env");
    let desired = OwnedResources::build(&cluster);
    let env = desired
        .stateful_set
        .spec
        .as_ref()
        .unwrap()
        .template
        .spec
        .as_ref()
        .unwrap()
        .containers[0]
        .env
        .as_ref()
        .unwrap();

    assert!(env.iter().any(|var| {
        var.name == "HYDRACACHE_BACKUP_LOCATION"
            && var.value.as_deref() == Some("file:///var/lib/hydracache/backups")
    }));
}

#[test]
fn backup_without_location_is_refused_loud() {
    let mut cluster = cluster("backup-no-location");
    cluster.spec.backup_schedule.as_mut().unwrap().location = " ".to_owned();

    let plan = plan_backup(&cluster, &ready_backup_observation(None));

    assert!(plan.admin_actions.is_empty());
    assert_eq!(plan.conditions[0].type_, BACKUP_BLOCKED_CONDITION);
    assert_eq!(plan.conditions[0].reason, "MissingBackupLocation");
}
