//! Level-triggered reconcile loop for `HydraCacheCluster`.

use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt;
use k8s_openapi::api::apps::v1::StatefulSet;
use k8s_openapi::api::coordination::v1::Lease;
use k8s_openapi::api::core::v1::{PersistentVolumeClaim, Secret, Service};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{Condition, Time};
use kube::api::{Api, DeleteParams, ListParams, Patch, PatchParams, PostParams};
use kube::runtime::controller::Action;
use kube::runtime::finalizer::{finalizer, Event as FinalizerEvent};
use kube::runtime::{watcher, Controller};
use kube::{Client, ResourceExt};
use serde_json::json;
use thiserror::Error;

use crate::crd::{HydraCacheCluster, HydraCacheClusterStatus};
use crate::resources::{cleanup_plan, headless_service_name, OwnedResources, FIELD_MANAGER};

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
}

impl Ctx {
    pub fn new(client: Client, identity: impl Into<String>, namespace: Option<String>) -> Self {
        Self {
            client,
            identity: identity.into(),
            namespace,
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
    let desired = OwnedResources::build(&cluster);
    let apply = PatchParams::apply(FIELD_MANAGER).force();

    let statefulsets: Api<StatefulSet> = Api::namespaced(ctx.client.clone(), &namespace);
    if let Some(existing) = get_optional(&statefulsets, &cluster.name_any()).await? {
        validate_statefulset_update(&existing, &desired.stateful_set)?;
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

    let secrets: Api<Secret> = Api::namespaced(ctx.client.clone(), &namespace);
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

    patch_status(
        &cluster,
        ctx.client.clone(),
        observed_status(&cluster, Some(&desired.stateful_set)),
    )
    .await?;

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
