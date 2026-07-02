//! Backup/PITR planning for `HydraCacheCluster`.

use k8s_openapi::apimachinery::pkg::apis::meta::v1::Condition;

use crate::controller::{HEALTHY_HEALTH, READY_PHASE};
use crate::crd::HydraCacheCluster;
use crate::scale::{scale_condition, AdminAction};

pub const BACKUP_PROGRESSING_CONDITION: &str = "BackupProgressing";
pub const BACKUP_BLOCKED_CONDITION: &str = "BackupBlocked";
pub const BACKUP_FAILED_CONDITION: &str = "BackupFailed";
pub const BACKUP_COMPLETED_CONDITION: &str = "BackupCompleted";
pub const RESTORE_PLANNED_CONDITION: &str = "RestorePlanned";
pub const RESTORE_BLOCKED_CONDITION: &str = "RestoreBlocked";

/// Runtime view needed to decide whether the operator loop should request backup now.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupObservation {
    pub phase: String,
    pub health: String,
    pub ready_replicas: u32,
    pub last_backup: Option<String>,
}

/// Deterministic backup decision.
#[derive(Clone, Debug, PartialEq)]
pub struct BackupPlan {
    pub conditions: Vec<Condition>,
    pub admin_actions: Vec<AdminAction>,
    pub record_last_backup_on_success: bool,
}

impl BackupPlan {
    pub fn steady() -> Self {
        Self {
            conditions: Vec::new(),
            admin_actions: Vec::new(),
            record_last_backup_on_success: false,
        }
    }

    pub fn deferred(generation: Option<i64>) -> Self {
        Self {
            conditions: vec![condition(
                BACKUP_PROGRESSING_CONDITION,
                "BackupDeferredForLifecycle",
                "deferring scheduled backup while another lifecycle action is active",
                generation,
            )],
            admin_actions: Vec::new(),
            record_last_backup_on_success: false,
        }
    }
}

/// Request a backup through the W0 admin surface when a configured schedule has not recorded one.
pub fn plan_backup(cluster: &HydraCacheCluster, observed: &BackupObservation) -> BackupPlan {
    let Some(schedule) = cluster.spec.backup_schedule.as_ref() else {
        return BackupPlan::steady();
    };
    let generation = cluster.metadata.generation;

    if schedule.location.trim().is_empty() {
        return blocked(
            "MissingBackupLocation",
            "refusing scheduled backup without spec.backupSchedule.location",
            generation,
        );
    }

    if observed.phase != READY_PHASE {
        return BackupPlan::deferred(generation);
    }

    if observed.health != HEALTHY_HEALTH || observed.ready_replicas < cluster.spec.replicas {
        return blocked(
            "BackupRequiresHealthyCluster",
            "waiting for a healthy cluster before requesting scheduled backup",
            generation,
        );
    }

    if observed.last_backup.is_some() {
        return BackupPlan::steady();
    }

    BackupPlan {
        conditions: vec![condition(
            BACKUP_PROGRESSING_CONDITION,
            "ScheduledBackupRequested",
            &format!(
                "requesting backup for schedule {} to {}",
                schedule.schedule, schedule.location
            ),
            generation,
        )],
        admin_actions: vec![AdminAction::Backup { ordinal: 0 }],
        record_last_backup_on_success: true,
    }
}

/// Restore request used by the documented PITR restore path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PitrRestoreRequest {
    pub manifest_key: String,
    pub pitr_key: Option<String>,
    pub target_epoch: u64,
}

/// Deterministic restore preflight for a fresh `HydraCacheCluster`.
#[derive(Clone, Debug, PartialEq)]
pub struct PitrRestorePlan {
    pub conditions: Vec<Condition>,
    pub restore_allowed: bool,
    pub authority_epoch: u64,
}

pub fn plan_pitr_restore_into_fresh_cluster(
    cluster: &HydraCacheCluster,
    request: &PitrRestoreRequest,
    observed_replicas: u32,
) -> PitrRestorePlan {
    if observed_replicas > 0 {
        return PitrRestorePlan {
            conditions: vec![condition(
                RESTORE_BLOCKED_CONDITION,
                "RestoreRequiresFreshCluster",
                "refusing PITR restore into a cluster that already has running replicas",
                cluster.metadata.generation,
            )],
            restore_allowed: false,
            authority_epoch: request.target_epoch,
        };
    }

    PitrRestorePlan {
        conditions: vec![condition(
            RESTORE_PLANNED_CONDITION,
            "PitrRestorePrepared",
            &format!(
                "restore manifest {}{} at authority epoch {}",
                request.manifest_key,
                request
                    .pitr_key
                    .as_ref()
                    .map(|key| format!(" with PITR log {key}"))
                    .unwrap_or_default(),
                request.target_epoch
            ),
            cluster.metadata.generation,
        )],
        restore_allowed: true,
        authority_epoch: request.target_epoch,
    }
}

pub fn backup_completed_condition(completed_at: &str, generation: Option<i64>) -> Condition {
    condition(
        BACKUP_COMPLETED_CONDITION,
        "BackupSucceeded",
        &format!("scheduled backup completed at {completed_at}"),
        generation,
    )
}

pub fn backup_failed_condition(error: &str, generation: Option<i64>) -> Condition {
    condition(
        BACKUP_FAILED_CONDITION,
        "AdminBackupFailed",
        error,
        generation,
    )
}

fn blocked(reason: &str, message: &str, generation: Option<i64>) -> BackupPlan {
    BackupPlan {
        conditions: vec![condition(
            BACKUP_BLOCKED_CONDITION,
            reason,
            message,
            generation,
        )],
        admin_actions: Vec::new(),
        record_last_backup_on_success: false,
    }
}

pub fn condition(type_: &str, reason: &str, message: &str, generation: Option<i64>) -> Condition {
    scale_condition(type_, "True", reason, message, generation)
}
