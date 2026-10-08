//! Local diagnostic request handler, deliberately not enrolled in production.
//! No live backend, listener, key provisioning or command-line entry point.
use crate::diagnostic_lease::{
    read_active, read_bounded_document, sync_directory, validate_identity, validate_state,
    write_new, DiagnosticBackend, DiagnosticClock, DiagnosticCoordinator, DiagnosticError,
    DiagnosticIdentity, DiagnosticState, SOURCE_COMMIT,
};
use crate::host_execution::{lock_host_root, HostExecutionError};
use crate::{canonical_json, is_hash, sha256_hex};
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use thiserror::Error;

pub const SIGNATURE_DOMAIN: &[u8] = b"hydracache-diagnostic-request-074-v1";
pub const PRESET: &str = "baseline-get-four-cells-p0-v1";
pub const MAX_PACKET_BYTES: usize = 16_384;
pub const LEDGER_NAME: &str = "diagnostic-requests-v1.json";
pub const PENDING_NAME: &str = ".diagnostic-requests.pending";
pub const MAX_LEDGER_BYTES: u64 = 262_144;
pub const MAX_LEDGER_ENTRIES: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticOperation {
    Reserve,
    Heartbeat,
    Status,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticController {
    pub repository_id: u64,
    pub run_id: u64,
    pub run_attempt: u32,
    pub actor_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticRequest {
    pub schema_version: u32,
    pub request_id: String,
    pub nonce_sha256: String,
    pub operation: DiagnosticOperation,
    pub expected_state_revision: u64,
    pub identity: DiagnosticIdentity,
    pub source_commit: String,
    pub preset: String,
    pub controller: DiagnosticController,
    pub issued_at_unix_seconds: u64,
    pub expires_at_unix_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedDiagnosticRequest {
    pub request: DiagnosticRequest,
    pub signature_hex: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticResponse {
    pub schema_version: u32,
    pub request_id: String,
    pub lease_id: String,
    pub ok: bool,
    pub state: Option<DiagnosticState>,
    /// Deliberately bounded, never arbitrary backend text or paths.
    pub error_code: Option<String>,
}

pub struct DiagnosticPolicy {
    pub repository_id: u64,
    pub actor_ids: Vec<u64>,
    pub client_uids: Vec<u32>,
    pub client_gid: u32,
    pub verifying_key: VerifyingKey,
}

/// Explicit portable test seam. Linux callers must obtain these from the kernel,
/// not JSON. No production caller currently enrolls this handler.
pub struct DiagnosticPeer {
    pub uid: u32,
    pub gids: Vec<u32>,
}

#[derive(Debug, Error)]
pub enum DiagnosticIpcError {
    #[error("diagnostic packet is malformed, oversized or outside its fixed preset")]
    Packet,
    #[error("diagnostic signature is invalid")]
    Signature,
    #[error("diagnostic authorization is outside its 60-second window")]
    Time,
    #[error("diagnostic peer or signed principal is not allowlisted")]
    Principal,
    #[error("diagnostic request id or nonce conflicts with retained history")]
    Replay,
    #[error("diagnostic request outcome is uncertain; explicit reconciliation is required")]
    Uncertain,
    #[error("diagnostic request does not match the original lease/controller")]
    Binding,
    #[error("diagnostic expected revision or clock is stale")]
    Revision,
    #[error("diagnostic request ledger is unsafe or corrupt")]
    Ledger,
    #[error("diagnostic request ledger capacity exhausted; no eviction allowed")]
    Capacity,
    #[error("diagnostic model refused the transaction: {0}")]
    Model(#[from] DiagnosticError),
    #[error("diagnostic host transaction failed: {0}")]
    Host(#[from] HostExecutionError),
    #[error("diagnostic request I/O failed: {0}")]
    Io(#[from] std::io::Error),
}

pub fn signing_message(request: &DiagnosticRequest) -> Result<Vec<u8>, DiagnosticIpcError> {
    validate_request(request)?;
    let mut bytes = SIGNATURE_DOMAIN.to_vec();
    bytes.push(0);
    bytes.extend(canonical_json(request).map_err(|_| DiagnosticIpcError::Packet)?);
    Ok(bytes)
}

fn validate_request(r: &DiagnosticRequest) -> Result<(), DiagnosticIpcError> {
    validate_identity(&r.identity).map_err(|_| DiagnosticIpcError::Packet)?;
    if r.schema_version != 1
        || !is_hash(&r.request_id)
        || !is_hash(&r.nonce_sha256)
        || r.source_commit != SOURCE_COMMIT
        || r.preset != PRESET
        || r.controller.repository_id == 0
        || r.controller.run_id == 0
        || r.controller.run_attempt == 0
        || r.controller.actor_id == 0
        || r.issued_at_unix_seconds == 0
        || r.expires_at_unix_seconds <= r.issued_at_unix_seconds
        || r.expires_at_unix_seconds - r.issued_at_unix_seconds > 60
        || (r.operation == DiagnosticOperation::Reserve) != (r.expected_state_revision == 0)
    {
        return Err(DiagnosticIpcError::Packet);
    }
    Ok(())
}

pub fn authenticate(
    packet: &[u8],
    peer: &DiagnosticPeer,
    policy: &DiagnosticPolicy,
    now_unix_seconds: u64,
) -> Result<DiagnosticRequest, DiagnosticIpcError> {
    if !policy.client_uids.contains(&peer.uid) || !peer.gids.contains(&policy.client_gid) {
        return Err(DiagnosticIpcError::Principal);
    }
    if packet.is_empty() || packet.len() > MAX_PACKET_BYTES {
        return Err(DiagnosticIpcError::Packet);
    }
    let signed: SignedDiagnosticRequest =
        serde_json::from_slice(packet).map_err(|_| DiagnosticIpcError::Packet)?;
    let message = signing_message(&signed.request)?;
    let signature = signature(&signed.signature_hex)?;
    policy
        .verifying_key
        .verify_strict(&message, &signature)
        .map_err(|_| DiagnosticIpcError::Signature)?;
    let r = signed.request;
    if r.controller.repository_id != policy.repository_id
        || !policy.actor_ids.contains(&r.controller.actor_id)
    {
        return Err(DiagnosticIpcError::Principal);
    }
    // No future-clock allowance; exclusive expiry. Wall time gates authorization
    // only. Lease correctness continues to use the coordinator's monotonic clock.
    if now_unix_seconds < r.issued_at_unix_seconds || now_unix_seconds >= r.expires_at_unix_seconds
    {
        return Err(DiagnosticIpcError::Time);
    }
    Ok(r)
}

fn signature(hex: &str) -> Result<Signature, DiagnosticIpcError> {
    if hex.len() != 128
        || !hex
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(DiagnosticIpcError::Packet);
    }
    let mut bytes = [0; 64];
    for (i, pair) in hex.as_bytes().chunks_exact(2).enumerate() {
        let nibble = |b: u8| if b <= b'9' { b - b'0' } else { b - b'a' + 10 };
        bytes[i] = (nibble(pair[0]) << 4) | nibble(pair[1]);
    }
    Ok(Signature::from_bytes(&bytes))
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    request: DiagnosticRequest,
    request_sha256: String,
    response: Option<DiagnosticResponse>,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Ledger {
    entries: Vec<Entry>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    schema_version: u32,
    ledger: Ledger,
    ledger_sha256: String,
}

fn request_hash(request: &DiagnosticRequest) -> Result<String, DiagnosticIpcError> {
    Ok(sha256_hex(
        &canonical_json(request).map_err(|_| DiagnosticIpcError::Ledger)?,
    ))
}

fn encode(ledger: &Ledger) -> Result<Vec<u8>, DiagnosticIpcError> {
    let ledger_sha256 =
        sha256_hex(&canonical_json(ledger).map_err(|_| DiagnosticIpcError::Ledger)?);
    let mut bytes = canonical_json(&Envelope {
        schema_version: 1,
        ledger: Ledger {
            entries: ledger.entries.clone(),
        },
        ledger_sha256,
    })
    .map_err(|_| DiagnosticIpcError::Ledger)?;
    bytes.push(b'\n');
    if bytes.len() as u64 > MAX_LEDGER_BYTES || ledger.entries.len() > MAX_LEDGER_ENTRIES {
        return Err(DiagnosticIpcError::Capacity);
    }
    Ok(bytes)
}

fn load(root: &Path) -> Result<Ledger, DiagnosticIpcError> {
    match fs::symlink_metadata(root.join(PENDING_NAME)) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
        Err(e) => return Err(e.into()),
        Ok(_) => return Err(DiagnosticIpcError::Uncertain),
    }
    let path = root.join(LEDGER_NAME);
    match fs::symlink_metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Ledger::default()),
        Err(e) => return Err(e.into()),
        Ok(_) => (),
    }
    let bytes =
        read_bounded_document(&path, MAX_LEDGER_BYTES).map_err(|_| DiagnosticIpcError::Ledger)?;
    let envelope: Envelope =
        serde_json::from_slice(&bytes).map_err(|_| DiagnosticIpcError::Ledger)?;
    if envelope.schema_version != 1 || encode(&envelope.ledger)? != bytes {
        return Err(DiagnosticIpcError::Ledger);
    }
    // Encode comparison independently checks the recorded digest and canonical
    // bytes. Validate entry identities/uniqueness and response shapes as well.
    for (i, entry) in envelope.ledger.entries.iter().enumerate() {
        validate_request(&entry.request).map_err(|_| DiagnosticIpcError::Ledger)?;
        if entry.request_sha256 != request_hash(&entry.request)?
            || envelope.ledger.entries[..i].iter().any(|prior| {
                prior.request.request_id == entry.request.request_id
                    || prior.request.nonce_sha256 == entry.request.nonce_sha256
            })
        {
            return Err(DiagnosticIpcError::Ledger);
        }
        if let Some(response) = &entry.response {
            if response.schema_version != 1
                || response.request_id != entry.request.request_id
                || response.lease_id != entry.request.identity.lease_id
                || response.ok != response.state.is_some()
                || response.ok != response.error_code.is_none()
                || response
                    .error_code
                    .as_deref()
                    .is_some_and(|c| c != "model_refused")
            {
                return Err(DiagnosticIpcError::Ledger);
            }
            if let Some(state) = &response.state {
                validate_state(state).map_err(|_| DiagnosticIpcError::Ledger)?;
                if state.identity != entry.request.identity {
                    return Err(DiagnosticIpcError::Ledger);
                }
            }
        }
    }
    Ok(envelope.ledger)
}

fn publish(root: &Path, ledger: &Ledger) -> Result<(), DiagnosticIpcError> {
    let bytes = encode(ledger)?;
    let pending = root.join(PENDING_NAME);
    write_new(&pending, &bytes)?;
    fs::rename(pending, root.join(LEDGER_NAME))?;
    sync_directory(root)?;
    Ok(())
}

/// Authenticate and transact against the LOCAL model. No production caller.
/// The shared fence is kept until intent, mutation and cached receipt are durable.
pub fn handle_local<B: DiagnosticBackend>(
    root: &Path,
    packet: &[u8],
    peer: &DiagnosticPeer,
    policy: &DiagnosticPolicy,
    now_unix_seconds: u64,
    clock: &DiagnosticClock,
    backend: &mut B,
) -> Result<DiagnosticResponse, DiagnosticIpcError> {
    let request = authenticate(packet, peer, policy, now_unix_seconds)?;
    let (root, _fence) = lock_host_root(root)?;
    let mut ledger = load(&root)?;
    let digest = request_hash(&request)?;
    if let Some(prior) = ledger.entries.iter().find(|e| {
        e.request.request_id == request.request_id || e.request.nonce_sha256 == request.nonce_sha256
    }) {
        if prior.request_sha256 != digest {
            return Err(DiagnosticIpcError::Replay);
        }
        return prior.response.clone().ok_or(DiagnosticIpcError::Uncertain);
    }
    if ledger.entries.iter().any(|e| e.response.is_none()) {
        return Err(DiagnosticIpcError::Uncertain);
    }
    if ledger.entries.len() >= MAX_LEDGER_ENTRIES {
        return Err(DiagnosticIpcError::Capacity);
    }
    if clock.boot_id != request.identity.boot_id {
        return Err(DiagnosticIpcError::Binding);
    }
    let prior_lease = ledger
        .entries
        .iter()
        .find(|e| e.request.identity.lease_id == request.identity.lease_id);
    let active = read_active(&root)?;
    if request.operation == DiagnosticOperation::Reserve {
        if prior_lease.is_some() || active.is_some() {
            return Err(DiagnosticIpcError::Binding);
        }
    } else {
        let first = prior_lease.ok_or(DiagnosticIpcError::Binding)?;
        if first.request.operation != DiagnosticOperation::Reserve
            || first.response.as_ref().is_none_or(|r| !r.ok)
            || first.request.controller != request.controller
            || first.request.identity != request.identity
        {
            return Err(DiagnosticIpcError::Binding);
        }
        let state = active.as_ref().ok_or(DiagnosticIpcError::Binding)?;
        if state.identity != request.identity {
            return Err(DiagnosticIpcError::Binding);
        }
        if state.revision != request.expected_state_revision
            || clock.monotonic_ns < state.last_observed_monotonic_ns
        {
            return Err(DiagnosticIpcError::Revision);
        }
    }
    ledger.entries.push(Entry {
        request: request.clone(),
        request_sha256: digest,
        response: None,
    });
    // Reserve enough room for the bounded model response BEFORE mutation. If
    // persistence fails after intent, recovery stays uncertain, never repeats.
    let mut ceiling = ledger.entries.clone();
    ceiling.last_mut().expect("just appended").response = Some(DiagnosticResponse {
        schema_version: 1,
        request_id: request.request_id.clone(),
        lease_id: request.identity.lease_id.clone(),
        ok: false,
        state: None,
        error_code: Some("model_refused".into()),
    });
    if encode(&Ledger { entries: ceiling })?.len() as u64 + 4096 > MAX_LEDGER_BYTES {
        return Err(DiagnosticIpcError::Capacity);
    }
    publish(&root, &ledger)?;
    let result = match request.operation {
        DiagnosticOperation::Reserve => {
            DiagnosticCoordinator::reserve_fenced(&root, request.identity.clone(), clock)
                .and_then(|_| read_active(&root)?.ok_or(DiagnosticError::Invalid))
        }
        DiagnosticOperation::Status => active.ok_or(DiagnosticError::Invalid),
        DiagnosticOperation::Heartbeat => {
            DiagnosticCoordinator::recover_fenced(&root, &request.identity.lease_id)
                .and_then(|c| c.heartbeat_fenced(clock))
        }
        DiagnosticOperation::Cancel => {
            DiagnosticCoordinator::recover_fenced(&root, &request.identity.lease_id)
                .and_then(|c| c.drive_fenced(clock, backend, true))
        }
    };
    let response = DiagnosticResponse {
        schema_version: 1,
        request_id: request.request_id,
        lease_id: request.identity.lease_id,
        ok: result.is_ok(),
        state: result.ok(),
        error_code: None,
    };
    let response = DiagnosticResponse {
        error_code: (!response.ok).then(|| "model_refused".into()),
        ..response
    };
    ledger.entries.last_mut().expect("just appended").response = Some(response.clone());
    publish(&root, &ledger)?;
    Ok(response)
}

/// One local fixture connection. Production server dispatch is unchanged.
#[cfg(target_os = "linux")]
pub fn handle_local_connection<B: DiagnosticBackend>(
    root: &Path,
    connection: &crate::unix_transport::SeqpacketConnection,
    policy: &DiagnosticPolicy,
    now_unix_seconds: u64,
    clock: &DiagnosticClock,
    backend: &mut B,
) -> Result<DiagnosticResponse, DiagnosticIpcError> {
    let credentials = connection
        .peer_credentials()
        .map_err(|_| DiagnosticIpcError::Principal)?;
    let mut gids = credentials.supplemental_gids;
    gids.push(credentials.gid);
    let packet = connection
        .receive_packet_timeout(std::time::Duration::from_secs(1))
        .map_err(|_| DiagnosticIpcError::Packet)?
        .ok_or(DiagnosticIpcError::Packet)?;
    let response = handle_local(
        root,
        &packet,
        &DiagnosticPeer {
            uid: credentials.uid,
            gids,
        },
        policy,
        now_unix_seconds,
        clock,
        backend,
    )?;
    let bytes = canonical_json(&response).map_err(|_| DiagnosticIpcError::Packet)?;
    connection
        .send_packet(&bytes)
        .map_err(|_| DiagnosticIpcError::Packet)?;
    Ok(response)
}
