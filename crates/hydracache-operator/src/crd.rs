//! `HydraCacheCluster` custom resource definition.

use k8s_openapi::api::core::v1::ResourceRequirements;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::Condition;
use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Kubernetes API group for HydraCache custom resources.
pub const HYDRACACHE_GROUP: &str = "hydracache.io";
/// Kubernetes API version for the 0.56 operator.
pub const HYDRACACHE_VERSION: &str = "v1alpha1";
/// CRD plural name.
pub const HYDRACACHE_CLUSTER_PLURAL: &str = "hydracacheclusters";
/// CRD fully qualified name.
pub const HYDRACACHE_CLUSTER_CRD_NAME: &str = "hydracacheclusters.hydracache.io";

/// Desired HydraCache cluster state.
#[allow(clippy::duplicated_attributes)]
#[derive(CustomResource, Serialize, Deserialize, Clone, Debug, JsonSchema, PartialEq)]
#[kube(
    group = "hydracache.io",
    version = "v1alpha1",
    kind = "HydraCacheCluster",
    plural = "hydracacheclusters",
    namespaced,
    status = "HydraCacheClusterStatus",
    shortname = "hcc",
    printcolumn(json_path = ".status.phase", name = "Phase", type_ = "string"),
    printcolumn(json_path = ".status.leader", name = "Leader", type_ = "string"),
    scale(
        spec_replicas_path = ".spec.replicas",
        status_replicas_path = ".status.observedReplicas"
    )
)]
#[serde(rename_all = "camelCase")]
pub struct HydraCacheClusterSpec {
    /// `hydracache-server` image reference.
    #[schemars(length(min = 1))]
    pub image: String,
    /// HydraCache server version. Rolling upgrades reject skipped incompatible jumps later.
    #[schemars(length(min = 1))]
    pub version: String,
    /// Desired member count. Kubernetes OpenAPI rejects zero before reconcile.
    #[schemars(range(min = 1))]
    pub replicas: u32,
    /// Placement regions/zones used by the 0.45 placement model.
    #[schemars(length(min = 1))]
    pub regions: Vec<RegionZone>,
    /// Optional durable value storage/PVC policy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persistence: Option<PersistenceSpec>,
    /// Optional mTLS Secret reference.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<TlsSpec>,
    /// Optional Kubernetes resource requirements for the server container.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourceRequirements>,
    /// Optional backup/PITR schedule.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_schedule: Option<BackupScheduleSpec>,
}

/// Region and zone tuple for placement-aware clusters.
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RegionZone {
    /// Region label.
    #[schemars(length(min = 1))]
    pub region: String,
    /// Zone label inside the region.
    #[schemars(length(min = 1))]
    pub zone: String,
}

/// Durable storage policy mapped to StatefulSet PVC templates.
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PersistenceSpec {
    /// StorageClass used by the data PVC.
    #[schemars(length(min = 1))]
    pub storage_class_name: String,
    /// Requested storage size, for example `20Gi`.
    #[schemars(length(min = 1))]
    pub size: String,
    /// PVC reclaim behavior. Defaults to `Retain` for data safety.
    #[serde(default)]
    pub reclaim_policy: PvcReclaimPolicy,
}

/// PVC reclaim policy.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, JsonSchema, PartialEq, Eq, Default)]
#[serde(rename_all = "PascalCase")]
pub enum PvcReclaimPolicy {
    /// Keep PVCs when a HydraCacheCluster is deleted.
    #[default]
    Retain,
    /// Delete PVCs only when explicitly requested.
    Delete,
}

/// mTLS Secret reference.
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TlsSpec {
    /// Secret containing `tls.crt`, `tls.key`, and `ca.crt`.
    #[schemars(length(min = 1))]
    pub secret_name: String,
}

/// Scheduled backup/PITR policy.
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BackupScheduleSpec {
    /// Cron expression understood by the operator.
    #[schemars(length(min = 1))]
    pub schedule: String,
    /// Backup target location, for example `s3://bucket/prefix` or `file:///backups`.
    #[schemars(length(min = 1))]
    pub location: String,
    /// Retention window such as `168h`.
    #[schemars(length(min = 1))]
    pub retention: String,
}

/// Observed cluster state.
#[derive(Serialize, Deserialize, Clone, Debug, Default, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HydraCacheClusterStatus {
    /// Observed ready/created replicas.
    pub observed_replicas: u32,
    /// Initial bootstrap cohort size used to derive deterministic pod start mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bootstrap_replicas: Option<u32>,
    /// Current server leader if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leader: Option<String>,
    /// Coarse health state: `Healthy`, `Degraded`, or `Forming`.
    pub health: String,
    /// Lifecycle phase: `Ready`, `Scaling`, `Upgrading`, or `Rebalancing`.
    pub phase: String,
    /// Last successful backup timestamp, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_backup: Option<String>,
    /// Kubernetes status conditions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<Condition>,
}

/// Build a minimal valid spec used by tests and examples.
pub fn sample_spec() -> HydraCacheClusterSpec {
    HydraCacheClusterSpec {
        image: "ghcr.io/javaquasar/hydracache-server:0.56.0".to_owned(),
        version: "0.56.0".to_owned(),
        replicas: 3,
        regions: vec![RegionZone {
            region: "local".to_owned(),
            zone: "zone-a".to_owned(),
        }],
        persistence: Some(PersistenceSpec {
            storage_class_name: "standard".to_owned(),
            size: "20Gi".to_owned(),
            reclaim_policy: PvcReclaimPolicy::Retain,
        }),
        tls: Some(TlsSpec {
            secret_name: "hydracache-mtls".to_owned(),
        }),
        resources: None,
        backup_schedule: Some(BackupScheduleSpec {
            schedule: "0 * * * *".to_owned(),
            location: "file:///var/lib/hydracache/backups".to_owned(),
            retention: "168h".to_owned(),
        }),
    }
}
