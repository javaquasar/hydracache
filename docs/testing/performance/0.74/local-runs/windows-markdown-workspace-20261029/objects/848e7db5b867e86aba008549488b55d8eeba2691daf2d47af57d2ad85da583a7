//! Bounded supplied-byte inspection, not filesystem history or lifecycle proof.
use super::{CheckedIntentProjection, IntentCheck, IntentProjectionError};
use crate::diagnostic_ipc::DiagnosticPeer;
use crate::diagnostic_lease::{DiagnosticStage, DiagnosticState};
use crate::diagnostic_output_preparation::intent_journal::{
    marker_bytes, record_bytes, Record, MAX_CELLS, MAX_RECORD_BYTES, RECORD_SCHEMA,
};
use crate::sha256_hex;
use std::collections::BTreeSet;
use thiserror::Error;

/// Borrowed assertions only. No descriptor identity or commit acknowledgement.
pub struct RetainedIntentPair<'a> {
    pub record: &'a [u8],
    pub commit: &'a [u8],
}

/// Copyable inspection facts, never permission to append, replay or execute.
///
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::intent_projection::signed_history::SignedHistoryObservation;
/// fn start(view: SignedHistoryObservation) { view.start_once(); }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedHistoryObservation {
    pub retained_cells: usize,
    pub anchor_sha256: String,
    pub last_record_sha256: Option<String>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SignedHistoryError {
    #[error("supplied record is malformed, noncanonical or over budget")]
    Document,
    #[error("supplied history conflicts with its anchor, cell or Reserved projections")]
    History,
    #[error("supplied commit is partial or does not match its exact record")]
    Uncertain,
    #[error("retained request ID or nonce is duplicated")]
    Replay,
    #[error("original signed intent refused: {0}")]
    Intent(#[from] IntentProjectionError),
}

/// Inspect a contiguous prefix anchored to supplied initial-attempt bytes.
/// Signatures and necessary revision/clock bounds do not prove omitted state
/// transitions, cell outcomes, durability or original-object ownership.
pub fn inspect_signed_history(
    original: &mut CheckedIntentProjection<'_>,
    current_bytes: &[u8],
    check: &IntentCheck<'_>,
    anchor_bytes: &[u8],
    pairs: &[RetainedIntentPair<'_>],
) -> Result<SignedHistoryObservation, SignedHistoryError> {
    let result = inspect(original, current_bytes, check, anchor_bytes, pairs);
    if result.is_err() {
        original.latch_refusal();
    }
    result
}

fn decode(raw: &[u8]) -> Result<Record, SignedHistoryError> {
    if raw.is_empty() || raw.len() > MAX_RECORD_BYTES {
        return Err(SignedHistoryError::Document);
    }
    let record: Record = serde_json::from_slice(raw).map_err(|_| SignedHistoryError::Document)?;
    if record.schema_version != RECORD_SCHEMA
        || record_bytes(&record).map_err(|_| SignedHistoryError::Document)? != raw
    {
        return Err(SignedHistoryError::Document);
    }
    Ok(record)
}

// A necessary lower bound only. Heartbeats may add revisions; absence of their
// transition trace must never be interpreted as successful cell completion.
fn plausible(previous: &DiagnosticState, next: &DiagnosticState) -> bool {
    if previous.identity != next.identity
        || previous.reserved_monotonic_ns != next.reserved_monotonic_ns
        || next.completed_cells < previous.completed_cells
        || next.last_observed_monotonic_ns < previous.last_observed_monotonic_ns
        || next.controller_monotonic_ns < previous.controller_monotonic_ns
    {
        return false;
    }
    let cells = next.completed_cells - previous.completed_cells;
    let Some(minimum) = previous.revision.checked_add(cells as u64 * 3) else {
        return false;
    };
    next.revision >= minimum
}

fn verify_record(
    original: &mut CheckedIntentProjection<'_>,
    record: &Record,
    check: &IntentCheck<'_>,
) -> Result<super::IntentProjectionBody, SignedHistoryError> {
    let body = original.verify_consumed(
        record.signed_intent.as_bytes(),
        &record.state,
        &DiagnosticPeer {
            uid: record.peer_uid,
            gids: record.peer_gids.clone(),
        },
        check,
    )?;
    if body.issued_at_unix_seconds > check.now_unix_seconds {
        return Err(IntentProjectionError::Time.into());
    }
    Ok(body)
}

fn inspect(
    original: &mut CheckedIntentProjection<'_>,
    current_bytes: &[u8],
    check: &IntentCheck<'_>,
    anchor_bytes: &[u8],
    pairs: &[RetainedIntentPair<'_>],
) -> Result<SignedHistoryObservation, SignedHistoryError> {
    original.revalidate(current_bytes, check)?;
    if pairs.len() > MAX_CELLS {
        return Err(SignedHistoryError::History);
    }
    let anchor = decode(anchor_bytes)?;
    verify_record(original, &anchor, check)?;
    if anchor.state.stage != DiagnosticStage::Reserved
        || anchor.state.revision != 1
        || anchor.state.completed_cells != 0
        || anchor.previous_record_sha256.is_some()
        || !plausible(&anchor.state, check.state)
    {
        return Err(SignedHistoryError::History);
    }
    if pairs.is_empty() {
        if check.state != &anchor.state {
            return Err(SignedHistoryError::History);
        }
    } else if !(pairs.len() == check.state.completed_cells
        || pairs.len() == check.state.completed_cells + 1)
        || pairs[0].record != anchor_bytes
    {
        return Err(SignedHistoryError::History);
    }
    let mut previous_state = anchor.state.clone();
    let mut previous_hash = None;
    let mut requests = BTreeSet::new();
    let mut nonces = BTreeSet::new();
    for (cell, pair) in pairs.iter().enumerate() {
        let record = decode(pair.record)?;
        if pair.commit != marker_bytes(pair.record) {
            return Err(SignedHistoryError::Uncertain);
        }
        if record.state.completed_cells != cell
            || record.previous_record_sha256 != previous_hash
            || !plausible(&previous_state, &record.state)
            || !plausible(&record.state, check.state)
        {
            return Err(SignedHistoryError::History);
        }
        let body = verify_record(original, &record, check)?;
        if !requests.insert(body.request_id) || !nonces.insert(body.nonce_sha256) {
            return Err(SignedHistoryError::Replay);
        }
        previous_hash = Some(sha256_hex(pair.record));
        previous_state = record.state;
    }
    original.revalidate(current_bytes, check)?;
    Ok(SignedHistoryObservation {
        retained_cells: pairs.len(),
        anchor_sha256: sha256_hex(anchor_bytes),
        last_record_sha256: previous_hash,
    })
}
