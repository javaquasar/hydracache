//! Signed collector requirements only, never a descriptor, unit or start capability.
use super::{
    bounded_sorted, controller_digest, line, unit_projection, CheckedWorkerPolicy, IntentCheck,
    IntentProjectionBody, IntentProjectionError, ReservedProjection, WorkerPolicyError,
    MAX_INTENT_BODY_BYTES, MAX_SIGNED_INTENT_BYTES,
};
use crate::diagnostic_lease::{DiagnosticState, MAX_RECEIPT_BYTES};
use crate::diagnostic_receipts::STREAM_BYTES;
use crate::diagnostic_unit::build_diagnostic_unit_spec;
use crate::systemd_unit::UnitProperty;
use crate::{is_hash, sha256_hex};
use serde::{Deserialize, Serialize};
use serde_json::json;

pub const COLLECTOR_INTENT_SCHEMA: &str = "diagnostic-collector-intent-074-v1";
pub const SIGNED_COLLECTOR_INTENT_SCHEMA: &str = "diagnostic-signed-collector-intent-074-v1";
pub const COLLECTOR_UNIT_SCHEMA: &str = "diagnostic-collector-unit-projection-074-v1";
pub const COLLECTOR_INTENT_DOMAIN: &[u8] = b"hydracache-diagnostic-collector-intent-074-v1";

/// Unverified envelope. Reused body fields do not imply legacy compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedCollectorIntentProjection {
    pub schema_version: String,
    pub intent: IntentProjectionBody,
    pub signature_hex: String,
}

/// Canonical semantic requirements, deliberately not a TransientUnitSpec.
/// No raw descriptor number, caller path or execution conversion is accepted.
pub fn collector_unit_projection(
    state: &DiagnosticState,
) -> Result<Vec<u8>, IntentProjectionError> {
    // Reuse the exact existing Reserved/lease grammar, not a more permissive copy.
    super::project_reserved_state(state)?;
    let mut spec = build_diagnostic_unit_spec(state).map_err(|_| IntentProjectionError::State)?;
    spec.properties.retain(|(name, _)| {
        !matches!(
            *name,
            "StandardOutputFileToAppend" | "StandardErrorFileToAppend"
        )
    });
    for (name, value) in &mut spec.properties {
        match *name {
            "Description" => {
                *value = UnitProperty::Text(
                    "HydraCache collector diagnostic requirements; not qualification".into(),
                )
            }
            "WorkingDirectory" => *value = UnitProperty::Text("/".into()),
            "ReadWritePaths" => *value = UnitProperty::Strings(Vec::new()),
            "LimitFSIZE" => *value = UnitProperty::Unsigned(0),
            _ => (),
        }
    }
    line(&json!({
        "schema_version": COLLECTOR_UNIT_SCHEMA,
        "worker": unit_projection(&spec),
        "output": {
            "transport": "original-owned-pipes",
            "slots": [["stdout", "collector-stdout.bin"], ["stderr", "collector-stderr.bin"]],
            "sink_namespace": "collector-attempt-v1",
            "sink_owner_uid": 0,
            "sink_directory_mode": "0700",
            "sink_creation": "exclusive-no-follow-no-adoption",
            "evidence_writer": "sole-root-collector",
            "stream_byte_limit": STREAM_BYTES,
            "combined_byte_limit": MAX_RECEIPT_BYTES,
            "completion_requires": [
                "authenticated-original-start",
                "original-cgroup-revoked-and-empty",
                "all-creator-manager-writer-copies-closed",
                "both-kernel-read-eofs-after-drain",
                "exact-byte-and-hash-reconciliation",
                "files-synced-and-readonly",
                "directory-synced",
                "all-evidence-writers-explicitly-closed"
            ]
        }
    }))
}

pub fn project_collector_reserved_state(
    state: &DiagnosticState,
) -> Result<ReservedProjection, IntentProjectionError> {
    Ok(ReservedProjection {
        state_sha256: super::project_reserved_state(state)?.state_sha256,
        unit_spec_sha256: sha256_hex(&collector_unit_projection(state)?),
    })
}

/// Bytes to sign under a distinct domain, not a signature or launch permission.
pub fn collector_intent_signing_message(
    body: &IntentProjectionBody,
) -> Result<Vec<u8>, IntentProjectionError> {
    let raw = line(body)?;
    if body.schema_version != COLLECTOR_INTENT_SCHEMA || raw.len() > MAX_INTENT_BODY_BYTES {
        return Err(IntentProjectionError::Document);
    }
    let mut message = COLLECTOR_INTENT_DOMAIN.to_vec();
    message.push(0);
    message.extend(raw);
    Ok(message)
}

/// Exclusively retained byte verification, not authenticated original start.
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::intent_projection::collector::CheckedCollectorIntentProjection;
/// fn clone(g: CheckedCollectorIntentProjection<'_>) { g.clone(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::intent_projection::collector::CheckedCollectorIntentProjection;
/// fn start(mut g: CheckedCollectorIntentProjection<'_>) { g.start_once(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::intent_projection::{CheckedIntentProjection, collector::CheckedCollectorIntentProjection};
/// fn promote(g: CheckedIntentProjection<'_>) { let _: CheckedCollectorIntentProjection<'_> = g.into(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::intent_projection::collector::CheckedCollectorIntentProjection;
/// fn decode<T: serde::de::DeserializeOwned>() {}
/// decode::<CheckedCollectorIntentProjection<'static>>();
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::intent_projection::collector::CheckedCollectorIntentProjection;
/// fn complete(mut g: CheckedCollectorIntentProjection<'_>) { g.publish_completion(); }
/// ```
pub struct CheckedCollectorIntentProjection<'worker> {
    worker: &'worker mut CheckedWorkerPolicy,
    envelope_sha256: String,
    state: DiagnosticState,
    controller_sha256: String,
    peer_uid: u32,
    peer_gids: Vec<u32>,
    refused: bool,
}
impl CheckedCollectorIntentProjection<'_> {
    pub fn intent_sha256(&self) -> &str {
        &self.envelope_sha256
    }
    pub fn is_refused(&self) -> bool {
        self.refused || self.worker.is_refused()
    }
    pub fn revalidate(
        &mut self,
        bytes: &[u8],
        check: &IntentCheck<'_>,
    ) -> Result<(), IntentProjectionError> {
        if self.is_refused() {
            self.refused = true;
            return Err(IntentProjectionError::Refused);
        }
        let result = (|| {
            if controller_digest(check.controller_policy)? != self.controller_sha256
                || check.peer.uid != self.peer_uid
                || check.peer.gids != self.peer_gids
            {
                return Err(IntentProjectionError::Drift);
            }
            if check.state != &self.state {
                return Err(IntentProjectionError::Binding);
            }
            let body = authenticate(bytes, check)?;
            if sha256_hex(bytes) != self.envelope_sha256 {
                return Err(IntentProjectionError::Binding);
            }
            bind_worker(self.worker, &body, check)
        })();
        if result.is_err() {
            self.refused = true;
        }
        result
    }
}

pub fn verify_collector_intent_projection<'worker>(
    original: &'worker mut CheckedWorkerPolicy,
    bytes: &[u8],
    check: &IntentCheck<'_>,
) -> Result<CheckedCollectorIntentProjection<'worker>, IntentProjectionError> {
    if original.is_refused() {
        return Err(WorkerPolicyError::Refused.into());
    }
    let body = authenticate(bytes, check)?;
    bind_worker(original, &body, check)?;
    Ok(CheckedCollectorIntentProjection {
        worker: original,
        envelope_sha256: sha256_hex(bytes),
        state: check.state.clone(),
        controller_sha256: controller_digest(check.controller_policy)?,
        peer_uid: check.peer.uid,
        peer_gids: check.peer.gids.clone(),
        refused: false,
    })
}

// Original kernel-worker owner brackets this private byte check; no second guard.
pub(crate) fn capture_original(
    worker: &mut CheckedWorkerPolicy,
    bytes: &[u8],
    check: &IntentCheck<'_>,
) -> Result<IntentProjectionBody, IntentProjectionError> {
    if worker.is_refused() {
        return Err(WorkerPolicyError::Refused.into());
    }
    let body = authenticate(bytes, check)?;
    bind_worker(worker, &body, check)?;
    Ok(body)
}

fn authenticate(
    bytes: &[u8],
    check: &IntentCheck<'_>,
) -> Result<IntentProjectionBody, IntentProjectionError> {
    controller_digest(check.controller_policy)?;
    let peer = check.peer;
    let policy = check.controller_policy;
    if !super::super::valid_id(peer.uid)
        || !bounded_sorted(&peer.gids, |id| super::super::valid_id(*id))
        || !policy.client_uids.contains(&peer.uid)
        || !peer.gids.contains(&policy.client_gid)
    {
        return Err(IntentProjectionError::Principal);
    }
    if bytes.is_empty() || bytes.len() > MAX_SIGNED_INTENT_BYTES {
        return Err(IntentProjectionError::Document);
    }
    let document: SignedCollectorIntentProjection =
        serde_json::from_slice(bytes).map_err(|_| IntentProjectionError::Document)?;
    if document.schema_version != SIGNED_COLLECTOR_INTENT_SCHEMA
        || document.intent.schema_version != COLLECTOR_INTENT_SCHEMA
        || line(&document)? != bytes
    {
        return Err(IntentProjectionError::Document);
    }
    let body = document.intent;
    let signature = super::super::signature(&document.signature_hex)
        .map_err(|_| IntentProjectionError::Document)?;
    policy
        .verifying_key
        .verify_strict(&collector_intent_signing_message(&body)?, &signature)
        .map_err(|_| IntentProjectionError::Signature)?;
    if body.controller.repository_id != policy.repository_id
        || body.controller.run_id == 0
        || body.controller.run_attempt == 0
        || !policy.actor_ids.contains(&body.controller.actor_id)
    {
        return Err(IntentProjectionError::Principal);
    }
    if [
        &body.request_id,
        &body.nonce_sha256,
        &body.state_sha256,
        &body.worker_policy_sha256,
        &body.unit_spec_sha256,
    ]
    .iter()
    .any(|hash| !is_hash(hash))
        || body.worker_policy_epoch == 0
    {
        return Err(IntentProjectionError::Document);
    }
    if body.issued_at_unix_seconds == 0
        || body.expires_at_unix_seconds <= body.issued_at_unix_seconds
        || body.expires_at_unix_seconds - body.issued_at_unix_seconds > 60
        || check.now_unix_seconds < body.issued_at_unix_seconds
        || check.now_unix_seconds >= body.expires_at_unix_seconds
    {
        return Err(IntentProjectionError::Time);
    }
    Ok(body)
}

fn bind_worker(
    worker: &mut CheckedWorkerPolicy,
    body: &IntentProjectionBody,
    check: &IntentCheck<'_>,
) -> Result<(), IntentProjectionError> {
    worker.revalidate(check.worker_bytes, check.worker_trust, check.worker_host)?;
    let projection = project_collector_reserved_state(check.state)?;
    if check.state.identity.boot_id != worker.context.boot_id
        || body.boot_id != worker.context.boot_id
        || body.worker_policy_sha256 != worker.trust.policy_sha256
        || body.worker_policy_epoch != worker.original.policy_epoch
        || body.state_sha256 != projection.state_sha256
        || body.unit_spec_sha256 != projection.unit_spec_sha256
    {
        return Err(IntentProjectionError::Binding);
    }
    Ok(())
}
