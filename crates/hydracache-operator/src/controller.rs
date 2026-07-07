//! Level-triggered reconcile loop for `HydraCacheCluster`.

use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt;
use k8s_openapi::api::apps::v1::StatefulSet;
use k8s_openapi::api::coordination::v1::Lease;
use k8s_openapi::api::core::v1::{PersistentVolumeClaim, Pod, Secret, Service};
use k8s_openapi::api::policy::v1::PodDisruptionBudget;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{Condition, Time};
use kube::api::{Api, DeleteParams, ListParams, Patch, PatchParams, PostParams};
use kube::runtime::controller::Action;
use kube::runtime::finalizer::{finalizer, Event as FinalizerEvent};
use kube::runtime::{watcher, Controller};
use kube::{Client, ResourceExt};
use serde_json::json;
use thiserror::Error;

use crate::backup::{
    backup_completed_condition, backup_failed_condition, plan_backup, BackupObservation, BackupPlan,
};
use crate::crd::{HydraCacheCluster, HydraCacheClusterStatus};
use crate::persistence::plan_persistence;
use crate::resources::{
    cleanup_plan, headless_service_name, pod_selector_labels, OwnedResources, FIELD_MANAGER,
};
use crate::scale::{
    plan_scale, pod_name, scale_condition, ScaleAdminClient, ScaleObservation,
    SCALE_ACTION_FAILED_CONDITION,
};
use crate::tls::{
    plan_tls_rotation, plan_tls_secret, tls_deferred_for_lifecycle, TlsPodObservation,
    TlsRotationObservation, TlsRotationPlan, TlsSecretObservation,
    TLS_ROTATION_ACTION_FAILED_CONDITION, TLS_ROTATION_FAILED_CONDITION,
};
use crate::upgrade::{
    plan_upgrade, upgrade_deferred_for_lifecycle, PodObservation, UpgradeObservation, UpgradePlan,
    UPGRADE_ACTION_FAILED_CONDITION, UPGRADE_FAILED_CONDITION,
};

pub const FINALIZER: &str = "hydracache.io/finalizer";
pub const READY_PHASE: &str = "Ready";
pub const FORMING_HEALTH: &str = "Forming";
pub const HEALTHY_HEALTH: &str = "Healthy";
pub const DEGRADED_HEALTH: &str = "Degraded";

#[derive(Clone)]
pub struct Ctx {
    pub client: Client,
    pub identity: String,
    pub namespace: Option<String>,
    pub scale_admin: ScaleAdminClient,
}

impl Ctx {
    pub fn new(client: Client, identity: impl Into<String>, namespace: Option<String>) -> Self {
        Self {
            client,
            identity: identity.into(),
            namespace,
            scale_admin: ScaleAdminClient::default(),
        }
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("HydraCacheCluster {0} has no namespace")]
    MissingNamespace(String),
    #[error("kubernetes api error: {0}")]
    Kube(#[from] kube::Error),
    #[error("finalizer error: {0}")]
    Finalizer(String),
    #[error("statefulset immutable field changed: {field}")]
    ImmutableField { field: &'static str },
}

pub async fn run(ctx: Ctx) {
    let client = ctx.client.clone();
    let clusters: Api<HydraCacheCluster> = match &ctx.namespace {
        Some(namespace) => Api::namespaced(client.clone(), namespace),
        None => Api::all(client.clone()),
    };

    Controller::new(clusters, watcher::Config::default())
        .owns(
            Api::<StatefulSet>::all(client.clone()),
            watcher::Config::default(),
        )
        .owns(
            Api::<Service>::all(client.clone()),
            watcher::Config::default(),
        )
        .owns(Api::<Secret>::all(client), watcher::Config::default())
        .owns(
            Api::<PodDisruptionBudget>::all(ctx.client.clone()),
            watcher::Config::default(),
        )
        .run(reconcile, error_policy, Arc::new(ctx))
        .for_each(|result| async move {
            if let Err(error) = result {
                eprintln!("hydracache operator reconcile error: {error}");
            }
        })
        .await;
}

pub async fn reconcile(cluster: Arc<HydraCacheCluster>, ctx: Arc<Ctx>) -> Result<Action, Error> {
    if !holds_leader_lease(ctx.client.clone(), &cluster, &ctx.identity).await? {
        return Ok(Action::requeue(Duration::from_secs(15)));
    }

    let namespace = cluster
        .namespace()
        .ok_or_else(|| Error::MissingNamespace(cluster.name_any()))?;
    let clusters: Api<HydraCacheCluster> = Api::namespaced(ctx.client.clone(), &namespace);

    finalizer(&clusters, FINALIZER, cluster, |event| async {
        match event {
            FinalizerEvent::Apply(cluster) => apply_cluster(cluster, ctx.clone()).await,
            FinalizerEvent::Cleanup(cluster) => cleanup_cluster(cluster, ctx.clone()).await,
        }
    })
    .await
    .map_err(|error| Error::Finalizer(error.to_string()))
}

pub fn error_policy(_: Arc<HydraCacheCluster>, _: &Error, _: Arc<Ctx>) -> Action {
    Action::requeue(Duration::from_secs(15))
}

pub async fn apply_cluster(
    cluster: Arc<HydraCacheCluster>,
    ctx: Arc<Ctx>,
) -> Result<Action, Error> {
    let namespace = cluster
        .namespace()
        .ok_or_else(|| Error::MissingNamespace(cluster.name_any()))?;
    let apply = PatchParams::apply(FIELD_MANAGER).force();

    let statefulsets: Api<StatefulSet> = Api::namespaced(ctx.client.clone(), &namespace);
    let existing = get_optional(&statefulsets, &cluster.name_any()).await?;
    let mut scale_observation = ScaleObservation::from_statefulset(&cluster, existing.as_ref());
    if scale_observation.current_replicas > 0 {
        scale_observation.admin_status = ctx
            .scale_admin
            .status(&namespace, &cluster.name_any(), 0)
            .await
            .ok();
    }
    let scale_plan = plan_scale(&cluster, &scale_observation);

    let persistence_plan = plan_persistence(&cluster);
    if persistence_plan.blocked {
        let mut status = observed_status(&cluster, existing.as_ref());
        status.health = DEGRADED_HEALTH.to_owned();
        status.conditions.extend(persistence_plan.conditions);
        patch_status(&cluster, ctx.client.clone(), status).await?;
        return Ok(Action::requeue(Duration::from_secs(30)));
    }

    let secrets: Api<Secret> = Api::namespaced(ctx.client.clone(), &namespace);
    let tls_secret_observation = observe_tls_secret(&secrets, &cluster).await?;
    let tls_secret_plan = plan_tls_secret(&cluster, &tls_secret_observation);
    if tls_secret_plan.blocked {
        let mut status = observed_status(&cluster, existing.as_ref());
        status.health = DEGRADED_HEALTH.to_owned();
        status.conditions.extend(tls_secret_plan.conditions);
        patch_status(&cluster, ctx.client.clone(), status).await?;
        return Ok(Action::requeue(Duration::from_secs(30)));
    }

    let desired = OwnedResources::build_with_replicas_and_tls_fingerprint(
        &cluster,
        scale_plan.effective_replicas,
        tls_secret_plan.fingerprint.as_deref(),
    );

    if let Some(existing) = existing.as_ref() {
        validate_statefulset_update(existing, &desired.stateful_set)?;
    }
    statefulsets
        .patch(
            &cluster.name_any(),
            &apply,
            &Patch::Apply(&desired.stateful_set),
        )
        .await?;

    let services: Api<Service> = Api::namespaced(ctx.client.clone(), &namespace);
    services
        .patch(
            &headless_service_name(&cluster.name_any()),
            &apply,
            &Patch::Apply(&desired.headless_service),
        )
        .await?;
    services
        .patch(
            &cluster.name_any(),
            &apply,
            &Patch::Apply(&desired.client_service),
        )
        .await?;

    secrets
        .patch(
            desired
                .admin_secret
                .metadata
                .name
                .as_deref()
                .expect("admin secret builder sets name"),
            &apply,
            &Patch::Apply(&desired.admin_secret),
        )
        .await?;

    let pdbs: Api<PodDisruptionBudget> = Api::namespaced(ctx.client.clone(), &namespace);
    pdbs.patch(
        &cluster.name_any(),
        &apply,
        &Patch::Apply(&desired.pod_disruption_budget),
    )
    .await?;

    let mut status = observed_status(&cluster, existing.as_ref().or(Some(&desired.stateful_set)));
    status.phase = scale_plan.phase.to_owned();
    if let Some(admin_status) = scale_observation.admin_status.as_ref() {
        status.leader = admin_status.leader.clone();
    }
    status.conditions.extend(scale_plan.conditions.clone());
    if !scale_plan.admin_actions.is_empty() {
        match ctx
            .scale_admin
            .perform(&namespace, &cluster.name_any(), &scale_plan.admin_actions)
            .await
        {
            Ok(()) => {
                if let Some(drain_ordinal) =
                    scale_plan
                        .admin_actions
                        .iter()
                        .find_map(|action| match action {
                            crate::scale::AdminAction::Drain { ordinal } => Some(*ordinal),
                            crate::scale::AdminAction::Reshard { .. }
                            | crate::scale::AdminAction::Backup { .. } => None,
                        })
                {
                    let draining_pod = pod_name(&cluster.name_any(), drain_ordinal);
                    status.conditions.push(scale_condition(
                        crate::scale::SCALE_PROGRESSING_CONDITION,
                        "True",
                        "DrainComplete",
                        &format!("drain complete for {draining_pod}"),
                        cluster.metadata.generation,
                    ));
                }
            }
            Err(error) => status.conditions.push(scale_condition(
                SCALE_ACTION_FAILED_CONDITION,
                "True",
                "AdminActionFailed",
                &error.to_string(),
                cluster.metadata.generation,
            )),
        }
    }

    let pods: Api<Pod> = Api::namespaced(ctx.client.clone(), &namespace);
    let upgrade_plan = if scale_plan.phase == READY_PHASE {
        let upgrade_observation = UpgradeObservation {
            current_replicas: scale_observation.current_replicas,
            ready_replicas: scale_observation.ready_replicas,
            previous_phase: cluster.status.as_ref().map(|status| status.phase.clone()),
            admin_status: scale_observation.admin_status.clone(),
            pods: list_upgrade_pods(&pods, &cluster.name_any()).await?,
        };
        plan_upgrade(&cluster, &upgrade_observation)
    } else {
        UpgradePlan {
            phase: READY_PHASE,
            conditions: vec![upgrade_deferred_for_lifecycle(cluster.metadata.generation)],
            admin_actions: Vec::new(),
            delete_pod: None,
        }
    };
    if scale_plan.phase == READY_PHASE {
        status.phase = upgrade_plan.phase.to_owned();
    }
    status.conditions.extend(upgrade_plan.conditions.clone());
    if !upgrade_plan.admin_actions.is_empty() {
        match ctx
            .scale_admin
            .perform(&namespace, &cluster.name_any(), &upgrade_plan.admin_actions)
            .await
        {
            Ok(()) => {
                if let Some(pod_name) = upgrade_plan.delete_pod.as_deref() {
                    if let Err(error) = pods.delete(pod_name, &DeleteParams::default()).await {
                        status.conditions.push(scale_condition(
                            UPGRADE_FAILED_CONDITION,
                            "True",
                            "PodDeleteFailed",
                            &error.to_string(),
                            cluster.metadata.generation,
                        ));
                    }
                }
            }
            Err(error) => status.conditions.push(scale_condition(
                UPGRADE_ACTION_FAILED_CONDITION,
                "True",
                "AdminActionFailed",
                &error.to_string(),
                cluster.metadata.generation,
            )),
        }
    }

    let tls_rotation_plan = if cluster.spec.tls.is_some() {
        if scale_plan.phase == READY_PHASE && upgrade_plan.phase == READY_PHASE {
            let tls_observation = TlsRotationObservation {
                current_replicas: scale_observation.current_replicas,
                ready_replicas: scale_observation.ready_replicas,
                admin_status: scale_observation.admin_status.clone(),
                secret: tls_secret_observation,
                pods: list_tls_pods(&pods, &cluster.name_any()).await?,
            };
            plan_tls_rotation(&cluster, &tls_observation)
        } else {
            TlsRotationPlan {
                phase: READY_PHASE,
                conditions: vec![tls_deferred_for_lifecycle(cluster.metadata.generation)],
                admin_actions: Vec::new(),
                delete_pod: None,
            }
        }
    } else {
        TlsRotationPlan::steady()
    };
    if scale_plan.phase == READY_PHASE && upgrade_plan.phase == READY_PHASE {
        status.phase = tls_rotation_plan.phase.to_owned();
    }
    status
        .conditions
        .extend(tls_rotation_plan.conditions.clone());
    if !tls_rotation_plan.admin_actions.is_empty() {
        match ctx
            .scale_admin
            .perform(
                &namespace,
                &cluster.name_any(),
                &tls_rotation_plan.admin_actions,
            )
            .await
        {
            Ok(()) => {
                if let Some(pod_name) = tls_rotation_plan.delete_pod.as_deref() {
                    if let Err(error) = pods.delete(pod_name, &DeleteParams::default()).await {
                        status.conditions.push(scale_condition(
                            TLS_ROTATION_FAILED_CONDITION,
                            "True",
                            "PodDeleteFailed",
                            &error.to_string(),
                            cluster.metadata.generation,
                        ));
                    }
                }
            }
            Err(error) => status.conditions.push(scale_condition(
                TLS_ROTATION_ACTION_FAILED_CONDITION,
                "True",
                "AdminActionFailed",
                &error.to_string(),
                cluster.metadata.generation,
            )),
        }
    }

    let backup_plan = if cluster.spec.backup_schedule.is_some() {
        if scale_plan.phase == READY_PHASE
            && upgrade_plan.phase == READY_PHASE
            && tls_rotation_plan.phase == READY_PHASE
        {
            let backup_observation = BackupObservation {
                phase: status.phase.clone(),
                health: status.health.clone(),
                ready_replicas: scale_observation.ready_replicas,
                last_backup: status.last_backup.clone(),
            };
            plan_backup(&cluster, &backup_observation)
        } else {
            BackupPlan::deferred(cluster.metadata.generation)
        }
    } else {
        BackupPlan::steady()
    };
    status.conditions.extend(backup_plan.conditions.clone());
    if !backup_plan.admin_actions.is_empty() {
        match ctx
            .scale_admin
            .perform(&namespace, &cluster.name_any(), &backup_plan.admin_actions)
            .await
        {
            Ok(()) => {
                if backup_plan.record_last_backup_on_success {
                    let completed_at = k8s_openapi::jiff::Timestamp::now().to_string();
                    status.last_backup = Some(completed_at.clone());
                    status.conditions.push(backup_completed_condition(
                        &completed_at,
                        cluster.metadata.generation,
                    ));
                }
            }
            Err(error) => {
                status.health = DEGRADED_HEALTH.to_owned();
                status.conditions.push(backup_failed_condition(
                    &error.to_string(),
                    cluster.metadata.generation,
                ));
            }
        }
    }

    patch_status(&cluster, ctx.client.clone(), status).await?;

    Ok(Action::requeue(Duration::from_secs(30)))
}

pub async fn cleanup_cluster(
    cluster: Arc<HydraCacheCluster>,
    ctx: Arc<Ctx>,
) -> Result<Action, Error> {
    let namespace = cluster
        .namespace()
        .ok_or_else(|| Error::MissingNamespace(cluster.name_any()))?;
    let cleanup = cleanup_plan(&cluster);

    if cleanup.delete_pvcs {
        let selector = cleanup
            .pvc_selector
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join(",");
        let pvcs: Api<PersistentVolumeClaim> = Api::namespaced(ctx.client.clone(), &namespace);
        let listed = pvcs.list(&ListParams::default().labels(&selector)).await?;
        for pvc in listed {
            pvcs.delete(&pvc.name_any(), &DeleteParams::default())
                .await?;
        }
    }

    Ok(Action::await_change())
}

pub fn observed_status(
    cluster: &HydraCacheCluster,
    stateful_set: Option<&StatefulSet>,
) -> HydraCacheClusterStatus {
    let observed_replicas = stateful_set
        .and_then(|sts| sts.status.as_ref())
        .map(|status| status.ready_replicas.unwrap_or(status.replicas))
        .map(|replicas| replicas.max(0) as u32)
        .unwrap_or(0);
    let desired = cluster.spec.replicas;
    HydraCacheClusterStatus {
        observed_replicas,
        bootstrap_replicas: bootstrap_replicas(cluster, stateful_set),
        leader: None,
        health: if observed_replicas == 0 {
            FORMING_HEALTH.to_owned()
        } else if observed_replicas >= desired {
            HEALTHY_HEALTH.to_owned()
        } else {
            DEGRADED_HEALTH.to_owned()
        },
        phase: READY_PHASE.to_owned(),
        last_backup: cluster
            .status
            .as_ref()
            .and_then(|status| status.last_backup.clone()),
        conditions: vec![condition(
            "Reconciled",
            "True",
            "ResourcesApplied",
            "Owned Kubernetes resources were applied with server-side apply",
            cluster.metadata.generation,
        )],
    }
}

pub async fn patch_status(
    cluster: &HydraCacheCluster,
    client: Client,
    status: HydraCacheClusterStatus,
) -> Result<(), Error> {
    let namespace = cluster
        .namespace()
        .ok_or_else(|| Error::MissingNamespace(cluster.name_any()))?;
    let clusters: Api<HydraCacheCluster> = Api::namespaced(client, &namespace);
    clusters
        .patch_status(
            &cluster.name_any(),
            &PatchParams::apply(FIELD_MANAGER).force(),
            &Patch::Apply(json!({
                "apiVersion": "hydracache.io/v1alpha1",
                "kind": "HydraCacheCluster",
                "status": status,
            })),
        )
        .await?;
    Ok(())
}

fn bootstrap_replicas(
    cluster: &HydraCacheCluster,
    stateful_set: Option<&StatefulSet>,
) -> Option<u32> {
    cluster
        .status
        .as_ref()
        .and_then(|status| status.bootstrap_replicas)
        .or_else(|| stateful_set.and_then(statefulset_baseline_replicas))
}

fn statefulset_baseline_replicas(stateful_set: &StatefulSet) -> Option<u32> {
    stateful_set
        .status
        .as_ref()
        .map(|status| status.replicas)
        .filter(|replicas| *replicas > 0)
        .or_else(|| {
            stateful_set
                .spec
                .as_ref()
                .and_then(|spec| spec.replicas)
                .filter(|replicas| *replicas > 0)
        })
        .map(|replicas| replicas as u32)
}

pub fn immutable_change_condition(field: &'static str, generation: Option<i64>) -> Condition {
    condition(
        "ReconcileBlocked",
        "True",
        "ImmutableStatefulSetField",
        &format!("StatefulSet immutable field changed: {field}"),
        generation,
    )
}

pub fn validate_statefulset_update(
    existing: &StatefulSet,
    desired: &StatefulSet,
) -> Result<(), Error> {
    let existing_spec = existing.spec.as_ref();
    let desired_spec = desired.spec.as_ref();

    if existing_spec.and_then(|spec| spec.service_name.as_deref())
        != desired_spec.and_then(|spec| spec.service_name.as_deref())
    {
        return Err(Error::ImmutableField {
            field: "spec.serviceName",
        });
    }

    if existing_spec.and_then(volume_template_fingerprint)
        != desired_spec.and_then(volume_template_fingerprint)
    {
        return Err(Error::ImmutableField {
            field: "spec.volumeClaimTemplates",
        });
    }

    Ok(())
}

pub async fn holds_leader_lease(
    client: Client,
    cluster: &HydraCacheCluster,
    identity: &str,
) -> Result<bool, Error> {
    let namespace = cluster
        .namespace()
        .ok_or_else(|| Error::MissingNamespace(cluster.name_any()))?;
    let leases: Api<Lease> = Api::namespaced(client, &namespace);
    let name = lease_name(cluster);

    match leases.get(&name).await {
        Ok(lease) => Ok(is_leader(identity, &lease)),
        Err(kube::Error::Api(error)) if error.code == 404 => {
            let lease = operator_lease_for_cluster(cluster, identity);
            match leases.create(&PostParams::default(), &lease).await {
                Ok(created) => Ok(is_leader(identity, &created)),
                Err(kube::Error::Api(error)) if error.code == 409 => Ok(false),
                Err(error) => Err(Error::Kube(error)),
            }
        }
        Err(error) => Err(Error::Kube(error)),
    }
}

pub fn lease_name(cluster: &HydraCacheCluster) -> String {
    format!("{}-operator", cluster.name_any())
}

pub fn operator_lease_for_cluster(cluster: &HydraCacheCluster, identity: &str) -> Lease {
    Lease {
        metadata: kube::core::ObjectMeta {
            name: Some(lease_name(cluster)),
            namespace: cluster.namespace(),
            owner_references: crate::resources::owner_reference(cluster).map(|owner| vec![owner]),
            ..Default::default()
        },
        spec: Some(k8s_openapi::api::coordination::v1::LeaseSpec {
            holder_identity: Some(identity.to_owned()),
            lease_duration_seconds: Some(15),
            ..Default::default()
        }),
    }
}

pub fn is_leader(identity: &str, lease: &Lease) -> bool {
    lease
        .spec
        .as_ref()
        .and_then(|spec| spec.holder_identity.as_deref())
        .is_some_and(|holder| holder == identity)
}

fn volume_template_fingerprint(
    spec: &k8s_openapi::api::apps::v1::StatefulSetSpec,
) -> Option<Vec<String>> {
    spec.volume_claim_templates.as_ref().map(|templates| {
        templates
            .iter()
            .map(|claim| {
                let name = claim.metadata.name.clone().unwrap_or_default();
                let storage_class = claim
                    .spec
                    .as_ref()
                    .and_then(|spec| spec.storage_class_name.clone())
                    .unwrap_or_default();
                let size = claim
                    .spec
                    .as_ref()
                    .and_then(|spec| spec.resources.as_ref())
                    .and_then(|resources| resources.requests.as_ref())
                    .and_then(|requests| requests.get("storage"))
                    .map(|quantity| quantity.0.clone())
                    .unwrap_or_default();
                format!("{name}:{storage_class}:{size}")
            })
            .collect()
    })
}

fn condition(
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

async fn get_optional<K>(api: &Api<K>, name: &str) -> Result<Option<K>, kube::Error>
where
    K: Clone + serde::de::DeserializeOwned + kube::Resource + std::fmt::Debug,
    <K as kube::Resource>::DynamicType: Default,
{
    match api.get(name).await {
        Ok(object) => Ok(Some(object)),
        Err(kube::Error::Api(error)) if error.code == 404 => Ok(None),
        Err(error) => Err(error),
    }
}

async fn list_upgrade_pods(
    api: &Api<Pod>,
    cluster_name: &str,
) -> Result<Vec<PodObservation>, Error> {
    let selector = pod_selector_labels(cluster_name)
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join(",");
    let listed = api.list(&ListParams::default().labels(&selector)).await?;
    Ok(listed
        .items
        .iter()
        .filter_map(|pod| PodObservation::from_pod(cluster_name, pod))
        .collect())
}

async fn list_tls_pods(
    api: &Api<Pod>,
    cluster_name: &str,
) -> Result<Vec<TlsPodObservation>, Error> {
    let selector = pod_selector_labels(cluster_name)
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join(",");
    let listed = api.list(&ListParams::default().labels(&selector)).await?;
    Ok(listed
        .items
        .iter()
        .filter_map(|pod| TlsPodObservation::from_pod(cluster_name, pod))
        .collect())
}

async fn observe_tls_secret(
    api: &Api<Secret>,
    cluster: &HydraCacheCluster,
) -> Result<TlsSecretObservation, Error> {
    let Some(tls) = cluster.spec.tls.as_ref() else {
        return Ok(TlsSecretObservation::disabled());
    };
    let secret = get_optional(api, &tls.secret_name).await?;
    Ok(TlsSecretObservation::from_secret(
        &tls.secret_name,
        secret.as_ref(),
    ))
}
