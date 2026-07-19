//! Scale planning for `HydraCacheCluster`.

use std::time::Duration;

use http::{Method, Request};
use k8s_openapi::api::apps::v1::StatefulSet;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{Condition, Time};
use kube::{Client, ResourceExt};
use serde::Deserialize;
use thiserror::Error;

use crate::crd::HydraCacheCluster;
use crate::resources::{headless_service_name, ADMIN_PORT};

pub const SCALING_PHASE: &str = "Scaling";
pub const REBALANCING_PHASE: &str = "Rebalancing";
pub const SCALE_BLOCKED_CONDITION: &str = "ScaleBlocked";
pub const SCALE_PROGRESSING_CONDITION: &str = "ScaleProgressing";
pub const SCALE_ACTION_FAILED_CONDITION: &str = "ScaleActionFailed";
pub const SCALE_ADMIN_STATUS_UNAVAILABLE_CONDITION: &str = "ScaleAdminStatusUnavailable";

const ADMIN_STATUS_PATH: &str = "/admin/status";
const ADMIN_DRAIN_PATH: &str = "/admin/drain";
const ADMIN_RESHARD_PATH: &str = "/admin/reshard";
const ADMIN_BACKUP_PATH: &str = "/admin/backup";
const HYDRACACHE_CLIENT_ID_HEADER: &str = "x-hydracache-client-id";
const HYDRACACHE_TENANT_HEADER: &str = "x-hydracache-tenant";
const HYDRACACHE_ADMIN_HEADER: &str = "x-hydracache-admin";

/// Runtime view used by the scale planner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScaleObservation {
    pub current_replicas: u32,
    pub ready_replicas: u32,
    pub previous_phase: Option<String>,
    pub drain_requested_for: Option<String>,
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
        let drain_requested_for = cluster
            .status
            .as_ref()
            .and_then(|status| {
                status
                    .conditions
                    .iter()
                    .find(|condition| {
                        condition.type_ == SCALE_PROGRESSING_CONDITION
                            && condition.status == "True"
                            && condition.reason == "DrainRequested"
                    })
                    .map(|condition| condition.message.clone())
            })
            .and_then(|message| {
                message
                    .strip_prefix("drain requested for ")
                    .map(str::to_owned)
            });

        Self {
            current_replicas,
            ready_replicas,
            previous_phase,
            drain_requested_for,
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
    pub voters: u32,
    pub reshard_phase: String,
    pub draining: bool,
}

/// Admin action the reconcile loop should fire through the W0 HTTP surface.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdminAction {
    Reshard { ordinal: u32 },
    Drain { ordinal: u32 },
    Backup { ordinal: u32 },
}

impl AdminAction {
    fn path(&self) -> &'static str {
        match self {
            Self::Reshard { .. } => ADMIN_RESHARD_PATH,
            Self::Drain { .. } => ADMIN_DRAIN_PATH,
            Self::Backup { .. } => ADMIN_BACKUP_PATH,
        }
    }

    fn ordinal(&self) -> u32 {
        match self {
            Self::Reshard { ordinal } | Self::Drain { ordinal } | Self::Backup { ordinal } => {
                *ordinal
            }
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

    let drain_ordinal = current - 1;
    let drain_pod = pod_name(&cluster.name_any(), drain_ordinal);
    if observed.drain_complete_for.as_deref() == Some(drain_pod.as_str()) {
        if observed.ready_replicas < desired {
            return ScalePlan {
                effective_replicas: current,
                phase: SCALING_PHASE,
                conditions: vec![scale_condition(
                    SCALE_PROGRESSING_CONDITION,
                    "True",
                    "WaitingForSurvivorReplicas",
                    &format!(
                        "waiting for {desired} survivor replicas after draining {drain_pod}; observed {}",
                        observed.ready_replicas
                    ),
                    generation,
                )],
                admin_actions: Vec::new(),
            };
        }

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

    if observed.drain_requested_for.as_deref() == Some(drain_pod.as_str()) {
        if observed.admin_status.as_ref().is_some_and(|status| {
            status.quorum_ok && status.members <= desired && status.voters <= desired
        }) {
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

        return ScalePlan {
            effective_replicas: current,
            phase: SCALING_PHASE,
            conditions: vec![
                scale_condition(
                    SCALE_PROGRESSING_CONDITION,
                    "True",
                    "DrainRequested",
                    &format!("drain requested for {drain_pod}"),
                    generation,
                ),
                scale_condition(
                    SCALE_PROGRESSING_CONDITION,
                    "True",
                    "WaitingForDrainCommit",
                    &format!(
                        "waiting for committed voter removal before removing {drain_pod}; target members/voters {desired}"
                    ),
                    generation,
                ),
            ],
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

    ScalePlan {
        effective_replicas: current,
        phase: SCALING_PHASE,
        conditions: vec![scale_condition(
            SCALE_PROGRESSING_CONDITION,
            "True",
            "DrainBeforeRemove",
            &format!("drain requested for {drain_pod}"),
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

/// Thin W0 admin client routed through the Kubernetes pod proxy.
///
/// The proxy keeps the same transport working both in-cluster and for a
/// controller process using an external kubeconfig, without relying on the
/// latter being able to resolve or route to cluster-only pod DNS names.
#[derive(Clone)]
pub struct ScaleAdminClient {
    client: Client,
    timeout: Duration,
}

impl ScaleAdminClient {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            timeout: Duration::from_secs(2),
        }
    }

    pub async fn status(
        &self,
        namespace: &str,
        cluster_name: &str,
        ordinal: u32,
    ) -> Result<AdminStatus, ScaleAdminError> {
        let uri = admin_proxy_uri(namespace, cluster_name, ordinal, ADMIN_STATUS_PATH);
        let request = admin_proxy_request(Method::GET, &uri)?;
        let response =
            tokio::time::timeout(self.timeout, self.client.request::<AdminStatus>(request))
                .await
                .map_err(|_| ScaleAdminError::Timeout { uri: uri.clone() })?;
        response.map_err(|source| ScaleAdminError::KubernetesProxy {
            uri,
            source: Box::new(source),
        })
    }

    pub async fn perform(
        &self,
        namespace: &str,
        cluster_name: &str,
        actions: &[AdminAction],
    ) -> Result<(), ScaleAdminError> {
        for action in actions {
            let uri = admin_proxy_uri(namespace, cluster_name, action.ordinal(), action.path());
            let request = admin_proxy_request(Method::POST, &uri)?;
            let response = tokio::time::timeout(self.timeout, self.client.request_text(request))
                .await
                .map_err(|_| ScaleAdminError::Timeout { uri: uri.clone() })?;
            response.map_err(|source| ScaleAdminError::KubernetesProxy {
                uri,
                source: Box::new(source),
            })?;
        }
        Ok(())
    }
}

fn admin_proxy_uri(namespace: &str, cluster_name: &str, ordinal: u32, admin_path: &str) -> String {
    format!(
        "/api/v1/namespaces/{namespace}/pods/{}:{ADMIN_PORT}/proxy{admin_path}",
        pod_name(cluster_name, ordinal)
    )
}

fn admin_proxy_request(method: Method, uri: &str) -> Result<Request<Vec<u8>>, ScaleAdminError> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(HYDRACACHE_CLIENT_ID_HEADER, "operator")
        .header(HYDRACACHE_TENANT_HEADER, "system")
        .header(HYDRACACHE_ADMIN_HEADER, "true")
        .body(Vec::new())
        .map_err(|source| ScaleAdminError::RequestBuild {
            uri: uri.to_owned(),
            source,
        })
}

#[derive(Debug, Error)]
pub enum ScaleAdminError {
    #[error("could not build admin Kubernetes pod-proxy request {uri}: {source}")]
    RequestBuild {
        uri: String,
        #[source]
        source: http::Error,
    },
    #[error("admin Kubernetes pod-proxy request {uri} timed out")]
    Timeout { uri: String },
    #[error("admin Kubernetes pod-proxy request {uri} failed: {source}")]
    KubernetesProxy {
        uri: String,
        #[source]
        source: Box<kube::Error>,
    },
}
