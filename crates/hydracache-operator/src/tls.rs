//! mTLS Secret observation and rotation planning for `HydraCacheCluster`.

use k8s_openapi::api::core::v1::{Pod, Secret};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::Condition;
use kube::ResourceExt;

use crate::controller::READY_PHASE;
use crate::crd::HydraCacheCluster;
use crate::scale::{
    pod_name, quorum_for, scale_condition, AdminAction, AdminStatus, SCALE_PROGRESSING_CONDITION,
};
use crate::upgrade::{pod_ordinal, UPGRADING_PHASE};

pub const TLS_SECRET_NAME_ANNOTATION: &str = "hydracache.io/tls-secret-name";
pub const TLS_SECRET_FINGERPRINT_ANNOTATION: &str = "hydracache.io/tls-secret-fingerprint";
pub const TLS_ROTATION_PROGRESSING_CONDITION: &str = "TlsRotationProgressing";
pub const TLS_ROTATION_BLOCKED_CONDITION: &str = "TlsRotationBlocked";
pub const TLS_ROTATION_FAILED_CONDITION: &str = "TlsRotationFailed";
pub const TLS_ROTATION_ACTION_FAILED_CONDITION: &str = "TlsRotationActionFailed";
pub const TLS_REQUIRED_SECRET_KEYS: [&str; 3] = ["tls.crt", "tls.key", "ca.crt"];

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0001_0000_01b3;

/// Observed TLS Secret material used to stamp pod templates and decide rotation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TlsSecretObservation {
    pub name: Option<String>,
    pub exists: bool,
    pub fingerprint: Option<String>,
    pub missing_keys: Vec<String>,
}

impl TlsSecretObservation {
    pub fn disabled() -> Self {
        Self {
            name: None,
            exists: false,
            fingerprint: None,
            missing_keys: Vec::new(),
        }
    }

    pub fn from_secret(name: &str, secret: Option<&Secret>) -> Self {
        let Some(secret) = secret else {
            return Self {
                name: Some(name.to_owned()),
                exists: false,
                fingerprint: None,
                missing_keys: Vec::new(),
            };
        };

        let missing_keys = TLS_REQUIRED_SECRET_KEYS
            .iter()
            .filter(|key| secret_value(secret, key).is_none())
            .map(|key| (*key).to_owned())
            .collect::<Vec<_>>();
        let fingerprint = missing_keys.is_empty().then(|| secret_fingerprint(secret));

        Self {
            name: Some(name.to_owned()),
            exists: true,
            fingerprint,
            missing_keys,
        }
    }
}

/// Preflight result for a referenced TLS Secret.
#[derive(Clone, Debug, PartialEq)]
pub struct TlsSecretPlan {
    pub blocked: bool,
    pub fingerprint: Option<String>,
    pub conditions: Vec<Condition>,
}

impl TlsSecretPlan {
    fn ready(fingerprint: Option<String>) -> Self {
        Self {
            blocked: false,
            fingerprint,
            conditions: Vec::new(),
        }
    }

    fn blocked(reason: &str, message: &str, generation: Option<i64>) -> Self {
        Self {
            blocked: true,
            fingerprint: None,
            conditions: vec![condition(
                TLS_ROTATION_BLOCKED_CONDITION,
                reason,
                message,
                generation,
            )],
        }
    }
}

/// Refuse missing or incomplete referenced TLS Secrets loudly before workload apply.
pub fn plan_tls_secret(
    cluster: &HydraCacheCluster,
    observed: &TlsSecretObservation,
) -> TlsSecretPlan {
    let Some(tls) = cluster.spec.tls.as_ref() else {
        return TlsSecretPlan::ready(None);
    };

    if !observed.exists {
        return TlsSecretPlan::blocked(
            "TlsSecretMissing",
            &format!("referenced TLS Secret {} was not found", tls.secret_name),
            cluster.metadata.generation,
        );
    }

    if !observed.missing_keys.is_empty() {
        return TlsSecretPlan::blocked(
            "TlsSecretIncomplete",
            &format!(
                "referenced TLS Secret {} is missing required keys: {}",
                tls.secret_name,
                observed.missing_keys.join(", ")
            ),
            cluster.metadata.generation,
        );
    }

    match observed.fingerprint.clone() {
        Some(fingerprint) => TlsSecretPlan::ready(Some(fingerprint)),
        None => TlsSecretPlan::blocked(
            "TlsSecretFingerprintUnavailable",
            &format!(
                "referenced TLS Secret {} has no usable material",
                tls.secret_name
            ),
            cluster.metadata.generation,
        ),
    }
}

/// Runtime pod TLS state used by the rotation planner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TlsPodObservation {
    pub name: String,
    pub ordinal: u32,
    pub tls_fingerprint: Option<String>,
    pub ready: bool,
    pub deleting: bool,
}

impl TlsPodObservation {
    pub fn from_pod(cluster_name: &str, pod: &Pod) -> Option<Self> {
        let name = pod.name_any();
        let ordinal = pod_ordinal(cluster_name, &name)?;
        let tls_fingerprint =
            pod.metadata.annotations.as_ref().and_then(|annotations| {
                annotations.get(TLS_SECRET_FINGERPRINT_ANNOTATION).cloned()
            });
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
            tls_fingerprint,
            ready,
            deleting: pod.metadata.deletion_timestamp.is_some(),
        })
    }
}

/// Runtime view used by the mTLS rotation planner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TlsRotationObservation {
    pub current_replicas: u32,
    pub ready_replicas: u32,
    pub admin_status: Option<AdminStatus>,
    pub secret: TlsSecretObservation,
    pub pods: Vec<TlsPodObservation>,
}

/// Deterministic rotation decision. The controller applies at most one pod deletion per reconcile.
#[derive(Clone, Debug, PartialEq)]
pub struct TlsRotationPlan {
    pub phase: &'static str,
    pub conditions: Vec<Condition>,
    pub admin_actions: Vec<AdminAction>,
    pub delete_pod: Option<String>,
}

impl TlsRotationPlan {
    pub fn steady() -> Self {
        Self {
            phase: READY_PHASE,
            conditions: Vec::new(),
            admin_actions: Vec::new(),
            delete_pod: None,
        }
    }
}

pub fn plan_tls_rotation(
    cluster: &HydraCacheCluster,
    observed: &TlsRotationObservation,
) -> TlsRotationPlan {
    if cluster.spec.tls.is_none() {
        return TlsRotationPlan::steady();
    }

    let secret_plan = plan_tls_secret(cluster, &observed.secret);
    if secret_plan.blocked {
        return TlsRotationPlan {
            phase: UPGRADING_PHASE,
            conditions: secret_plan.conditions,
            admin_actions: Vec::new(),
            delete_pod: None,
        };
    }
    let Some(desired_fingerprint) = secret_plan.fingerprint.as_deref() else {
        return TlsRotationPlan::steady();
    };
    let generation = cluster.metadata.generation;

    if observed.current_replicas == 0 || observed.pods.is_empty() {
        return TlsRotationPlan::steady();
    }

    let admin_status = match observed.admin_status.as_ref() {
        Some(status) => status,
        None => {
            return blocked(
                "WaitingForAdminStatus",
                "waiting for /admin/status before rotating TLS material",
                generation,
            )
        }
    };

    if !admin_status.quorum_ok || observed.ready_replicas < quorum_for(observed.current_replicas) {
        return blocked(
            "TlsRotationQuorumUnavailable",
            "waiting for quorum before replacing any pod for TLS rotation",
            generation,
        );
    }

    if observed.pods.iter().any(|pod| pod.deleting || !pod.ready) {
        return progressing(
            "WaitingForReadyPod",
            "waiting for the current TLS replacement pod to become Ready before continuing",
            generation,
        );
    }

    let mut outdated = observed
        .pods
        .iter()
        .filter(|pod| pod.tls_fingerprint.as_deref() != Some(desired_fingerprint))
        .collect::<Vec<_>>();
    outdated.sort_by_key(|pod| std::cmp::Reverse(pod.ordinal));

    if outdated.is_empty() {
        return TlsRotationPlan::steady();
    }

    let leader = admin_status.leader.as_deref();
    let selected = outdated
        .iter()
        .copied()
        .find(|pod| Some(pod.name.as_str()) != leader)
        .unwrap_or(outdated[0]);

    if Some(selected.name.as_str()) == leader && observed.current_replicas > 1 {
        return TlsRotationPlan {
            phase: UPGRADING_PHASE,
            conditions: vec![condition(
                TLS_ROTATION_PROGRESSING_CONDITION,
                "TlsLeaderReelectionRequested",
                &format!(
                    "requesting leader drain for {} before deleting the pod for TLS rotation",
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

    TlsRotationPlan {
        phase: UPGRADING_PHASE,
        conditions: vec![condition(
            TLS_ROTATION_PROGRESSING_CONDITION,
            "TlsPodDrainAndReplace",
            &format!(
                "draining and deleting {} for one-at-a-time TLS material rotation",
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

pub fn tls_deferred_for_lifecycle(generation: Option<i64>) -> Condition {
    condition(
        SCALE_PROGRESSING_CONDITION,
        "TlsRotationDeferredForLifecycle",
        "deferring TLS rotation while another lifecycle action is active",
        generation,
    )
}

pub fn expected_pod_name(cluster_name: &str, ordinal: u32) -> String {
    pod_name(cluster_name, ordinal)
}

pub fn condition(type_: &str, reason: &str, message: &str, generation: Option<i64>) -> Condition {
    scale_condition(type_, "True", reason, message, generation)
}

fn progressing(reason: &str, message: &str, generation: Option<i64>) -> TlsRotationPlan {
    TlsRotationPlan {
        phase: UPGRADING_PHASE,
        conditions: vec![condition(
            TLS_ROTATION_PROGRESSING_CONDITION,
            reason,
            message,
            generation,
        )],
        admin_actions: Vec::new(),
        delete_pod: None,
    }
}

fn blocked(reason: &str, message: &str, generation: Option<i64>) -> TlsRotationPlan {
    TlsRotationPlan {
        phase: UPGRADING_PHASE,
        conditions: vec![condition(
            TLS_ROTATION_BLOCKED_CONDITION,
            reason,
            message,
            generation,
        )],
        admin_actions: Vec::new(),
        delete_pod: None,
    }
}

fn secret_fingerprint(secret: &Secret) -> String {
    let mut hash = FNV_OFFSET;
    for key in TLS_REQUIRED_SECRET_KEYS {
        hash = hash_bytes(hash, key.as_bytes());
        hash = hash_bytes(hash, &[0]);
        if let Some(value) = secret_value(secret, key) {
            hash = hash_bytes(hash, &value);
        }
        hash = hash_bytes(hash, &[0xff]);
    }
    format!("{hash:016x}")
}

fn secret_value(secret: &Secret, key: &str) -> Option<Vec<u8>> {
    if let Some(value) = secret.data.as_ref().and_then(|data| data.get(key)) {
        if !value.0.is_empty() {
            return Some(value.0.clone());
        }
    }

    secret
        .string_data
        .as_ref()
        .and_then(|data| data.get(key))
        .filter(|value| !value.is_empty())
        .map(|value| value.as_bytes().to_vec())
}

fn hash_bytes(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}
