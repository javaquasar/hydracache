//! Rolling-upgrade planning for `HydraCacheCluster`.

use k8s_openapi::api::core::v1::Pod;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::Condition;
use kube::ResourceExt;

use crate::controller::READY_PHASE;
use crate::crd::HydraCacheCluster;
use crate::resources::SERVER_CONTAINER;
use crate::scale::{
    pod_name, quorum_for, scale_condition, AdminAction, AdminStatus, SCALE_PROGRESSING_CONDITION,
};

pub const UPGRADING_PHASE: &str = "Upgrading";
pub const UPGRADE_PROGRESSING_CONDITION: &str = "UpgradeProgressing";
pub const UPGRADE_BLOCKED_CONDITION: &str = "UpgradeBlocked";
pub const UPGRADE_FAILED_CONDITION: &str = "UpgradeFailed";
pub const UPGRADE_ACTION_FAILED_CONDITION: &str = "UpgradeActionFailed";
pub const VERSION_ANNOTATION: &str = "hydracache.io/version";
pub const UPGRADE_STEP_TIMEOUT_SECS: u64 = 300;

/// Runtime pod state used by the upgrade planner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PodObservation {
    pub name: String,
    pub ordinal: u32,
    pub image: Option<String>,
    pub version: Option<String>,
    pub ready: bool,
    pub deleting: bool,
    pub not_ready_for_seconds: Option<u64>,
}

impl PodObservation {
    pub fn from_pod(cluster_name: &str, pod: &Pod) -> Option<Self> {
        let name = pod.name_any();
        let ordinal = pod_ordinal(cluster_name, &name)?;
        let image = pod
            .spec
            .as_ref()
            .and_then(|spec| {
                spec.containers
                    .iter()
                    .find(|container| container.name == SERVER_CONTAINER)
            })
            .and_then(|container| container.image.clone());
        let version = pod
            .metadata
            .annotations
            .as_ref()
            .and_then(|annotations| annotations.get(VERSION_ANNOTATION).cloned());
        let ready = pod
            .status
            .as_ref()
            .and_then(|status| status.conditions.as_ref())
            .is_some_and(|conditions| {
                conditions
                    .iter()
                    .any(|condition| condition.type_ == "Ready" && condition.status == "True")
            });

        Some(Self {
            name,
            ordinal,
            image,
            version,
            ready,
            deleting: pod.metadata.deletion_timestamp.is_some(),
            not_ready_for_seconds: None,
        })
    }

    pub fn is_current(&self, desired_image: &str, desired_version: &str) -> bool {
        self.image.as_deref() == Some(desired_image)
            && self.version.as_deref() == Some(desired_version)
    }
}

/// Runtime view used by the rolling-upgrade planner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpgradeObservation {
    pub current_replicas: u32,
    pub ready_replicas: u32,
    pub previous_phase: Option<String>,
    pub admin_status: Option<AdminStatus>,
    pub pods: Vec<PodObservation>,
}

/// Deterministic upgrade decision. The controller applies at most one pod deletion per reconcile.
#[derive(Clone, Debug, PartialEq)]
pub struct UpgradePlan {
    pub phase: &'static str,
    pub conditions: Vec<Condition>,
    pub admin_actions: Vec<AdminAction>,
    pub delete_pod: Option<String>,
}

impl UpgradePlan {
    pub fn steady() -> Self {
        Self {
            phase: READY_PHASE,
            conditions: Vec::new(),
            admin_actions: Vec::new(),
            delete_pod: None,
        }
    }
}

pub fn plan_upgrade(cluster: &HydraCacheCluster, observed: &UpgradeObservation) -> UpgradePlan {
    let desired_image = cluster.spec.image.as_str();
    let desired_version = cluster.spec.version.as_str();
    let generation = cluster.metadata.generation;

    if observed.current_replicas == 0 || observed.pods.is_empty() {
        return UpgradePlan::steady();
    }

    let admin_status = match observed.admin_status.as_ref() {
        Some(status) => status,
        None => {
            return blocked(
                "WaitingForAdminStatus",
                "waiting for /admin/status before rolling pod replacement",
                generation,
            )
        }
    };

    if !admin_status.quorum_ok || observed.ready_replicas < quorum_for(observed.current_replicas) {
        return blocked(
            "UpgradeQuorumUnavailable",
            "waiting for quorum before replacing any pod",
            generation,
        );
    }

    if let Some(pod) = observed.pods.iter().find(|pod| {
        !pod.ready
            && pod
                .not_ready_for_seconds
                .is_some_and(|seconds| seconds >= UPGRADE_STEP_TIMEOUT_SECS)
    }) {
        return failed(
            "UpgradePodNotReadyTimeout",
            &format!(
                "halting rollout because pod {} has not become Ready within {} seconds",
                pod.name, UPGRADE_STEP_TIMEOUT_SECS
            ),
            generation,
        );
    }

    if observed.pods.iter().any(|pod| pod.deleting || !pod.ready) {
        return progressing(
            "WaitingForReadyPod",
            "waiting for the current replacement pod to become Ready before continuing",
            generation,
        );
    }

    let mut outdated = observed
        .pods
        .iter()
        .filter(|pod| !pod.is_current(desired_image, desired_version))
        .collect::<Vec<_>>();
    outdated.sort_by_key(|pod| std::cmp::Reverse(pod.ordinal));

    if outdated.is_empty() {
        return UpgradePlan::steady();
    }

    if let Some(from_version) = outdated.iter().find_map(|pod| pod.version.as_deref()) {
        if !version_skew_supported(from_version, desired_version) {
            return blocked(
                "UnsupportedVersionSkew",
                &format!(
                    "refusing rolling upgrade from {from_version} to {desired_version}; only one minor version skew is supported"
                ),
                generation,
            );
        }
    }

    let leader = admin_status.leader.as_deref();
    let selected = outdated
        .iter()
        .copied()
        .find(|pod| Some(pod.name.as_str()) != leader)
        .unwrap_or(outdated[0]);

    if Some(selected.name.as_str()) == leader && observed.current_replicas > 1 {
        return UpgradePlan {
            phase: UPGRADING_PHASE,
            conditions: vec![condition(
                UPGRADE_PROGRESSING_CONDITION,
                "LeaderReelectionRequested",
                &format!(
                    "requesting leader drain for {} before deleting the pod",
                    selected.name
                ),
                generation,
            )],
            admin_actions: vec![AdminAction::Drain {
                ordinal: selected.ordinal,
            }],
            delete_pod: None,
        };
    }

    UpgradePlan {
        phase: UPGRADING_PHASE,
        conditions: vec![condition(
            UPGRADE_PROGRESSING_CONDITION,
            "PodDrainAndReplace",
            &format!(
                "draining and deleting {} for one-at-a-time rolling replacement",
                selected.name
            ),
            generation,
        )],
        admin_actions: vec![AdminAction::Drain {
            ordinal: selected.ordinal,
        }],
        delete_pod: Some(selected.name.clone()),
    }
}

pub fn pod_ordinal(cluster_name: &str, pod_name_value: &str) -> Option<u32> {
    let prefix = format!("{cluster_name}-");
    pod_name_value.strip_prefix(&prefix)?.parse().ok()
}

pub fn version_skew_supported(from: &str, to: &str) -> bool {
    let Some((from_major, from_minor)) = major_minor(from) else {
        return false;
    };
    let Some((to_major, to_minor)) = major_minor(to) else {
        return false;
    };
    from_major == to_major && from_minor.abs_diff(to_minor) <= 1
}

fn major_minor(version: &str) -> Option<(u64, u64)> {
    let mut parts = version.trim_start_matches('v').split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    Some((major, minor))
}

fn progressing(reason: &str, message: &str, generation: Option<i64>) -> UpgradePlan {
    UpgradePlan {
        phase: UPGRADING_PHASE,
        conditions: vec![condition(
            UPGRADE_PROGRESSING_CONDITION,
            reason,
            message,
            generation,
        )],
        admin_actions: Vec::new(),
        delete_pod: None,
    }
}

fn blocked(reason: &str, message: &str, generation: Option<i64>) -> UpgradePlan {
    UpgradePlan {
        phase: UPGRADING_PHASE,
        conditions: vec![condition(
            UPGRADE_BLOCKED_CONDITION,
            reason,
            message,
            generation,
        )],
        admin_actions: Vec::new(),
        delete_pod: None,
    }
}

fn failed(reason: &str, message: &str, generation: Option<i64>) -> UpgradePlan {
    UpgradePlan {
        phase: UPGRADING_PHASE,
        conditions: vec![condition(
            UPGRADE_FAILED_CONDITION,
            reason,
            message,
            generation,
        )],
        admin_actions: Vec::new(),
        delete_pod: None,
    }
}

pub fn condition(type_: &str, reason: &str, message: &str, generation: Option<i64>) -> Condition {
    scale_condition(type_, "True", reason, message, generation)
}

pub fn upgrade_deferred_for_lifecycle(generation: Option<i64>) -> Condition {
    condition(
        SCALE_PROGRESSING_CONDITION,
        "UpgradeDeferredForLifecycle",
        "deferring rolling upgrade while another lifecycle action is active",
        generation,
    )
}

pub fn expected_pod_name(cluster_name: &str, ordinal: u32) -> String {
    pod_name(cluster_name, ordinal)
}
