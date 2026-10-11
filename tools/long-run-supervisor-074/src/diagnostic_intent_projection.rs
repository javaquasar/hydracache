//! Signed expected bytes only. No durable replay ledger, host proof or start API.
use super::{AssertedWorkerHost, CheckedWorkerPolicy, WorkerPolicyError, WorkerPolicyTrust};
use crate::diagnostic_ipc::{DiagnosticController, DiagnosticPeer, DiagnosticPolicy};
use crate::diagnostic_lease::{
    validate_state, DiagnosticStage, DiagnosticState, CONTROLLER_LOSS_SECONDS,
};
use crate::diagnostic_unit::build_diagnostic_unit_spec;
use crate::systemd_unit::{TransientUnitSpec, UnitProperty};
use crate::{is_hash, sha256_hex};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;

pub const INTENT_SCHEMA: &str = "diagnostic-intent-projection-074-v1";
pub const SIGNED_INTENT_SCHEMA: &str = "diagnostic-signed-intent-projection-074-v1";
pub const INTENT_DOMAIN: &[u8] = b"hydracache-diagnostic-intent-projection-074-v1";
pub const MAX_INTENT_BODY_BYTES: usize = 4096;
pub const MAX_SIGNED_INTENT_BYTES: usize = 8192;
const PRINCIPAL_LIMIT: usize = 32;

/// Unverified wire body. A signature does not authenticate live host state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntentProjectionBody {
    pub schema_version: String,
    pub request_id: String,
    pub nonce_sha256: String,
    pub controller: DiagnosticController,
    pub issued_at_unix_seconds: u64,
    pub expires_at_unix_seconds: u64,
    pub state_sha256: String,
    pub worker_policy_sha256: String,
    pub worker_policy_epoch: u64,
    pub boot_id: String,
    pub unit_spec_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedIntentProjection {
    pub schema_version: String,
    pub intent: IntentProjectionBody,
    pub signature_hex: String,
}

/// Projection of asserted state/configuration, not persisted-state proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReservedProjection {
    pub state_sha256: String,
    pub unit_spec_sha256: String,
}

/// External assertions and pins. Caller must eventually authenticate peer/clock
/// and retain fenced state; this byte verifier does none of those operations.
pub struct IntentCheck<'a> {
    pub state: &'a DiagnosticState,
    pub peer: &'a DiagnosticPeer,
    pub controller_policy: &'a DiagnosticPolicy,
    pub now_unix_seconds: u64,
    pub worker_bytes: &'a [u8],
    pub worker_trust: &'a WorkerPolicyTrust,
    pub worker_host: &'a AssertedWorkerHost,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum IntentProjectionError {
    #[error("intent document is malformed, noncanonical or over budget")]
    Document,
    #[error("intent signature is invalid under the external controller key")]
    Signature,
    #[error("external controller policy or peer/principal is not admitted")]
    Principal,
    #[error("intent is outside its positive bounded validity window")]
    Time,
    #[error("intent requires a valid unexpired asserted Reserved state")]
    State,
    #[error("intent differs from the original state, worker or unit projection")]
    Binding,
    #[error("original controller configuration or peer changed")]
    Drift,
    #[error("original worker policy refused: {0}")]
    Worker(#[from] WorkerPolicyError),
    #[error("original checked intent previously refused")]
    Refused,
}

/// No execution conversion or refresh; exclusively retains the original worker.
///
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::intent_projection::CheckedIntentProjection;
/// fn clone_guard(guard: &CheckedIntentProjection<'_>) { let _ = guard.clone(); }
/// fn require_clone<T: Clone>() {}
/// require_clone::<CheckedIntentProjection<'static>>();
/// ```
///
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::intent_projection::CheckedIntentProjection;
/// fn start(guard: &mut CheckedIntentProjection<'_>) { guard.start_once(); }
/// ```
pub struct CheckedIntentProjection<'worker> {
    worker: &'worker mut CheckedWorkerPolicy,
    envelope_sha256: String,
    state: DiagnosticState,
    controller_sha256: String,
    peer_uid: u32,
    peer_gids: Vec<u32>,
    controller: DiagnosticController,
    refused: bool,
}

impl CheckedIntentProjection<'_> {
    pub(crate) fn latch_refusal(&mut self) {
        self.refused = true;
    }
    /// Historical signature/binding only, never renewal of launch eligibility.
    pub(crate) fn verify_consumed(
        &mut self,
        bytes: &[u8],
        state: &DiagnosticState,
        peer: &DiagnosticPeer,
        check: &IntentCheck<'_>,
    ) -> Result<IntentProjectionBody, IntentProjectionError> {
        if self.is_refused() {
            return Err(IntentProjectionError::Refused);
        }
        if bytes.len() > MAX_SIGNED_INTENT_BYTES {
            return Err(IntentProjectionError::Document);
        }
        if peer.uid != self.peer_uid
            || peer.gids != self.peer_gids
            || controller_digest(check.controller_policy)? != self.controller_sha256
        {
            return Err(IntentProjectionError::Drift);
        }
        let document: SignedIntentProjection =
            serde_json::from_slice(bytes).map_err(|_| IntentProjectionError::Document)?;
        let historical = IntentCheck {
            state,
            peer,
            now_unix_seconds: document.intent.issued_at_unix_seconds,
            ..*check
        };
        let body = authenticate(bytes, &historical)?;
        if body.controller != self.controller {
            return Err(IntentProjectionError::Binding);
        }
        bind_worker(self.worker, &body, &historical)?;
        Ok(body)
    }
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

pub fn verify_intent_projection<'worker>(
    original: &'worker mut CheckedWorkerPolicy,
    bytes: &[u8],
    check: &IntentCheck<'_>,
) -> Result<CheckedIntentProjection<'worker>, IntentProjectionError> {
    if original.is_refused() {
        return Err(WorkerPolicyError::Refused.into());
    }
    let body = authenticate(bytes, check)?;
    bind_worker(original, &body, check)?;
    Ok(CheckedIntentProjection {
        worker: original,
        envelope_sha256: sha256_hex(bytes),
        state: check.state.clone(),
        controller_sha256: controller_digest(check.controller_policy)?,
        peer_uid: check.peer.uid,
        peer_gids: check.peer.gids.clone(),
        controller: body.controller,
        refused: false,
    })
}

pub fn project_reserved_state(
    state: &DiagnosticState,
) -> Result<ReservedProjection, IntentProjectionError> {
    validate_state(state).map_err(|_| IntentProjectionError::State)?;
    if state.stage != DiagnosticStage::Reserved
        || state.last_observed_monotonic_ns - state.controller_monotonic_ns
            >= CONTROLLER_LOSS_SECONDS * 1_000_000_000
    {
        return Err(IntentProjectionError::State);
    }
    let spec = build_diagnostic_unit_spec(state).map_err(|_| IntentProjectionError::State)?;
    Ok(ReservedProjection {
        state_sha256: sha256_hex(&line(state)?),
        unit_spec_sha256: sha256_hex(&line(&unit_projection(&spec))?),
    })
}

/// Constructs bytes to sign, not a signature or execution authorization.
pub fn intent_signing_message(
    body: &IntentProjectionBody,
) -> Result<Vec<u8>, IntentProjectionError> {
    let bytes = line(body)?;
    if bytes.len() > MAX_INTENT_BODY_BYTES {
        return Err(IntentProjectionError::Document);
    }
    let mut message = INTENT_DOMAIN.to_vec();
    message.push(0);
    message.extend(bytes);
    Ok(message)
}

fn authenticate(
    bytes: &[u8],
    check: &IntentCheck<'_>,
) -> Result<IntentProjectionBody, IntentProjectionError> {
    controller_digest(check.controller_policy)?;
    let peer = check.peer;
    let policy = check.controller_policy;
    if !super::valid_id(peer.uid)
        || !bounded_sorted(&peer.gids, |id| super::valid_id(*id))
        || !policy.client_uids.contains(&peer.uid)
        || !peer.gids.contains(&policy.client_gid)
    {
        return Err(IntentProjectionError::Principal);
    }
    if bytes.is_empty() || bytes.len() > MAX_SIGNED_INTENT_BYTES {
        return Err(IntentProjectionError::Document);
    }
    let document: SignedIntentProjection =
        serde_json::from_slice(bytes).map_err(|_| IntentProjectionError::Document)?;
    if document.schema_version != SIGNED_INTENT_SCHEMA
        || document.intent.schema_version != INTENT_SCHEMA
        || line(&document)? != bytes
    {
        return Err(IntentProjectionError::Document);
    }
    let body = document.intent;
    let sig =
        super::signature(&document.signature_hex).map_err(|_| IntentProjectionError::Document)?;
    policy
        .verifying_key
        .verify_strict(&intent_signing_message(&body)?, &sig)
        .map_err(|_| IntentProjectionError::Signature)?;
    if body.controller.repository_id != policy.repository_id
        || body.controller.run_id == 0
        || body.controller.run_attempt == 0
        || !policy.actor_ids.contains(&body.controller.actor_id)
    {
        return Err(IntentProjectionError::Principal);
    }
    if ![
        &body.request_id,
        &body.nonce_sha256,
        &body.state_sha256,
        &body.worker_policy_sha256,
        &body.unit_spec_sha256,
    ]
    .into_iter()
    .all(|s| is_hash(s))
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
    bind_projection(worker, body, check)
}

fn bind_projection(
    worker: &CheckedWorkerPolicy,
    body: &IntentProjectionBody,
    check: &IntentCheck<'_>,
) -> Result<(), IntentProjectionError> {
    let projection = project_reserved_state(check.state)?;
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

// Private immutable data only. The original runtime authority must bracket its
// use; this helper cannot mint a checked worker, runtime object or start permit.
#[derive(PartialEq, Eq)]
pub(crate) struct BoundIntentPins {
    envelope_sha256: String,
    state: DiagnosticState,
    controller_sha256: String,
    controller: DiagnosticController,
    peer_uid: u32,
    peer_gids: Vec<u32>,
}
pub(crate) fn capture_bound_intent(
    worker: &CheckedWorkerPolicy,
    bytes: &[u8],
    check: &IntentCheck<'_>,
) -> Result<BoundIntentPins, IntentProjectionError> {
    if worker.is_refused() {
        return Err(WorkerPolicyError::Refused.into());
    }
    if check.worker_trust != &worker.trust
        || check.worker_host != &worker.context
        || check.worker_bytes.len() > super::MAX_SIGNED_WORKER_POLICY_BYTES
        || sha256_hex(check.worker_bytes) != worker.envelope_sha256
    {
        return Err(IntentProjectionError::Binding);
    }
    let body = authenticate(bytes, check)?;
    bind_projection(worker, &body, check)?;
    Ok(BoundIntentPins {
        envelope_sha256: sha256_hex(bytes),
        state: check.state.clone(),
        controller_sha256: controller_digest(check.controller_policy)?,
        controller: body.controller,
        peer_uid: check.peer.uid,
        peer_gids: check.peer.gids.clone(),
    })
}

fn bounded_sorted<T: Ord>(values: &[T], valid: impl Fn(&T) -> bool) -> bool {
    !values.is_empty()
        && values.len() <= PRINCIPAL_LIMIT
        && values.iter().all(valid)
        && values.windows(2).all(|v| v[0] < v[1])
}

fn controller_digest(policy: &DiagnosticPolicy) -> Result<String, IntentProjectionError> {
    if policy.repository_id != super::REPOSITORY_ID
        || policy.verifying_key.is_weak()
        || !super::valid_id(policy.client_gid)
        || !bounded_sorted(&policy.actor_ids, |id| *id != 0)
        || !bounded_sorted(&policy.client_uids, |id| super::valid_id(*id))
    {
        return Err(IntentProjectionError::Principal);
    }
    Ok(sha256_hex(&line(&json!({
        "repository_id": policy.repository_id, "actor_ids": policy.actor_ids,
        "client_uids": policy.client_uids, "client_gid": policy.client_gid,
        "controller_key": policy.verifying_key.as_bytes(),
    }))?))
}

fn line(value: &impl Serialize) -> Result<Vec<u8>, IntentProjectionError> {
    super::canonical_line(value).map_err(|_| IntentProjectionError::Document)
}

fn unit_projection(spec: &TransientUnitSpec) -> Value {
    let properties: Vec<_> =
        spec.properties
            .iter()
            .map(|(name, property)| {
                let (tag, value) =
                    match property {
                        UnitProperty::Text(v) => ("text", json!(v)),
                        UnitProperty::Boolean(v) => ("boolean", json!(v)),
                        UnitProperty::Unsigned(v) => ("unsigned", json!(v)),
                        UnitProperty::Bytes(v) => ("bytes", json!(v)),
                        UnitProperty::Strings(v) => ("strings", json!(v)),
                        UnitProperty::Commands(v) => (
                            "commands",
                            json!(v.iter().map(|c| json!({
                "path": c.path, "argv": c.argv, "ignore_failure": c.ignore_failure,
            })).collect::<Vec<_>>()),
                        ),
                    };
                json!([name, { "type": tag, "value": value }])
            })
            .collect();
    json!({ "unit_name": spec.unit_name, "properties": properties })
}

#[cfg(test)]
#[path = "diagnostic_intent_projection_tests.rs"]
mod tests;
