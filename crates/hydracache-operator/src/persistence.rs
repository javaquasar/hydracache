//! Persistence validation and PVC policy helpers for `HydraCacheCluster`.

use k8s_openapi::apimachinery::pkg::apis::meta::v1::Condition;

use crate::crd::HydraCacheCluster;
use crate::scale::scale_condition;

pub const PERSISTENCE_BLOCKED_CONDITION: &str = "PersistenceBlocked";

/// Deterministic preflight result for the PVC-backed value plane.
#[derive(Clone, Debug, PartialEq)]
pub struct PersistencePlan {
    pub blocked: bool,
    pub conditions: Vec<Condition>,
}

impl PersistencePlan {
    fn ready() -> Self {
        Self {
            blocked: false,
            conditions: Vec::new(),
        }
    }

    fn blocked(reason: &str, message: &str, generation: Option<i64>) -> Self {
        Self {
            blocked: true,
            conditions: vec![scale_condition(
                PERSISTENCE_BLOCKED_CONDITION,
                "True",
                reason,
                message,
                generation,
            )],
        }
    }
}

/// Refuse durable storage specs that cannot bind a PVC loudly before workload apply.
pub fn plan_persistence(cluster: &HydraCacheCluster) -> PersistencePlan {
    let Some(persistence) = cluster.spec.persistence.as_ref() else {
        return PersistencePlan::ready();
    };

    if persistence.storage_class_name.trim().is_empty() {
        return PersistencePlan::blocked(
            "MissingStorageClass",
            "refusing durable persistence without spec.persistence.storageClassName",
            cluster.metadata.generation,
        );
    }

    if persistence.size.trim().is_empty() {
        return PersistencePlan::blocked(
            "MissingStorageSize",
            "refusing durable persistence without spec.persistence.size",
            cluster.metadata.generation,
        );
    }

    PersistencePlan::ready()
}
