//! Scale planning for `HydraCacheCluster`.

use std::time::Duration;

use k8s_openapi::api::apps::v1::StatefulSet;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{Condition, Time};
use kube::ResourceExt;
use serde::Deserialize;
use thiserror::Error;

use crate::crd::HydraCacheCluster;
use crate::resources::{headless_service_name, ADMIN_PORT};

pub const SCALING_PHASE: &str = "Scaling";
pub const REBALANCING_PHASE: &str = "Rebalancing";
pub const SCALE_BLOCKED_CONDITION: &str = "ScaleBlocked";
pub const SCALE_PROGRESSING_CONDITION: &str = "ScaleProgressing";
pub const SCALE_ACTION_FAILED_CONDITION: &str = "ScaleActionFailed";

const ADMIN_STATUS_PATH: &str = "/admin/status";
const ADMIN_DRAIN_PATH: &str = "/admin/drain";
const ADMIN_RESHARD_PATH: &str = "/admin/reshard";
const HYDRACACHE_CLIENT_ID_HEADER: &str = "x-hydracache-client-id";
const HYDRACACHE_TENANT_HEADER: &str = "x-hydracache-tenant";
const HYDRACACHE_ADMIN_HEADER: &str = "x-hydracache-admin";

/// Runtime view used by the scale planner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScaleObservation {
    pub current_replicas: u32,
    pub ready_replicas: u32,
    pub previous_phase: Option<String>,
    pub drain_complete_for: Option<String>,
    pub admin_status: Option<AdminStatus>,
}

impl ScaleObservation {
    pub fn from_statefulset(
        cluster: &HydraCacheCluster,
        stateful_set: Option<&StatefulSet>,
    ) -> Self {
        let current_replicas = stateful_set
            .and_then(|sts| sts.spec.as_ref())
            .and_then(|spec| spec.replicas)
            .map(|replicas| replicas.max(0) as u32)
            .unwrap_or(0);
        let ready_replicas = stateful_set
            .and_then(|sts| sts.status.as_ref())
            .map(|status| status.ready_replicas.unwrap_or(status.replicas))
            .map(|replicas| replicas.max(0) as u32)
            .unwrap_or(0);
        let previous_phase = cluster.status.as_ref().map(|status| status.phase.clone());
        let drain_complete_for = cluster
            .status
            .as_ref()
            .and_then(|status| {
                status
                    .conditions
                    .iter()
                    .find(|condition| {
                        condition.type_ == SCALE_PROGRESSING_CONDITION
                            && condition.status == "True"
                            && condition.reason == "DrainComplete"
                    })
                    .map(|condition| condition.message.clone())
            })
            .and_then(|message| {
                message
                    .strip_prefix("drain complete for ")
                    .map(str::to_owned)
            });

        Self {
            current_replicas,
            ready_replicas,
            previous_phase,
            drain_complete_for,
            admin_status: None,
        }
    }
}

/// `/admin/status` fields the operator needs for lifecycle decisions.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct AdminStatus {
    pub leader: Option<String>,
    pub quorum_ok: bool,
    pub members: u32,
    pub reshard_phase: String,
    pub draining: bool,
}

/// Admin action the reconcile loop should fire through the W0 HTTP surface.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdminAction {
    Reshard { ordinal: u32 },
    Drain { ordinal: u32 },
}

impl AdminAction {
    fn path(&self) -> &'static str {
        match self {
            Self::Reshard { .. } => ADMIN_RESHARD_PATH,
            Self::Drain { .. } => ADMIN_DRAIN_PATH,
        }
    }

    fn ordinal(&self) -> u32 {
        match self {
            Self::Reshard { ordinal } | Self::Drain { ordinal } => *ordinal,
        }
    }
}

/// Deterministic scale decision.
#[derive(Clone, Debug, PartialEq)]
pub struct ScalePlan {
    pub effective_replicas: u32,
    pub phase: &'static str,
    pub conditions: Vec<Condition>,
    pub admin_actions: Vec<AdminAction>,
}

impl ScalePlan {
    pub fn steady(replicas: u32) -> Self {
        Self {
            effective_replicas: replicas,
            phase: crate::controller::READY_PHASE,
            conditions: Vec::new(),
            admin_actions: Vec::new(),
        }
    }
}

/// Build the plan that keeps scale changes quorum-safe and resumable.
pub fn plan_scale(cluster: &HydraCacheCluster, observed: &ScaleObservation) -> ScalePlan {
    let desired = cluster.spec.replicas;
    let current = observed.current_replicas;
    let generation = cluster.metadata.generation;

    if current == 0 || desired > current {
        return ScalePlan {
            effective_replicas: desired,
            phase: SCALING_PHASE,
            conditions: vec![scale_condition(
                SCALE_PROGRESSING_CONDITION,
                "True",
                "ScaleUpCreatingPods",
                &format!("scaling StatefulSet replicas from {current} to {desired}"),
                generation,
            )],
            admin_actions: Vec::new(),
        };
    }

    if desired == current {
        if observed.ready_replicas < desired {
            return ScalePlan {
                effective_replicas: desired,
                phase: SCALING_PHASE,
                conditions: vec![scale_condition(
                    SCALE_PROGRESSING_CONDITION,
                    "True",
                    "WaitingForReadyReplicas",
                    &format!(
                        "waiting for {desired} ready replicas; observed {}",
                        observed.ready_replicas
                    ),
                    generation,
                )],
                admin_actions: Vec::new(),
            };
        }

        if observed.previous_phase.as_deref() == Some(REBALANCING_PHASE)
            && observed
                .admin_status
                .as_ref()
                .is_some_and(|status| status.reshard_phase == "idle")
        {
            return ScalePlan::steady(desired);
        }

        return ScalePlan {
            effective_replicas: desired,
            phase: REBALANCING_PHASE,
            conditions: vec![scale_condition(
                SCALE_PROGRESSING_CONDITION,
                "True",
                "ScaleUpReshardRequested",
                "requesting online reshard after scale-up",
                generation,
            )],
            admin_actions: vec![AdminAction::Reshard { ordinal: 0 }],
        };
    }

    let minimum_survivors = quorum_for(current);
    if desired < minimum_survivors {
        return ScalePlan {
            effective_replicas: current,
            phase: SCALING_PHASE,
            conditions: vec![scale_condition(
                SCALE_BLOCKED_CONDITION,
                "True",
                "ScaleBelowQuorumRefused",
                &format!(
                    "refusing scale-down from {current} to {desired}; at least {minimum_survivors} members must remain for quorum"
                ),
                generation,
            )],
            admin_actions: Vec::new(),
        };
    }

    if observed.ready_replicas < current {
        return ScalePlan {
            effective_replicas: current,
            phase: SCALING_PHASE,
            conditions: vec![scale_condition(
                SCALE_PROGRESSING_CONDITION,
                "True",
                "WaitingForCurrentReplicas",
                &format!(
                    "waiting for all {current} current replicas before scale-down; observed {}",
                    observed.ready_replicas
                ),
                generation,
            )],
            admin_actions: Vec::new(),
        };
    }

    let drain_ordinal = current - 1;
    let drain_pod = pod_name(&cluster.name_any(), drain_ordinal);
    if observed
        .admin_status
        .as_ref()
        .and_then(|status| status.leader.as_ref())
        .is_some_and(|leader| leader == &drain_pod)
    {
        return ScalePlan {
            effective_replicas: current,
            phase: SCALING_PHASE,
            conditions: vec![scale_condition(
                SCALE_BLOCKED_CONDITION,
                "True",
                "LeaderDrainDeferred",
                &format!("refusing to drain leader pod {drain_pod} before re-election"),
                generation,
            )],
            admin_actions: Vec::new(),
        };
    }

    if observed.drain_complete_for.as_deref() == Some(drain_pod.as_str()) {
        return ScalePlan {
            effective_replicas: desired,
            phase: REBALANCING_PHASE,
            conditions: vec![scale_condition(
                SCALE_PROGRESSING_CONDITION,
                "True",
                "DrainComplete",
                &format!("drain complete for {drain_pod}"),
                generation,
            )],
            admin_actions: Vec::new(),
        };
    }

    ScalePlan {
        effective_replicas: current,
        phase: SCALING_PHASE,
        conditions: vec![scale_condition(
            SCALE_PROGRESSING_CONDITION,
            "True",
            "DrainBeforeRemove",
            &format!("drain complete for {drain_pod}"),
            generation,
        )],
        admin_actions: vec![
            AdminAction::Reshard {
                ordinal: drain_ordinal,
            },
            AdminAction::Drain {
                ordinal: drain_ordinal,
            },
        ],
    }
}

pub fn quorum_for(replicas: u32) -> u32 {
    if replicas == 0 {
        0
    } else {
        replicas / 2 + 1
    }
}

pub fn pod_name(cluster_name: &str, ordinal: u32) -> String {
    format!("{cluster_name}-{ordinal}")
}

pub fn admin_base_url(namespace: &str, cluster_name: &str, ordinal: u32) -> String {
    format!(
        "http://{}.{}.{}.svc:{ADMIN_PORT}",
        pod_name(cluster_name, ordinal),
        headless_service_name(cluster_name),
        namespace
    )
}

pub fn scale_condition(
    type_: &str,
    status: &str,
    reason: &str,
    message: &str,
    generation: Option<i64>,
) -> Condition {
    Condition {
        last_transition_time: Time(k8s_openapi::jiff::Timestamp::now()),
        message: message.to_owned(),
        observed_generation: generation,
        reason: reason.to_owned(),
        status: status.to_owned(),
        type_: type_.to_owned(),
    }
}

/// Thin W0 admin HTTP client used by the controller inside the cluster.
#[derive(Clone, Debug)]
pub struct ScaleAdminClient {
    http: reqwest::Client,
    timeout: Duration,
}

impl Default for ScaleAdminClient {
    fn default() -> Self {
        Self {
            http: reqwest::Client::new(),
            timeout: Duration::from_secs(2),
        }
    }
}

impl ScaleAdminClient {
    pub async fn status(
        &self,
        namespace: &str,
        cluster_name: &str,
        ordinal: u32,
    ) -> Result<AdminStatus, ScaleAdminError> {
        let url = format!(
            "{}{}",
            admin_base_url(namespace, cluster_name, ordinal),
            ADMIN_STATUS_PATH
        );
        let response = self
            .admin_headers(self.http.get(url))
            .timeout(self.timeout)
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(ScaleAdminError::Rejected(response.status().as_u16()));
        }
        Ok(response.json::<AdminStatus>().await?)
    }

    pub async fn perform(
        &self,
        namespace: &str,
        cluster_name: &str,
        actions: &[AdminAction],
    ) -> Result<(), ScaleAdminError> {
        for action in actions {
            let url = format!(
                "{}{}",
                admin_base_url(namespace, cluster_name, action.ordinal()),
                action.path()
            );
            let response = self
                .admin_headers(self.http.post(url))
                .timeout(self.timeout)
                .send()
                .await?;
            if !response.status().is_success() {
                return Err(ScaleAdminError::Rejected(response.status().as_u16()));
            }
        }
        Ok(())
    }

    fn admin_headers(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        request
            .header(HYDRACACHE_CLIENT_ID_HEADER, "operator")
            .header(HYDRACACHE_TENANT_HEADER, "system")
            .header(HYDRACACHE_ADMIN_HEADER, "true")
    }
}

#[derive(Debug, Error)]
pub enum ScaleAdminError {
    #[error("admin HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("admin HTTP action rejected with status {0}")]
    Rejected(u16),
}
